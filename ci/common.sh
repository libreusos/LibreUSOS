# Sourced by the ci/*.sh scripts. POSIX sh: the Alpine images have no bash.
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIST="$ROOT/dist"
mkdir -p "$DIST"

# Version: the tag without a leading "v", else git describe, else the short SHA.
VERSION=${CI_COMMIT_TAG:-$(git -C "$ROOT" describe --tags --always 2>/dev/null || echo "${CI_COMMIT_SHORT_SHA:-dev}")}
VERSION=${VERSION#v}

# require VAR...: fail fast, before any compiling, naming the unset variables.
require() {
    missing=
    for name in "$@"; do
        eval "value=\${$name:-}"
        [ -n "$value" ] || missing="$missing $name"
    done
    [ -z "$missing" ] || { echo "error: unset CI variable(s):$missing" >&2; exit 1; }
}

# Maps the PolSl variables to the names usos-core/build.rs reads. Never prints them.
use_consumer_key() {
    require USOS_POLSL_CONSUMER_KEY USOS_POLSL_CONSUMER_SECRET
    export USOS_CONSUMER_KEY=$USOS_POLSL_CONSUMER_KEY
    export USOS_CONSUMER_SECRET=$USOS_POLSL_CONSUMER_SECRET
    export USOS_KEY_BASE_URL=https://usosapi.polsl.pl/
}

# rustc flags: hide the checkout and cargo paths in the binaries.
remap_flags() {
    printf -- '--remap-path-prefix=%s=/libreusos --remap-path-prefix=%s=/cargo' "$ROOT" "$CARGO_HOME"
}

# use_consumer_key_unless_mr: like use_consumer_key, but a merge request pipeline builds without an
# embedded key (protected variables may be missing there, and its artifacts must not carry the secret).
use_consumer_key_unless_mr() {
    if [ "${CI_PIPELINE_SOURCE:-}" = merge_request_event ]; then
        export USOS_NO_EMBEDDED_KEY=1
        echo "note: merge request pipeline, building without an embedded consumer key"
    else
        use_consumer_key
    fi
}

# System libraries GPUI builds against, on the rust:*-bookworm image.
install_desktop_deps() {
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends \
        clang pkg-config libfontconfig-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
        libxcb1-dev libxcb-xkb-dev libx11-dev libvulkan-dev libasound2-dev >/dev/null
}
