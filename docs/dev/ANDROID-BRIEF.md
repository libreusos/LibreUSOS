# LibreUSOS: Android development brief

You lead the **Android app** of LibreUSOS, an unofficial open-source USOS client. You decide, plan, brief subagents,
review their results, and talk to the user. Follow `~/.claude/CLAUDE.md` strictly: subagent routing with `model` and
`effort` on every Agent call, the Fedora Atomic and toolbox rules, the delete rules, and code in English.
Delegate implementation, tests, builds and reviews to subagents.

## Read first

- `docs/dev/BRIEF.md`: project decisions and working rules. They apply here too: one step at a time, test, have a
  separate subagent review, commit, and hand off to a fresh session when context gets large.
- `docs/dev/PROGRESS.md`: the state of the Rust core. It is written by the backend session; don't edit it.
- `docs/research/tech-stack.md` (UniFFI, cargo-ndk, F-Droid recipes, WorkManager, widgets),
  `docs/research/existing-apps.md` (the official app's features and the parity matrix),
  `docs/research/usos-api.md` (login callback, polling for notifications).

## Decided

- **Android only**, distributed on **F-Droid**: buildable from source, no proprietary dependencies, **no Google Play
  Services/FCM**. Custom ROMs (LineageOS, GrapheneOS, /e/, CalyxOS) matter more than stock Android.
- **UI in Kotlin.** Only the Rust core is shared with desktop; no UI is shared. You reach the core through a new crate
  `crates/usos-ffi` (UniFFI, version pinned exactly) and cargo-ndk.
- Home-screen **timetable widgets are required**.
- UI languages are pl and en. The installation base URL is entered by hand for now; a dropdown of all
  universities comes later.
- Login: OAuth 1.0a in the browser, with a custom-scheme callback. The core already has
  `PendingLogin::begin`/`resume`/`parse_redirect` for this. Store the token in the Android Keystore.
- The consumer key for PolSl is in `.env` (gitignored). **Never print, log or commit the secret.** How the key gets
  into an F-Droid build is undecided: offer the options, and default to entering it in-app during development.

## Was open, now decided (2026-11-04)

Compose with Material 3 dynamic color; MVVM with manual DI; Glance widgets fed from the core cache; WorkManager
sync every 6 h; no reminders for now; UnifiedPush later; minSdk 26; package `io.github.voxix.libreusos`;
license GPL-3.0-or-later. The list below is kept for history.

## Originally not decided

- Compose vs Views, Material 3/dynamic color, navigation and architecture (MVVM, MVI and so on).
- Widgets: Glance vs RemoteViews. Data source: core cache vs the iCal share URL.
- Background sync: WorkManager interval, exact alarms for class reminders, opt-in UnifiedPush through the user's own
  `usos-daemon` and ntfy.
- minSdk, the package id (`org.libreusos...`?), and the license (still undecided overall).

Ask through the normal question prompt (AskUserQuestion), with a recommendation as the first option. The user
lifted the earlier no-prompts limit on 2026-11-04. The user works through Ghostex and its terminal only, with no
normal browser. Ask for as little as possible, and say exactly what to do when you need them
(for example, the one Android login in the emulator).

## Environment (ready, verified 2026-11-04)

- **`android` toolbox** has JDK 21, SDK `~/Android/Sdk` (android-36, build-tools 36.0.0, NDK 28.2.13676358, cmake),
  rustup under `~/Android/{rustup,cargo}` with the 4 Android targets, cargo-ndk 4.1.2, and fdroidserver
  (`~/Android/fdroid-venv`). In scripts use `toolbox run -c android bash -lc 'source ~/Android/env.sh && ...'`.
  Use the Gradle wrapper, which is checked in.
- **Emulator** runs natively on the host: `~/Android/emulator.sh` (add `-headless` for no window; `AVD=<name>`
  picks another AVD). The AVD is `aosp` (android-36, AOSP without Play, pixel_6) and boots in about 25 s. Always
  use `-gpu host`; the SwiftShader modes segfault here. adb lives in the toolbox
  (`~/Android/Sdk/platform-tools/adb`) and sees `emulator-5554`.
- **USB:** udev rules are installed on the host, but a real phone hasn't been tested yet.
- **Host cargo** is Fedora's native toolchain, for host builds and tests. Android cross-builds use the toolbox
  rustup. Never write into `~/.cargo/bin` from the toolbox.

## Ownership and git

- The backend session (`S17:P3l0h:G9d2f`, "LibreUSOS backend dev") keeps working in the same tree on `usos-core`,
  `usos-store`, `usos-cli`, `usos-mcp` and `usos-daemon`.
- **You own `android/` and `crates/usos-ffi/`.** You may add `usos-ffi` to the workspace members in the root
  `Cargo.toml`.
- **Don't edit other crates.** If you need a core change, send the backend session a precise request with
  `ghostex agents send S17:P3l0h:G9d2f ...`, or agree with it which of you makes the change.
- Local git only, never push. Commit only your own paths (`git add <paths>`, never `git add -A`), in small commits.
  Keep `docs/dev/ANDROID-PROGRESS.md` (yours) up to date.

## Suggested first scope

1. Present the undecided options and get the user's choices.
2. Write `usos-ffi`, which exposes login (begin/resume), whoami, terms and timetable, and build it for x86_64 and
   arm64 with cargo-ndk.
3. Create a minimal Gradle project in `android/` that loads the `.so`. Log in once in the emulator, then show
   whoami and today's timetable. This proves the whole chain.
4. Then one feature per step: timetable screen, widget, grades, tests, and background sync with notifications.
   Add an F-Droid metadata draft and run `fdroid lint` early.

There is no main coordinating session any more (the user closed it on 2026-11-06 and wants no relay). Talk to the
user directly in your own chat. Report milestones there in 3 lines.
F-Droid key decision (2026-11-06): distribute through IzzyOnDroid or our own F-Droid repo, with the key injected by
CI as now; never commit the key; not the main F-Droid repo.
