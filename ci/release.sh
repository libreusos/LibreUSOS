#!/bin/sh
# Attaches the build artifacts to the GitLab release of the tag: adds links to a release that already
# exists (created in the UI), else creates it (tag pushed with git). Runs in alpine.
set -eu
. "$(dirname "$0")/common.sh"

apk add --no-cache curl jq >/dev/null
API="$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"
TAG_URI=$(jq -rn --arg t "$CI_COMMIT_TAG" '$t|@uri')
ART="$CI_PROJECT_URL/-/jobs/artifacts/$TAG_URI/raw/dist"

# call METHOD URL [JSON]: prints the body, then the HTTP status on the last line.
call() {
    curl -sS --globoff -X "$1" -H "JOB-TOKEN: $CI_JOB_TOKEN" -H 'Content-Type: application/json' \
        -w '\n%{http_code}' ${3:+--data "$3"} "$2"
}

# link NAME FILE JOB: JSON of one package link
link() { printf '{"name":"%s","url":"%s/%s?job=%s","link_type":"package"}' "$1" "$ART" "$2" "$3"; }

L1=$(link "usos CLI, Linux x86_64 (static)" "usos-$VERSION-x86_64-linux.tar.gz" build-linux)
L2=$(link "usos CLI, Windows x86_64" "usos-$VERSION-x86_64-windows.zip" build-windows)
L3=$(link "LibreUSOS for Android (APK)" "libreusos-$VERSION.apk" build-android)
L4=$(link "LibreUSOS desktop, Linux x86_64 (glibc)" "usos-desktop-$VERSION-x86_64-linux.tar.gz" build-desktop-linux)
L5=$(link "LibreUSOS desktop, Windows x86_64" "usos-desktop-$VERSION-x86_64-windows.zip" build-desktop-windows)

if [ "$(call GET "$API/$TAG_URI" | tail -n 1)" = 200 ]; then
    for l in "$L1" "$L2" "$L3" "$L4" "$L5"; do
        out=$(call POST "$API/$TAG_URI/assets/links" "$l")
        case "$out" in
            *2[0-9][0-9]) ;;
            *'already been taken'*) echo "link exists, skipping" ;;
            *) echo "error: $out" >&2; exit 1 ;;
        esac
    done
else
    out=$(call POST "$API" "{\"tag_name\":\"$CI_COMMIT_TAG\",\"description\":\"LibreUSOS $VERSION\",\"assets\":{\"links\":[$L1,$L2,$L3,$L4,$L5]}}")
    case "$out" in *2[0-9][0-9]) ;; *) echo "error: $out" >&2; exit 1 ;; esac
fi
