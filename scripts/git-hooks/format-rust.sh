#!/usr/bin/env bash
# Format the given Rust files in place, one at a time, through stdin.
#
# Why not `rustfmt FILE...`: given a path, rustfmt also follows `mod x;` into the files beside
# it and rewrites them, including files nobody staged. Reading from stdin formats exactly the
# one file and nothing else. (`rustfmt --check` on stdin is no better: it prints a diff but
# exits 0, so the content is compared instead of trusting its status.)
#
# A file that fails to format (a syntax error) stops the commit and is left as it was: the
# result goes to a temporary file first and replaces the original only on success.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
edition=""
if [[ -f "$root/Cargo.toml" ]]; then
    edition="$(sed -n 's/^edition *= *"\([0-9]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
fi
edition="${edition:-2024}"

command -v rustfmt > /dev/null 2>&1 || { echo "format-rust: rustfmt is not installed (rustup component add rustfmt)" >&2; exit 1; }

status=0
for file in "$@"; do
    temporary="$(mktemp)"
    if rustfmt --edition "$edition" < "$file" > "$temporary" 2> /dev/null; then
        # Rewrite only when something changed, so the file's timestamp and mode stay put.
        if ! cmp -s "$file" "$temporary"; then
            cat "$temporary" > "$file"
            echo "formatted $file"
        fi
    else
        echo "format-rust: rustfmt cannot format $file (syntax error?)" >&2
        status=1
    fi
    rm -f "$temporary"
done
exit "$status"
