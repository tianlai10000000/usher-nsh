#!/usr/bin/env bash
# Download a pinned lefthook (Linux x86_64) into the current directory, verifying its checksum.
# Used by CI only; on a Mac it is `brew install lefthook`.
set -euo pipefail

version="2.2.2"
# SHA-256 of lefthook_${version}_Linux_x86_64, from lefthook's own lefthook_checksums.txt.
sha256="4fbf89eb7748c6aed84d833ee1a29edac48fae614468ca9a82896ec32f2800b6"
asset="lefthook_${version}_Linux_x86_64"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --fail --silent --show-error --location --retry 3 --proto '=https' --tlsv1.2 \
    --output "$work/$asset" \
    "https://github.com/evilmartians/lefthook/releases/download/v${version}/${asset}"

if command -v sha256sum > /dev/null 2>&1; then
    actual="$(sha256sum "$work/$asset" | cut -d ' ' -f 1)"
else
    actual="$(shasum -a 256 "$work/$asset" | cut -d ' ' -f 1)"
fi
if [[ "$actual" != "$sha256" ]]; then
    echo "::error::$asset does not match the pinned checksum (expected $sha256, got $actual)" >&2
    exit 1
fi

install -m 0755 "$work/$asset" ./lefthook
./lefthook version
