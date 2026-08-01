#!/bin/sh
set -eu
test $# -ge 1 || { echo "usage: $0 path/to/Markdown Annotator.app [notary-profile]" >&2; exit 2; }
codesign --verify --deep --strict --verbose=2 "$1"
if test -n "${2:-}"; then
  archive="$(mktemp -d)/Markdown-Annotator.zip"
  trap 'rm -f "$archive"; rmdir "$(dirname "$archive")" 2>/dev/null || true' EXIT
  ditto -c -k --keepParent "$1" "$archive"
  xcrun notarytool submit "$archive" --keychain-profile "$2" --wait
  xcrun stapler staple "$1"
fi
xcrun stapler validate "$1"
spctl --assess --type execute --verbose=2 "$1"
