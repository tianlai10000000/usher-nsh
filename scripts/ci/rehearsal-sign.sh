#!/usr/bin/env bash
# Rehearse the release's update files without any real key.
#
# Usage: rehearsal-sign.sh ARTIFACTS_DIR OUT_DIR SHELL_CHANGED
#
# Generates throwaway manifest, root and release keys in a temporary directory, certifies the
# release key, then builds the update files with the SAME script the publish job uses
# (make-update-files.sh) and checks them with the SAME checker (check_update_files.py). It then
# proves the checks really refuse damaged input, because a checker that accepts everything would
# pass the steps above: a swapped archive, a swapped package and an unsigned manifest must each
# fail, and the code package must verify against the throwaway root and not against another.
#
# The real keys are never read: this script runs in a job that has no secrets. What it cannot
# show is that the real keys are configured correctly, which only a real release shows.
set -euo pipefail

artifacts="${1:?usage: rehearsal-sign.sh ARTIFACTS_DIR OUT_DIR SHELL_CHANGED}"
out="${2:?usage: rehearsal-sign.sh ARTIFACTS_DIR OUT_DIR SHELL_CHANGED}"
shell_changed="${3:?usage: rehearsal-sign.sh ARTIFACTS_DIR OUT_DIR SHELL_CHANGED}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
umask 077

shopt -s nullglob
archives=("$artifacts"/*-macos-aarch64.tar.gz)
[[ ${#archives[@]} -eq 1 ]] || { echo "::error::Expected one whole-app archive in $artifacts, found ${#archives[@]}" >&2; exit 1; }

# The keys, and a copy of the project whose nsh.json trusts them. The real nsh.json trusts the
# real keys, which a rehearsal must not touch, so the throwaway ones go into a copy.
keys="$work/keys"
mkdir "$keys"
for name in manifest root release other; do nsh keygen "$keys/$name" > /dev/null; done
nsh certify --root "$keys/root.secret" --key "$keys/release.pub" \
    --apps '*' --days 2 --out "$keys/release.cert" > /dev/null

project="$work/project"
mkdir "$project"
for path in App.tsx src ui i18n platforms crates build.rs Cargo.toml Cargo.lock tsconfig.json .cargo nsh.json; do
    [[ -e "$repo/$path" ]] && cp -R "$repo/$path" "$project/"
done
python3 - "$project/nsh.json" "$keys" <<'PY'
import json, sys
path, keys = sys.argv[1:]
data = json.load(open(path))
read = lambda name: open(f"{keys}/{name}").read().strip()
data["updater"] = {
    "github": "rehearsal/rehearsal",
    "publicKey": read("manifest.pub"),
    "packages": {"roots": [read("root.pub")]},
}
json.dump(data, open(path, "w"), indent=2)
PY

# The archive is named for the version in nsh.json, which the rehearsal does not change.
mkdir "$work/archives"
cp "${archives[0]}" "$work/archives/"

export MANIFEST_KEY="$keys/manifest.secret" RELEASE_KEY="$keys/release.secret" RELEASE_CERT="$keys/release.cert" PROJECT="$project"
bash "$here/make-update-files.sh" "$work/archives" "$out" "$shell_changed"
python3 "$repo/scripts/release/check_update_files.py" "$out" --entry "$project" --shell-changed "$shell_changed"

# A checker that accepts everything would pass the steps above. Prove it does not.
expect_failure() {
    local description="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        echo "::error::The checks accepted $description; they are broken" >&2
        exit 1
    fi
    echo "refused as expected: $description"
}
check=(python3 "$repo/scripts/release/check_update_files.py")

damaged="$work/damaged"
archive_name="$(basename "${archives[0]}")"

mkdir "$damaged"
cp "$out"/* "$damaged"/
printf 'x' >> "$damaged/$archive_name"
expect_failure "a swapped whole-app archive" "${check[@]}" "$damaged" --entry "$project" --shell-changed "$shell_changed"

rm -rf "$damaged"
mkdir "$damaged"
cp "$out"/* "$damaged"/
rm "$damaged/latest.json.sig"
expect_failure "an unsigned manifest" "${check[@]}" "$damaged" --entry "$project" --shell-changed "$shell_changed"

rm -rf "$damaged"
mkdir "$damaged"
cp "$out"/* "$damaged"/
opposite="true"
[[ "$shell_changed" == "true" ]] && opposite="false"
expect_failure "a release that disagrees about the shell change" "${check[@]}" "$damaged" --entry "$project" --shell-changed "$opposite"

if [[ "$shell_changed" == "false" ]]; then
    package=("$out"/*.nshpkg)
    [[ ${#package[@]} -eq 1 ]] || { echo "::error::Expected one code package in $out" >&2; exit 1; }
    # Verified as an app trusting the throwaway root would, and refused by an app trusting another.
    nsh verify --root "$keys/root.pub" "${package[0]}"
    expect_failure "a code package under another root" nsh verify --root "$keys/other.pub" "${package[0]}"

    rm -rf "$damaged"
    mkdir "$damaged"
    cp "${package[0]}" "$damaged/changed.nshpkg"
    printf 'x' >> "$damaged/changed.nshpkg"
    expect_failure "a changed code package" nsh verify --root "$keys/root.pub" "$damaged/changed.nshpkg"
fi

echo "Rehearsal signing passed."
