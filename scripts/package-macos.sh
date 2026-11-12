#!/bin/sh
set -eu

if [ "$#" -gt 1 ]; then
    printf 'Usage: sh scripts/package-macos.sh [standard|appkit]\n' >&2
    exit 2
fi

variant=${1-standard}
case "$variant" in
    standard) USHER_MACOS_APPKIT=0 ;;
    appkit) USHER_MACOS_APPKIT=1 ;;
    *)
        printf 'Usage: sh scripts/package-macos.sh [standard|appkit]\n' >&2
        exit 2
        ;;
esac
export USHER_MACOS_APPKIT

cd "$(dirname "$0")/.."
exec nsh package --target macos --out ".nsh/out/macos-$variant"
