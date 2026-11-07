# Initial research: summary

Date: 2026-11-04. Detailed reports:

- [usos-api.md](usos-api.md): how USOS and the USOS API work, auth, modules, notifications, PolSl specifics
- [existing-apps.md](existing-apps.md): the official Mobile USOS 2.0 app (decompiled), unofficial clients and libraries, feature-parity matrix
- [tech-stack.md](tech-stack.md): stack options, comparison table, recommended combinations

## Key facts

**USOS API**
- One API per university. PolSl: API `https://usosapi.polsl.pl/`, USOSweb `https://usosweb.polsl.pl/`, CAS `usoscas.polsl.pl`. API version 7.3.1.
- Auth is OAuth 1.0a only, with no PKCE or public-client flow. Consumer keys are per installation and self-service at `<base>/developers/`.
- An access token granted with the `offline_access` scope is long-lived and revocable. That is the answer to persisting login: store it in the Android Keystore or Secret Service, and re-run OAuth on 401. The USOSweb session drops quickly because it is only a short cache of the CAS session.
- The official app holds a single embedded key (in a native lib) and lists 64 hard-coded installations. We don't have that key and must not reuse it.

**Notifications**
- USOS pushes only through FCM or webhooks, and neither fits us. Poll `events2/latest_notifications`, `grades/latest`, `crstests` and `tt` with WorkManager (Android) or the daemon (desktop).
- Class reminders are local alarms built from the cached timetable.
- `tt/upcoming_ical` works through a per-user share URL without a token. Widgets can use it as a fallback.

**mLegitymacja**
- The ID itself lives in the government mObywatel app.
- A client can only order or activate it, and only if the university grants the `cards` scope.

**Existing projects**
- Nothing open source covers everything.
- Learn from or reuse: `fake-usos` (mock server, MIT; use it for CI), `usos4j` (Java, MIT), OpenUSOS (Flutter, GPL, on F-Droid, 3 universities, relies on its own proxy), `usos-rs` (Rust, AGPL; learn from it, don't depend on it), `usos-widget-android` (iCal-based widget).

**PolSl quirks**
- Term ids look like `2026/2027-Z`, and several terms show as active at once, so pick the current term by date.
- `apisrv/info` is not public.
- A forbidden call returns 400 and `field_not_found` returns 500.
- Probe `services/apiref` at runtime, because versions differ between universities.

## Recommended architecture (combo A)

```
crates/
  usos-api      OAuth 1.0a signer (in-house, ~150 lines), typed client, LangDict, errors
  usos-store    SQLite cache and offline store (rusqlite), migrations
  usos-sync     sync engine, change detection -> notifications
  usos-cli      clap
  usos-daemon   headless: periodic sync, desktop notifications
  usos-mcp      local MCP server (rmcp, stdio)
  usos-ffi      UniFFI bindings for Kotlin (pin the exact UniFFI version)
android/        Kotlin + Jetpack Compose (M3, dynamic color) + Glance widgets + WorkManager
desktop/        GTK4 + libadwaita (relm4), kept thin over the core
```

- The core, the CLI and the MCP server come first: the fastest way to get value and to test against real data. Android comes next, then the GTK desktop.
- Runner-up: Compose Multiplatform for both UIs. It shares the most UI code, but means a JVM on the desktop and XWayland on Sway.
- Rejected: Tauri (no F-Droid precedent, needs a JS toolchain, full WebKit), Flutter (very heavy F-Droid builds).
- Watchlist: Slint (its Android support is still rough).

## Key strategy (consumer key)

1. Bring your own key: the user registers a key at their university's `/developers/` page and pastes it in. This always works.
2. Embedded per-university keys, agreed with each university's admin. Start with PolSl: Pomoctechniczna.USOS@polsl.pl, or usos@usos.edu.pl for the whole USOS consortium.
3. Keep the signer behind a trait, so the key source can change later.

## Dev environment (installed)

- `android` toolbox: JDK 21, SDK (android-36, build-tools 36, NDK 28.2, CMake), AVD `aosp` (AOSP image), rustup in `~/Android/{rustup,cargo}` with 4 Android targets and cargo-ndk, fdroidserver 2.4.5. `~/Android` takes 6.9 GB.
  - Aliases: `adb`, `sdkmanager`, `avdmanager`, `emulator`, `fdroid`, `android-sh`.
- `reverse` toolbox: jadx 1.5.6 at `/opt/jadx` (aliases `jadx`, `jadx-gui`).
- Emulator: runs natively on the host via `~/Android/emulator.sh` with `-gpu host`. Aliases: `emulator` for the GUI, `emulator-headless`; in scripts call the script directly, and pick another AVD with `AVD=<name>`.
  - AVD `aosp` (android-36 default x86_64, pixel_6) boots in about 15–19 s.
  - `adb` from the toolbox sees `emulator-5554`.
  - SwiftShader GPU modes (`swiftshader_indirect`, `guest`, `off`) segfault on this host, so don't use them.
- USB adb needs Android udev rules on the host (user action). Wi-Fi pairing (`adb pair`) needs no udev rules.

## Open questions

- PolSl: limits on scopes or keys per person, token and session lifetimes, whether `cards` is granted to third parties.
- The USOS API terms of service and the official stance on scraping. Ask usos@usos.edu.pl.
- License. If permissive, we can't vendor AGPL `usos-rs` code. GPL-3.0 matches OpenUSOS and is common on F-Droid.
