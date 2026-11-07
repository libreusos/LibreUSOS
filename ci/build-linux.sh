#!/bin/sh
# Static Linux x86_64 CLI (musl) -> dist/usos-<version>-x86_64-linux.tar.gz. Runs in rust:*-alpine.
set -eu
. "$(dirname "$0")/common.sh"
use_consumer_key

TARGET=x86_64-unknown-linux-musl
NAME="usos-$VERSION-x86_64-linux"

apk add --no-cache file >/dev/null
rustup target add "$TARGET"
cd "$ROOT"
RUSTFLAGS="$(remap_flags) -C target-feature=+crt-static" CARGO_PROFILE_RELEASE_STRIP=symbols \
    cargo build --release --locked -p usos-cli --target "$TARGET"

BIN="target/$TARGET/release/usos"
file "$BIN"
file "$BIN" | grep -Eq 'statically linked|static-pie linked' || { echo "error: $BIN is not static" >&2; exit 1; }

mkdir -p "stage/$NAME"
cp "$BIN" LICENSE "stage/$NAME/"
tar -C stage -czf "$DIST/$NAME.tar.gz" "$NAME"
