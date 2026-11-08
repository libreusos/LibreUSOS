#!/bin/sh
# Format, lint and test the whole workspace. Runs in rust:*-bookworm on merge requests.
set -eu
. "$(dirname "$0")/common.sh"
export USOS_NO_EMBEDDED_KEY=1

install_desktop_deps
rustup component add rustfmt clippy
cd "$ROOT"
cargo fmt --all --check
# --no-deps: lint the desktop crate only; the other crates have older findings.
cargo clippy -p usos-desktop --no-deps --all-targets --locked -- -D warnings
cargo test --workspace --locked
