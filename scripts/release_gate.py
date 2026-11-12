#!/usr/bin/env python3
"""Decide whether a run may build, and what it is: a release or a rehearsal.

A pushed tag is a release. A manual run is a rehearsal: the whole build chain runs, so the
CD can be tested at any time, but nothing is signed with the real key and nothing is
published. A rehearsal still refuses a build that would be meaningless (the version differs
between the files, or the build number is invalid). What only matters to publishing (a
missing changelog section, a build number that is not higher) is reported as a warning
together with `release_ready=false`, so the rehearsal answers "would a tag be accepted?"
without failing.

Checks, in this order (releases only, unless noted):

1. the event is a tag push, and the tag is `v<semver>` and *new* (not moved, not deleted);
2. the tagged commit is on the default branch;
3. the version agrees in the tag, `nsh.json`, and the root `Cargo.toml`  (both modes);
4. the build number is a positive integer (both modes) and higher than the last release's;
5. `CHANGELOG.md` has a section for exactly this version, with content;
6. `nsh.json` has an `updater` that points at this repository's releases and names the keys
   it trusts (releases only). Without it the packaged app could never update itself, and
   nothing would say so until a user asked.

Only formal versions (`v<major>.<minor>.<patch>`) are released: there is one update channel,
and the updater reads the latest non-pre-release GitHub release (docs/update.md, D7). A tag
with a pre-release suffix is refused, so no pre-release can be published by mistake. A
rehearsal may still run on a pre-release version: it builds and publishes nothing.

On success it prints `key=value` lines for `$GITHUB_OUTPUT`: version, build, rehearsal,
release_ready, and writes the notes to the file named by `--notes`. Warnings go to stderr as
`::warning::`.

Everything is a plain function of its inputs, so the tests need no Git and no network.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)
ZERO = "0" * 40


class GateError(Exception):
    pass


def parse_tag(tag: str) -> str:
    """`v1.2.3` -> `1.2.3`. Anything else is refused, pre-release suffixes included."""
    if not tag.startswith("v") or not SEMVER.match(tag[1:]):
        raise GateError(f"{tag!r} is not a release tag: expected v<major>.<minor>.<patch>.")
    return tag[1:]


def is_prerelease(version: str) -> bool:
    return bool(SEMVER.match(version).group(4))


def require_formal(version: str) -> None:
    """A release has one channel, the formal one. GitHub's `latest` skips pre-releases, so a
    pre-release could not be reached by the updater, and publishing one would only mislead."""
    if is_prerelease(version):
        raise GateError(
            f"{version} is a pre-release version. Only formal versions are released: "
            "there is no preview channel."
        )


def check_event(event: dict, ref: str) -> None:
    """A release is a newly created tag. Moving or re-pushing one would replace the
    commit that an earlier release was built from."""
    if event.get("ref") != ref:
        raise GateError("The event's ref does not match the checked-out ref.")
    if event.get("created") is not True or event.get("deleted") is not False:
        raise GateError("A release needs a newly created tag.")
    if event.get("forced") is not False or event.get("before") != ZERO:
        raise GateError("A release needs a new tag, not an updated one.")


def read_nsh_json(path: Path) -> tuple[str, int]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise GateError(f"Cannot read {path}: {error}") from error
    version, build = data.get("version"), data.get("build")
    if not isinstance(version, str):
        raise GateError(f"{path} has no version.")
    # bool is an int in Python; `true` is not a build number.
    if isinstance(build, bool) or not isinstance(build, int) or build < 1:
        raise GateError(f"{path} needs a positive integer build number.")
    return version, build


def read_cargo_version(path: Path) -> str:
    """The `[package]` version of a Cargo.toml, without a TOML library on old Pythons."""
    in_package = False
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
            continue
        match = in_package and re.match(r'version\s*=\s*"([^"]+)"\s*(?:#.*)?$', stripped)
        if match:
            return match.group(1)
    raise GateError(f"{path} has no [package] version.")


def changelog_section(text: str, version: str) -> str:
    """The body under `## [version]` (or `## version`), up to the next `## ` heading."""
    # Horizontal whitespace only: `\s` would run over the newline into the section body.
    heading = re.compile(rf"^##[ \t]+\[?{re.escape(version)}\]?(?:[ \t]+[-–(].*)?[ \t]*$", re.MULTILINE)
    match = heading.search(text)
    if not match:
        raise GateError(f"CHANGELOG.md has no section for {version}.")
    rest = text[match.end():]
    nxt = re.search(r"^##\s", rest, re.MULTILINE)
    body = (rest[: nxt.start()] if nxt else rest).strip()
    if not body:
        raise GateError(f"The CHANGELOG.md section for {version} is empty.")
    return body


def updater_problems(root: Path, repository: str) -> list[str]:
    """What is wrong with `nsh.json`'s `updater`, in the terms of docs/update.md section 4.3.

    The keys themselves are public and belong in the repository, where a change to the trust
    root is a reviewable diff. Whether they are the right keys is not something a gate can know.
    """
    try:
        data = json.loads((root / "nsh.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return []  # read_nsh_json reports an unreadable nsh.json, once
    updater = data.get("updater")
    if not isinstance(updater, dict):
        return ["nsh.json has no \"updater\": the packaged app could never update itself."]
    problems = []
    github = updater.get("github")
    if github is None:
        problems.append("nsh.json updater.github is not set: the update source must be this repository's releases.")
    elif repository and github != repository:
        problems.append(f"nsh.json updater.github is {github!r}, this repository is {repository!r}.")
    if "url" in updater:
        problems.append("nsh.json updater.url must not be set: there is one update source, updater.github.")
    if not isinstance(updater.get("publicKey"), str) or not updater["publicKey"]:
        problems.append("nsh.json updater.publicKey is missing: manifests would not be checked.")
    packages = updater.get("packages")
    roots = packages.get("roots") if isinstance(packages, dict) else None
    if not isinstance(roots, list) or not roots or not all(isinstance(r, str) and r for r in roots):
        problems.append("nsh.json updater.packages.roots names no root key: code packages would not be accepted.")
    return problems


def decide(
    tag: str,
    root: Path,
    *,
    previous_build: int | None,
    strict: bool = True,
    repository: str = "",
) -> dict:
    """What a run with this `tag` is. `strict` is a release: every problem is fatal. Not
    strict is a rehearsal: only problems that make the build itself meaningless are fatal,
    the rest come back in `warnings`."""
    version = parse_tag(tag)
    if strict:
        require_formal(version)
    nsh_version, build = read_nsh_json(root / "nsh.json")
    cargo_version = read_cargo_version(root / "Cargo.toml")
    if not (version == nsh_version == cargo_version):
        raise GateError(
            "The version differs between the tag and the files: "
            f"tag {version}, nsh.json {nsh_version}, Cargo.toml {cargo_version}."
        )
    warnings: list[str] = []
    if not strict and is_prerelease(version):
        # A rehearsal builds a pre-release version fine; only a real release refuses it.
        warnings.append(f"{version} is a pre-release version and would be refused: only formal versions are released.")

    def soft(message: str) -> None:
        if strict:
            raise GateError(message)
        warnings.append(message)

    for problem in updater_problems(root, repository):
        soft(problem)
    if previous_build is not None and build <= previous_build:
        soft(f"nsh.json build {build} must be higher than the previous release's {previous_build}.")
    try:
        notes = changelog_section((root / "CHANGELOG.md").read_text(encoding="utf-8"), version)
    except GateError as error:
        soft(str(error))
        notes = f"Rehearsal build of {version}: CHANGELOG.md has no notes for it yet."
    except OSError as error:
        soft(f"Cannot read CHANGELOG.md: {error}.")
        notes = f"Rehearsal build of {version}: CHANGELOG.md has no notes for it yet."
    return {
        "version": version,
        "build": str(build),
        "notes": notes,
        "warnings": warnings,
    }


def git(*arguments: str) -> str:
    # --no-replace-objects: a replace ref must not change what the history says.
    result = subprocess.run(
        ["git", "--no-replace-objects", *arguments], capture_output=True, text=True, timeout=60
    )
    if result.returncode != 0:
        raise GateError("Git verification failed: " + " ".join(arguments))
    return result.stdout.strip()


def check_on_default_branch(ref: str, default_branch: str) -> None:
    commit = git("rev-parse", "--verify", "--end-of-options", ref + "^{commit}")
    if git("rev-parse", "HEAD") != commit:
        raise GateError("The checkout is not the tagged commit.")
    remote = f"refs/remotes/origin/{default_branch}"
    git("rev-parse", "--verify", remote + "^{commit}")
    result = subprocess.run(
        ["git", "--no-replace-objects", "merge-base", "--is-ancestor", commit, remote],
        capture_output=True,
        timeout=60,
    )
    if result.returncode != 0:
        raise GateError(f"The tagged commit is not on {default_branch}.")


RECENT_RELEASES = 20


def highest_build(releases: list[dict], fetch) -> int | None:
    """The highest build among published releases' `build.txt` assets, pre-releases
    included (`/releases/latest` skips them). Only formal versions are released now, but earlier
    pre-releases may exist and still count for the build number.
    Drafts are ignored. A `build.txt` that cannot be read is an error, never "none":
    guessing would let a lower build number through."""
    best = None
    for release in releases[:RECENT_RELEASES]:
        if release.get("draft"):
            continue
        for asset in release.get("assets", []):
            if asset.get("name") != "build.txt":
                continue
            try:
                value = int(fetch(asset["browser_download_url"]).decode().strip())
            except (ValueError, OSError) as error:
                raise GateError(f"A previous release's build.txt is unreadable: {error}.") from error
            if value < 1:
                raise GateError("A previous release's build.txt holds an invalid build number.")
            best = value if best is None else max(best, value)
    return best


def previous_release_build(repository: str, token: str | None) -> int | None:
    import urllib.error
    import urllib.request

    def fetch(url: str) -> bytes:
        request = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
        if token:
            request.add_header("Authorization", f"Bearer {token}")
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.read()

    try:
        releases = json.loads(
            fetch(f"https://api.github.com/repos/{repository}/releases?per_page={RECENT_RELEASES}")
        )
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise GateError(f"Cannot list releases: HTTP {error.code}.") from error
    except (OSError, ValueError) as error:
        raise GateError(f"Cannot list releases: {error}.") from error
    if not isinstance(releases, list):
        raise GateError("The releases listing has an unexpected shape.")
    return highest_build(releases, fetch)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--notes", type=Path, help="where to write the notes")
    parser.add_argument("--skip-git", action="store_true", help="skip the tag and branch checks (local runs)")
    args = parser.parse_args(argv)
    try:
        ref = os.environ.get("GITHUB_REF", "")
        event_name = os.environ.get("GITHUB_EVENT_NAME", "")
        if event_name == "workflow_dispatch":
            # A rehearsal builds whatever commit was chosen. No tag is needed, so the version
            # is the one the files say, and the rules that only matter to publishing relax.
            rehearsal = True
            tag = "v" + read_nsh_json(args.root / "nsh.json")[0]
        elif event_name == "push" and ref.startswith("refs/tags/"):
            rehearsal = False
            tag = ref.removeprefix("refs/tags/")
            if not args.skip_git:
                event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
                check_event(event, ref)
                check_on_default_branch(ref, os.environ.get("DEFAULT_BRANCH", "main"))
        else:
            raise GateError("A run is either a pushed tag or a manual rehearsal.")
        repository = os.environ.get("GITHUB_REPOSITORY", "")
        previous = (
            previous_release_build(repository, os.environ.get("GITHUB_TOKEN"))
            if repository
            else None
        )
        result = decide(tag, args.root, previous_build=previous, strict=not rehearsal, repository=repository)
    except (GateError, OSError, KeyError, ValueError, subprocess.TimeoutExpired) as error:
        message = str(error) if isinstance(error, GateError) else "Cannot verify the run."
        print(f"::error::{message}", file=sys.stderr)
        return 1
    notes = result.pop("notes")
    warnings = result.pop("warnings")
    for warning in warnings:
        print(f"::warning::A tag push would be refused: {warning}", file=sys.stderr)
    if args.notes:
        args.notes.write_text(notes + "\n", encoding="utf-8")
    result["rehearsal"] = "true" if rehearsal else "false"
    result["release_ready"] = "false" if warnings else "true"
    for key, value in result.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
