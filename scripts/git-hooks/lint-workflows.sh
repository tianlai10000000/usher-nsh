#!/usr/bin/env bash
# actionlint the given workflow files as they are staged.
#
# actionlint wants a repository on disk, and a file named on its own command line is checked
# without the rest of the project. The staged content is written into a scratch repository so
# what is checked is what would be committed, not a later edit in the working copy. Include
# the staged project configuration so custom runner labels and other lint settings apply.
set -euo pipefail

command -v actionlint > /dev/null 2>&1 || { echo "skipped: actionlint is not installed (brew install actionlint); CI will run this check" >&2; exit 0; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
git init -q "$scratch"
for file in "$@"; do
    mkdir -p "$scratch/$(dirname "$file")"
    git show ":$file" > "$scratch/$file"
done
for config in .github/actionlint.yaml .github/actionlint.yml; do
    if git cat-file -e ":$config" 2> /dev/null; then
        mkdir -p "$scratch/.github"
        git show ":$config" > "$scratch/$config"
    fi
done
(cd "$scratch" && actionlint)
