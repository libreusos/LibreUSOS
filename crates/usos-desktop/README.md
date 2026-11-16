# usos-desktop

The LibreUSOS desktop app (GPUI, GPL-3.0-or-later). It talks to USOS through `usos-core` and
`usos-store`; it adds no network or cache code of its own.

## Build and run

On Linux, GPUI needs the development packages for fontconfig, Wayland, xkbcommon, xcb, X11, Vulkan and
ALSA (the Debian list is `install_desktop_deps` in `ci/common.sh`). On Windows, the MSVC
toolchain and the Windows SDK.

```sh
cargo run -p usos-desktop
cargo test -p usos-desktop
cargo clippy -p usos-desktop --no-deps --all-targets -- -D warnings
```

`--no-deps` is needed because `usos-core` has pre-existing clippy findings. A consumer key
must be embedded at build time or set at run time (see `usos-core` `auth::consumer`).

Dependencies: `gpui-pre =0.3.11` (renamed `gpui`), `gpui-pre-platform` (lib `gpui_platform`,
the Wayland/X11/Windows backends), `gpui-component 0.9.0`, `gpui-kit-assets 0.9.0`.
ashpd is gated to Linux/FreeBSD so the Windows target builds (checked with
`cargo check -p usos-desktop --target x86_64-pc-windows-gnu`).

## CI and releases

`.github/workflows/release.yml` has two desktop jobs. They run on tags and manual runs like the other builds, and
`.github/workflows/ci.yml` runs them on pull requests that change `crates/`, `Cargo.*`, `ci/` or
`.github/workflows/`. A pull request build has no embedded consumer key and its artifacts are kept 7 days (tag and
manual builds require `USOS_POLSL_CONSUMER_KEY` and `USOS_POLSL_CONSUMER_SECRET`, as for the CLI). Tag runs
upload both archives to the release.

| Job | Script | Result |
| --- | --- | --- |
| `build-desktop-linux` | `ci/build-desktop-linux.sh` | `dist/usos-desktop-<version>-x86_64-linux.tar.gz`: binary, `LICENSE`, `pl.libreusos.LibreUSOS.desktop`, icon (`.png`, `.svg`) |
| `build-desktop-windows` | `ci/build-desktop-windows.ps1`, native on a Windows runner | `dist/usos-desktop-<version>-x86_64-windows.zip`: `usos-desktop.exe`, `LICENSE` |

Linux runs a release build (the `test` job runs fmt, clippy and the workspace tests on pull
requests only). The binary is dynamic (GPUI
cannot be static) and links only libc and X11/xkbcommon: `libc`, `libm`, `libgcc_s`, `libxcb`,
`libxkbcommon`, `libxkbcommon-x11` (plus `libXau` and `libxcb-xkb` through them). The script fails on any
other direct dependency (checked here on a Fedora 43 build). Debian 12 (glibc 2.36) is the
build base, so the binary runs on newer distributions. Wayland, fontconfig, Vulkan and the XDG portal are
loaded at run time. Install: put `usos-desktop` on `PATH`, the `.desktop` file in `~/.local/share/applications` and
the icon in `~/.local/share/icons`.

Windows builds natively with MSVC and a static CRT, not as a mingw cross-build like the CLI:
`gpui-pre-windows` compiles its HLSL shaders in `build.rs` with the Windows SDK's `fxc.exe`, and only
when the build script runs on Windows. Cross-compiling stops with `couldn't find file
.../shaders_bytes.rs`, and `vkd3d-compiler` cannot compile these shaders (`StructuredBuffer`). The
script installs the Build Tools, NASM and CMake with Chocolatey if the runner lacks them, and fails if
the exe imports a VC runtime or MinGW DLL. `Cargo.lock` is checked with `--locked` on both jobs.

Packaging files added for the Linux archive: `packaging/linux/pl.libreusos.LibreUSOS.desktop` and
`packaging/linux/pl.libreusos.LibreUSOS.png` (256 px, the LibreUSOS logo, GPL-3.0-or-later, copied
from `design/logo/png/libreusos-256.png`); the SVG icon is `assets/logo/libreusos.svg`. The app id
`pl.libreusos.LibreUSOS` matches `APP_ID` in `main.rs`. The Windows exe has no embedded icon yet.

## Module map

| Module | Role |
| --- | --- |
| `main.rs` | Start-up, key bindings, window |
| `shell.rs` | Window layout: sidebar or rail, overlays (account menu, confirm, palette), login gate |
| `nav.rs` | `Route`, `Group`, history, keyboard actions. `Nav::go(route, cx)` changes page |
| `page.rs` | `Page::new(title)...`: page header, banners and body for a screen |
| `palette.rs`, `palette/matching.rs` | Ctrl+K command palette: items from pure `items()`, fuzzy match that ignores case and Polish diacritics |
| `shortcuts.rs` | The shortcut list Settings shows, and the F1 / `?` / Ctrl+/ bindings |
| `view_util.rs` | Shared pure helpers for the view models (`fold`, `pick`, dates) |
| `demo.rs` | Debug builds only: `USOS_DEMO_GO`, `USOS_DEMO_TERM`, `USOS_DEMO_COURSE`, `USOS_DEMO_TEST` steer the app for screenshots |
| `chrome.rs` | Window frame and drag area |
| `state/` | `AppState` (session, sync, cached data, login), `data.rs` snapshot and sync status, `login.rs` presets and error mapping |
| `settings.rs` | `Prefs` (theme, accent, language, motion, contrast), saved to `<config dir>/libreusos/desktop.toml` |
| `theme/` | Tokens (`ThemeExt::tokens`), gpui-component mapping, system dark/accent watching |
| `i18n/` | `Text::new(pl, en)`, `I18nExt::t`, shared `strings` |
| `components/` | `Btn`, `Banner`, `Badge`, `Segmented`, `Switch`, `Kbd`, `MenuChip`, `GradeTile`, `EmptyState`, progress bar, typography helpers |
| `assets.rs` | Embedded fonts, icons (`icon(name, size, filled)`), logo; licences in `assets/NOTICE.md` |
| `screens/` | One view per route |

## State

`AppState::global(cx)` is an `Entity<AppState>`. Read `state.data` (a `Snapshot`: user, plan,
terms, grades, tests; each `Option` until fetched or cached), `state.sync` (`SyncStatus`) and
`state.auth`. A view that must repaint on data changes calls `cx.observe(&AppState::global(cx), ...)`.
`AppState::sync` refreshes; Ctrl+R and F5 call it.

## Screens

| Screen | Status |
| --- | --- |
| Login (d-01, d-17, failures) | Done |
| Grades (d-04, d-13) | Done: term picker, course table, detail pane, summary (`screens/grades.rs`, view model in `grades/model.rs`, `grades/term.rs`). Grade strings are shown as USOS gives them |
| Tests (d-05) | Done: test tree, points, hidden and group-only results (`screens/tests.rs`, `tests/model.rs`) |
| Settings (d-06, d-16, d-19, d-21) | Done: appearance, accent, accessibility, language, account, sync, shortcuts, about. Notifications are laid out but disabled with "wkrotce" / "soon" |
| Command palette (d-08), shortcuts (d-21) | Done; entries that need unbuilt screens are disabled with "wkrotce" / "soon" |
| Today, Timetable | Done: now card, day timeline, week grid, class detail (`screens/{today,week}.rs`) |
| Deadlines, Registrations, Surveys, Find a timetable, Payments, News, Messages | Shown disabled with "wkrotce" / "soon" (need core support) |

To implement a screen, replace the body of its file and keep the struct name and `new()`.
Add strings to `i18n/strings.rs` (or local `Text` consts) in both languages.

## Screenshots

Headless: run `sway` with `WLR_BACKENDS=headless`, start the app on that `WAYLAND_DISPLAY`, and use
`grim`. Samples are in `dist/desktop/`. A headless compositor has no input devices, so a debug build
reads `USOS_DEMO_GO=grades|tests|settings|appearance|notifications|shortcuts|palette` (see `demo.rs`).
`cargo test -p usos-desktop -- --ignored seed_demo_grades` fills a throw-away cache
(`XDG_DATA_HOME=...`) with invented grades and tests.
