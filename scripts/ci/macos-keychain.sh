#!/usr/bin/env bash
# A temporary keychain for Apple code signing on a CI runner.
#
# Usage: macos-keychain.sh install|remove
#
# install reads CERTIFICATE_BASE64 (a .p12), CERTIFICATE_PASSWORD and KEYCHAIN_PASSWORD
# from the environment and puts the certificate in a keychain that exists only for this
# job. remove deletes it, and runs even when the job failed. Nothing is printed that
# could contain a secret.
set -euo pipefail

keychain="${RUNNER_TEMP:?RUNNER_TEMP is required}/usher-signing.keychain-db"
certificate="${RUNNER_TEMP}/usher-signing.p12"

case "${1:-}" in
    install)
        for name in CERTIFICATE_BASE64 CERTIFICATE_PASSWORD KEYCHAIN_PASSWORD; do
            [[ -n "${!name:-}" ]] || { echo "::error::$name is not set" >&2; exit 1; }
        done
        # Mask the values so they cannot appear in a log by accident.
        for name in CERTIFICATE_PASSWORD KEYCHAIN_PASSWORD; do
            echo "::add-mask::${!name}"
        done
        umask 077
        printf '%s' "$CERTIFICATE_BASE64" | base64 --decode > "$certificate"
        security create-keychain -p "$KEYCHAIN_PASSWORD" "$keychain"
        # Lock after six hours and on sleep; a job never lasts that long.
        security set-keychain-settings -lut 21600 "$keychain"
        security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$keychain"
        security import "$certificate" -P "$CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$keychain"
        # Let codesign use the key without a prompt that no one is there to answer.
        security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" "$keychain" > /dev/null
        security list-keychains -d user -s "$keychain" "$(security default-keychain -d user | tr -d ' "')"
        rm -f "$certificate"
        ;;
    remove)
        rm -f "$certificate"
        security delete-keychain "$keychain" 2>/dev/null || true
        ;;
    *)
        echo "usage: macos-keychain.sh install|remove" >&2
        exit 2
        ;;
esac
