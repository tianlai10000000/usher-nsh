#!/usr/bin/env bash
# Build the update files of a release: the code package, the manifest and its signature.
#
# Usage: make-update-files.sh ARCHIVE_DIR OUT_DIR SHELL_CHANGED
#
# Environment (file paths, never key contents on a command line):
#   MANIFEST_KEY   the manifest key's secret file (`nsh keygen`'s .secret); signs latest.json
#   RELEASE_KEY    the release key's secret file; signs the code package
#   RELEASE_CERT   the release key's certificate (`nsh certify`)
#   PROJECT        the directory whose nsh.json describes this release (default: .)
#
# ARCHIVE_DIR holds the whole-app archive `nsh package` wrote, under the name it chose:
# <name>-<version>-macos-aarch64.tar.gz. That name is not ours to change: `nsh update-manifest`
# reads the version and platform from it. OUT_DIR receives the archive, latest.json,
# latest.json.sig and, when SHELL_CHANGED is false, the .nshpkg. Nothing here knows which keys
# are real: the publish job passes the release keys, the rehearsal throwaway ones.
#
# A manifest is only trusted if it is signed, so this script fails loudly and leaves no
# manifest behind rather than let a half-built release through: `nsh update-manifest` writes
# latest.json before it checks the key, and a wrong key leaves it unsigned (observed with
# nsh 0.5.0), hence the cleanup on failure and the check that the signature exists.
set -euo pipefail

archives="${1:?usage: make-update-files.sh ARCHIVE_DIR OUT_DIR SHELL_CHANGED}"
out="${2:?usage: make-update-files.sh ARCHIVE_DIR OUT_DIR SHELL_CHANGED}"
shell_changed="${3:?usage: make-update-files.sh ARCHIVE_DIR OUT_DIR SHELL_CHANGED}"
project="${PROJECT:-.}"

[[ "$shell_changed" == "true" || "$shell_changed" == "false" ]] || { echo "::error::SHELL_CHANGED must be true or false, not '$shell_changed'" >&2; exit 1; }
[[ -d "$archives" ]] || { echo "::error::$archives is not a directory" >&2; exit 1; }
for name in MANIFEST_KEY RELEASE_KEY RELEASE_CERT; do
    [[ -n "${!name:-}" ]] || { echo "::error::$name is not set" >&2; exit 1; }
    [[ -f "${!name}" ]] || { echo "::error::$name does not name a file" >&2; exit 1; }
done

shopt -s nullglob
found=("$archives"/*-macos-aarch64.tar.gz)
if [[ ${#found[@]} -ne 1 ]]; then
    echo "::error::Expected exactly one *-macos-aarch64.tar.gz in $archives, found ${#found[@]}" >&2
    exit 1
fi
archive="${found[0]}"

mkdir -p "$out"
rm -f "$out"/latest.json "$out"/latest.json.sig "$out"/*.nshpkg
cp "$archive" "$out"/

# A failure from here on must not leave a manifest that looks publishable.
trap 'rm -f "$out"/latest.json "$out"/latest.json.sig' ERR

args=(update-manifest --entry "$project" --key "$MANIFEST_KEY" -o "$out")
if [[ -f "${NOTES:-}" ]]; then args+=(--notes "$NOTES"); fi

if [[ "$shell_changed" == "false" ]]; then
    # The code package, signed by the release key; the app's roots trust its certificate.
    (cd "$project" && nsh publish --key "$RELEASE_KEY" --cert "$RELEASE_CERT" -o "$out/package.nshpkg.tmp")
    # Named by the package's own identity so a leftover cannot be mistaken for this release's.
    id="$(python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(d['id'] + '-' + str(d['build']))" "$project/nsh.json")"
    mv "$out/package.nshpkg.tmp" "$out/$id.nshpkg"
    args+=(--package "$out/$id.nshpkg")
fi
args+=("$out/$(basename "$archive")")

nsh "${args[@]}"

[[ -s "$out/latest.json" ]] || { echo "::error::latest.json was not written" >&2; exit 1; }
[[ -s "$out/latest.json.sig" ]] || { echo "::error::latest.json.sig was not written: the manifest is unsigned" >&2; exit 1; }
trap - ERR
ls -l "$out"
