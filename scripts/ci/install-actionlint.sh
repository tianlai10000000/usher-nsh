#!/usr/bin/env bash
# Download a pinned actionlint into the current directory, verifying its checksum.
set -euo pipefail

version="1.7.12"
# SHA-256 of actionlint_${version}_linux_amd64.tar.gz.
sha256="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"
asset="actionlint_${version}_linux_amd64.tar.gz"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --fail --silent --show-error --location --retry 3 --proto '=https' --tlsv1.2 \
    --output "$work/$asset" \
    "https://github.com/rhysd/actionlint/releases/download/v${version}/${asset}"
# `shasum` (macOS, and Perl-based Linux) and `sha256sum` (GNU) both exist somewhere; compute the
# digest and compare it ourselves instead of depending on either one's --check format.
if command -v sha256sum > /dev/null 2>&1; then
    actual="$(sha256sum "$work/$asset" | cut -d ' ' -f 1)"
else
    actual="$(shasum -a 256 "$work/$asset" | cut -d ' ' -f 1)"
fi
if [[ "$actual" != "$sha256" ]]; then
    echo "::error::$asset does not match the pinned checksum (expected $sha256, got $actual)" >&2
    exit 1
fi
tar -xzf "$work/$asset" -C . actionlint
chmod +x ./actionlint
./actionlint -version | head -n 1
