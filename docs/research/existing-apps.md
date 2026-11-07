# Existing USOS apps and libraries

Research date: 2026-11-04. Scope: official USOS mobile app (feature list, APK analysis), unofficial clients and
libraries, and what we can reuse. No secrets are reproduced here (consumer keys, Maps API key, etc. are deliberately
left out). The APK was fetched from apkpure (no Google account) and only statically decompiled, never executed.

## 0. Key takeaways for our project

1. The official app is **Mobile USOS 2.0** ("Mobilny USOS"), package `pl.edu.usos.mobilny`, publisher MUCI. It is a
   single multi-university app (64 hard-coded installations in v2.6.0a). Native Kotlin + Jetpack Compose, not
   Flutter/React Native.
2. It talks **directly to each university's USOS API installation** (`https://<uni-api-host>/services/...`), with
   OAuth 1.0a (HMAC-SHA1) and the browser-based authorize page. There is no MUCI proxy for data. The OAuth signing
   (including the consumer key) lives in a native library, `liboauth.so`.
3. **mLegitymacja is not displayed or signed by Mobile USOS.** Mobile USOS only orders it (USOS API
   `services/cards/make_mcard_order`) and shows the activation credentials (QR + password); the student then pastes
   them into the government **mObywatel** app, which holds the actual signed document. A third-party client cannot
   show the mLegitymacja, but can reproduce the whole ordering/activation/revocation flow if the installation grants
   the `cards` scope to our consumer key.
4. **The big blocker is API access, not features.** Every USOS installation issues its own consumer keys, many
   universities restrict key registration, and the official app uses keys that third parties do not have (see
   section 4). fake-usos's README states that access is getting harder and "substantial paperwork" is needed. The
   alternatives (private iCal link as used by the Android widget, scraping USOSweb) are limited. Plan: per-university
   user-supplied consumer key (like Tuios) plus a key registration helper, and a mock server for CI.
5. **Push**: official app uses FCM with USOS-side notifier (`services/events/register_fcm_token`). A client on F-Droid
   without Play Services cannot receive those pushes. We need polling (`events2`, `grades/latest`, `tt/user`) and
   optionally UnifiedPush; the server side would need a notifier we do not control.
6. A **Rust crate exists**: `usos-rs` (crates.io, AGPL-3.0-or-later, auth only, last updated 2024-03). Another
   workspace, `Adimac93/usos`, has a code generator from the API reference (no license file, no README).
7. The best reusable assets: **usos4j** (MIT, most complete Java client, good test suite), **fake-usos** (MIT mock API
   server inferred from usos4j) for CI, **MUCI/usosapi-python** and the official **API reference**
   (`services/apiref/*` self-describing methods) for generating a Rust client.

## A. Official app(s)

### A.1 Identification

| Item | Value |
|------|-------|
| Name | Mobile USOS 2.0 / Mobilny USOS |
| Package | `pl.edu.usos.mobilny` |
| Publisher | MUCI (Miedzyuniwersyteckie Centrum Informatyzacji), developed by "USOS Workgroup" |
| Latest seen | 2.6.0a, versionCode 2060001, updated 2026-10-04 on apkpure ("added some new universities and fixed timetable widget issues") |
| Platforms | Android (Google Play), iOS (App Store). Not on F-Droid. |
| History | Originally built at the University of Warsaw (EU-funded eUW project, service E3). Android 2018, iOS 2019. Before 2.0 each university had its own app/package (Mobile USOS UW, `pl.edu.uj.mobilny` UJ, UL, PW, PO, UMK `pl.umk.mobilny`, PCz `pl.pcz.mobilny`, ...). Universities tell users to uninstall the old ones and use 2.0. |
| Source | Not public. usos.edu.pl states universities may reuse the project version as open source with or without support, but no repo was found (github.com/MUCI has only jquery-usos, usosapi-python, usosapi-browser, emrex-elmo-extensions). |
| Accessibility | A 2020-21 audit found it compliant with the accessibility act (per UW). |

Per-university branded apps: none found beyond the legacy per-university builds above. The 2.0 app is the same
binary for everyone; it picks a university at first launch (`InstallationSelectActivity`). Some installations get
extras (WordPress news feeds for UW and MUP Oswiecim, NFC strategy for some, `muci` demo installation).

### A.2 Feature list (store listing + APK modules)

Play/apkpure description: timetable, USOSmail, grades and test results (staff can enter them), university surveys,
apply for mLegitymacja via mObywatel, payment status, show student/staff/academic ID as QR code or barcode, scan QR
codes from university resources, university calendar (days off).

Modules actually present in the 2.6.0a code (`pl/edu/usos/mobilny/...`, feature flags in `usosapi/Feature`), with the
minimum USOS API version where one is gated:

| Module | Details |
|--------|---------|
| Timetable | Day/week calendar from `tt/user`; entry dialog with room, lecturers, course; hide breaks / exam sessions / holidays (server-side `mobile_usos_config` flags); home-screen widget (`CalendarWidget`, RemoteViews, since API 6.4) |
| Calendar | University academic calendar (`calendar/search`) |
| Grades | Course grades (`grades/grade`, `grades/latest`), ECTS points (`courses/user_ects_points`), per-term views |
| Tests (sprawdziany) | Student: `crstests/user_points`, `user_grades`, node tree (since 6.4.0.0-16). Staff: `employeetests` module, edit points/grades/node properties, examinee/participant lists, node stats |
| Groups / courses | `groups/*`, `courses/*`, `csgroups`, `prgroups`, course search, ECTS |
| Registrations | Course registration rounds, cart, register/unregister, token rounds, exam registrations (`registrations/*`, `exams/search_exams_registrations`) |
| Surveys | Student: surveys to fill out, fill-out, "about me" (staff); employee survey reports (6.6.0.0-13) |
| USOSmail (umail) | `mailclient/*` (list, read, delete, create_and_send_message) (7.1.0.0-5) |
| News / articles | `news/*`; WordPress feed for selected universities |
| Useful info / guide | `guide/*` (university guide pages, chapters, entries) |
| Map / buildings / rooms | `geo/building_index`, `geo/room`, room links to Google Maps (module `map`, `buildings`) |
| Search | People, courses, faculties (`users/users`, `courses/search`, `fac/search`) |
| Payments | `payments/user_payments`, `payment`, `user_remittances`, `choose_installment_plan`; settled/unsettled/installment/remittances screens (6.5.2.0-10) |
| mLegitymacja | Eligibility check, order, status, revoke, deep-link into mObywatel (7.2.0.0-4); see A.3 |
| eID (QR/barcode ID) | Show student/staff/academic ID as QR code or barcode (module `eID`), biometric gate configurable per installation (`BiometricEnforce`) |
| NFC card emulation | HCE service, AID `F576593131`, emulates the physical card for readers (see A.4), `requireDeviceUnlock=true` |
| QR scanner | CameraX/zxing, scans university QR codes (e.g. attendance, resources) |
| Administrative documents | `adm_documents/*`: user documents, UPO/UPD downloads, receive document with access code, signing (6.6.1.0-2) |
| Theses | Diploma exam reports, signing (`theses/*`, `examrep*`), 6.5.2.0-11 |
| Attendance lists | Staff: create lists from timetable, mark attendance, change mode (6.7.2.0-4) |
| Protocols | Staff: grading protocols (module `protocols`, 6.4.0.0-13) |
| Dashboard | Home screen with upcoming classes, latest grades, etc. |
| Notifications | FCM push for grades, schedule changes, surveys, mCard status, etc.; a hidden `NotificationTestActivity` with `events/test_my_fcm` |
| Settings / multi-account | Installation select, "new login code", "welcome back", about, theme/language (Polish/English), dark mode |
| Calendar export | The app requests READ/WRITE_CALENDAR (writes timetable into the device calendar). No iCal file export seen. |
| Staff features | Employee tests, protocols, attendance, employee survey reports, signing exam reports/diplomas, staff perspective scope |

### A.3 mLegitymacja (mobile student ID)

Source: EUNIS 2020 talk by J. Mincer-Daszkiewicz (usos.edu.pl), university guides (Opole, AGH, PWr, UW), and the
APK.

* Document is issued and held by the **Ministry of Digital Affairs (MC) / mObywatel** system (NASK). Mobile USOS is
  just the order and activation front end. Since 1 July 2025 mLegitymacja is a full, default form of the student ID
  in Poland (earlier it was only an addition to the plastic card).
* Flow: (1) student presses order in Mobile USOS or USOSweb. Preconditions checked by USOS: PESEL on file, active
  plastic ID, student number, photo, active student status, personal data unchanged since the plastic card was issued.
  (2) A USOS daemon forwards orders to the Ministry API in batches, over a VPN. (3) The daemon reads the issued
  cards back and the USOS notifier pushes a message. (4) Mobile USOS shows an **activation code + QR / password**.
  (5) Student opens mObywatel, "Add document" -> "mLegitymacja studencka", scans/pastes. One phone per card.
  (6) Revocation from Mobile USOS is sent to the Ministry immediately but may take up to 24 h to take effect.
  (7) Renewal is needed each semester (cards live as long as the plastic one), about 30 days before expiry.
* In the code: API methods `cards/make_mcard_order`, `cards/mcard2`, `cards/revoke_mcard`, `cards/user`; entities
  `MCard` (`expected_expiration_date`, `latest_order`, `orders`, `why_cannot_order`, `why_cannot_revoke`) and
  `MCardOrder` (`distribution_pass`, `token`, `status`, `student_number`, `expiration_date`, `error_description`).
  `MCardFragment` launches `pl.nask.mobywatel` (mObywatel) via its launch intent or Play Store.
* No signing, no cryptography and no document rendering happen in Mobile USOS. Verification of the document
  (hologram/QR signed by the Ministry) is entirely inside mObywatel and the verifier app.
* **Can a third party show it?** No, not as an official document: the signed credential lives in mObywatel, and we
  must not clone or re-render it. **Can a third party order/activate/revoke it?** Technically yes, if our consumer
  key is allowed the `cards` scope on the installation (the official app requests it in its default scopes). We can
  show the code/QR and deep-link to mObywatel (`pl.nask.mobywatel`). Legally this is just a normal USOS API client,
  but the university can refuse the scope for third-party keys; this must be verified per installation.

### A.4 NFC feature (not mLegitymacja)

`NfcCardHostApduService` is an Android Host Card Emulation service that answers SELECT on a fixed AID and returns an
identifier so the phone can act like the physical student/library card at readers (doors, library kiosks). The
identifier comes from a per-installation strategy: `CONTACTLESS_UID` (UID of the physical card from `cards/user`),
`USER_LIBRARY_ID` (library card number, hex-converted), or `SALTED_USER_ID_HASH` (HMAC computed in `liboauth.so` from
installation code and user id; a university-specific secret). Reproducing the third strategy requires the secret, so
only the first two are portable. Only some installations enable it.

### A.5 User complaints / UX pain points

Public review text is sparse (Play reviews could not be fetched; no store scraper without a Google account). What
was found:

* App Store reviews of the old UW app (rating 2.0/5 from 4 ratings): push notifications arrive empty (no text), the
  app often drops to the home screen (crashes/resets), and a user could not log in (stuck on a verification step).
* The official feature set is tied to university-side USOS version: "each university has its own version of Mobile
  USOS which depends on the USOS version"; features vary per installation (Feature flags above).
* Structural UX issues visible from the code and flow: login goes through the web authorize page and a custom-scheme
  redirect (`pl.edu.usos.mobilny://usos_api_redirect`); short-lived access tokens handled with `offline_access`; the
  `BadApiVersionActivity` and technical-break screens block use when the server is old/under maintenance (USOSweb
  maintenance windows); mLegitymacja requires copy/paste between two apps and waiting for a batch job; no offline-first
  timetable; no F-Droid/de-Googled support.
* Ideas for "better UX": offline-first cache, instant start, clear per-feature availability with reasons, grade change
  diffs, a proper timetable widget (the official widget was the subject of fixes in 2.6.0a), iCal export, multi-account,
  no Play dependency, readable error states for technical breaks, polling-based notifications.

## B. APK analysis (Mobile USOS 2.6.0a)

Obtained from `https://d.apkpure.net/b/XAPK/pl.edu.usos.mobilny?version=latest` (no Google account), XAPK of 3 splits
(base + armeabi-v7a + hdpi, ~15 MB). Decompiled successfully with jadx 1.5.6 (the app code is R8
minified in parts but class, package and string-constant names are largely intact). Environment: toolbox `reverse`
(Fedora 43, `java-21-openjdk-headless`; jadx is not packaged in Fedora so it is the GitHub release zip in the
scratchpad, run with `toolbox run -c reverse <path>/jadx`). The scratchpad copy is disposable.

### B.1 Technology

* Native Android, Kotlin, Jetpack Compose + Material3, a mix of Fragments/ViewModels (`UsosViewModel`), coroutines,
  OkHttp (`PublicSuffixDatabase.list` present), Jackson-style field deserializers (`ApiField<T>` wrapper with
  "requested field not received" semantics), DataStore, splitties, zxing (journeyapps), Firebase.
* No Flutter (`libflutter.so`) and no React Native bundle. Native libs: only `liboauth.so` (custom), `libandroidx.graphics.path.so`,
  `libdatastore_shared_counter.so`.
* minSdk 29 (Android 10), targetSdk/compileSdk 37, versionCode 2060001. `allowBackup=false`, `largeHeap`, custom
  `network_security_config` with extra trust anchors (HARICA 2021, Certum) in addition to system CAs (some Polish
  universities use these CAs).
* **Google Play licence check**: the application class is `com.pairip.application.Application` calling
  `LicenseClient.checkLicense` with permission `com.android.vending.CHECK_LICENSE`. Official app is therefore
  designed for Google Play installs and may refuse to run when sideloaded or without Play services. Irrelevant for our
  own client, but explains why F-Droid users cannot use the official app.

### B.2 Backend and installations

* Direct to each university's USOS API: `InstallationConfig.apiBaseUrl` such as `https://usosapps.<uni>/`, then
  `services/<module>/<method>`. 64 `ProductionInstallation(...)` entries (code name, name resource, API base URL,
  optional NFC strategy, optional WordPress feed config, feature bits) plus `TestInstallation` and `DevInstallation`.
  Examples: AGH `apps.usos.agh.edu.pl`, PB `usos-apps.uci.pb.edu.pl`, PRz `usosapps.prz.edu.pl`, demo `usosapps.demo.usos.edu.pl`.
* Per-installation discovery calls: `services/apisrv/installation` (base_url, version, machine_version,
  `usos_schema_version`, `schac_id`, `mcards_support`, institution name, contact emails) and
  `services/apisrv/mobile_usos_config` (calendar hide flags, app configuration pushed from the university).
* The only MUCI-hosted web resource seen: university logos at `https://mobilny.usos.edu.pl/uni-logos/<id>/{light|dark}-<n>.webp`.
* API version gating via `ApiVersion` ("6.4.0.0-18" ... "7.3.1.0-0") per module/field, so the app adapts to old
  installations. We should do the same (read `apisrv/installation` and degrade).

### B.3 Authentication

* OAuth 1.0a, 3-legged, with HMAC-SHA1 over standard `oauth_*` parameters. `OAuthSigner` is a thin JNI wrapper:
  `initialize(requestType, service)`, `pushCallParam(...)`, `compute()` returning the `Authorization` header. The
  signing code (and the embedded consumer key/secret for the installation, and the NFC hash secret) is in
  `liboauth.so`, not in the Java code. Only the existence is noted here, values were not extracted.
* Flow: `services/oauth/request_token` (callback `pl.edu.usos.mobilny://usos_api_redirect`, scopes list) ->
  open `services/oauth/authorize?interactivity=minimal&oauth_token=...` in a browser/Custom Tab ->
  redirect back with `oauth_verifier` -> `services/oauth/access_token`; `services/oauth/revoke_token` on logout.
  `NewLoginCodeActivity` handles an alternative login code path.
* Default scopes requested: `offline_access, email, other_emails, studies, staff_perspective, grades, personal,
  photo, surveys_filling, surveys_reports, mailclient, cards, crstests, events, payments, adm_documents`
  (`payments` and `adm_documents` only if the installation version supports them).
* Optional biometric lock (`USE_BIOMETRIC`), policy per installation (`BiometricEnforce`).
* The Java side never contains a per-user password; USOS login happens in the web view/browser via the
  university's CAS.

### B.4 API methods used (111 `services/...` strings)

adm_documents: generate_code, get_upd, get_upo, receive, user_documents. apisrv: installation, mobile_usos_config.
attendance: attendance, change_list_mode, create_attendance_list, create_from_tt, delete_list, group,
update_attendance, user. blobbox: get. calendar: search. cards: make_mcard_order, mcard2, revoke_mcard, user.
courses: classtypes_index, course, course_edition, search, units, user, user_ects_points. crstests: examinee, node,
node2, node_stats, participant, root_nodes, student_grade, student_point, update_grades, update_points,
user_grades, user_points, change_*_node_properties (folder, grade, root, task). csgroups: user. events:
register_fcm_token, registered_fcm_tokens, test_my_fcm. events2: search statements (notifications). examrep and
examrep2 (exam, user2, session, examinee, grader, grade_type, student_grade(s), update_student_grade). exams:
search_exams_registrations. fac: faculties, faculty, search. geo: building_index, building2/buildings2, room.
grades: grade, latest. groups: group, is_lecturer, user. guide: chapter, entries, guide, pages. mailclient:
create_and_send_message, delete_messages, message, user. news: article, search (news). oauth: request_token,
authorize, access_token, revoke_token. payments: choose_installment_plan, payment, user_payments, user_remittances.
prgroups: primary_groups. progs: programmes, student. registrations: course_registration_rounds, courses_cart,
register, registration_round_courses, search_rounds, search_token_rounds, unregister, user_registrations. surveys:
survey, surveys, fill_out, surveys_to_fill, surveys_about_me. terms: search, term, terms. theses: exam_report,
signable_reports, sign_report. tt: user, room, course_edition, staff, classgroup_dates2. users: user, users, search2.

This list is a ready-made scope for our Rust API client's v1 and for a mock server.

### B.5 Push mechanism

* FCM via `MessagingService extends FirebaseMessagingService`. The device registers its FCM token with the university
  through `services/events/register_fcm_token` (`registered_fcm_tokens` to list), so **the USOS installation itself
  sends FCM messages** (via the sender project configured for the official app). Data message fields observed:
  `meta__installation_url` and `meta__session_id`. The app validates the session id against the logged-in user,
  then fetches details with API calls (`events2`) and builds a local notification; per-feature gating (grades, tests,
  mcard) decides what to show. `AllowAllDataInFirebase` is a per-installation flag.
* Consequence: a third-party app cannot reuse that channel (the FCM sender is bound to the official app's Firebase
  project). On F-Droid we need polling, with a configurable interval via WorkManager (Android) / background timer
  (desktop), maybe UnifiedPush later.

### B.6 Widgets, permissions, trackers

* Widgets: one, the timetable `CalendarWidget` + `CalendarWidgetService` (RemoteViews list).
* Permissions: INTERNET, ACCESS_NETWORK_STATE, READ_CALENDAR, WRITE_CALENDAR, POST_NOTIFICATIONS, CAMERA (QR scanner,
  optional hardware), NFC (HCE, optional), USE_BIOMETRIC, USE_FINGERPRINT, WAKE_LOCK, `c2dm.permission.RECEIVE`,
  Play licence, internal signature permission. `<queries>` for `pl.nask.mobywatel` and Google Maps.
* Components: custom scheme `pl.edu.usos.mobilny://` (LoginActivity), `NfcCardHostApduService` (exported, BIND_NFC_SERVICE).
* Trackers / third-party SDKs: Firebase Cloud Messaging, Firebase Crashlytics (collection disabled by default via
  `firebase_crashlytics_collection_enabled=false`, enabled only with user consent), Firebase installations/sessions,
  Google Play Services, Google Maps SDK (a Maps API key is in the manifest), Play licence client. No ad or analytics
  SDKs found in the packages. The Exodus Privacy report could not be fetched from here, so this list is inferred from
  the code; check https://reports.exodus-privacy.eu.org/en/reports/search/pl.edu.usos.mobilny/ manually.

## C. Unofficial clients and libraries

### C.1 Apps and clients

| Project | URL | Lang | License | Activity / stars | Notes |
|---------|-----|------|---------|-------------------|-------|
| OpenUSOS | github.com/OpenUSOS/OpenUSOS, F-Droid `com.open_usos` | Dart/Flutter | GPL-3.0 | last push 2024-10, 5 stars, v1.1.0 (F-Droid has 1.0.1) | **Only third-party USOS app on F-Droid.** Notifications for classes, grades, tests, schedule, year calendar + own events, surveys, mail to staff. UJ, PWr, CAT Warsaw only. Anti-feature "Non-Free Network Services". Uses its own proxy server (OpenUSOS-server, Python/Flask, GPL-3.0, based on usosapi-python) that holds per-university consumer keys; self-hosting possible. Roadmap: group view, registration, iOS. Stale. |
| ProstoKery/usos-widget-android (+ iOS Scriptable) | github.com/ProstoKery/usos-widget-android | Kotlin | MIT | created 2026-11-04, 0 stars | Timetable widgets and change alerts for **any** university using USOSweb. **Login-free**: uses the private iCal export link from USOSweb (Plan zajec -> eksportuj). Neat low-barrier fallback requiring no consumer key. |
| Tuios | github.com/szymon-zygula/tuios | Rust | MIT | 2026-10, 1 star | Rust TUI for teacher view (students, tests, grades), mock mode, fuzzy search. States its code was written by Claude. First start explains how to get an API key. Official API only. Useful as proof of Rust + user-supplied consumer key. |
| USOSL | github.com/mkkf1532/USOSL | Kotlin/Swift (monorepo) | none | 2026-10, 0 stars | Compose + SwiftUI client for Politechnika Slaska (README is aspirational: offline-first, OAuth 1.0a, hardware keystore, zero telemetry). Unverified, no license. |
| USOS++ (zuukie/USOSPlus) | github.com/zuukie/USOSPlus | JS | MIT | 2026-11 | Modernised USOSweb front end (browser). |
| USOSweb-automated | mkochanowski/USOSweb-automated | Python | MIT | 2018 | Scraper/automation of USOSweb. Stale. |
| usos-rpc | Kacper0510/usos-rpc | C++ | MIT | 2024 | USOS timetable as Discord rich presence. |
| usos-discord / UsosBot | rzakx/usos-discord (GPL-3.0), calKU0/UsosBot | Python | - | 2023 | Discord bots using the API. |
| Usosfetch | V0ldek/Usosfetch | Python | GPL-3.0 | 2022 | E-mail on grade changes. |
| usos-auto-registration / usos-rejestracja | yksen/..., itsqbxx/... | Python | - | 2023, 2026 | Auto-registration for classes; shows demand for registration timing features. |
| USOSmobile, USOS_mobile, WatUsos, UsosClient | various | LLVM/Dart/Java | - | 2015-2024 | Student projects, abandoned. |
| USOSweb-Dark-Mode etc. | pokjump/... | JS | MIT | 2026 | Userscripts; evidence of UX pain with USOSweb (dark mode, timetable size). |

F-Droid check: `f-droid.org/packages/com.open_usos` (OpenUSOS) is the only USOS-related app. IzzyOnDroid mirrors it.
No other third-party USOS app found on F-Droid.

### C.2 Libraries and API wrappers

| Library | Lang | License | Status | Notes |
|---------|------|---------|--------|-------|
| **usos-rs** (crates.io `usos-rs` 1.0.0, gitlab.com/fili_pk/usos-rs) | Rust | **AGPL-3.0-or-later** | 2024-03, ~1.5k downloads, 0 stars | Only Rust crate named for USOS on crates.io (search "usos" yields one). Covers OAuth 1.0a request token, PIN authorize, access token and signed `reqwest` client (reqwest-oauth1, oauth1-request); scopes enum; no typed endpoints. AGPL means we should not depend on it from a non-AGPL project. Good reference for the flow; trivial to rewrite. |
| Adimac93/usos | Rust | none declared | 2025-02, 107 commits, 0 stars | Workspace `usos-core` (oauth1, scopes, errors, key handling), `usos-codegen` (generates endpoint code from the API reference), `usos` (calendar, faculties modules), `usos-cli`. Closest to our target architecture. No license, so learn from it only; contact the author before reuse. |
| **usos4j** (maven `pl.wmsdev:usos4j`) | Java 17 | MIT | active (push 2026-11-01), 39 stars | Most complete, tested wrapper: OAuth (console/web/Spring Security), typed modules, generic endpoint for anything else (`services/apiref/module`). Good model for module coverage and test data. |
| **fake-usos** (WMS-DEV/fake-usos) | Java | MIT | 2025-07, 15 stars | Mock USOS API server (Docker) auto-stubbed from usos4j; schema-compliant realistic data. Use in CI for our client tests. Its README documents the access problem ("many universities limit access to employees only"). |
| MUCI/usosapi-python | Python 3 | not declared | 2024-05, 7 stars | Listed on the official developer page; rauth + requests; desktop OAuth with PIN. Basis of OpenUSOS-server and usosapi-fiddler. |
| usos-api (PyPI 0.3.4, Antoni-Czaplicki/usos-api) | Python async | not declared | 2026-06 | In development, docs on readthedocs. |
| TSOS (`@maciejzuj/tsos` on npm) | TypeScript | MIT | 2026-10 | Typed wrapper, OAuth, error handling, UJ preset. |
| usos-bridge (piotr-ginal) | Python | MIT | 2025-10 | Web credentials -> API. Credential-based (avoid). |
| UsosApi4j (mateusz-lisik) | Java | Apache-2.0 | 2015 | Legacy. |
| MUCI/usosapi-browser, savix/usosapi-fiddler | C#, JS | - | old | API exploration tools. |
| MUCI/jquery-usos | JS | - | 2026-02 | UI widgets for USOS web apps. |
| Solvro/script-promochator-usos-scraper | Python | MIT | 2024 | Public-data scrapers via official API plus other methods. |

### C.3 What is reusable

* **API reference as data**: the mother server exposes self-describing methods (`services/apiref/method_index`,
  `services/apiref/method`, `services/apiref/module`); at the time of writing the mother server returned HTTP 500 for
  `method_index` anonymously, so fetch it from a specific installation with a key. Adimac93's `usos-codegen` shows
  the approach of generating Rust endpoint code from it.
* **OAuth handling**: only 3 small crates matter (`oauth1-request`, `reqwest-oauth1`, or hand-rolled HMAC-SHA1). Our
  own implementation avoids AGPL.
* **Mock**: fake-usos (MIT) for integration tests, extended with the endpoints in B.4.
* **Feature gating by API version**: copy the idea of the official `Feature`/`ApiVersion` checks.
* **Login-free timetable**: iCal export link approach (usos-widget-android) as a zero-setup tier.

## 4. API access model (affects everything above)

From apps.usos.edu.pl/developers/api (authorization, definitions/installations, definitions/rules):

* Each university runs its own **USOS API installation** and issues its own consumer keys (82 entries listed on the
  mother server page). You cannot register a key on the mother server itself.
* Key sign-up is at `/developers/` of each installation; administrative keys are granted by request only. Many
  universities now limit registration to employees and require paperwork (fake-usos README).
* OAuth 1.0a with `oauth_callback` (browser redirect with PIN/verifier) or `oob` (user types the PIN). Access tokens
  last about 2 hours unless scope `offline_access`. Scopes include `grades, email, personal, photo, studies,
  offline_access, cards, payments, ...`.
* The docs advise a server-side proxy for public mobile/desktop apps because embedded consumer secrets can leak.
  The official app embeds its key in native code anyway. OpenUSOS chose a proxy.
* "Data from all installations is copyrighted, even when available anonymously"; do not call the mother server
  without consulting the USOS API developers.
* Implication: our client must let each user/university supply a consumer key, and ideally provide per-university
  key registration instructions. Copying the official app's key would be both ToS-problematic and unnecessary; we
  did not extract it.

## 5. Feature parity checklist (official vs unofficial)

Legend: Y = present, P = partial, - = absent. "OUR" is the target for the new client (M = must for v1, S = should,
C = could, X = out of scope).

| Feature | Official mUSOS | OpenUSOS | USOS Widget | Tuios | USOSL (claimed) | OUR |
|---------|:-:|:-:|:-:|:-:|:-:|:-:|
| University picker / multi-installation | Y (64) | P (3) | Y (iCal, any) | P (1) | P (PolSL) | M |
| Login via OAuth 1.0a in browser | Y | via proxy | - (iCal link) | Y (PIN) | Y | M |
| Offline cache / instant start | P | P | Y (widget) | - | claimed | M |
| Timetable (day/week) | Y | Y | Y | - | Y | M |
| Timetable widget | Y | - | Y (2 widgets) | - | claimed | M |
| Timetable change alerts | Y (push) | Y (class alarms) | Y | - | - | M |
| iCal / calendar export or sync | P (writes device calendar) | P | uses iCal | - | - | S |
| Grades | Y | Y | - | P (staff) | Y | M |
| New-grade notifications | Y | Y | - | - | - | M |
| Course tests (sprawdziany) / points | Y | Y | - | Y (staff) | - | M |
| Groups / courses / ECTS | Y | P | - | - | P | S |
| Course registrations (rounds, cart) | Y | - (planned) | - | - | - | S |
| Exam registrations | Y | - | - | - | - | C |
| Surveys (fill out) | Y | Y | - | - | - | S |
| USOSmail (read/send) | Y | P (send) | - | - | - | S |
| News / articles | Y | - | - | - | - | C |
| University guide / useful info | Y | - | - | - | - | C |
| Academic year calendar | Y | Y | - | - | - | S |
| Buildings, rooms, map | Y | - | - | - | - | C |
| People / course search | Y | - | - | P (fuzzy, own students) | - | S |
| Payments (view, installment plan) | Y | - | - | - | - | S |
| mLegitymacja order / status / revoke / mObywatel deep link | Y | - | - | - | - | S (M on Android if scope available) |
| Show ID as QR / barcode (eID) | Y | - | - | - | - | S |
| NFC card emulation (HCE) | Y (some unis) | - | - | - | - | C (Android only, UID/library strategies) |
| QR scanner | Y | - | - | - | - | C |
| Administrative documents | Y | - | - | - | - | C |
| Theses / diploma exam reports | Y | - | - | - | - | C |
| Biometric app lock | Y | - | - | - | claimed | S |
| Dark mode, i18n (PL/EN) | Y | P | Y (4 langs) | - | Y | M |
| Staff: enter grades/points | Y | - | - | Y | - | X for v1 (C later) |
| Staff: attendance lists, protocols | Y | - | - | - | - | X |
| Push without Google Play Services | N (FCM) | P (local alarms + proxy) | P (polling) | - | - | M (polling/UnifiedPush) |
| Works without Google Play (F-Droid) | N (pairip licence check) | Y | Y (APK) | - | - | M |
| Desktop app | N (web USOSweb only) | N | N | Y (TUI) | N | M (Rust core) |

## 6. Open questions / follow-ups

* Per-university consumer key policy and feasibility of a user-supplied key flow: test with 2-3 target universities
  (UW, UJ, PW, AGH) by attempting registration as a student.
* Whether the `cards` scope is granted to third-party keys (mLegitymacja ordering).
* Fetch live `services/apiref/method_index` from a university installation to build the endpoint schema for codegen.
* Play Store review text and Exodus report could not be retrieved automatically; review manually if needed.
* Contact the author of `Adimac93/usos` about a license, and the OpenUSOS team about collaboration.

## 7. Source list

usos.edu.pl/node/5143; apkpure.net/mobile-usos-2-0/pl.edu.usos.mobilny; usos.edu.pl EUNIS 2020 mLegitymacja slides;
university mLegitymacja guides (Opole, AGH, PWr, UW, PB); pb.edu.pl/uci (Mobilny USOS 2.0 announcement);
apps.usos.edu.pl/developers/api (authorization, definitions/installations, definitions/rules, examples);
f-droid.org/packages/com.open_usos; github.com/OpenUSOS/OpenUSOS and OpenUSOS-server; github.com/WMS-DEV/usos4j and
fake-usos; github.com/MUCI; github.com/Adimac93/usos; gitlab.com/fili_pk/usos-rs and crates.io/crates/usos-rs;
github.com/maciejzujtu/TSOS; github.com/ProstoKery/usos-widget-android; github.com/szymon-zygula/tuios;
github.com/mkkf1532/USOSL; own jadx analysis of the 2.6.0a APK.
