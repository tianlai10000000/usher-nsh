#!/usr/bin/env bash
# Install the pinned NSH CLI on a CI runner and put it on PATH.
#
# The archive's SHA-256 is pinned in the workflow (NSH_MACOS_ARM64_SHA256), not read from
# a file next to the download: a compromised release could swap both, but not our workflow.
#
# Environment: NSH_VERSION (tag, e.g. v0.4.0), NSH_MACOS_ARM64_SHA256.
set -euo pipefail

version="${NSH_VERSION:?NSH_VERSION is required}"
expected="${NSH_MACOS_ARM64_SHA256:?NSH_MACOS_ARM64_SHA256 is required}"

case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) asset="nsh-macos-arm64.tar.gz" ;;
    *)
        echo "::error::No pinned NSH build for $(uname -s)-$(uname -m)" >&2
        exit 1
        ;;
esac

[[ "$expected" =~ ^[0-9a-f]{64}$ ]] || { echo "::error::NSH_MACOS_ARM64_SHA256 is not a SHA-256" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

curl --fail --silent --show-error --location --retry 3 --proto '=https' --tlsv1.2 \
    --output "$work/$asset" \
    "https://github.com/longbridge/nsh/releases/download/${version}/${asset}"

actual="$(shasum -a 256 "$work/$asset" | cut -d ' ' -f 1)"
if [[ "$actual" != "$expected" ]]; then
    echo "::error::$asset does not match the pinned checksum (expected $expected, got $actual)" >&2
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

if [[ -n "${GITHUB_PATH:-}" ]]; then
    echo "$dest/bin" >> "$GITHUB_PATH"
fi
# NSH finds its runtimes beside the binary; PATH alone is enough for later steps.
"$dest/bin/nsh" --version
