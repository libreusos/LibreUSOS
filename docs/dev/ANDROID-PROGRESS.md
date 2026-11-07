# LibreUSOS Android: progress

Owned by the Android session (`docs/dev/ANDROID-BRIEF.md`).

## Done

- **usos-ffi** (`crates/usos-ffi`): UniFFI `=0.32.2` bindings, proc-macro only, Kotlin package `org.libreusos.core`.
  - `UsosClient::with_embedded_key(base_url, cache_path, token_store)` uses the consumer key that usos-core embeds at
    build time (from `USOS_CONSUMER_KEY`/`USOS_CONSUMER_SECRET` or the workspace `.env`). `has_embedded_key(base_url)`
    tells the app whether this build has one. The secret is never exposed: no getter, not in errors or Debug.
  - `UsosClient::new(base_url, consumer_key, consumer_secret, cache_path, token_store)` stays for other installations
    with a user-supplied key.
  - The app drops in-app key entry for PolSl; it asks for a key only when `has_embedded_key` is false.
  - It offers `has_token`, `begin_login` returning a `PendingLogin` (`authorize_url`, `state`, `complete(redirect)`),
    `resume_login(state)`, `whoami`, `terms`, `timetable`, `cached_timetable`, `sync` and `logout`, plus
    `installation_time_zone()`.
  - Network calls are `suspend` (tokio runtime).
- **Kotlin contract:** call everything from `Dispatchers.IO`, because futures are polled on the caller's thread.
- **Dates:** `YYYY-MM-DD`, local date-times as `YYYY-MM-DDTHH:MM:SS` in the installation zone (Europe/Warsaw),
  `fetched_at` as epoch seconds UTC.
- **Secrets:** `PendingLogin` state must survive process death while the browser is open. Kotlin persists it, and the
  `TokenRecord`, encrypted only, and never logs them.
- **Login:** retryable. A failed exchange keeps the pending state. Once the token is saved, problems come back as
  warnings, not errors.
- **Logout:** deletes the token first, then wipes the cache on a best-effort basis.
- **`sync`:** single-flight, runs on the shared tokio runtime (core futures are `Send` since 51476e7).
- **Verified:** 16 host tests, clippy `-D warnings`, fmt. The cargo-ndk release build for x86_64 and arm64-v8a
  succeeded (API 26, `CARGO_TARGET_DIR=target/android`) before the review fixes; re-run after disk is freed.
  Reviewed by a subagent and fixes applied.

- **Rebuilt after the fixes** (lean, debug x86_64 only): host build plus 17 tests pass (one checks `has_embedded_key` for PolSl), the Android `.so`
  (about 103 MB, unstripped debug) builds, and the Kotlin bindings regenerated (now with `hasEmbeddedKey` and `withEmbeddedKey`). The cargo-ndk build in the
  toolbox embeds the key: build.rs reads the workspace `.env` by file path, so no env forwarding is needed.
- **Bindgen gotcha:** `uniffi_bindgen`'s askama derive fails when `~/.cargo` is reached through the `/home`
  symlink. Run it with `CARGO_HOME=/var/home/c0bra/.cargo` and a scratch `CARGO_TARGET_DIR`:
  `cargo run -p usos-ffi --features bindgen-cli --bin uniffi-bindgen -- generate --library <.so> --language kotlin
  --out-dir target/uniffi-kotlin`.

## Open

- **Android TLS:** done in core 51476e7 (webpki roots on Android, no JNI).
- **`loopback` feature:** usos-core's default `loopback` feature stays on via the workspace dependency. This is
  harmless.
- **Undecided UI and project options:** see the brief. The user has not answered yet.
- **Disk:** `/var/home` is full; builds are blocked.

## Decisions (user, 2026-11-04)

- **UI:** Jetpack Compose with Material 3, dynamic color on Android 12+ and a fixed palette below that.
- **Architecture:** MVVM (ViewModel + StateFlow), Navigation Compose, dependencies wired by hand (no Hilt).
- **Widgets:** Glance, reading from the core SQLite cache.
- **Sync:** WorkManager every 6 h, plus a refresh on app open and pull-to-refresh. The interval can be changed in
  settings.
- **Class reminders:** not now.
- **Push:** later, as opt-in UnifiedPush through `usos-daemon` and ntfy, once polling works.
- **minSdk:** 26. **Package id:** `io.github.voxix.libreusos`. **License:** GPL-3.0-or-later, for the whole project.
- **Consumer key:** embedded at build time by usos-core (main session decision). It never goes in Kotlin or Gradle.

## Milestone: the whole chain works (2026-11-05)

- **Live login on the emulator:** the `aosp` AVD ran the debug APK and logged in live against PolSl. The Home screen
  showed the user's name and today's three classes from live data. Release builds and arm64 have not been
  checked on a real phone yet.
- **Remote login recipe:** the user can't see the emulator, so login goes through the Ghostex browser.
  1. In the app, tap Log in. Read the request token from the browser's address bar with `uiautomator dump`.
  2. Run `python3 android/tools/catch_oauth_callback.py <out-file>`. It watches CEF DevTools on port 9334 and saves
     the `libreusos://` redirect with mode 0600.
  3. Open `https://usosapi.polsl.pl/services/oauth/authorize?oauth_token=<T>&interactivity=minimal` with
     `ghostex browser open ... --project-id P3l0h`, and the user logs in.
  4. Deliver the redirect: `adb shell "am start -a android.intent.action.VIEW -d '<url>' io.github.voxix.libreusos"`.
- **Next UI fixes, as seen live:**
  - Fixed (shared `ActivityCard`, checked 2026-11-06): the title is the course name; the class type is a chip.
  - Still open: the building line is the full faculty name (it fits on one line on the emulator); a short building code would be nicer.
  - Fixed: overlapping classes get an "Overlaps" chip, and non-weekly ones a frequency chip.

## Timetable tab (64fb547)

- **Layout:** a NavigationBar switches between Today and Timetable. The Timetable tab has a Mon–Sun strip, a
  HorizontalPager over days, week arrows that use `targetPage`, and a Today button.
- **Loading:** cached data shows first, then live data. Pull-to-refresh joins a fetch already in flight. Live data
  older than 15 min is refetched on resume. A failed refresh keeps the old data and marks it not live.
- **Cards:** the course name is the title. Chips show the class type, the group, and "Overlaps" (from the core
  `clashes`). A frequency chip appears only for non-weekly classes; it comes from the typed core `Frequency`
  (7ae14ba), mirrored as a UniFFI enum.
- **Week parity:** USOS has none. Each `tt/user` entry is a dated occurrence.
- **Fixed:** the Today screen's "fetched" label now uses the installation zone (`fetchedLabel` in data/Time.kt).

## Widget and background sync (2026-11-06)

- **Sync:** a `SyncWorker` (CoroutineWorker) runs core `sync(today, 3 weeks)` on IO. Unique periodic work every 6 h
  (KEEP; needs network and a battery that is not low) is queued after login and cancelled on logout. A one-off sync
  runs on app start. On Unauthorized it stops the schedule and the widget refresh. Network errors retry with backoff,
  up to 5 attempts. WorkManager starts on demand through `Configuration.Provider`. The app, the worker and the widget
  share one `UsosClient`.
- **Glance widget "Today/Next":** shows today's remaining classes from the cache, otherwise the next day with classes,
  with "cached HH:mm" in Europe/Warsaw. Tapping a class opens the Timetable tab. Logged out or empty: a message, and
  a tap opens the app. It checks `hasToken` before it reads the cache. Dynamic color, resizable 2x2 to 4x2, en and pl.
- **Refresh:** after each sync, at login and logout, at midnight and at each class end. This uses an inexact,
  idempotent `WidgetRefreshWorker` chain with no exact-alarm permission, plus a `TIME_SET` receiver.
- **Verified on the emulator:**
  - Widget placed from the picker, with live classes.
  - The start-up sync redrew it.
  - Tapping a class opens the Timetable tab.
  - The one-off sync and the refresh worker returned SUCCESS.
  - No crashes; unit tests pass.
  Reviewed by a subagent and 10 fixes applied.
- **Not exercised yet:** the 6 h periodic run firing, the logout, Unauthorized and Network paths on a device, and the
  compact 2x2 layout. API 26–30 pickers get no preview image (`previewLayout` is API 31+).

## Grades tab (2026-11-06)

- **FFI:** `grades(term_ids)` (live, writes the cache, falls back to it offline; each `TermGrades` has its own
  `source`), `cached_grades(term_ids)` (NotCached when nothing is cached) and `cached_terms(today)`. The records are
  `TermGrades`, `CourseGrades`, `UnitGrades` and `Grade`, which keeps the raw `symbol`, `passes`,
  `counts_into_average`, `session` and dates. All term ids go in one POST to `grades/terms2`.
- **UI:** a third tab, Grades ("Oceny").
  - Terms are listed newest first. The term average and ECTS-weighted average appear only when core returns them.
  - Courses sort by locale `Collator`. Each course card lists its sittings grouped by exam report, plus unit grades
    (lab and so on).
  - The card tint is PASS if any report's latest sitting passed, FAIL if all failed; with no course grade it falls
    back to the unit grades.
  - Loading: cached data first, then live, with pull-to-refresh and an offline or failed note. The load state
    transitions are a pure reducer (`GradesState.kt`).
- **Verified:**
  - 21 FFI tests and 54 Android unit tests pass.
  - On the emulator: the real account has no grades yet ("No grades yet."). The cards, the offline note and pl were
    checked with a temporary fake cache row, since removed.
  - Reviewed by a subagent and 9 fixes applied.
- **Not seen yet:** real grade data from a live account.

## Tests view (2026-11-06)

- **FFI:** `course_tests(term_ids)` returns `CourseTestsResult { terms, failed_terms }`. It is live, writes the cache and
  runs one term at a time: a failed term falls back to that term's cache, else its id goes to `failed_terms`; only
  Unauthorized fails the whole call. `cached_course_tests(term_ids)` skips uncached terms (NotCached when none is
  cached). Records: `TermTests`, `CourseTest`, `TestNode` (recursive, with `TestNodeKind` and `NodeGrade`).
- **UI:** a "Grades | Tests" `PrimaryTabRow` inside the Grades tab (no fourth bottom item).
  - Terms newest first, courses by `Collator` (then root id), folders collapsible (course open, folder closed by
    default; choices kept as a set of flipped keys and pruned on every live result). Row keys end in the sibling
    index, so duplicate API ids stay unique.
  - Points come from the core totals only: `pointsLabel(points, max)` ("x / max"), hidden when both are null. Hidden
    nodes show "hidden by the lecturer" and dimmed points; "For selected groups only" marks limited courses.
  - `formatPoints` rounds HALF_UP to 2 decimals and never shows -0.
  - Loading: all cached terms first, then live for the 2 newest terms plus any term with no cache. A failed or
    skipped term shows the "some couldn't be loaded" note next to the partial result. Reducers are pure
    (`TestsState.kt`); the previous skipped count is kept when the answer was an offline fallback.
  - `TermsRepository` (in `AppContainer`) is shared with Grades: single-flight terms call, in-memory result per
    today-date, cleared on logout and on session expiry.
- **Verified:**
  - 26 FFI tests and 25 Tests unit tests (16 model, 9 state) pass; clippy `-D warnings` and fmt are clean.
  - On the emulator with the real account: "No tests yet." (the account has none). Trees, hidden rows, offline note and
    pl were checked earlier with a temporary fake cache row, since removed.
- **Not seen yet:** real test data from a live account.

## Distribution (2026-11-06)

- **Decision (user):** IzzyOnDroid or our own F-Droid repo, not the main F-Droid repo. CI or the local env injects the
  consumer key at build time (usos-core's build.rs); it is never committed.
- **Metadata:** `android/fdroid/io.github.voxix.libreusos.yml` (passes `fdroid lint`). Store text is in `fastlane/metadata/android/{en-US,pl-PL}`
  at the repo root, the only place IzzyOnDroid and fdroidserver read.
- **Signing:** release is signed when `LIBREUSOS_KEYSTORE`, `LIBREUSOS_KEYSTORE_PASSWORD`, `LIBREUSOS_KEY_ALIAS` and
  `LIBREUSOS_KEY_PASSWORD` are set; otherwise the APK is unsigned.
- **ABIs:** `gradle.properties` no longer pins them: debug x86_64, release arm64-v8a, armeabi-v7a and x86_64;
  `-Plibreusos.abis=a,b` overrides.
- **Release command:** `cd android && ./gradlew assembleRelease` (with the key and signing env vars set).
- **Not tested:** the release build (R8 and Rust release); only 2.4 GB was free, so it was skipped.
- **Missing:** a public repo, CI, the first tag, screenshots and an icon for fastlane.
