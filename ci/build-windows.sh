#!/bin/sh
# Windows x86_64 CLI, cross-compiled with mingw -> dist/usos-<version>-x86_64-windows.zip.
# Runs in rust:*-alpine.
set -eu
. "$(dirname "$0")/common.sh"
use_consumer_key

TARGET=x86_64-pc-windows-gnu
NAME="usos-$VERSION-x86_64-windows"

apk add --no-cache mingw-w64-gcc mingw-w64-winpthreads zip >/dev/null
rustup target add "$TARGET"
cd "$ROOT"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
RUSTFLAGS="$(remap_flags) -C target-feature=+crt-static" CARGO_PROFILE_RELEASE_STRIP=symbols \
    cargo build --release --locked -p usos-cli --target "$TARGET"

EXE="target/$TARGET/release/usos.exe"
# Only Windows system DLLs: no libgcc, libwinpthread, libstdc++.
DLLS=$(x86_64-w64-mingw32-objdump -p "$EXE" | sed -n 's/^[[:space:]]*DLL Name: //p')
echo "$DLLS"
echo "$DLLS" | grep -Eiq '^(lib|msys)' && { echo "error: $EXE imports a non-system DLL" >&2; exit 1; }

mkdir -p "stage/$NAME"
cp "$EXE" LICENSE "stage/$NAME/"
(cd stage && zip -qr "$DIST/$NAME.zip" "$NAME")
