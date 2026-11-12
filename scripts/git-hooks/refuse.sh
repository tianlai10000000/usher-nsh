#!/usr/bin/env bash
# Fail the commit and list the files that caused it.
#
# Usage: refuse.sh "reason" FILE...
reason="${1:?usage: refuse.sh REASON FILE...}"
shift
echo "$reason" >&2
printf '    %s\n' "$@" >&2
echo "  fix:  git restore --staged <file>" >&2
exit 1
