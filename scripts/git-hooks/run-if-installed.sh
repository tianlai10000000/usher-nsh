#!/usr/bin/env bash
# Run a linter on the given files, or skip with a visible message if it is not installed.
#
# Usage: run-if-installed.sh TOOL "install hint" [args...]
#
# A missing optional linter must not block a commit (CI runs it anyway), but it must not be
# silent: the message below is printed, so the person committing knows the check did not run.
# A linter that IS installed and fails fails the commit, with its own output.
set -euo pipefail

tool="${1:?usage: run-if-installed.sh TOOL HINT [ARGS...]}"
hint="${2:?usage: run-if-installed.sh TOOL HINT [ARGS...]}"
shift 2

if ! command -v "$tool" > /dev/null 2>&1; then
    echo "skipped: $tool is not installed ($hint); CI will run this check" >&2
    exit 0
fi
exec "$tool" "$@"
