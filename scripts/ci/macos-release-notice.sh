#!/usr/bin/env bash
# Write the macOS signing notice that goes at the top of the release notes.
#
# Usage: macos-release-notice.sh true|false NOTES_FILE
# The notes file already holds the changelog section; the notice is added in front of it,
# so a person deciding whether to install sees the Gatekeeper caveat first. For `true` the
# notes are left as they are.
set -euo pipefail

enabled="${1:?usage: macos-release-notice.sh true|false NOTES_FILE}"
notes="${2:?usage: macos-release-notice.sh true|false NOTES_FILE}"
[[ -f "$notes" ]] || { echo "::error::$notes does not exist" >&2; exit 1; }

case "$enabled" in
    true) exit 0 ;;
    false) ;;
    *)
        echo "::error::The signing mode must be true or false, not '$enabled'." >&2
        exit 1
        ;;
esac

work="$(mktemp)"
trap 'rm -f "$work"' EXIT
{
    echo "> **macOS:** this build is not signed with an Apple Developer ID and is not notarized."
    echo "> Gatekeeper will refuse to open it at first. To open it anyway, Control-click the app,"
    echo "> choose **Open**, then confirm; or run \`xattr -dr com.apple.quarantine <path to the app>\`."
    echo
    cat "$notes"
} > "$work"
cp "$work" "$notes"
