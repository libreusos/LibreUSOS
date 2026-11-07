# USOS client: tech stack research

Date: 2026-11-04.

Legend: **[V]** = verified today against crates.io / GitHub API / fdroiddata / official docs. **[M]** = from prior
knowledge, not re-verified in this session (treat as a claim to check). **[E]** = my estimate, not measured; a spike
should measure it before it drives a decision.

## TL;DR

- Core: one Rust workspace (`usos-api`, `usos-core`, `usos-ffi`, `usos-cli`, `usos-daemon`, `usos-mcp`, UI crate). `reqwest`
  0.13 + rustls, `rusqlite` 0.41, `tokio`, `clap`, `rmcp` 3.8, `uniffi` 0.32. Write the OAuth 1.0a signer in-house.
- Android: Kotlin + Jetpack Compose (Material 3, dynamic color) + Glance widgets, core through UniFFI built with
  `cargo-ndk` from a script. There is direct F-Droid precedent for this shape (Delta Chat, Cfait, Fatto, Ruslin, Mages).
- Whatever UI toolkit is chosen, **the widgets are Kotlin** (`AppWidgetProvider` + `RemoteViews`, or Glance on top of it).
  No cross-platform toolkit removes that.
- Desktop: GTK4 + libadwaita (relm4) for Linux-first; Slint if Windows/macOS become real. Compose Multiplatform is the
  only option that truly shares UI with Android, but it costs a JVM and has no native Wayland.
- Pick: **Combo A** (Rust core + UniFFI + Kotlin/Compose/Glance + GTK4/relm4 desktop). Runner-up: **Combo B** (Compose
  Multiplatform on both). Combo C (Slint everywhere) is the purest but the riskiest on Android in late 2026.

---

## 0. Verified data points

### Crate versions (crates.io `max_stable_version`, queried 2026-11-04) [V]

| Crate | Version | Updated | Note |
|---|---|---|---|
| rmcp (official MCP Rust SDK) | 3.8.0 | 2026-10-26 | releases every few days; v3 line; breaking changes between 3.0 betas and 3.1 |
| uniffi | 0.32.2 | 2026-09-23 | 0.31.0 (2026-01-14), 0.32.0 (2026-06-30); breaking changes at each minor |
| reqwest | 0.13.5 | 2026-09-08 | |
| rustls | 0.23.46 | 2026-10-14 | |
| rusqlite | 0.41.1 | 2026-10-28 | |
| sqlx | 0.9.0 | 2026-05-21 | |
| tokio-rusqlite | 0.8.0 | 2026-09-06 | |
| rusqlite_migration | 2.6.0 | 2026-05-28 | |
| tokio | 1.54.1 | 2026-10-21 | |
| clap | 4.6.7 | 2026-09-14 | |
| axum | 0.8.9 | 2026-04-14 | |
| oo7 (Secret Service, pure Rust) | 0.6.0 | 2026-09-21 | |
| keyring | 4.2.0 | 2026-08-29 | |
| zbus | 6.2.1 | 2026-11-01 | |
| ashpd (XDG portals) | 0.13.13 | 2026-07-17 | |
| oauth1-request | 0.6.1 | 2024-06-19 | stale |
| reqwest-oauth1 | 0.3.0 | 2024-08-28 | stale |
| oauth1 | 1.0.0 | 2020-01-12 | dead |
| icalendar | 0.17.17 | 2026-10-26 | active; `ics` is stale (2022) |
| cargo-ndk | 4.1.2 | 2025-08-09 | de-facto pin in F-Droid recipes |
| gobley-uniffi-bindgen | 0.3.7 | 2025-10-08 | gobley repo pushed 2026-10-02, last release 2025-10-08 |
| jni | 0.22.4 | 2026-03-16 | |
| android-activity | 0.6.1 | 2026-03-24 | |
| flutter_rust_bridge | 2.13.0 | 2026-09-12 | |
| slint | 1.18.1 | 2026-09-21 | 1.17.1 (2026-07-07), 1.18.0 (2026-09-16); master docs already say 1.19 |
| iced | 0.14.0 | 2025-12-07 | no release for 11 months |
| egui | 0.37.0 | 2026-10-15 | |
| gpui | 0.2.2 | 2026-10-30 (crate page) | standalone crate on crates.io |
| dioxus | 0.7.10 | 2026-10-16 | 0.8.0-alpha.3 also 2026-10-16 |
| blitz-dom / dioxus-native | 0.2.4 / 0.7.10 | 2026-10-16 | Blitz is still 0.x |
| tauri | 2.12.5 | 2026-10-21 | |
| relm4 | 0.11.0 | 2026-04-08 | |
| gtk4 (gtk-rs) | 0.11.5 | 2026-09-20 | |
| libadwaita (gtk-rs) | 0.9.2 | 2026-07-07 | |
| cxx-qt | 0.10.0 | 2026-08-24 | |
| freya | 0.4.3 | 2026-10-25 | |
| androidx.glance:glance-appwidget | 1.2.0 stable, 1.3.0-alpha02 | (Google Maven) | |
| androidx.work (WorkManager) | 2.12.0 | (Google Maven) | |
| androidx.compose.material3 | 1.5.0-beta02 | (Google Maven) | |
| Compose Multiplatform | 1.12.2 stable (2026-10-15); 1.13.0-beta01 (2026-10-23) | | |

### GitHub activity [V]

| Repo | Stars | Last push |
|---|---|---|
| modelcontextprotocol/rust-sdk | 4.1k | 2026-11-03 |
| mozilla/uniffi-rs | 5.1k | 2026-10-28 |
| gobley/gobley | 444 | 2026-10-02 |
| bbqsrc/cargo-ndk | 986 | 2026-10-17 |
| mozilla/rust-android-gradle | 1.3k | 2026-09-24 |
| slint-ui/slint | 24.4k | 2026-11-04 |
| tauri-apps/tauri | 112k | 2026-11-04 |
| DioxusLabs/dioxus | 39.6k | 2026-11-03 |
| DioxusLabs/blitz | 4.3k | 2026-11-03 |
| zed-industries/zed (GPUI) | 92.7k | 2026-11-04 |
| iced-rs/iced | 31.9k | 2026-11-01 |
| emilk/egui | 32k | 2026-11-02 |
| Relm4/Relm4 | 2.0k | 2026-11-02 |
| gtk-rs/gtk4-rs | 2.4k | 2026-11-04 |
| KDAB/cxx-qt | 1.6k | 2026-11-02 |
| marc2332/freya | 3.3k | 2026-11-03 |
| fzyzcjy/flutter_rust_bridge | 5.5k | 2026-11-03 |
| JetBrains/compose-multiplatform | 19.5k | 2026-11-04 |
| JetBrains/skiko | 2.2k | 2026-11-03 |
| mlm-games/mages (CMP + UniFFI Matrix client) | 181 | 2026-11-04 |

Nothing on the shortlist is abandoned. The differentiators are maturity and platform fit, not maintenance.

---

## 1. Shared Rust core architecture

### Workspace layout

```
usos/
  Cargo.toml                 # workspace, rust-toolchain.toml pinned, Cargo.lock committed
  crates/
    usos-api/    # typed USOS API client: OAuth 1.0a signer, request builder, serde models, backoff/rate-limit
    usos-core/   # domain model, SQLite store + migrations, sync engine, iCal export, reminder planner, SecretStore trait
    usos-ffi/    # cdylib: UniFFI facade, thin. Records + async methods + callback interfaces (SecretStore, Logger)
    usos-cli/    # clap binary (also hosts `daemon` subcommand, or a separate usos-daemon bin)
    usos-daemon/ # headless periodic sync, desktop notifications (notify-rust / portal), optional local socket/HTTP API
    usos-mcp/    # rmcp server: stdio, optional Streamable HTTP on loopback; thin adapter over usos-core
    usos-gui/    # desktop UI crate (toolkit: section 4)
  android/       # Kotlin app (Compose + Glance), builds jniLibs via scripts/build-android-rust.sh
```

Rules that keep the core reusable:

1. `usos-core` is a plain Rust library with no UniFFI, no UI, no platform paths. Everything platform-specific enters
   through traits (`SecretStore`, `Clock`, `DataDir`, `Notifier`).
2. `usos-ffi` only wraps; the Kotlin side sees a small `UsosClient` object with `suspend` methods and flows of plain records.
3. CLI, daemon, MCP and desktop GUI depend on `usos-core` directly in-process (no FFI, no IPC unless the daemon is running).
   If the GUI and the daemon must share one DB, SQLite WAL with one writer (the daemon) and the GUI as reader/IPC client
   is the simplest model. [design recommendation]

### Crate choices

| Concern | Choice | Reason / evidence |
|---|---|---|
| HTTP | `reqwest` 0.13, `default-features = false`, `rustls` + `http2`, `json` | No OpenSSL, so Android cross-compile needs no C deps. Cfait's F-Droid recipe still installs `libssl-dev`, which is what to avoid. |
| TLS roots | Decide deliberately: `webpki-roots` (self-contained, ignores user-installed CAs) vs `rustls-platform-verifier` (uses system store, needs JNI init on Android) | On custom ROMs users do install their own CAs; USOS hosts use public CAs, so `webpki-roots` is the low-risk default. [M] |
| OAuth 1.0a | **Implement in-house** (HMAC-SHA1 over RFC 5849 base string; about 150 lines on `hmac`, `sha1`, `percent-encoding`, `rand`) with RFC test vectors | `oauth1-request` last release 2024-06, `reqwest-oauth1` 2024-08, `oauth1` 2020-01. A stale dependency is a worse risk than 150 audited lines, and the 3-legged flow (request token, authorize, verifier/PIN, access token) needs control anyway. |
| Serde | `serde`, `serde_json`; tolerate unknown fields and nulls (USOS responses are loosely typed, many fields are `{pl,en}` language dicts) | [M] |
| SQLite | `rusqlite` 0.41 with `bundled` + `rusqlite_migration`; `tokio-rusqlite` or `spawn_blocking` | Local, small data set; no need for sqlx's pool/macros; `bundled` compiles under cargo-ndk. `sqlx` 0.9 is fine technically but heavier and its compile-time query checking complicates cross builds. |
| Async | `tokio` multi-thread on desktop; small pool on Android | UniFFI 0.32 supports `async_runtime = "tokio"`, including on trait exports. |
| Secrets, Linux | `oo7` 0.6 (pure Rust Secret Service client; works in Flatpak via the portal) or `keyring` 4.2 | Both active. Fallback for headless/daemon without a keyring: `0600` file in `$XDG_STATE_HOME`, clearly documented. |
| Secrets, Android | Kotlin side: Android Keystore AES-GCM key wrapping the token blob; expose to Rust as a UniFFI callback interface `SecretStore` | Keeps `jni` out of the core. Alternative `jni` 0.22 crate works but needs a Java class registered; callback interface is cleaner. |
| iCal | `icalendar` 0.17 | Active; generate VEVENT with stable UIDs and VTIMEZONE (Europe/Warsaw). |
| CLI | `clap` 4.6 derive, `clap_complete` for shell completions | |
| MCP | `rmcp` 3.8.0 (official modelcontextprotocol/rust-sdk, 4.1k stars, pushed 2026-11-03) | Stdio server via the `transport-io` feature; Streamable HTTP server via `transport-streamable-http-server` (axum-based). Feature names changed between 3.0 betas and 3.1, so pin the version and check the README for it. Not to be confused with the third-party `rust-mcp-sdk` (0.9.0). MCP spec revision referenced by that third-party crate: 2025-11-25. |
| HTTP/IPC for daemon | `axum` 0.8 on a Unix socket or loopback with a token | Reuse the same handler layer as the MCP HTTP transport. |

### Core design notes (USOS-specific) [M]

- USOS API is OAuth 1.0a; each university runs its own installation, and an app needs a consumer key/secret registered
  per installation. An open-source client cannot keep a consumer secret secret; plan for per-user/per-university
  consumer registration or a documented "bring your own key" flow. This is outside the stack choice but affects
  the architecture (config layer, key storage).
- Sync engine: pull-only (USOS has no push or webhooks to us), conditional refresh by TTL per resource (timetable,
  grades, news), diff against SQLite, emit change events (new grade, changed room) to the notifier. Store raw JSON
  next to typed columns to survive API field additions.

---

## 2. Rust to Android bridging and F-Droid

### UniFFI [V]

- `uniffi` 0.32.2; Kotlin bindings use JNA (the app adds `net.java.dev.jna:jna:<ver>@aar`). Proc-macro mode
  (`#[uniffi::export]`) and library-mode bindgen; async Rust maps to Kotlin `suspend`; callback interfaces and foreign
  traits let Kotlin implement `SecretStore`.
- Churn: 0.31 broke 0.30 checksums; 0.32 breaks async primary constructors, `[ByRef] bytes` handling, and `--config`
  semantics. 0.31.2 fixed JNA direct-mapped `u8/u16` returns on Android ARM32; 0.32.1 fixed a Kotlin checksum failure on
  aarch64. Pin library and generator to the same exact version and expect a small migration at each minor.
- Alternatives: `flutter_rust_bridge` 2.13 (Flutter only); hand-written `jni` 0.22 (more control, much more code).

### Build glue

| Option | Status | Verdict |
|---|---|---|
| Script calling `cargo-ndk` 4.1.2 (+ `uniffi-bindgen`), output to `jniLibs/<abi>` | cargo-ndk pushed 2026-10-17; used by ~40 fdroiddata recipes | **Use this.** Gradle calls the script or a plain `Exec` task. Simplest to port into an F-Droid recipe. |
| `mozilla/rust-android-gradle` | pushed 2026-09-24, 1.3k stars | Works, but the plugin pulls from the Gradle plugin portal and hides the toolchain; more to patch for F-Droid. |
| gobley (`gobley-uniffi-bindgen`, Gradle plugins) | last release 2025-10-08 (v0.3.7) | Aimed at Kotlin Multiplatform with Cargo integration; useful for Combo B, but a year without a release and it tracks its own uniffi version. Check compatibility with uniffi 0.32 before adopting. |

### How F-Droid builds Rust-in-APK apps [V, from fdroiddata master metadata, 9,837 files grepped]

Counts: 171 recipes mention `rustup`, 83 call `cargo `, about 40 use `cargo-ndk`, 5 mention `uniffi`
(`com.brokenpip3.fatto`, `net.koofr.vault`, `net.nymtech.nymvpn`, `org.dianqk.ruslin`, `org.mlm.mages`).

Closest precedents:

| App | Shape | Recipe highlights |
|---|---|---|
| **Cfait** (`com.trougnouf.cfait`, CalDAV tasks; Rust core + desktop + Android) | Rust lib + Kotlin app, closest to this project | `srclibs: rustup@1.28.2`; `rustup-init.sh --default-toolchain $(from rust-toolchain.toml) --target aarch64/armv7/x86_64-linux-android`; `cargo install --locked cargo-ndk@4.1.2`; `RUSTFLAGS="--remap-path-prefix ... -C link-arg=-Wl,--build-id=none"` for reproducibility; `cargo ndk -t ... -o android/app/src/main/jniLibs build --release --lib`; JDK 21 from Debian trixie; `ndk: r27d`; `Binaries:` set so F-Droid can compare against upstream APK. |
| **Mages** (`org.mlm.mages`, Matrix client) | **Compose Multiplatform UI + Rust core via UniFFI** (its README: not using matrix-sdk-ffi) | Same `rustup@1.28.2` srclib, Rust version read from `rust-toolchain.toml`, cargo-ndk version read from a workflow file, `gradle :composeApp:assembleRelease`, per-ABI split builds, `binary:` reproducibility check. Ships Android (F-Droid) plus Linux desktop (AppImage, AUR, Snap, Flathub), unsigned Windows/macOS. This is Combo B proven in the wild, albeit with 181 stars. |
| **Fatto** (`com.brokenpip3.fatto`) | Rust + UniFFI + `cargo ndk` | Debian `rustup` package, `rustup toolchain install 1.94.1`, `cargo install --locked cargo-ndk`, remap-path-prefix flags. |
| **Ruslin** (`org.dianqk.ruslin`) | Rust + UniFFI (`ruslin-data-uniffi`) | Older recipe (`ndk: r25c`) using `~/.cargo/env`. |
| **Delta Chat** (`com.b44t.messenger`, v2.67.0) | Large Rust core + Java app | One build entry per ABI (`VercodeOperation: 10 * %c + N`), `apt-get install rustup make g++ cmake`, `rustup default $(cat scripts/rust-toolchain)`, `scripts/ndk-make.sh <abi>`, `ndk: r27`, `scandelete` of vendored core dir. Toolchain pinned inside the app repo. |
| **Slint Viewer** (`dev.slint.viewer`, Slint's own recipe in `tools/viewer/android/fdroid/`) | Slint on Android on F-Droid | `apt-get install clang libclang-dev python3 ninja-build make rustup`, `./build-native.sh <abi>` with `FORCE_SKIA_BUILD=1` because "F-Droid forbids the prebuilt Skia rust-skia downloads"; reproducible-build verified against the upstream APK; `ndk: r27`. Shows Slint is F-Droid-buildable but you compile Skia from source (slow). |
| **Mullvad VPN** (`net.mullvad.mullvadvpn`) | Big Rust workspace + Gradle | `init:` script, `build-essential protobuf-compiler`, `timeout: 10800`, deletes desktop/iOS/Windows dirs. |
| **Element X** (`io.element.android.x`) | matrix-rust-sdk via UniFFI | Recipe builds only the Kotlin/Gradle side with `ndk: r28c` and verifies against upstream signed APK via `binary:` and `reproducible-apk-tools` (`--page-size 16`). The Rust SDK arrives as an upstream-built artifact; a new small app should not count on that being accepted, and should build Rust from source. Also shows `rm: libraries/pushproviders/firebase` to strip FCM. |
| **RustDesk** (`com.carriez.flutter_hbb`) | Flutter + Rust (flutter_rust_bridge) | Works, but needed a custom Flutter engine build for x86, `timeout: 14400`, 21 GB disk, and a dedicated upstream `flutter/build_fdroid.sh` written by an F-Droid maintainer. The most expensive recipe in the set. |

Secondary evidence (GitHub PRs from projects preparing F-Droid recipes; not authoritative): reviewers asked for `rustup`
from Debian's package or the `rustup` srclib instead of ad-hoc installs; compiling Rust in `prebuild:` can trip the source
scanner on `target/`, so compile in `build:`; F-Droid's autoupdate copies the previous build entry, so keep toolchain
versions in a script or `rust-toolchain.toml` in the app repo, not in the YAML.

Practical rules for our repo:

- Commit `Cargo.lock`, `rust-toolchain.toml`, and use `cargo ... --locked`. Crates are fetched from crates.io during the
  build; this is accepted practice (all recipes above do it). `cargo vendor` is optional hardening.
- Reproducibility: `--remap-path-prefix` for the workspace and `$HOME/.cargo`, `-C link-arg=-Wl,--build-id=none`
  (both used by Cfait/Fatto), `strip`, fixed NDK (r27d or r28c), fixed `cargo-ndk`.
- 16 KB page size: Android 15+ devices may use 16 KB pages; Play requires it, F-Droid apps should comply. NDK r28+ aligns by
  default; with r27 pass `-Wl,-z,max-page-size=16384`. Element's recipe post-processes with `--page-size 16`. [M for r27 flag]
- Build only the ABIs you ship (arm64-v8a mandatory; x86_64 optional for emulator; armeabi-v7a only if wanted). Fewer ABIs
  shorten the F-Droid build and shrink the APK. Delta Chat and Element ship one APK per ABI; Mages uses `enableApkSplits`.
- Release profile: `opt-level="z"` or `"s"`, `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip="symbols"`.
  **APK size [E]:** a reqwest+rustls+rusqlite(bundled)+serde cdylib lands roughly 3 to 6 MB per ABI; JNA adds about
  1 to 2 MB of `libjnidispatch.so` per ABI. Compose + Material 3 adds about 4 to 8 MB. Expect a 10 to 20 MB APK per ABI
  for Combo A. Measure with a spike.

---

## 3. Android UI options

Widgets baseline [V/M]: a home-screen widget is an `AppWidgetProvider` (a `BroadcastReceiver`) declared in the manifest,
with UI expressed as `RemoteViews`. Jetpack Glance (`androidx.glance:glance-appwidget`, 1.2.0 stable, 1.3.0-alpha02
current alpha; Apache-2.0, from Google Maven, no GMS dependency [M]) gives a Compose-like API that compiles down to
`RemoteViews`. There is no way to write a widget in Rust, Flutter, Slint or a webview: the provider class and its
rendering must be Kotlin/Java. The only choice is how much Kotlin you write, and how the widget gets its data
(recommended: the core writes a small "widget snapshot" (next N events) to app storage after every sync, and the widget
reads that; the widget does not need to load the whole core).

### 3a. Kotlin + Jetpack Compose + Glance, Rust via UniFFI (Combo A)

- Widgets: first-class (Glance). Material 3 dynamic color works from API 31 (Material You) and falls back gracefully;
  on AOSP-based ROMs (Lineage, Graphene, /e/, Calyx) dynamic color follows the wallpaper in the same way as stock.
- F-Droid: most-proven path (Element X, Delta Chat, Cfait, Fatto, Ruslin). AndroidX comes from Google Maven; F-Droid's
  scanner accepts AndroidX in practice. [M]
- Custom ROMs: it is plain AOSP APIs; no Play Services; no dependency on FCM. Best fit.
- Performance: native; Compose startup acceptable, enable baseline profiles if the build allows (needs extra tooling).
- Code sharing with desktop: the core (all logic) only. UI is separate. Mitigation: keep UI thin and screens few.
- Cost: you need Kotlin/Compose competence in addition to Rust.

### 3b. Compose Multiplatform (CMP) shared with desktop JVM (Combo B)

- CMP 1.12.2 stable (2026-10-15), 1.13.0-beta01 (2026-10-23); active. Android target is Jetpack Compose itself, so Glance
  widgets remain Kotlin in the Android module as in 3a. [V]
- Precedent: Mages (CMP + UniFFI Rust core) ships on F-Droid, AUR, Flathub, Snap, AppImage. [V]
- Desktop costs: needs a JVM (bundled JRE via jpackage adds roughly 40 to 80 MB to the package [E]; idle RAM roughly
  150 to 300 MB [E]), startup noticeably slower than native. UniFFI Kotlin bindings use JNA, which also works on desktop JVM,
  but you must ship the core `.so/.dll/.dylib` per platform.
- Linux/Wayland: Compose Desktop renders through Skiko over AWT; as of today it runs on X11/XWayland on Sway. Native
  Wayland for Skiko exists only as an open PR for the **Kotlin/Native Linux** target (skiko #1224, opened 2026-07-04),
  not the JVM target. JetBrains moved its IDEs to native Wayland in 2026.1 (WLToolkit) but that does not change Skiko. [V
  for the issue/PR; the XWayland conclusion is inferred]. Expect fractional-scaling blur or quirks on Sway. Accessibility on
  Compose Desktop/Linux is weak. [M]
- Material 3 dynamic color is Android-only; desktop gets a static theme.
- Code sharing: highest of any native-quality option (UI plus ViewModels). The Rust CLI/daemon/MCP remain separate anyway.

### 3c. Flutter + flutter_rust_bridge (+ `home_widget`)

- frb 2.13.0 active (5.5k stars). Flutter apps on F-Droid are common, but the build is heavy: RustDesk's recipe needs
  a custom engine for x86, a 4-hour timeout and 21 GB. [V]
- Widgets: `home_widget` only bridges data; the widget UI and provider are still native Kotlin/XML. [M]
- Third language (Dart). Flutter Linux desktop uses a GTK embedder; works under Wayland; theming is not native.
- Verdict: viable and the best "one UI everywhere" cross-platform story outside Rust, but it adds Dart plus a heavy
  F-Droid build and does not remove the widget Kotlin. Not recommended here.

### 3d. Rust UI toolkits on Android

| Toolkit | Android status | Widgets | Verdict |
|---|---|---|---|
| **Slint** 1.18.1 | Supported for Rust and C++ via `android-activity` (`NativeActivity`, `minSdk 26`); backend feature `backend-android-activity-06`. Open issues as of late Oct 2026: IME composition handling (#14216, #14291), Android backend aborts on pointer scroll (`todo!()`, #14494, fixed/in-flight), Skia GL context leaks memory on Mali (#14397), SwiftKey text input problems (#9240), dynamic color not yet used (#14235/#14244), reduced-motion just landed (#14547). [V] | Needs a Kotlin `AppWidgetProvider` added next to the NativeActivity manifest (hybrid APK). Possible, but unusual. | Promising but the text input / scrolling surface is still being filled in, and it is the main interaction in a timetable/grades app. Rust-only F-Droid precedent exists (Slint Viewer, Skia compiled from source). Revisit in 6 to 12 months. |
| **Dioxus** 0.7.10 (0.8 alpha) | Mobile = system WebView hosting HTML (like Tauri); `dioxus-native`/Blitz (0.2.x) is the experimental native renderer and not a documented Android path. [V partially] | Kotlin | Same webview trade-offs as Tauri. |
| **Tauri 2** 2.12.5 | Android via system WebView; Kotlin glue; JS/TS frontend toolchain (Node) required. I found **no merged fdroiddata recipe for a Tauri v2 app**; several projects document F-Droid prep with reproducibility issues (Tauri CLI uses the newest NDK in `$ANDROID_HOME/ndk` regardless of `NDK_HOME`; asset-hash ordering fixed in 2.12.1). [V from secondary PRs] | Kotlin | Not recommended for F-Droid + widgets + custom-ROM WebView variance (GrapheneOS ships Vanadium, Lineage ships its own System WebView). |
| **egui/eframe** 0.37 | Works through `android-activity`; immediate mode, non-native look, IME quirks. [M] | Kotlin | Fine for tools, poor fit for a polished student app. |
| **Iced** 0.14 | No official Android support; experimental forks only; no release in 11 months. [V for release gap] | n/a | No. |

---

## 4. Desktop UI options

All entries run on top of the same Rust core in-process. Memory and size numbers are estimates [E] unless stated.

| Toolkit | Version / activity | Wayland (Sway) | Accessibility | Theming | Share UI with Android | Packaging | RAM / size [E] | Verdict |
|---|---|---|---|---|---|---|---|---|
| **GTK4 + libadwaita** via gtk-rs / **relm4** | gtk4 0.11.5, libadwaita 0.9.2, relm4 0.11.0; all pushed this week [V] | Native, best | Best on Linux (AT-SPI via GTK) | Follows system light/dark and accent; looks right on GNOME, acceptable on Sway with some tweaking (theme/portal settings) | None | Flatpak (GNOME runtime) is first-class; AUR/rpm/deb easy; Windows and macOS are possible but painful (gvsbuild / bundling GTK, non-native look) | 40 to 100 MB RSS, few-MB binary + system GTK | **Best Linux-first choice.** Cost: Windows/macOS later is hard. |
| **Slint** 1.18.1 | very active, 24k stars, GPL-3.0 / royalty-free / commercial licences [V] | Native via winit (femtovg / Skia / software renderers) | AccessKit on desktop incl. AT-SPI [M] | Built-in Fluent/Material/Cupertino/Cosmic styles; no true GTK theming | Possible in theory (`.slint` files on Android), immature today | Single binary; Flatpak/AUR/Windows/macOS all easy | 30 to 80 MB RSS, small binary | **Best cross-platform pure-Rust option.** Slightly foreign look on Linux. |
| **Compose Multiplatform Desktop** | 1.12.2 [V] | XWayland only for JVM [V/inferred] | Weak [M] | Material 3 static | **Highest** (same Kotlin UI) | jpackage deb/rpm/msi/dmg, Flatpak with JRE, AppImage | 150 to 300 MB RSS + JRE | Only if code reuse outranks "no heavy stack". |
| **Iced** 0.14 (COSMIC) | no release since 2025-12; repo active [V] | Native | Poor upstream (long-standing issue #552) [M] | Custom, COSMIC theme via libcosmic | None | Easy | 40 to 100 MB | Good for COSMIC desktop apps; weaker a11y. |
| **egui** 0.37 | active [V] | Native (winit) | AccessKit supported [M] | Non-native look | None for polished UI | Easy | Low | Tools and dev UIs, not a student app. |
| **GPUI** 0.2.2 (Zed) | on crates.io as standalone; Zed repo very active [V] | Wayland + X11 default features [V] | AccessKit integration exists in GPUI forks; upstream status unverified | Custom, `gpui-component` library exists [M] | None (no Android) | DIY | Low, GPU-rendered | Exciting but API churn tied to Zed; desktop-only. Not for v1. |
| **Dioxus desktop** 0.7.10 | very active, 0.8 alpha [V] | Webview (WebKitGTK) or experimental Blitz native | Webview a11y; Blitz has AccessKit (default feature) [V partially] | HTML/CSS (any look) | Via Dioxus mobile (webview) | OK | WebKitGTK footprint | Only if you want a web-style UI in Rust. |
| **Tauri 2** 2.12.5 | 112k stars [V] | WebKitGTK 4.1 under Wayland works; reported DMABUF renderer problems on some GPU/driver combos, workaround `WEBKIT_DISABLE_DMABUF_RENDERER=1` (single user report in the WebKitGTK/Tauri context) [V partially] | Webview a11y (good) | CSS, any look | Same web UI could run on Tauri mobile (but see 3d) | deb/rpm/AppImage/Flatpak, Windows/macOS easy | Not Electron (no bundled Chromium, 5 to 10 MB binary), but a full WebKit engine with 2 to 3 helper processes: 100 to 300 MB RSS; "Electron-lite" in spirit; GitHub issue #5889 reports WebKitGTK webviews using more memory than Electron in some measurements (varies by method). | Acceptable but conflicts with "no Electron-style stacks"; needs a JS toolchain. |
| **Qt via cxx-qt** 0.10 | active [V] | Excellent | Good (QtAccessibility, AT-SPI) | Follows Qt style (Breeze on KDE) | QML could be reused with Qt for Android, painful | Flatpak KDE runtime, Windows/macOS fine | Moderate | Mature toolkit, but C++/QML bridging weight and Qt on F-Droid is rare. |
| **Freya** 0.4.3 | active, small ecosystem [V] | Native | AccessKit [M] | Custom | None | Easy | Low | Too young. |

Conclusion for desktop: GTK4/libadwaita + relm4 for Linux first; fall back to Slint on desktop if Windows/macOS become
non-optional. Keep `usos-gui` strictly a view layer so it can be replaced.

---

## 5. Background sync and notifications on Android without FCM

- **Polling with WorkManager** (`androidx.work` 2.12.0): `PeriodicWorkRequest` minimum interval is 15 minutes; actual run
  time is decided by JobScheduler and shifts with Doze, App Standby Buckets (rarely used apps get fewer runs), battery saver
  and constraints. Treat the period as a hint. [V: Android docs]
- **Timetable use case needs very little polling**: schedules change rarely. Recommended cadence: sync on app open,
  on widget tap/refresh, and one periodic job every 3 to 6 hours with `NetworkType.CONNECTED`, exponential backoff; grades
  and announcements can be polled slightly more often (1 to 2 hours) if the user enables notifications.
- **Class reminders must not depend on network.** Compute the next N reminders from the cached timetable and schedule them
  with `AlarmManager`: `setExactAndAllowWhileIdle` needs `SCHEDULE_EXACT_ALARM` (user-grantable "Alarms & reminders" special
  access) or `USE_EXACT_ALARM` (auto-granted, intended for alarm/calendar apps; a timetable app is arguably a calendar app
  but stores like Play police this, F-Droid does not). `setAlarmClock` needs no permission and survives Doze but shows an
  alarm icon in the status bar. Inexact `setAndAllowWhileIdle` is the fallback when permission is denied. Reschedule on
  `BOOT_COMPLETED`, `MY_PACKAGE_REPLACED`, timezone change, and after every sync. [M]
- **Foreground services**: avoid. They need a persistent notification, and Android 15 limits `dataSync` foreground services
  to a few hours per day. Not justified for a pull API. [M]
- **Widgets**: update from the periodic worker and from alarms at class boundaries via `GlanceAppWidget.update()`;
  `updatePeriodMillis` in the widget XML is limited to 30 minutes minimum and is best left at 0. [M]
- **UnifiedPush / ntfy** [V for architecture]: UnifiedPush has a distributor app (ntfy, NextPush, Conversations, Sunup) that
  receives pushes, but a **server of ours must send** the web push request to the distributor endpoint (RFC 8030/8291, VAPID
  RFC 8292 since 2025). USOS does not push to us, so using it requires a relay that polls USOS on the user's behalf, which
  means handing it the user's OAuth tokens. Reject as default. As an opt-in power feature it fits well with our own
  **daemon**: a user can run `usos-daemon` on a home server/VPS, which polls USOS and notifies the phone through their own
  ntfy/UnifiedPush topic, and also feeds MCP/iCal. That keeps tokens under user control.
- **Custom ROMs**: LineageOS, /e/, CalyxOS and GrapheneOS follow AOSP Doze/standby rules; they lack OEM "app killers" that
  plague MIUI/ColorOS/OneUI, which makes WorkManager and alarms more reliable than on stock vendor ROMs. [M] GrapheneOS
  adds per-app Network and Notification toggles and has no Play Services by default; a user can revoke network access, so
  show clear errors rather than silently failing. GrapheneOS's usage page notes that Play Services (if installed) needs a
  battery-optimization exception for push; that is irrelevant for us. [V] Advise users to exempt the app from battery
  optimization only if they report missed reminders; do not request it by default.

---

## 6. Recommended full-stack combinations

### Comparison

| | **A. Native Android + GTK desktop** | **B. Compose Multiplatform both** | **C. Slint everywhere** | D. Tauri 2 both |
|---|---|---|---|---|
| Core | Rust workspace | Rust workspace | Rust workspace | Rust workspace |
| Android UI | Kotlin, Compose M3, Glance | Kotlin CMP, Glance in Android module | Slint (NativeActivity) + Kotlin shim for widgets | WebView + Kotlin for widgets |
| Desktop UI | GTK4/libadwaita (relm4) | CMP Desktop on JVM | Slint | Tauri (WebKitGTK) |
| Bridge | UniFFI Kotlin (Android only) | UniFFI Kotlin (Android and desktop JVM, JNA) | none (Rust direct) | Tauri IPC / commands |
| Languages | Rust + Kotlin | Rust + Kotlin | Rust + Slint DSL (+ small Kotlin) | Rust + TS + Kotlin |
| UI code reuse | none; core only | high (about 80% UI) | highest in theory | high (web) |
| Widgets | Glance, native | Glance, native | hybrid, awkward | Kotlin, awkward |
| F-Droid precedent | strong (5+ apps) | Mages (small) | Slint Viewer (Skia from source) | none found |
| Custom-ROM behaviour | best | best on Android | risk: IME/scroll/GL issues open | WebView varies per ROM |
| Linux desktop quality | best native | JVM, XWayland | good | OK (WebKitGTK) |
| Windows/macOS | hard (GTK) | easy | easy | easy |
| Footprint desktop [E] | smallest | largest | small | medium |
| Maturity risk | low | medium | high (Android) | medium-high |

### Combo A (recommended)

Rust core + UniFFI + Kotlin/Compose/Glance on Android + GTK4/libadwaita (relm4) on desktop, plus Rust CLI, daemon, MCP.

- Why: it meets every hard requirement with the least risk. Widgets are first-class, F-Droid recipes have many direct
  precedents (Cfait's recipe is nearly a template), custom ROMs are the target platform of AOSP APIs, desktop is the lightest
  and most native on Fedora Sway with proper AT-SPI accessibility, and all logic sits in one Rust library reused by
  five front-ends.
- Honest costs: two UIs to build and keep consistent; Kotlin competence required; GTK makes Windows/macOS packaging
  painful (accept Linux-only at first, or swap `usos-gui` for Slint later because the UI is a thin layer); UniFFI minor
  upgrades cost a regeneration/migration each time.
- Mitigation for UI duplication: keep screens few and driven by core view-models (the core returns display-ready records:
  grouped timetable days, formatted times, grade summaries), so both UIs are mostly rendering.

### Combo B (runner-up, if UI reuse outranks footprint)

Rust core + UniFFI + Compose Multiplatform for Android and desktop (Mages is the precedent), Glance for widgets.

- Why: one UI codebase, one language for the front-end, UniFFI bindings shared; fastest route to feature parity.
- Honest costs: JVM on the desktop (RAM, startup, bundle size), no native Wayland (XWayland on Sway), weaker Linux
  accessibility, desktop theming not native; Gradle-based desktop packaging for Flatpak/AUR is heavier. This is exactly the
  "heavy stack" the requirements ask to avoid, though it is not Electron. The CLI/daemon/MCP remain Rust either way, so
  the reuse win is limited to the GUI.

### Combo C (watchlist, not for v1)

Rust core + Slint on both desktop and Android (Kotlin shim for widgets).

- Why: one language, one UI definition, native Rust performance, small binaries; Slint is the most active Rust UI toolkit
  and has an F-Droid-built Android viewer.
- Honest costs: Android is where it is least mature (IME composition, scroll events, GL memory leak on Mali, dynamic color
  open as of the last two weeks); widgets need a hybrid APK with Kotlin anyway; accessibility on Android unclear; Skia must
  be compiled from source for F-Droid (long builds); licensing is GPL-3.0 for open source (fine here). Re-evaluate when
  the Android issues above close.

### D. Tauri 2 everywhere (not recommended)

No confirmed F-Droid precedent, a JS toolchain on the F-Droid buildserver, WebView variance across custom ROMs, widgets
still Kotlin, and on the desktop a full WebKit engine, which is the closest to what the requirements rule out.

### Pick

**Combo A**, with `usos-gui` kept thin so Slint (or CMP) can replace GTK if Windows/macOS become required. Start by
building the core + CLI + MCP first (no UI toolkit dependency), then the Android app (the platform with the hardest
constraints: F-Droid recipe, widgets, background work), then the GTK client.

Recommended first spikes (each under a day), because several numbers above are estimates:
1. `cargo ndk` + UniFFI hello-world APK built via a local F-Droid `fdroid build` recipe modelled on Cfait; check
   16 KB alignment and reproducibility (`apksigner`/`diffoscope` against a second build).
2. Glance widget reading a JSON snapshot written by Rust; check update behaviour on a LineageOS and a GrapheneOS device.
3. Measure idle RSS of a relm4 window vs Slint vs CMP Desktop with a trivial timetable view on Sway.
4. `rmcp` 3.8 stdio server exposing 2 tools over the core; test with Claude Code / another MCP client.

## Open items / not verified

- Exact Glance license/F-Droid scanner acceptance (believed fine; Element X and others use AndroidX).
- Real APK and RAM numbers (all [E]).
- Native Wayland status of Compose Desktop on JVM (inferred from Skiko issue tracker, not from a JetBrains statement).
- GPUI upstream accessibility status (evidence came from forks).
- USOS-specific details (token lifetime/scopes, consumer-key registration policy) were not researched here.
