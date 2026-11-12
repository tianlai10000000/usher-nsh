#!/usr/bin/env bash
# Refuse a commit that adds a private key block to any file. A key pasted into a text file is
# the other way keys leak, besides a key file.
set -euo pipefail

if git diff --cached --diff-filter=ACMR -U0 | grep -Eq '^\+.*-----BEGIN [A-Z ]*PRIVATE KEY-----'; then
    echo "a private key block is being added; remove it before committing" >&2
    exit 1
fi
