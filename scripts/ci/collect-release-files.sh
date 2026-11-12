#!/usr/bin/env bash
# Gather the built packages into one directory, the way a release and a rehearsal both need.
#
# Usage: collect-release-files.sh ARTIFACTS_DIR OUT_DIR BUILD
#
# Copies every .zip from ARTIFACTS_DIR into OUT_DIR and writes build.txt, which the next
# release's gate reads to refuse a lower build number. Shared by the publish job and the
# rehearsal job so the two cannot drift apart.
set -euo pipefail

artifacts="${1:?usage: collect-release-files.sh ARTIFACTS_DIR OUT_DIR BUILD}"
out="${2:?usage: collect-release-files.sh ARTIFACTS_DIR OUT_DIR BUILD}"
build="${3:?usage: collect-release-files.sh ARTIFACTS_DIR OUT_DIR BUILD}"

[[ "$build" =~ ^[1-9][0-9]*$ ]] || { echo "::error::The build number '$build' is not a positive integer" >&2; exit 1; }
[[ -d "$artifacts" ]] || { echo "::error::$artifacts is not a directory" >&2; exit 1; }

shopt -s nullglob
zips=("$artifacts"/*.zip)
if [[ ${#zips[@]} -eq 0 ]]; then
    echo "::error::No .zip packages were found in $artifacts" >&2
    exit 1
fi

# The update package must be there under its exact name: the updater selects it by name.
expected="Usher-aarch64-apple-darwin.zip"
found=0
for zip in "${zips[@]}"; do
    [[ "$(basename "$zip")" == "$expected" ]] && found=1
done
if [[ $found -ne 1 ]]; then
    echo "::error::The update package $expected is missing" >&2
    exit 1
fi

mkdir -p "$out"
cp "${zips[@]}" "$out"/
printf '%s\n' "$build" > "$out/build.txt"
ls -l "$out"
