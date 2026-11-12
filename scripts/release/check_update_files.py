#!/usr/bin/env python3
"""Checks the update files of a release against the project, before they are published.

`nsh update-manifest` writes `latest.json` and signs it. This script is the second opinion: it
reads what is about to be published and refuses anything that would make the updater offer a
wrong or half-built update (docs/update.md, section 4.3). It runs in the publish job and in the
rehearsal, on the same files.

What it checks:
  * `latest.json` and a non-empty `latest.json.sig` exist (an unsigned manifest must never go out);
  * the manifest is for this app and this version;
  * the whole-app archive it lists is the file next to it, with the same SHA-256 and size;
  * a code package is listed exactly when the release is not whole-app only, with this build, and
    the file next to it has the SHA-256 and size listed;
  * every URL is a bare file name next to the manifest, never another host or a path.

What it cannot check: the manifest's signature (no Ed25519 verifier is available to the build; the
signing step verifies its own key) and the package's signature (`nsh verify` does that).

Everything is a plain function of the files in the directory, so the tests need no `nsh`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

MANIFEST = "latest.json"
SIGNATURE = "latest.json.sig"
# The one platform a release builds (docs/update.md section 1).
PLATFORM = "macos-aarch64"


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def is_bare_name(url: object) -> bool:
    """A file name next to the manifest: no scheme, no directory, no traversal."""
    return isinstance(url, str) and bool(url) and "/" not in url and "\\" not in url and ":" not in url and url not in (".", "..")


def check_entry(directory: Path, label: str, entry: object) -> list[str]:
    """The file an entry names exists next to the manifest and matches its hash and size."""
    if not isinstance(entry, dict):
        return [f"{label} is not an object."]
    problems: list[str] = []
    url = entry.get("url")
    if not is_bare_name(url):
        return [f"{label} url {url!r} must be a file name next to the manifest."]
    path = directory / url
    if not path.is_file():
        return [f"{label} names {url}, which is not in the release."]
    if entry.get("sha256") != sha256_of(path):
        problems.append(f"{label} sha256 does not match {url}.")
    if entry.get("size") != path.stat().st_size:
        problems.append(f"{label} size does not match {url}.")
    return problems


def check_files(directory: Path, *, app_id: str, version: str, build: int, shell_changed: bool) -> list[str]:
    """Every problem with the update files in `directory`; an empty list means they are consistent."""
    manifest_path = directory / MANIFEST
    signature_path = directory / SIGNATURE
    if not manifest_path.is_file():
        return [f"{MANIFEST} is missing."]
    problems: list[str] = []
    if not signature_path.is_file() or signature_path.stat().st_size == 0:
        problems.append(f"{SIGNATURE} is missing or empty: an unsigned manifest must not be published.")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return problems + [f"{MANIFEST} cannot be read: {error}."]
    if not isinstance(manifest, dict):
        return problems + [f"{MANIFEST} is not an object."]

    if manifest.get("id") != app_id:
        problems.append(f"The manifest is for {manifest.get('id')!r}, this app is {app_id!r}.")
    if manifest.get("version") != version:
        problems.append(f"The manifest offers {manifest.get('version')!r}, this release is {version!r}.")

    platforms = manifest.get("platforms")
    if not isinstance(platforms, dict) or set(platforms) != {PLATFORM}:
        problems.append(f"The manifest must list exactly the {PLATFORM} archive, not {sorted(platforms) if isinstance(platforms, dict) else platforms!r}.")
    else:
        problems += check_entry(directory, f"platforms.{PLATFORM}", platforms[PLATFORM])

    package = manifest.get("package")
    packages = sorted(path.name for path in directory.glob("*.nshpkg"))
    if shell_changed:
        # The shell changed: a code package built for the new shell would run on old shells too.
        if package is not None:
            problems.append("The shell changed, so this release is whole-app only, but the manifest lists a code package.")
        if packages:
            problems.append(f"The shell changed, so no code package may be released, but {packages} is present.")
    else:
        if package is None:
            problems.append("The shell did not change, so the release must list a code package, but the manifest has none.")
        else:
            problems += check_entry(directory, "package", package)
            if isinstance(package, dict):
                if package.get("build") != build:
                    problems.append(f"The package build is {package.get('build')!r}, this release is build {build}.")
                if package.get("version") != version:
                    problems.append(f"The package version is {package.get('version')!r}, this release is {version!r}.")
                if not package.get("minRuntime"):
                    problems.append("The package has no minRuntime: older runtimes would be offered a package they cannot run.")
        if len(packages) != 1:
            problems.append(f"Exactly one .nshpkg must be released, found {packages}.")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("directory", type=Path)
    parser.add_argument("--entry", type=Path, default=Path("."), help="the directory whose nsh.json has the id, version and build")
    parser.add_argument("--shell-changed", choices=["true", "false"], required=True)
    args = parser.parse_args(argv)
    try:
        nsh = json.loads((args.entry / "nsh.json").read_text(encoding="utf-8"))
        problems = check_files(
            args.directory,
            app_id=nsh["id"],
            version=nsh["version"],
            build=nsh["build"],
            shell_changed=args.shell_changed == "true",
        )
    except (OSError, ValueError, KeyError) as error:
        print(f"::error::Cannot check the update files: {error}", file=sys.stderr)
        return 1
    for problem in problems:
        print(f"::error::{problem}", file=sys.stderr)
    if not problems:
        print(f"update files consistent ({'whole-app only' if args.shell_changed == 'true' else 'with a code package'})")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
