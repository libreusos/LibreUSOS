#!/bin/bash
# Signed release APK -> dist/libreusos-<version>.apk. Runs in eclipse-temurin:21-jdk-noble.
set -euo pipefail
. "$(dirname "$0")/common.sh"
use_consumer_key
require LIBREUSOS_KEYSTORE_PASSWORD LIBREUSOS_KEY_ALIAS

RUST_VERSION=1.99.1
NDK_VERSION=28.2.13676358
export ANDROID_HOME=$HOME/android-sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/$NDK_VERSION
export CARGO_HOME=$ROOT/.ci/cargo          # cached by the job
export GRADLE_USER_HOME=$ROOT/.ci/gradle   # cached by the job
export PATH="$CARGO_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"

apt-get update -qq
apt-get install -y -qq --no-install-recommends curl ca-certificates unzip build-essential >/dev/null

# Release keystore: LIBREUSOS_KEYSTORE_B64 (base64 of the .jks), else an existing LIBREUSOS_KEYSTORE path.
if [ -n "${LIBREUSOS_KEYSTORE_B64:-}" ]; then
    LIBREUSOS_KEYSTORE=$(mktemp)
    trap 'rm -f "$LIBREUSOS_KEYSTORE"' EXIT
    printf '%s' "$LIBREUSOS_KEYSTORE_B64" | base64 -d > "$LIBREUSOS_KEYSTORE"
    unset LIBREUSOS_KEYSTORE_B64
fi
require LIBREUSOS_KEYSTORE
export LIBREUSOS_KEYSTORE

mkdir -p "$ANDROID_HOME/cmdline-tools"
curl -fsSL -o /tmp/tools.zip https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip
unzip -q /tmp/tools.zip -d /tmp/tools
mv /tmp/tools/cmdline-tools "$ANDROID_HOME/cmdline-tools/latest"
yes | sdkmanager --licenses >/dev/null || true
sdkmanager "platforms;android-36" "build-tools;36.0.0" "ndk;$NDK_VERSION" >/dev/null

curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$RUST_VERSION"
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk --locked --version 4.1.2

cd "$ROOT/android"
./gradlew --no-daemon --console=plain :app:assembleRelease
cp app/build/outputs/apk/release/app-release.apk "$DIST/libreusos-$VERSION.apk"
