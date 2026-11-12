#!/usr/bin/env bash
# Entry point of the local update simulation: forwards to sim.py with a stable interpreter.
#
# Usage: sim.sh list | run <scenario> | run --all | serve <scenario> | clean
# See docs/update.md, section 12.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# -I keeps the interpreter from importing modules out of the current directory.
exec python3 -I "$here/sim.py" "$@"
