#!/bin/sh
# Linux x86_64 desktop app (GPUI, glibc, dynamic) -> dist/usos-desktop-<version>-x86_64-linux.tar.gz.
# Runs in rust:*-bookworm (Debian 12, glibc 2.36: the binary runs on anything newer).
set -eu
. "$(dirname "$0")/common.sh"
use_consumer_key_unless_mr

NAME="usos-desktop-$VERSION-x86_64-linux"
APP_ID=pl.libreusos.LibreUSOS
PKG="$ROOT/crates/usos-desktop/packaging/linux"

install_desktop_deps
cd "$ROOT"
export RUSTFLAGS="$(remap_flags)"

CARGO_PROFILE_RELEASE_STRIP=symbols cargo build --release --locked -p usos-desktop

BIN=target/release/usos-desktop
ldd "$BIN"
# Direct dependencies: only libc and X11/xkbcommon libraries. GPUI loads Wayland, fontconfig, the
# Vulkan loader and the XDG portal at run time.
NEEDED=$(readelf -d "$BIN" | sed -n 's/.*Shared library: \[\(.*\)\]/\1/p')
echo "$NEEDED"
BAD=$(echo "$NEEDED" | grep -Ev '^(ld-linux-x86-64|libc|libm|libgcc_s|libdl|libpthread|librt|libxcb|libxcb-[a-z0-9-]+|libxkbcommon|libxkbcommon-x11)\.so' || true)
[ -z "$BAD" ] || { echo "error: $BIN needs unexpected libraries: $BAD" >&2; exit 1; }

mkdir -p "stage/$NAME"
cp "$BIN" LICENSE "stage/$NAME/"
cp crates/usos-desktop/assets/NOTICE.md "stage/$NAME/NOTICE-assets.md"
cp crates/usos-desktop/assets/fonts/OFL.txt "stage/$NAME/LICENSE-OFL-Geist.txt"
cp crates/usos-desktop/assets/icons/LICENSE-APACHE-2.0.txt "stage/$NAME/LICENSE-APACHE-2.0.txt"
cp "$PKG/$APP_ID.desktop" "$PKG/$APP_ID.png" "stage/$NAME/"
cp crates/usos-desktop/assets/logo/libreusos.svg "stage/$NAME/$APP_ID.svg"
tar -C stage -czf "$DIST/$NAME.tar.gz" "$NAME"
