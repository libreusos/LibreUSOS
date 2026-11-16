#!/bin/sh
# Attaches the build artifacts in dist/ to the GitHub release of the tag: uploads to a release that
# already exists (created in the UI), else creates it. Needs the gh CLI and GH_TOKEN.
set -eu
. "$(dirname "$0")/common.sh"

TAG=${GITHUB_REF_NAME:?GITHUB_REF_NAME is not set}
cd "$ROOT"

# Each entry is FILE#LABEL, the form gh takes for a display name.
set -- \
    "dist/usos-$VERSION-x86_64-linux.tar.gz#usos CLI, Linux x86_64 (static)" \
    "dist/usos-$VERSION-x86_64-windows.zip#usos CLI, Windows x86_64" \
    "dist/libreusos-$VERSION.apk#LibreUSOS for Android (APK)" \
    "dist/usos-desktop-$VERSION-x86_64-linux.tar.gz#LibreUSOS desktop, Linux x86_64 (glibc)" \
    "dist/usos-desktop-$VERSION-x86_64-windows.zip#LibreUSOS desktop, Windows x86_64"

for entry in "$@"; do
    [ -f "${entry%%#*}" ] || { echo "error: missing ${entry%%#*}" >&2; exit 1; }
done

if gh release view "$TAG" >/dev/null 2>&1; then
    gh release upload "$TAG" "$@" --clobber
else
    gh release create "$TAG" "$@" --title "LibreUSOS $VERSION" --notes "LibreUSOS $VERSION" --verify-tag
fi
