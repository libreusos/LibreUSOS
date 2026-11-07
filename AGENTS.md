# Guide for coding agents

LibreUSOS: unofficial USOS client. Rust core, Android app (Kotlin, Compose), `usos` CLI. Depth lives in the wiki: https://gitlab.com/voxix/LibreUSOS/-/wikis/home

## Layout

- `crates/usos-core`: USOS API client, OAuth 1.0a signer, login, terms, timetable, grades, course tests. `build.rs` embeds the consumer key.
- `crates/usos-store`: SQLite offline cache and online-or-cached orchestration.
- `crates/usos-ffi`: UniFFI bindings for Kotlin (`uniffi` pinned to `=0.32.2`).
- `crates/usos-cli`: the `usos` binary. Output only.
- `android/`: the app (package `io.gitlab.voxix.libreusos`). Gradle builds `usos-ffi` with cargo-ndk and generates the Kotlin bindings.
- `android/fdroid/`: F-Droid recipe. `fastlane/metadata/android/`: store text and changelogs (`en-US`, `pl-PL`).
- `ci/` and `.gitlab-ci.yml`: release builds.

## Commands

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

cd android
./gradlew testDebugUnitTest
./gradlew assembleDebug
```

Android builds need the SDK, NDK and cargo-ndk versions listed in the wiki page Building. CI has no lint or test job; run the commands above before pushing.

## Conventions

- Code, comments, identifiers and error messages are in English.
- User-facing Android strings go in `android/app/src/main/res/values/strings.xml` and `values-pl/strings.xml`. Add both.
- Front ends are thin. API, auth and storage logic belongs in `usos-core` or `usos-store`.
- The public API of `usos-core` and `usos-store` uses owned types only (no generics, lifetimes or `impl Trait` at the edge), so UniFFI can wrap it.
- Secrets are wrapped in `Secret` (redacted `Debug`). Do not log tokens, consumer secrets or URLs that carry them.
- Datetimes from USOS are local time without an offset. Keep the installation time zone with them.
- Add tests with changes. The Android reducers and models are pure functions with JVM unit tests in `android/app/src/test`.
- Schema changes in `usos-store` are new, append-only migrations. Never edit an existing one.
- Keep `Cargo.lock` committed and build with `--locked` in CI.

## Secrets

- Never commit, print or paste `.env`, keystores (`*.jks`), consumer secrets or tokens. `.env` and `libreusos-release.jks` are gitignored.
- Do not source `.env` in a shell. Build code reads it by path.
- The CI variable names are documented in `.env.example` and the wiki page Releasing. Never put values in the repository, logs or MR text.
- Tests use fixtures and fake servers. Do not commit real USOS responses with personal data.

## Merge requests

- `main` is the default branch. Work on a branch and open a merge request.
- Release flow and CI variables: the wiki page Releasing.
- An MR description lists only what the MR changes. No notes, status updates, links to work done elsewhere, or summaries of your process.
- No attribution in MR descriptions or commit messages: no "Generated with …" lines and no `Co-Authored-By` trailers for tools or agents.
