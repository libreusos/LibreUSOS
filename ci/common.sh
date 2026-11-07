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
