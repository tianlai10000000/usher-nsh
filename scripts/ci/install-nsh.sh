#!/usr/bin/env bash
# Install the NSH command line the project is written for, and put it on PATH.
#
# The version is read from Cargo.toml (the exact nsh-core pin), so CI cannot build with a
# different NSH than the project asks for. Upgrading NSH is a change to Cargo.toml and nothing
# else: there is no second copy of the version, and no checksum to keep in step with it.
#
# Checks made, and what they do not cover:
#   * the archive matches the .sha256 NSH publishes beside it  (catches a damaged or truncated download)
#   * the installed binary reports the version asked for        (catches a wrong or mislabelled release)
# Not covered: a release whose archive and .sha256 were both replaced would pass. Pinning the
# checksum in this repository would cover that, at the cost of editing it on every NSH upgrade.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

version="$(sed -n 's/^nsh-core *= *"=\{0,1\}\([0-9][0-9.]*\)".*/\1/p; s/^nsh-core *= *{.*version *= *"=\{0,1\}\([0-9][0-9.]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "::error::Cannot read the nsh-core version from Cargo.toml" >&2; exit 1; }

case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) asset="nsh-macos-arm64.tar.gz" ;;
    *)
        echo "::error::No NSH build for $(uname -s)-$(uname -m)" >&2
        exit 1
        ;;
esac

base="https://github.com/longbridge/nsh/releases/download/v${version}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fetch() {
    curl --fail --silent --show-error --location --retry 3 --proto '=https' --tlsv1.2 --output "$1" "$2"
}
fetch "$work/$asset" "$base/$asset" || { echo "::error::Could not download NSH v$version (does that release exist?)" >&2; exit 1; }
fetch "$work/$asset.sha256" "$base/$asset.sha256" || { echo "::error::Could not download the checksum for NSH v$version" >&2; exit 1; }

expected="$(cut -d ' ' -f 1 "$work/$asset.sha256")"
[[ "$expected" =~ ^[0-9a-f]{64}$ ]] || { echo "::error::The published checksum is not a SHA-256" >&2; exit 1; }
actual="$(shasum -a 256 "$work/$asset" | cut -d ' ' -f 1)"
if [[ "$actual" != "$expected" ]]; then
    echo "::error::$asset does not match its published checksum (expected $expected, got $actual)" >&2
    exit 1
fi

# The archive holds one top-level directory; refuse anything that would unpack elsewhere.
if tar -tzf "$work/$asset" | grep -qE '(^/|(^|/)\.\./)'; then
    echo "::error::$asset contains unsafe paths" >&2
    exit 1
fi

dest="${RUNNER_TEMP:-$work}/nsh-dist"
rm -rf "$dest"
mkdir -p "$dest"
tar -xzf "$work/$asset" -C "$dest" --strip-components=1
test -x "$dest/bin/nsh"

reported="$("$dest/bin/nsh" --version | cut -d ' ' -f 2)"
if [[ "$reported" != "$version" ]]; then
    echo "::error::The installed nsh reports $reported, not $version" >&2
    exit 1
fi

if [[ -n "${GITHUB_PATH:-}" ]]; then
    echo "$dest/bin" >> "$GITHUB_PATH"
fi
"$dest/bin/nsh" --version
