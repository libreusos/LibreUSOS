# LibreUSOS: progress

## Done

- **Step 1: workspace, signer, login** (commit `0755397`)
  - `crates/usos-core`: OAuth 1.0a HMAC-SHA1 signer (RFC 5849 §3.4.1 and Twitter vectors pinned),
    reqwest + rustls client (no openssl in the tree, redirects disabled, URLs stripped from errors),
    typed `ApiError`, `Secret` wrapper with redacted Debug, token store (Secret Service via `keyring` 3.6
    `async-secret-service`; fallback `~/.config/libreusos/credentials.json` 0600, dir 0700).
  - `crates/usos-cli` (binary `usos`): `login` (loopback `http://127.0.0.1:<port>/callback`, `--oob` PIN
    fallback, `--url-file`, `--open-cmd`), `whoami`, `raw <module/method> k=v...`, `logout [--revoke]`.
  - Verified: 29 unit tests, clippy `-D warnings`, fmt. Live: consumer-signed calls and request_token work at
    PolSl, which accepts the loopback callback. Reviewed by a subagent; fixes applied.

- **Login done (2026-11-04)** in the Ghostex embedded browser via the loopback callback. The token is in
  Secret Service (gnome-keyring). `usos whoami` and `usos raw apisrv/consumer fields=token_scopes` work live,
  and all 13 requested scopes were granted.

- **Step 2: terms** (commit `f7f9210`): `usos terms [--current]`, global `--lang pl|en`. The user's terms
  come from `courses/user fields=terms`. The current term is the shortest one containing today (by
  `end_date`), else the nearest one starting within 30 days, else the most recently ended one (by
  `finish_date`, falling back to `end_date`). Live: `2026/2027-Z`. Dates are `chrono::NaiveDate` project-wide.
- **Core/CLI split** (commit `6d9ba83`, at the user's request): the core holds all reusable logic, in
  `auth::{login,loopback,consumer,scopes}`, `session` and `users`. The login API is
  `PendingLogin::begin(&Session, Callback)`, `wait_loopback`, `complete(verifier)`, and `resume` +
  `parse_redirect` for Android's custom-scheme callback. Errors are typed (`LoginError`). The core never
  prints; warnings are returned. The `loopback` feature is on by default. `usos-cli` is clap and output only.

- **Step 3: timetable** (uncommitted): `usos-core::timetable` (`Activity`, `Client::user_timetable(start, days)`,
  `timetable_range(from, to)`, `week_of`, `group_by_day`) and `usos plan [--date D] [--days N] [--today]
  [--tomorrow] [--next] [--json]`. Uses `tt/user` (student + staff in one). PolSl accepts the fields `type`,
  `start_time`, `end_time`, `name`, `url`, `course_id`, `course_name`, `classtype_id`, `classtype_name`,
  `group_number`, `building_id`, `building_name`, `room_id`, `room_number`, `lecturer_ids`, `unit_id`,
  `frequency`, `classgroup_profile_url`, `cgwm_id`, `sm_id`; it rejects `slot_number` and `related_user_ids`
  (400). `days` must be 1..=7 (0 and 8 give 400). `building_name.en` is empty at PolSl (the `LangDict`
  fallback gives the Polish name) and building names are long faculty names.
  `Activity.frequency` is now `Option<Frequency>` (typed enum, serialized as the raw USOS code, cache unchanged).

- **Step 4: grades** (uncommitted): `usos-core::grades` (`Grade`, `UnitGrades`, `CourseGrades`, `TermGrades`,
  `LatestGrade`, `Client::term_grades(term_ids)`, `Client::latest_grades(days)`, pure `average` / `weighted_average`) and
  `usos grades [--term ID] [--all] [--latest [DAYS]] [--json]`. `grades/terms2` gives, per term and course id,
  `course_grades` (array of exam reports, each `{session: grade|null}`) and `course_units_grades` (unit id ->
  array of exam reports). It has no names, so one `courses/user` call maps course id -> name and unit id -> class
  type. `grades/latest` takes `course_edition[...]|unit[id|classtype_id]` as extra fields. One more call,
  `courses/user_ects_points` (`{term: {course: "6.00"|null}}`, best effort: a failure leaves `ects` empty), gives
  `CourseGrades.ects`. One grade per course: per course-level exam report the latest session, only
  `counts_into_average` numeric grades (`3,5` and `5!` = 5.5; `ZAL`/`NZAL` excluded), max across reports. The CLI
  shows the ECTS-weighted average (courses without positive ECTS skipped), or the unweighted one when no weight
  is usable; `--json` has both. Exams without `exam_id` get a positional key (`#N`). Shared lenient serde
  helpers moved to `usos-core::lenient`; chrono got the `alloc` feature (core did not build alone without it).
  PolSl at 2026-11-04: first-year user, 7 course editions with grade reports in 2026/2027-Z and all sessions
  `null`; 2025/2026 terms empty; `latest` empty even for 3650 days. The grade fields and the `latest` shape are
  therefore pinned by doc-based fixtures, not by live values. `terms2` maps unknown terms to `null`. `private_comment`
  needs an administrative key and is not requested.

- **Step 5: course tests** (uncommitted): `usos-core::crstests` (`TestRoot`, `TestNode`, `NodeKind`, `NodeGrade`,
  `Client::course_tests(term_ids)`, pure `sum_points` / `sum_max`) and `usos tests [--term ID] [--json]` (default: the
  current term). Calls: `crstests/participant2` (one call; course edition, term, group restriction and the root node;
  called with `active_terms_only=false` so past terms are included; the core filters by term), then per test `node2` on the root with `subnodes_deep[id|type]` (the
  shape) and `node2` per node (concurrency 8) with `order`, `name`, `results_visible_to_students`,
  `task_node_details[points_min|points_max|students_points[points|comment|last_changed]]`, `folder_node_details[points_max]`
  and `grade_node_details[students_grade[grade_value[symbol|passes|decimal_value|name]|comment|last_changed]]`.
  Requests per run: 1 + (1 + nodes) per test (+ 1 `courses/user` for the current term in the CLI). Quirks: `subnodes_deep`
  ignores every inner field except `id` and `type` (so the shape holds ids only), and `subnodes[...]` rejects `subnodes`
  inside it (400 field_invalid, only primary fields), so a one-call tree with names is impossible; nothing batches
  `points_max` (only `node2`/`task_node`, single node), which is why each node costs a call, and the same call carries
  the user's points and grade, so `user_points`/`user_grade` are not used. `all_user_points` / `all_user_grades` also
  return results hidden from students and are avoided. `participant` (v1) and `node` are deprecated. Folders report only
  `points_max`; folder and root points are summed in the core (`sum_points`), the root maximum is `sum_max`. Empty
  `students_points` come back as an object of nulls. USOS does not run automatic grading, only lecturer-entered values
  appear. PolSl at 2026-11-04: first-year user, `participant2` is `[]`, so everything is pinned by doc-based fixtures plus
  the live shapes of public nodes (`node2` works without a token on any node id). Added the `futures-util` dependency
  (alloc only, already in the tree) for `buffer_unordered`; `opt_datetime` moved to `lenient.rs`, with a new `opt_f64`.
  `course_tests(&[String])` returns `CourseTests { tests, skipped }`: auth/network errors are fatal, other API errors drop
  a node or skip a test; nodes with `results_visible == false` are kept with `hidden: true`, excluded from sums, and a
  folder with a hidden node below reports the sum of the visible maxima. One concurrency limit covers all tests.

- **FFI-friendly core API** (refactor): public edge uses owned types only (`current_term` returns `Option<Term>`;
  `terms`, `term_grades` take `&[String]`; `clashes(&[Activity])`; `group_by_day` returns `Vec<DayPlan>`;
  `week_of` returns `DateRange`). All data models derive `Serialize` + `Deserialize` (same field names) and have
  round-trip tests. Token storage is the `TokenStore` trait (`load/save/delete`, `StoredToken` = `AccessToken`);
  default `KeyringFileStore`; `config.toml` moved to the optional `ConfigStore`. `Session::new(Arc<dyn TokenStore>,
  Option<ConfigStore>, base_url, consumer)`; `Session::open` keeps the old default behaviour.

- **Step 6: offline cache** (uncommitted): new crate `crates/usos-store` (rusqlite 0.41 `bundled`, so no system sqlite
  and it cross-compiles for Android; workspace `rust-version` raised to 1.89 because rusqlite needs it). `Store`
  (`Mutex<Connection>`, `Send + Sync`, sync API) is opened with `Store::open(&str)` (the caller picks a plain file path: `:memory:`, empty and `file:` URIs are
  rejected and SQLite is opened without the URI flag; the parent dir is created 0700, a new file 0600, an existing
  one is tightened to 0600/0700 when owned, `-wal`/`-shm` end up 0600; WAL) or `open_in_memory()`. Migrations are hand-rolled in
  `PRAGMA user_version` (`MIGRATIONS`, append only; a newer file gives `SchemaTooNew`); each step runs in a
  `BEGIN IMMEDIATE` transaction that re-reads `user_version`, and the WAL switch retries on BUSY, so two processes
  opening a new file at once are safe (tested with two threads). Every row has an `account`
  (`account_key(base_url, user_id)` = `<base_url>#<user_id>`). Tables: `activities` (account, starts_at, ends_at, date =
  start day, course_id, kind, json = core `Activity`; index (account, date, starts_at)), `activity_days` (account,
  date, fetched_at: one row per fetched day, so empty days count as fetched and "cached at" works per day or range;
  chosen over a ranges table because overlapping fetches need no interval merging), `snapshots` (account, kind, key,
  json, hash = sha256 of the JSON, fetched_at; kinds `terms`, `user`, `grades` and `crstests` keyed by term id) and
  `meta` (active account per base URL, so offline code finds its account without asking the server).
  `replace_activities(account, from, to, &[Activity]) -> ReplaceOutcome { stored, removed, changed }` replaces the
  inclusive date range in one transaction (activities starting outside it are ignored); `activities(...)` sorts by
  start; `activity_coverage(...)` gives days cached and the stalest fetch time. `put_*` returns `PutOutcome { changed,
  previous_fetched_at }`; `get_*` returns `Option<CachedTerms | CachedUser | CachedTermGrades | CachedCourseTests>`,
  each a concrete `{ value, fetched_at }` struct (no generic `Cached<T>` at the public edge; the generic code is
  private). A snapshot or row that no longer deserializes counts as a cache miss. `usos_store::repo` has, per kind,
  `refresh_*` (fetch, store), `cached_*` (store only) and `*_or_cached` (only `Error::Http` falls back; 401 and other
  API errors propagate; with an empty cache the original network error is returned), results carry `Source::{Live,
  Cached { fetched_at }}` and `changed`, plus `sync(client, store, today, weeks) -> SyncReport` (counts and change
  flags only). `RepoError` = `Core` | `Store` | `NotCached`. Tests include a loopback fake USOS server (live, then
  killed, then cached; 401 does not fall back; sync). CLI: `usos sync [--weeks N]` (default 3), global `--offline`
  and `--cache PATH` (`LIBREUSOS_CACHE`), file default `<data dir>/libreusos/cache.sqlite`; `plan`, `grades` and
  `tests` go through `cli/src/cache.rs::Data` and print `(offline, cached <time>)` on stderr when serving the cache.
  Review fixes: `repo::remember_identity(store, base_url, user)` (used by `login` with the user it learned and by
  `refresh_identity`) makes the account active; `repo::forget_account(store, base_url)` deletes the active account's
  rows and the `meta` entry, and `usos logout` calls it first (then `--offline` fails with "no cached account").
  Other accounts' rows stay; switching is `remember_identity` again. `Data::open` takes the active account from the
  cache and skips `users/user` (only `login` and `sync` refresh it; without one it asks the server).
  `usos-core::term_grades`: a network error from `user_ects_points` now propagates (other API errors still leave the
  weights out), so incomplete grades are not cached. `refresh_course_tests` merges when `skipped` is non-empty: live
  tests replace, cached tests whose root id is skipped are kept. `PlanResult` has `days_cached`/`days_total` and the
  CLI note says "N of M days cached" when they differ; live plan results are filtered with the same start-date
  predicate as `replace_activities`. `term_grades_or_cached`, `refresh_term_grades` and `cached_term_grades` take
  `&[String]` (one live call, stored per term; cached returns the terms that exist, `NotCached` when none), used by
  `grades --all`. `grades --latest` is online only. Retention is out of scope (`// shortcut:` in `store.rs`).


- **Embedded consumer key (2026-11-04)**: `crates/usos-core/build.rs` reads `USOS_CONSUMER_KEY`,
  `USOS_CONSUMER_SECRET` and `USOS_KEY_BASE_URL` (default PolSl) from the build environment, else the workspace
  `.env` (`CONSUMER_KEY`/`CONSUMER_SECRET` or the `USOS_` names), and writes them to a generated file in
  `OUT_DIR` (not `cargo:rustc-env`, which `cargo build -vv` would print into logs). A build without a key still
  succeeds (with a `cargo:warning`, never containing values). The key and secret come as a pair from one source (env,
  then `.env`), never mixed. After adding a `.env`, run `touch crates/usos-core/build.rs`: a missing file cannot be
  watched without rebuilding on every call. `USOS_NO_EMBEDDED_KEY=1` builds without one. API: `auth::consumer::embedded(base_url)`, matched on the
  normalized base URL (room for more universities). `ConsumerSource::Default` resolves: `Fixed` (caller's own key)
  > runtime env vars > embedded; the config file has no consumer fields, so it is not consulted. The runtime
  `.env` loading is gone (`Session::open(base_url)`). `usos-ffi` gained `UsosClient::with_embedded_key` and
  `has_embedded_key(base_url)`.
  - **CI**: provide `USOS_CONSUMER_KEY`, `USOS_CONSUMER_SECRET` and `USOS_KEY_BASE_URL` as CI secrets in the build
    environment (cargo-ndk passes them through to `build.rs`). Never echo them in job logs.
  - **Open issue**: an F-Droid build server will not have these secrets, so F-Droid needs its own solution later:
    a key the user enters in the app (`UsosClient::new` with a `Fixed` consumer), or an F-Droid-specific key via a
    srclib or prebuild step.
- Android core requests (main session): (1) TLS roots without JNI. Cargo feature `usos-core/webpki-roots` (pass-through
  `usos-cli/webpki-roots`), always on for `target_os = "android"`: the client calls reqwest's `tls_certs_only` with the
  Mozilla roots from `webpki-root-certs` (DER, already in the lock; `webpki-roots` TrustAnchors can't be fed to reqwest
  without a hand-built `rustls::ClientConfig`), so `rustls-platform-verifier` is never called and needs no JNI.
  Provider stays aws-lc-rs; no openssl, one rustls. (2) `Client::course_tests` and so every `usos_store::repo` future
  is `Send` (the root ids are collected before the `buffer_unordered` stream instead of mapping over `&TestRoot`);
  compile-time `assert_send` tests in `crstests.rs` and `repo.rs`.

## Next

1. Android (separate session, see `docs/dev/ANDROID-BRIEF.md`): it owns `crates/usos-ffi` and `android/` and sends
   core change requests to this session. Backend next: handle those requests; `usos-mcp` only when asked.

## Decisions

- Scopes requested: `studies|grades|crstests|cards|events|email|photo|student_exams|surveys_filling|`
  `mailclient|placement_tests|adm_documents|offline_access` (`login` re-checks them against `apiref/scopes`).
  At the user's request, these are **not** requested: `personal`, `payments`, `student_exams_write`, `other_emails`
  and `mobile_numbers`. Features that need them (payments, exam registration) are out of scope unless the user
  changes this.
- Ghostex: the CEF DevTools port here is 9334, not the default, and `ghostex browser mcp` gives no output when
  driven by hand. Use `ghostex browser open <url> --project-id P3l0h` and always paste the link in chat too.
- `oauth_callback`/`oauth_verifier` are signed in the Authorization header, `scopes` in the POST body.
- Desktop GUI: GPUI + gpui-kit, after the backend (see BRIEF.md).
- Architecture rule (user): `usos-cli` and every other front end is a thin layer over `usos-core`. No API, auth
  or storage logic goes in a front end.
- Clippy and rustfmt aren't on the host: run them with `toolbox run -c rust cargo clippy|fmt`.
- Android/UniFFI (main session): the public API of `usos-core` and `usos-store` stays UniFFI-friendly: owned types,
  no generics, lifetimes or `impl Trait` at the edge, concrete `Cached*` structs, typed errors. Token storage is
  pluggable (`TokenStore`; Android uses the Keystore), and the login callback can be a custom scheme
  (`Callback::Custom` + `parse_redirect`). The DB path is always passed by the caller.
- Sessions (2026-11-06): no main/relay session. This backend session reports milestones and blockers to the user
  in its own chat, and coordinates core changes directly with the Android session (`S17:P3l0h:G0nu4`).
