#!/usr/bin/env bash
# Rehearse release signing without the real update key.
#
# Usage: rehearsal-sign.sh RELEASE_DIR VERSION CHANNEL
#
# Generates a throwaway key pair in a temporary directory, signs RELEASE_DIR's manifest with
# it, and verifies the result exactly as a client would, against a key list holding only the
# throwaway public key. Then it proves the verifier really refuses damaged input: a changed
# package, a changed manifest and an unlisted extra file must each fail verification.
#
# The real key is never read: this script runs in a job that has no secrets. What it cannot
# show is that the real key is set up correctly, which only a real release exercises.
#
# TOOL names the usher-update-tool binary (default: target/release/usher-update-tool).
set -euo pipefail

release="${1:?usage: rehearsal-sign.sh RELEASE_DIR VERSION CHANNEL}"
version="${2:?usage: rehearsal-sign.sh RELEASE_DIR VERSION CHANNEL}"
channel="${3:?usage: rehearsal-sign.sh RELEASE_DIR VERSION CHANNEL}"
tool="${TOOL:-target/release/usher-update-tool}"

[[ -x "$tool" ]] || { echo "::error::$tool is not built" >&2; exit 1; }
[[ -d "$release" ]] || { echo "::error::$release is not a directory" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The key goes outside the repository, as the tool insists.
"$tool" keygen --out "$work/key" > "$work/keygen.out"
printf '%s\n' "$(tr -d '\n' < "$work/key/update-signing.pub")" > "$work/keys.txt"

"$tool" sign "$release" --version "$version" --channel "$channel" \
    --key "$work/key/update-signing.secret" --keys "$work/keys.txt" > "$work/sign.out"
cat "$work/sign.out"
"$tool" verify "$release" --channel "$channel" --keys "$work/keys.txt"
test -s "$release/SHA256SUMS"
test -s "$release/SHA256SUMS.sig"

# A verifier that accepts everything would pass the checks above. Prove it does not.
expect_failure() {
    local description="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        echo "::error::Verification accepted $description; the verifier is broken" >&2
        exit 1
    fi
    echo "refused as expected: $description"
}

first="$(find "$release" -maxdepth 1 -name '*.zip' | LC_ALL=C sort | head -n 1)"
[[ -n "$first" ]] || { echo "::error::No package to damage" >&2; exit 1; }

damaged="$work/damaged"
mkdir "$damaged"
cp "$release"/* "$damaged"/
printf 'x' >> "$damaged/$(basename "$first")"
expect_failure "a changed package" "$tool" verify "$damaged" --channel "$channel" --keys "$work/keys.txt"

cp "$release"/* "$damaged"/
printf '\n' >> "$damaged/SHA256SUMS"
expect_failure "a changed manifest" "$tool" verify "$damaged" --channel "$channel" --keys "$work/keys.txt"

cp "$release"/* "$damaged"/
cp "$first" "$damaged/Usher-smuggled.zip"
expect_failure "an unlisted extra package" "$tool" verify "$damaged" --channel "$channel" --keys "$work/keys.txt"

other="stable"
[[ "$channel" == "stable" ]] && other="preview"
expect_failure "the wrong channel" "$tool" verify "$release" --channel "$other" --keys "$work/keys.txt"

echo "Rehearsal signing passed."
