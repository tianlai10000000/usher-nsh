#!/usr/bin/env python3
"""Decides whether a release changed the app's shell, and guards the list that defines it.

A signed code package (.nshpkg) holds only the app's TSX. Rust code, native adapters,
permissions and anything embedded into the binary live in the installed app, so a release that
touches them must ship as a whole-app update, never as a code package (docs/update.md, 4.2).

Subcommands:

  changed [--version V | --base REF] [--strict] [--root DIR]
      Prints `shell_changed=true|false` and `previous_tag=`: whether any shell input differs
      between the previous release tag and the working tree. With `--version` the previous tag
      is the highest formal `v<major>.<minor>.<patch>` tag below V; `--base` names it directly.
      With no previous release there is nothing to compare, so the shell counts as changed and
      the release is whole-app only.

      A changed shell must also raise the major or minor version. The client only runs a code
      package whose major.minor equals the installed shell's (docs/update.md, 3), so a shell
      change that kept the version line would hand the new code to old shells. `--strict` (a
      release) fails on that; without it (a rehearsal) it is a warning.
  check-embedded [--root DIR]
      Fails when Rust embeds a file that scripts/release/shell-inputs.txt does not cover, so a
      newly embedded resource cannot slip through as a "TSX only" release.

The input list is read from shell-inputs.txt; no path is written here. Everything is a plain
function of its inputs, so the tests need no network and, for `changed`, only a throwaway Git
repository.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

INPUT_LIST = Path(__file__).resolve().parent / "shell-inputs.txt"
# nsh.json changes its version and build on every release; those two alone are not a shell change.
IGNORED_NSH_JSON_FIELDS = ("version", "build")
# `include_str!("..")`, `include_bytes!("..")` and `include!("..")` with a literal path.
EMBED = re.compile(r'\binclude(?:_str|_bytes)?!\s*\(\s*"([^"]+)"')
# Rust files that only embed test fixtures; a fixture is not part of the shipped binary.
TEST_ONLY_MARKER = re.compile(r"#\[cfg\(test\)\]")


FORMAL_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


class ShellInputsError(Exception):
    pass


def version_key(version: str) -> tuple[int, int, int]:
    """`1.2.3` (a pre-release or build suffix is ignored) as numbers, so 0.10.0 sorts above 0.9.0."""
    core = re.split(r"[-+]", version, maxsplit=1)[0]
    parts = core.split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        raise ShellInputsError(f"{version!r} is not a version.")
    return int(parts[0]), int(parts[1]), int(parts[2])


def previous_release_tag(tags: list[str], version: str) -> str | None:
    """The highest formal release tag strictly below `version`. Pre-release and malformed tags
    are not releases (docs/update.md, D7) and are skipped."""
    current = version_key(version)
    older = [
        (tuple(int(g) for g in match.groups()), tag)
        for tag in tags
        if (match := FORMAL_TAG.match(tag)) and tuple(int(g) for g in match.groups()) < current
    ]
    return max(older)[1] if older else None


def epoch_problem(previous_tag: str, version: str) -> str | None:
    """A message when a shell change did not move the major.minor line, else None."""
    before, after = version_key(previous_tag.removeprefix("v")), version_key(version)
    if after[:2] > before[:2]:
        return None
    return (
        f"The shell changed since {previous_tag}, but {version} stays on the {before[0]}.{before[1]} line. "
        "Raise the major or minor version: a code package is only used by a shell on the same line."
    )


def read_inputs(path: Path = INPUT_LIST) -> list[str]:
    entries = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            entries.append(line.rstrip("/"))
    if not entries:
        raise ShellInputsError(f"{path} lists no shell inputs.")
    return entries


def covered(path: str, inputs: list[str]) -> bool:
    """Whether `path` (repository-relative, forward slashes) is one of the inputs or under one."""
    return any(path == entry or path.startswith(entry + "/") for entry in inputs)


def normalized_nsh_json(text: str) -> str:
    data = json.loads(text)
    for field in IGNORED_NSH_JSON_FIELDS:
        data.pop(field, None)
    return json.dumps(data, sort_keys=True)


def git(root: Path, *arguments: str) -> subprocess.CompletedProcess:
    # --no-replace-objects: a replace ref must not change what the history says.
    return subprocess.run(
        ["git", "--no-replace-objects", "-C", str(root), *arguments], capture_output=True, text=True, timeout=120
    )


def changed_files(root: Path, base: str, inputs: list[str]) -> list[str]:
    """Shell input files that differ between `base` and the working tree."""
    result = git(root, "rev-parse", "--verify", "--end-of-options", base + "^{commit}")
    if result.returncode != 0:
        raise ShellInputsError(f"{base!r} is not a commit.")
    diff = git(root, "diff", "--name-only", "--no-renames", "-z", base, "--", *inputs)
    if diff.returncode != 0:
        raise ShellInputsError("git diff failed: " + diff.stderr.strip())
    files = [name for name in diff.stdout.split("\0") if name]
    # nsh.json alone does not count when only its version and build moved.
    if "nsh.json" in files:
        previous = git(root, "show", f"{base}:nsh.json")
        if previous.returncode == 0:
            try:
                same = normalized_nsh_json(previous.stdout) == normalized_nsh_json((root / "nsh.json").read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError) as error:
                raise ShellInputsError(f"nsh.json cannot be compared: {error}.") from error
            if same:
                files.remove("nsh.json")
    # Untracked files are shell inputs too once they are added; a release is built from a commit,
    # so the working tree here equals HEAD and `git diff` against `base` is the whole answer.
    return sorted(files)


def embedded_files(root: Path) -> list[tuple[str, str]]:
    """(source file, embedded repository-relative path) for every literal include in shipped Rust.

    Files whose embedding sits in a `#[cfg(test)]` block are skipped: they read fixtures, and
    fixtures are not in the binary.
    """
    found = []
    for directory in ("src", "platforms", "crates"):
        for source in sorted((root / directory).rglob("*.rs")) if (root / directory).exists() else []:
            text = source.read_text(encoding="utf-8")
            test_start = TEST_ONLY_MARKER.search(text)
            production = text[: test_start.start()] if test_start else text
            for match in EMBED.finditer(production):
                target = (source.parent / match.group(1)).resolve()
                try:
                    relative = target.relative_to(root.resolve())
                except ValueError:
                    continue  # an OUT_DIR path or something outside the repository
                found.append((str(source.relative_to(root)), relative.as_posix()))
    build = root / "build.rs"
    if build.exists():
        for match in EMBED.finditer(build.read_text(encoding="utf-8")):
            target = (root / match.group(1)).resolve()
            try:
                found.append(("build.rs", target.relative_to(root.resolve()).as_posix()))
            except ValueError:
                pass
    return found


def check_embedded(root: Path, inputs: list[str]) -> list[str]:
    return [
        f"{source} embeds {path}, which is not covered by scripts/release/shell-inputs.txt."
        for source, path in embedded_files(root)
        if not covered(path, inputs)
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    changed = sub.add_parser("changed")
    changed.add_argument("--base", default="", help="the previous release tag; empty when there is none")
    changed.add_argument("--version", default="", help="the version being released; finds the previous formal tag itself")
    changed.add_argument("--strict", action="store_true", help="a release: a shell change without a minor bump is an error")
    changed.add_argument("--root", type=Path, default=Path("."))
    embedded = sub.add_parser("check-embedded")
    embedded.add_argument("--root", type=Path, default=Path("."))
    args = parser.parse_args(argv)
    try:
        inputs = read_inputs()
        if args.command == "changed":
            base = args.base
            if not base and args.version:
                listed = git(args.root, "tag", "--list", "v*")
                if listed.returncode != 0:
                    raise ShellInputsError("git tag failed: " + listed.stderr.strip())
                base = previous_release_tag(listed.stdout.split(), args.version) or ""
            if not base:
                print("::notice::No previous release: the release is whole-app only.", file=sys.stderr)
                print("previous_tag=")
                print("shell_changed=true")
                return 0
            files = changed_files(args.root, base, inputs)
            for name in files[:20]:
                print(f"::notice::Shell input changed: {name}", file=sys.stderr)
            print(f"previous_tag={base}")
            print(f"shell_changed={'true' if files else 'false'}")
            if files and args.version:
                message = epoch_problem(base, args.version)
                if message:
                    level = "error" if args.strict else "warning"
                    print(f"::{level}::{message}", file=sys.stderr)
                    return 1 if args.strict else 0
            return 0
        problems = check_embedded(args.root, inputs)
        for problem in problems:
            print(f"::error::{problem}", file=sys.stderr)
        return 1 if problems else 0
    except (ShellInputsError, OSError) as error:
        print(f"::error::{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
