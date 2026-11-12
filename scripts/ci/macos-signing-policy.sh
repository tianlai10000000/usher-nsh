#!/usr/bin/env bash
# Decide whether this build is signed and notarized with an Apple Developer ID.
#
# The switch is the repository Actions Variable MACOS_SIGNING_NOTARIZATION_ENABLED:
#   "true"          sign and notarize; every signing step is blocking
#   "false" / unset package unsigned and say so, loudly, to the people who will install it
#   anything else   fail before building (a typo must not silently ship unsigned)
#
# This is independent of the update-manifest signature, which is always required.
#
# Prints `enabled=true|false` for $GITHUB_OUTPUT. Warnings go to the job summary.
set -euo pipefail

value="${MACOS_SIGNING_NOTARIZATION_ENABLED-}"

case "$value" in
    true)
        echo "enabled=true"
        ;;
    false | "")
        echo "enabled=false"
        message="macOS packages in this release are not signed with an Apple Developer ID and are not notarized. Gatekeeper may block opening them."
        echo "::warning::$message" >&2
        if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
            printf '%s\n' "$message" >> "$GITHUB_STEP_SUMMARY"
        fi
        ;;
    *)
        echo "::error::MACOS_SIGNING_NOTARIZATION_ENABLED must be exactly true, false, or unset (got a different value)." >&2
        exit 1
        ;;
esac
