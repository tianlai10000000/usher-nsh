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
echo "${sha256}  $work/$asset" | sha256sum --check --status \
    || { echo "::error::$asset does not match the pinned checksum" >&2; exit 1; }
tar -xzf "$work/$asset" -C . actionlint
chmod +x ./actionlint
