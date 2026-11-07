# USOS from a client developer's point of view

Research date: 2026-11-04. Live data was fetched from `https://usosapps.uw.edu.pl` (University of Warsaw
installation, USOS API 7.3.1.0-2). Everything marked "live" was observed with unauthenticated `curl`; nothing
was registered or authenticated.

Status: this file is written incrementally. Sections 1-9 are complete; section 9 covers Politechnika Śląska specifically.

---

## 1. Ecosystem

### 1.1 Who develops it

- USOS = "Uniwersytecki System Obsługi Studiów". Developed by the **MUCI** consortium (Międzyuczelniane
  Centrum Informatyzacji), founded around the University of Warsaw (UW) team; the API's own contact address is
  `usos@usos.edu.pl` ("USOS Developers' Office"). Source: https://usosapps.uw.edu.pl/developers/api/contact/
- Universities pay/join MUCI for updates and support. The mobile app page says the app was built by UW (project
  "eUW", service E3, EU Mazovia ROP funding) and "will be provided to other universities as open source with
  technical support" (branding rules: keep the name, add the university abbreviation, keep colours, show EU and UW
  copyright). Source: https://www.usos.edu.pl/node/5143. I found **no public source repository** of the official
  mobile app (GitHub org `MUCI` only hosts `usosapi-python`, `usosapi-browser` (C#, 2013), `jquery-usos`,
  `emrex-elmo-extensions`). Source: https://github.com/MUCI
- Software is delivered in versions (`7.x`, `8.0`); each university runs its own database and its own API
  installation, upgraded on its own schedule (see 2.2).

### 1.2 Components (as seen by a client developer)

| Component | What it is | Client relevance |
|---|---|---|
| USOS (DB + core, Oracle-based) | The system of record per university | Not reachable directly |
| USOSweb | Server-rendered web UI for students/staff (login via the university CAS) | Source of truth for anything the API lacks (see section 5) |
| USOS API ("usosapi", "USOSapps") | REST-ish JSON API, OAuth 1.0a, per university | The only supported programmatic interface |
| USOSmail / mailclient | Mail-like messaging inside USOS; API module `mailclient`, `mailing` (admin) | `mailclient` is exposed to normal consumers with scope `mailclient` |
| IRK (Internetowa Rejestracja Kandydatów) | Candidate recruitment portal | Separate web app, not in the student API |
| APD (Archiwum Prac Dyplomowych) | Thesis archive / diploma process | API module `theses` is partly public |
| Rejestracje / żetony | Course registrations (also token-based registration) | API module `registrations` (see section 3) |
| eLegitymacja / mLegitymacja | Electronic student ID (ELS card) and mobile ID (mStudentCard) | API module `cards` (scope `cards`) |
| Mobilny USOS (2.0) | Official Android/iOS app by MUCI | Reference feature set; uses `events`/FCM (section 6) |
| Payments | Student fees/instalments | API module `payments` (scope `payments`); payment execution itself is outside the API |

Not every university enables every module; the official app also exposes only a per-university subset.
Source: https://pb.edu.pl/uci/2025/06/25/mobilny-usos-2-0-nowa-odslona-aplikacji-dla-studentow-i-pracownikow-politechniki-bialostockiej/

### 1.3 Who runs it

Live `GET /services/apisrv/installations` on UW returned **83 entries** (one is a duplicate http/https UW entry,
one – `apps.nencki.usos.pcss.pl` – returns no version/name). They include UW, UJ, UAM, PW, PWr, SGH, UŁ, UMK,
UWr, UMCS, UŚ, Politechnika Śląska/Poznańska/Rzeszowska/Białostocka/Częstochowska, many medical, music, physical
education and applied-science universities. Note: not all Polish universities use USOS (e.g. AGH uses its own
systems; some appear only in the Unitime-style exports). The list is self-reported by installations that opt in
as "public".

---

## 2. USOS API in depth

### 2.1 Documentation locations

- Human docs, per installation: `https://<installation>/developers/api/` (identical content, version footer shows the
  installation's version). Sections: Authorization, Libraries and Examples, Installations, Basic rules, Datatypes,
  Vocabulary, Diagrams, Error handling, API fiddler, then one page per module
  (`/developers/api/services/<module>/`).
- Machine-readable docs (`apiref` module, anonymous, JSON):
  - `GET /services/apiref/method_index` -> `[{name, brief_description}]` (live: **579 methods** on UW)
  - `GET /services/apiref/method?name=services/tt/user` -> `{name, short_name, description, ref_url,
    auth_options:{consumer,token,administrative_only,ssl_required,scopes[]}, arguments[{name,is_required,
    is_deprecated,type,default_value,description}], returns, errors, result_fields, beta, deprecated, is_internal}`
  - `GET /services/apiref/module?name=services/events` -> module description (HTML)
  - `GET /services/apiref/scopes` -> `[{key, developers_description}]`
  - Descriptions are HTML fragments, `result_fields` is mostly empty, and `auth_options.scopes` is advisory (see 2.8).
  This is good input for generating a typed Rust client or for runtime capability discovery.
- "Mother server" `http://apps.usos.edu.pl/`: not a real installation; no keys can be registered there and it
  must not be called without consulting the USOS developers. It may carry newer docs. Source:
  https://usosapps.uw.edu.pl/developers/api/definitions/installations/
- No official SDKs. Unofficial/semi-official libraries listed by the docs: jQuery-USOS (not updated since 2019),
  `usosapi-python` (MUCI), `usos4j` (WMS-DEV, Java), `usos-rs` (Rust bindings for authentication, repo
  `Adimac93/usos`, last push 2025-02, 0 stars). Source: https://usosapps.uw.edu.pl/developers/api/examples/

### 2.2 Installations, versioning

- Each university has **its own installation** and **its own consumer keys**: "Each installation issues its own
  Consumer Keys." Source: https://usosapps.uw.edu.pl/developers/api/definitions/installations/
- Discovery: `GET /services/apisrv/installations` (anonymous, any installation; UW's is canonical-ish) returns
  `[{base_url, version, institution_name:{pl,en}, contact_emails[]}]`. Live example:
  `{"base_url":"http://usosapps.amu.edu.pl/","version":"7.3.1.0-2, e2891efb, dirty (2026-10-06)",
  "institution_name":{"pl":"Uniwersytet im. Adama Mickiewicza w Poznaniu","en":"The Adam Mickiewicz University in
  Poznan"},"contact_emails":["usosweb@amu.edu.pl", ...]}`. Note `base_url` is returned as `http://` - the docs say
  to replace with `https://` ("in case of some methods"; in practice always use HTTPS).
  `GET /services/apisrv/installation` returns the same plus `machine_version` ("7.3.1.0-2") and
  `usos_schema_version` ("7.3.1.0").
- Version distribution (live, 83 installs): 7.3.1.0-2 (41), 7.3.1.0-1 (17), 8.0.0.0-2 (6), 7.3.1.0-3 (5),
  8.0.0.0-0 (2), 7.3.0.0-2/-3 (3), and a tail of old ones: 7.2.0.0-4, 7.1.2.0-x (3), 7.0.4.0-2 (2), 6.5.0.0-7.
  The docs advise: "use the oldest (lowest version number) installation as the documentation reference".
  Method lists and fields differ between versions, so the client should consult `apiref/method_index` and
  tolerate missing methods/fields (HTTP 400 `field_not_found` / 404-style errors).
- Time: `GET /services/apisrv/now` -> `"2026-11-04 11:20:21.862873"` (server local time string, quoted JSON).
- `base_url` quirk: one installation lives under a path (`http://api.pwsip.edu.pl/usosapps/`). Never assume the
  API is at the host root; append `services/...` to the stored base URL.
- Statement from docs for multi-installation apps: "You might, for example, periodically check [the installations
  list] against the list of installations your application supports, and notify yourself if there is a new one
  (then, you can generate a new API key and include it in your application)." This is an explicit official
  pattern: one consumer key per installation, shipped inside the app. See 2.6.

### 2.3 Request conventions (all from `/developers/api/definitions/*`)

- Methods accept GET or POST (POST preferred); unknown parameters are ignored; CORS is allowed
  (`Access-Control-Allow-Origin: *` seen live).
- URL pattern: `https://<base>/services/<module>/<method>?param=...`.
- `fields` selector: pipe-separated list: `id|name|users`. Subfield selector with brackets, arbitrarily nested:
  `id|name|users[id|email|student_programmes[id]]`. Not every method honours subfields; support can appear
  silently. You may get extra fields; do not rely on extras. Unknown field names -> error `field_not_found`
  (observed: HTTP **500** with `{"message":"Method services/geo/building does not contain postal_address field.",
  "error":"field_not_found","field_name":"postal_address","method_name":"services/geo/building"}`).
- Formats: `format=json` (default), `jsonp`, `xmlmap2`, deprecated `xmlmap`/`xmlitems`. Use JSON.
- IDs: always strings when possible (max 32 chars, never contain `|`); lists of IDs are pipe-separated.
  Beware legacy ints: e.g. `tt` `lecturer_ids` is "a list of integers, not strings".
- Encoding UTF-8.
- LangDict: `{"pl": "...", "en": "..."}` (either may be empty). As input either serialized JSON or `name_pl=`/
  `name_en=` suffixed params. There is no `lang` negotiation; the client picks the key. A few methods take
  `lang=pl|en` (e.g. `tt/upcoming_ical`, `courses/search`).
- Dates: `yyyy-mm-dd`, `hh:mm:ss`, `yyyy-mm-dd hh:mm:ss`, **always the installation's local time, no timezone
  offset**. The client must convert (all Polish universities -> `Europe/Warsaw`, but resolve per installation
  rather than hard-code).
- Partial responses: some multi-object methods take `partial=true`, returning
  `{success, partial_response, unexistent_keys}` instead of failing the whole call on one bad id.
- Dictionaries are extensible: ignore unknown keys.
- Pagination: no uniform scheme. Search-like methods use `start`/`num` and return `{items:[...], next_page: bool,
  total?}` (live: `courses/search` -> `{"items":[{"course_id":"1200-0ENMATEMATYKA","match":"1200-0EN<b>MATEMATYKA</b>"}],
  "next_page":false}`; `news/search` has `items,next_page,total`). Index methods (`terms_index`,
  `building_index`) return the whole list.
- Files as arguments: `multipart/form-data`, PLAINTEXT signature only, SSL mandatory.

### 2.4 Errors (https://usosapps.uw.edu.pl/developers/api/definitions/errors/)

- Success is HTTP 200. Failures: **HTTP 400** (client-side error, including forbidden access!), **401** (invalid or
  expired token / bad consumer or signature), **500** (server side; but also returned for `field_not_found` as
  observed live).
- JSON body: `{"message": "<developer text>", "error": "<code>", ...extra keys}`. Codes: `method_forbidden`
  (with `reason`), `param_missing` (`param_name`), `param_invalid`, `param_forbidden`, `field_not_found`,
  `field_invalid`, `field_forbidden`, `object_not_found`, `object_invalid`, `object_forbidden`. Reasons for
  `*_forbidden`: `consumer_missing`, `user_missing`, `secure_required`, `trusted_required`, `scope_missing`
  (+`missing_scopes[]`), `impersonate_required`, and per-method custom reasons.
- Live examples (anonymous calls):
  - `GET /services/tt/user` -> 400 `{"message":"Access to method denied. Access token is required.",
    "error":"method_forbidden","reason":"user_missing"}`
  - `GET /services/users/user?user_id=1` -> 400 `{"message":"Access to method denied. Consumer signature is
    required.","error":"method_forbidden","reason":"consumer_missing"}`
  - `GET /services/tt/classgroup_dates?unit_id=1&group_number=1` -> 500 `{"message":"Internal USOS API error has
    occured. We're working on fixing this!"}` (an unknown id leads to a generic 500 in some methods instead of
    `object_not_found`).
- `user_messages` (for form-like write methods): `{generic_message: LangDict}` or `{fields: {param: LangDict}}`;
  show them to the user verbatim.
- 401 handling: Access Token expiry (~2 hours), logout, or user revoking the app. Docs recommend treating it like
  a session expiry and re-running the flow. Also: a re-issued token may belong to a *different* user than
  expected; verify identity (`users/user` with no `user_id`, or compare `id`) after re-auth.

### 2.5 Rate limits, caching, stability

- **No rate limit / throttling is documented** anywhere in the public docs (I grepped all `/developers/api/`
  definition pages). The only visible throttles are per-method (e.g. `events/subscribe_event` ->
  `too_many_subscription_requests`; `mailclient` spam lock after ~100 messages/hour). Expect undocumented
  server-side protection (Apache front, 500 errors under load); be conservative: batch ids with the
  multi-object methods (`users/users`, `terms/terms`, `courses/courses`, `tt/classgroups`), use `fields`, cache.
- No ETag/`If-Modified-Since`, no change-feeds on core resources (see 6.1). No cache headers observed except
  none. Caching guidance is absent from docs; sensible TTLs are the client's decision (terms and faculties and
  buildings are near-static, grades should not be cached beyond a poll interval).
- Stability: the docs say HTTP 500 "should never happen"; in practice 500s are returned for some invalid-id
  inputs. Treat 500 as retriable-with-backoff, not necessarily as an outage.
- Data is copyrighted: "all the data (even when available anonymously) is copyrighted" (installations page).

### 2.6 Authentication: OAuth 1.0a, consumer keys, and the FOSS problem

#### 2.6.1 The flow (https://usosapps.uw.edu.pl/developers/api/authorization/)

1. Signed (2-legged, consumer key only) `POST/GET https://<base>/services/oauth/request_token` with
   - `oauth_callback` (required): an app URI, or the literal `oob`. Docs: "some URI schemes are not allowed here
     for security reasons. If you're developing a mobile application and you want to use a custom scheme, then we
     advise you to use a scheme unique to your application (e.g. your Android package name)."
   - `scopes` (USOS-specific, pipe-separated, e.g. `studies|grades|offline_access`). Must be part of the signed
     base URL/params.
   Response (form-encoded): `oauth_token=...&oauth_token_secret=...&oauth_callback_confirmed=true`. SSL required.
2. Redirect the user's browser to `https://<base>/services/oauth/authorize?oauth_token=<request token>`
   (optional `interactivity=minimal|confirm_user`). The user logs in through the university's CAS (see section 5),
   sees a consent screen listing the scopes. Then:
   - callback given: redirect to `callback?oauth_token=...&oauth_verifier=<PIN>`;
   - `oob`: a page displays the PIN (`oauth_verifier`) to type into the app.
3. Signed (consumer + request token) `services/oauth/access_token` with `oauth_verifier` -> `oauth_token`,
   `oauth_token_secret` (SSL required). The access token "MAY have access to more scopes than you declared" (it
   accumulates all scopes the user ever granted this consumer).
4. Sign every later request with consumer key/secret + access token/secret (HMAC-SHA1 is the normal choice;
   PLAINTEXT is allowed over SSL and required for file uploads).
5. Other OAuth methods: `services/oauth/revoke_token` (revoke an access token), `services/oauth/revoke_consumer_key`
   (owner-only, emails the registered address), `services/oauth/proxy` (call a method "with weakened permissions",
   mostly for public AJAX proxies), `services/apisrv/consumer` (`fields=name|url|email|date_registered|
   administrative_methods|token_scopes|is_verified`; handy to check which scopes a token really has), and an
   `services/oauth2/impersonated_token` (administrative only, see 2.6.5).

Token lifetime (live docs): request tokens ~30 min; **access tokens expire after ~2 hours OR when the user logs
out of USOS(web)**; tokens are invalidated when the user revokes the app. The `offline_access` scope makes tokens
long-lived. Without it a background client is impossible. The user can see/remove authorised apps in their
"USOSapps Administration Panel" (USOSweb).

#### 2.6.2 Consumer key registration

- Public self-service form per installation: `https://<base>/developers/` ("Sign up for an API key"): fields
  Application Name, Application's Webpage (optional, shown to users), email (hidden from users, must be valid,
  used for contact and key revocation). The result is a Consumer Key + Secret shown immediately. No admin
  approval, no documented fee, no review of the application. Source: https://usosapps.uw.edu.pl/developers/
  (form text fetched; the form was not submitted).
- A key from one installation does **not** work on another ("Each installation issues its own Consumer Keys").
  A multi-university app needs one key per installation (83 forms) or a different strategy.
- Scopes: all documented scopes are requestable by a normal consumer; the user grants them at consent time.
  Methods marked `administrative_only` or "administrative methods" need an **Administrative Consumer Key**
  obtained by emailing the installation admin (for UW `dsksysadm@adm.uw.edu.pl`), listing methods and reasons,
  possibly a written agreement with the university. The scope `session_debugging_perms` is "Administrative
  Consumers only". Not relevant for a student client.
- Key revocation: form on the same page, requires access to the registered email. Docs example: "when you
  accidentally committed your Consumer Secret to a public repository."

#### 2.6.3 Terms of use

- I found **no explicit Terms of Service / regulamin for the API** on the public docs (neither on `/developers/`,
  `/developers/api/`, nor `/contact/`). The only normative remarks: data is copyrighted; secrets must be kept safe;
  USOS can revoke keys; administrative keys may need a signed agreement; installation admins are the contacts.
  Each university's local rules (and their data protection officers) may add policies. To be confirmed by
  e-mail to `usos@usos.edu.pl` (see the "open questions" list in section 8).

#### 2.6.4 The embedded-secret question (central for this project)

What the official docs say:

- Authorization page footnote: "Please note, that if you plan to publicly deploy a desktop/mobile application,
  then your OAuth Consumer Key might be compromised. Secrets might be stolen. Consider using your own
  server-side proxy."
- Libraries page: a "Simple Proxy" (PHP) that "signs all requests that it receives (with a hardcoded Consumer
  Key)" intended for closed environments.
- The multi-installation advice (2.2) assumes the developer holds a key for every installation and "include[s] it
  in your application".
- The `oauth/proxy` method + `revoke_consumer_key` form exist for exactly the "leaked key" scenario.
- Nothing in the docs forbids shipping a consumer secret in an open-source client. OAuth 1.0a has no public-client
  mode; the secret only authenticates the *application*, never the user. A leaked consumer secret lets someone
  impersonate the app (e.g. spam / hit rate protections attributed to the key; bad actors can phish users with
  your app name on the consent screen) but cannot access any user's data without that user consenting.

How other FOSS clients handle it (found so far):

| Project | Approach | Source |
|---|---|---|
| VistulaUSOSmobile (React Native/Expo) | Secret lives only in a small server-side proxy (Node) that signs requests; the app holds an AES-GCM encrypted opaque session token (30-day); callback `vistulausos://oauth-callback` registered with the key; README admits a previous secret had been in client code and tells to rotate it | https://github.com/tarieeelov/VistulaUSOSmobile |
| `usosapi-python` (MUCI) | Developer supplies own key+secret; PIN (oob) flow for desktop | https://github.com/MUCI/usosapi-python |
| `usos4j` (WMS-DEV) and `fake-usos` | Java client with user-provided keys; `fake-usos` is a **mock USOS API** for dev/testing | https://github.com/WMS-DEV/usos4j , https://github.com/WMS-DEV/fake-usos |
| `StudentUsos` (OvrBtn, C#/.NET MAUI) | Open source mobile app for "Polish students", uses its own server (the app asks the server for the authorize URL, tokens held by the app, FCM tokens sent to the server); no embedded USOS secret seen in the client; unlicensed, PRs closed | https://github.com/OvrBtn/StudentUsos |
| `OpenUSOS` (Dart/Flutter, 2024) | Uses its own backend server (OpenUSOS-server, self-hostable; app only talks to that server, GPL-3.0); supports UJ, PWr, CAT | https://github.com/OpenUSOS/OpenUSOS |
| `better-auth-usos`, `SocialiteProviders/Usos`, `omniauth-usosumk` | Web-server login plugins; the secret stays server-side | GitHub |

Options for us (see the Implications section for the recommendation): (a) per-user key registration (the user
creates a key at `<base>/developers/` and pastes key+secret into the app: no shared secret, works for every
installation, but poor UX); (b) embed a key per installation in the open-source repo (officially anticipated but
the secret is public; revocable; each university's admin may notice); (c) our own signing proxy (central point of
failure, would see every user's tokens/traffic, privacy cost, contradicts local-first, and still needs 83 keys);
(d) hybrid: app ships with pre-registered keys for popular installations, and a "bring your own key" option for
any other.

#### 2.6.5 OAuth2 / PKCE / other flows

- There is a `services/oauth2` module on the API ("OAuth provider module") but it currently exposes exactly one
  method: `services/oauth2/impersonated_token` (consumer required, **administrative only**, "Get token without
  prompting user"). It is the server side of an OAuth2-ish provider for administrative consumers, not a public
  PKCE flow. I found no sign of authorization-code + PKCE for third-party apps (live `method_index` has 579
  methods; only this one in `oauth2`).
- The only user-facing flow for third parties remains OAuth 1.0a with a CAS login in the browser.

---

## 3. API modules relevant to a student/staff client

Counts are live from UW (`method_index`, 579 methods, 52 modules; module list below is truncated to relevant
ones). Auth columns: `C`=consumer signature (req/opt/ign), `T`=access token (req/opt/ign), scope as in
`auth_options.scopes` (**advisory**: e.g. `courses/user` shows no scope but returns the user's courses; always
test against a real token). "Anon" = callable without any OAuth (plain GET).

### 3.1 Scopes (live `apiref/scopes`)

| Scope | Unlocks |
|---|---|
| (none/default) | Basic user info (name, id) |
| `studies` | Programmes, courses, groups, classes of the user as student; `tt/user`, `tt/student`, `registrations/*`, `exams/student_exams`, `credits`, `prgroups` |
| `grades` | Grades (`grades/*`, `examrep2/*`, parts of `crstests`) |
| `crstests` | Course tests (sprawdziany / kolokwia, points per node) |
| `cards` | Student ID cards, mStudentCard (mLegitymacja) |
| `payments` | Fees, instalments, accounts, remittances |
| `events` | `events2/*` notification preferences and `latest_notifications` |
| `email` | User's email address |
| `other_emails` | Emails of other related users |
| `mobile_numbers` | User's phone numbers |
| `personal` | PESEL, birth date, previous names, etc. |
| `photo` | User photo + visibility preference |
| `student_exams` / `student_exams_write` | Exam lists / register and unregister for exams |
| `surveys_filling` | Fetch and fill in surveys |
| `surveys_reports` | Survey reports (lecturer side) |
| `mailclient` | Mailclient (USOS mail) in the user's name |
| `placement_tests` | Language placement test results |
| `adm_documents` | Administration documents |
| `staff_perspective` | Staff-only student data (student numbers, attendance, crstests lecturer side, theses protocols) |
| `grades_write`, `theses_protocols_write`, `slips`, `slips_admin`, `edit_user_attrs`, `change_all_preferences`, `dorm_admin` | Write/admin-ish, staff features |
| `offline_access` | Long-lived access tokens (no 2h expiry) |
| `session_debugging_perms` | Administrative consumers only |

### 3.2 Methods

(Selected; see `method_index` for the complete list. "Anon" per `auth_options`.)

| Module / method | Auth | What it returns |
|---|---|---|
| `users/user`, `user2` (`user_id`, `fields`) | C req, T opt | Profile: `id, first_name, middle_names, last_name, sex, titles{before,after}, student_status (0/1/2/null), staff_status, email (scope), photo_urls, student_number (studies+staff_perspective), previous_names (personal)`; with no `user_id` and a token returns the token owner |
| `users/users`, `search*`, `staff_index`, `student_index` | C req | Batch lookup / search of people (`search_current_teachers`: anon-ish) |
| `users/student_programmes` | T, `studies` | The student's programmes |
| `progs/student`, `student_programme(s)`, `programme(s)`, `stage` | opt/ign | Programme details, stages |
| `terms/terms_index`, `term(s)`, `search` | Anon | Academic terms (see 3.3) |
| `courses/user` | T req | `course_editions` per term + `terms`, option `active_terms_only` (default true), `user_ects_points` |
| `courses/course(s)`, `course_edition(2)`, `search`, `unit(s)`, `classtypes_index` | Anon / opt | Course catalogue |
| `groups/user`, `participant`, `group`, `lecturer`, `common_groups` | opt / `studies` | Class groups the user attends / teaches |
| `tt/user` | C req, T req, `studies` | Activities for the user, any of student/staff, `start` (default today), `days` (max **7**), `fields` (default `start_time|end_time|name`) |
| `tt/student`, `tt/staff` (staff: public, anon), `tt/room` (C req), `tt/classgroup(s)`, `tt/course_edition(s)`, `tt/classgroup_dates(2)` | mixed | Timetable queries; `tt/classgroup*` and `course_edition*` are Anon |
| `tt/upcoming_ical`, `tt/upcoming_share` | opt / T `studies` | iCal feed; see 3.4 |
| `grades/latest` (`days`=7), `grades/terms2` (`term_ids`, `course_ids`), `grades/course_edition2`, `grade`, `exam`, `grade_type_index` | T, `grades` | Final grades; grade types anon |
| `examrep2/user_grade`, `student_grade(s)` | T, `grades` | Exam-report grades |
| `crstests/participant2` (`active_terms_only`), `user_points`, `user_grades`, `node2`, `root_nodes`, `node_stats` | T, `crstests` | Course tests tree and the user's points/grades (partial grades, "sprawdziany") |
| `exams/user`, `student_exams`, `exam`, `examination_sessions`, ... | `studies`/`student_exams` | Exam schedule, registrations |
| `registrations/user_registrations`, `user_token_registrations`, `courses_cart`, `search_rounds`, `registration_round_courses`, `register`, `unregister`, `ranking`, `faculty_(token_)registrations` | T opt/req, `studies` for write | Course registration ("rejestracje") incl. token registrations ("żetonowe") |
| `payments/user_payments`, `payment`, `installment(_plan)`, `user_accounts`, `user_remittances`, `choose_installment_plan` | T, `payments` | Fees/instalments/bank accounts |
| `cards/user`, `user_cards`, `mcard(2)`, `mcard_order(2)`, `make_mcard_order`, `revoke_mcard` | T, `cards` | ELS card + mLegitymacja orders (no QR/token generation for the mobile ID is documented; check `mcard2` fields on a real account) |
| `surveys/surveys_to_fill(2)`, `survey(2)`, `fill_out(2)` | T, `surveys_filling` | Satisfaction surveys (ankiety) |
| `theses/search`, `thesis`, `theses`, `user` | opt (public) | Diploma thesis metadata, mostly public; protocol-signing requires staff scopes |
| `mailclient/*` (22 methods) | T, `mailclient` | `user`, `messages`, `create_message`, `send_message`, attachments, recipients: USOSmail |
| `news/search`, `article(s)`, `category_index` | C req | University/faculty news |
| `calendar/search`, `calendar_event` | C req | Academic calendar events |
| `geo/building_index`, `building(2)`, `buildings(2)`, `room(s)`, `staticmap`(admin) | Anon (rooms opt) | Buildings, rooms, coordinates |
| `fac/faculty`, `faculties`, `search`, `subfaculties_deep` | opt | Organisation units |
| `photos/photo` (opt), `users/photo` (T+`photo`), `photos/my_photo_visibility` | | Photos |
| `phones/phone_number`, `phone_type` | opt | Phone numbers (user phone via `users/user` with `mobile_numbers`) |
| `attendance/*` | `staff_perspective` | Attendance lists (staff side only) |
| `guide/*` | opt | University guide chapters/pages (Informator) |
| `housing/*` | Anon/opt | Dormitories, allocation periods |
| `adm_documents/*`, `statements/*`, `pit/*` | T | Administration documents, signing statements, PIT tax forms |
| `events/*`, `events2/*` | see section 6 | Subscriptions, FCM, notifications |
| `uprefs/*` | T | User preferences |
| `mobility/*` (48), `emrex/*`, `meetings/*` (31), `slips/*` (27), `csgroups/*`, `blobbox/*`, `fileshare/*`, `committees/*`, `internships/*`, `plctests/*` | various | Erasmus/mobility, meetings, clearance slips, custom groups, binary store, etc. |

Module sizes (live): crstests 54, mobility 48, meetings 31, slips 27, events2 27, housing 24, users 22, mailclient 22,
registrations 18, exams 17, courses 17, blobbox 15, prgroups 13, examrep2 13, tt 12, cards 12, theses 11, grades 11,
surveys 10, attendance 10, groups 9, progs 8, payments 8, geo 8, events 7, oauth 6, terms 4, news 4, calendar 2.

### 3.3 Terms (live)

`GET /services/terms/terms_index` (anon) returns every academic term:
`{"id":"2025L","name":{"pl":"Semestr letni 2025/26","en":"Summer semester 2025/26"},"start_date":"2026-02-16",
"end_date":"2026-06-07","order_key":3010.0,"finish_date":"2026-09-30","is_active":true}`.
Term id patterns on UW: `2025Z` (winter semester), `2025L` (summer), `2025` (whole academic year), `2025TZ`
(winter trimester), plus historic years `1966`... Several terms can be `is_active` at the same time (UW returned 8
active), so "current term" must be derived (dates, `order_key`) rather than "the active one". Ids are strings but
look numeric for years; other universities may use other schemes (for example `2023/SL`, `2023/24-Z`), treat them
as opaque.

### 3.4 Timetable details (live docs)

- `tt/user` returns "Activity" objects: common fields `type` (`classgroup`, `classgroup2`, `meeting`, `exam`;
  unknown types may appear), `start_time`, `end_time` (local `yyyy-mm-dd hh:mm:ss`), `name` (LangDict), `url`.
  `classgroup` extras: `course_id, course_name, classtype_name, lecturer_ids (ints), group_number,
  classgroup_profile_url, building_name, building_id, room_number, room_id, unit_id, classtype_id, cgwm_id,
  frequency`. Dates for `other`/`once` frequency classes cannot be derived: all possible days in the term come
  back, which makes the timetable look wrong; `classgroup2` is the confirmed-meeting variant.
- Max window 7 days per call (`days` <= 7). A widget needs a loop over weeks or the iCal feed.
- `tt/upcoming_share` (token + `studies`, `lang`) -> `{"webcal_url": "webcal://..."}` containing a per-user secret
  `key`; `tt/upcoming_ical?user_id=&lang=&key=` then works **anonymously** and returns a plain iCalendar file for
  the "near future" (undocumented length). This is the best low-privilege, offline-friendly source for widgets: the
  widget needs only the key URL, not OAuth tokens, and it is naturally long-lived. Caveat: the key is a bearer
  secret (store it like a token).

---

## 4. Notifications (preview; details in section 6)

Summary: the API has no push to arbitrary third-party apps without Google. Polling `events2/latest_notifications`,
`grades/latest` etc. is the fit; full discussion in section 6.

---

## 5. What is not in the API / scraping, CAS, official stance

Everything below is derived from the official docs plus the live probes in this report. Gaps are flagged.

### 5.1 Data and actions that exist only in USOSweb (no API method found)

Verified by comparing the live `method_index` (UW and PolSl) against what USOSweb offers:

- Not exposed in the API (or only to administrative consumers): password / account management, most
  *registrations* UI flows beyond `registrations/*` (queue/ranking presentation differs per university),
  application workflows of the recruitment system (IRK is a separate system; `applications/*` appears only as
  notification event types, there is no `applications` module), PDF printouts of arbitrary documents (some are
  reachable through `adm_documents/*`, `pit/fetch_pdf`, `emrex/*`, but not generically), free-form
  announcements specific to a faculty page, "dean's office" static pages, and per-university custom modules.
- Methods exist in the index but may not be enabled or may return empty data on a given installation (the
  installation admin can restrict modules). The live PolSl index has 575 methods, i.e. the same surface as UW,
  but whether e.g. `payments/*` or `registrations/*` actually return data for a student is only visible with a
  token. Always probe at runtime (`apiref/method`) and degrade gracefully.
- Write operations for students are limited: exam/registration sign-up (`registrations/register`,
  `exams/batch_register_exam`, `meetings/sign_up`), surveys (`surveys/fill_out*`), photo and user preferences,
  mail (`mailclient/*`), `uprefs`. Anything needing a "click through a wizard" in USOSweb should be treated as
  not available.

### 5.2 CAS / SSO

- The browser login for the OAuth `authorize` step goes to the university's own CAS (PolSl: `usoscas.polsl.pl`;
  UW: a UW CAS/USOS login; some universities federate with eduID / Shibboleth / Microsoft Entra on top of CAS).
  The client application never sees the credentials in the OAuth flow, it only sees `oauth_verifier` or the
  PIN. This is the strong reason to prefer OAuth over any credential-based scraping: SSO/MFA changes at the
  university do not break an OAuth client, they do break a scraper that posts the CAS form.
- The USOS CAS integration guide (MUCI, PDF) describes: a CAS session ticket-granting cookie (`TGC`) at the CAS
  host, service tickets validated by USOSweb which then opens its own PHP session, silent re-creation of an
  expired USOSweb session while the CAS session is alive, and single logout (SLO). Details in section 9.4.

### 5.3 Official stance on scraping / automation

- I found **no document that explicitly bans or explicitly allows** scraping USOSweb. The docs only say the
  API is the supported integration route and that administrators can revoke keys. USOSweb has no `robots.txt`
  policy relevant to authenticated pages that I could verify (PolSl: pages are behind login).
- Practical risk: automated login to CAS with a student password is a credential-handling problem for an
  open-source app (password storage, MFA, captcha/geolocation field on PolSl's form, account lockout), and
  university IT rules (regulamin korzystania z systemów) usually forbid sharing or automating credentials. I do
  not recommend scraping. If a data gap is critical, ask the installation admin or `usos@usos.edu.pl` first.
- Existing clients all use the API (see 2.6.4); none of the FOSS projects I looked at scrape USOSweb.

## 6. Notifications without Google

### 6.1 What the API offers

Live, from `services/apiref/*` and the module description:

1. **`services/events/*` - consumer-level webhooks (PubSubHubbub-style).** The *application owner* calls
   `events/subscribe_event(event_type, callback_url, verify_token?)` once per consumer per event type. Event
   types live: `grades/grade`, `crstests/user_grade`, `crstests/user_point`. USOS verifies the URL with a
   `hub.mode=subscribe&hub.challenge=...` GET (must echo the challenge), then POSTs JSON to the callback:
   `{"event_type":"grades/grade","entry":[{"time":1381951223,"related_user_ids":["123456"],"operation":"create",
   "exam_id":"1","exam_session_number":2}, ...]}` (up to 1000 entries per request), signed by
   `X-Hub-Signature: sha1=<HMAC-SHA1 body keyed with the consumer secret>`. Details: only ids/operation (no
   sensitive data), no retries, no redirects, subscriptions may be removed if the endpoint fails, and delivery is
   "very, very" likely but not guaranteed. A notification reaches you only if the consumer holds a valid token
   for that user with the right scopes (ask for `offline_access`). **This needs a public HTTPS server owned by the
   app publisher**, one subscription per (consumer, event type), so it cannot be run by each end user's phone and
   conflicts with a shared-consumer-key design (the secret HMAC key is the consumer secret).
2. **`events/register_fcm_token(fcm_registration_token, instance_id?, instance_name?)`** (token required) +
   `registered_fcm_tokens`, `test_my_fcm`: USOS API pushes "when a new event happens to the user" to the user's
   FCM instances. The push is sent by USOS' own Firebase project, so a token is only valid for a client built with
   the matching Firebase sender (the official app). A third-party app would need a token obtained under that
   sender id; do not count on this. There is **no UnifiedPush/webpush/generic endpoint** option.
3. **`events2/*` - user-centric notification system.** (Scope `events`.) `events2/latest_notifications` (T req,
   returns "List of objects") is a **polling endpoint** for the user's stored notifications. Live anonymous
   catalogue endpoints:
   - `events2/notification_channels_index` -> channels `1` "USOS MAIL" (email), `2` "Mobilny USOS" (the official
     app push), `3` "Zapis do bazy" ("Save in database": notifications stored and retrievable);
   - `events2/event_types_index` -> ids incl. `grades/grade`, `crstests/user_grade`, `crstests/user_point`,
     `applications/status_changed`, `applications/decision_to_receive`, `applications/decision_received`,
     `applications/new_application`, `meetings/meeting`, `mobility/la`, `theses/exam_report_to_sign`, `mcard`,
     `account_number`, `events/test_my_fcm`, each with `is_enabled`, `is_mandatory`, `send_by_default`;
   - `events2/user_preferences`, `update_user_preference`, `event_preference`, `event_channel_preference` to choose
     per event type and channel (so a client can enable channel `3` for the types it wants to poll);
   - `events2/notifications_schedule*`, `generate_one_time_code2`/`validate_one_time_code` (OTP via email/SMS),
     `statements`/`search_statements` (consent statements the user must sign).
   The exact JSON shape of `latest_notifications` items is not documented in `returns` ("List of objects") and
   must be captured with a real token (to capture on the first authenticated run).
4. **No change-feed / `since` / ETag support** on grades, timetable etc. Besides `events2/latest_notifications`,
   `grades/latest(days=7)` ("grades modified N days ago", sorted descending by modification date) is the
   polling-friendly grade endpoint.

### 6.2 What the official app uses

The official Mobilny USOS 2.0 offers push for "deadlines, grades, and timetable changes"
(https://pb.edu.pl/uci/2025/06/25/mobilny-usos-2-0-nowa-odslona-aplikacji-dla-studentow-i-pracownikow-politechniki-bialostockiej/).
The API exposes channel `2` "Mobilny USOS" and `events/register_fcm_token`, so the push transport is Firebase
Cloud Messaging (inferred from the API, not from the app's source/privacy page, which I could not retrieve).

### 6.3 Implications

- Without Google services the realistic design is **polling** from a scheduled job (WorkManager on Android,
  systemd timer/daemon on desktop), with these cheap probes: `events2/latest_notifications` (after enabling the DB
  channel via `events2` preferences, if the university has it), `grades/latest?days=N`, `crstests/user_points`
  diff, `tt/user` diff for next 7 days, `payments/user_payments`, plus `registrations` windows.
- Token lifetime: polling in the background requires the `offline_access` scope, otherwise the 2-hour token (or a
  logout in USOSweb) breaks it and a user re-login would be needed.
- UnifiedPush would require a relay server run by us (subscribe to `events/*` webhooks with our consumer key and
  forward to the user's UnifiedPush endpoint). That conflicts with the "no central server, local-first" stance and
  with per-user keys (only the consumer owner can subscribe), so treat it as an optional later feature.


## 7. Practical gotchas

- Local time without offset in all datetimes; convert using the installation's timezone.
- `terms`: several active at once; ids are installation-specific strings.
- LangDict: either language may be empty -> fall back to the other.
- IDs mixed int/string in legacy fields.
- Errors: forbidden access is HTTP 400, `field_not_found` is 500 in practice, unknown ids may be generic 500s.
- `tt/user` max 7 days; classgroup activities with `once`/`other` frequency expand to the whole term.
- Not all installations expose all modules/methods; version spread 6.5 -> 8.0.
- `base_url` in `installations` is `http://`; use https. Some installations are under a path prefix.
- `fields` default sets are minimal; always request what you need. Do not request fields that older versions
  lack; probe with `apiref/method`.

## 8. Implications for our client

Recommendations, ordered by importance. "Confirmed" means observed in docs or live probes, "inferred" means my
reasoning.

1. **Consumer key strategy (the central decision).**
   - Confirmed: a key is per installation, self-service, no approval seen, free; OAuth 1.0a has no public-client
     mode; no document forbids shipping a secret in an open-source app; the official text only warns it "might be
     compromised" and suggests a proxy.
   - Recommended hybrid: (a) **bring-your-own-key** as the always-working baseline (user opens
     `<base>/developers/`, pastes key+secret; also the only option for unregistered installations); (b) ship the
     project's own registered keys in the repo for a few installations (start with PolSl) after e-mailing the
     installation admin to say so; (c) design the signer behind an interface so a hosted proxy could be
     added later without touching the core. Treat the embedded secret as public: the key identifies the app,
     not the user, and can be rotated via `revoke_consumer_key`; expect that someone may revoke it, so ship it in a
     config file that can be updated without a new app release (but F-Droid builds are reproducible from source,
     so also allow a user-supplied override).
   - Do not run a central signing proxy by default: it would see every user's tokens, contradicts the local-first
     goal, and still needs one key per installation (OpenUSOS and StudentUsos took that route and carry the
     operational burden).
2. **Auth flow.** `request_token` with `scopes=offline_access|...`, callback via an Android deep link (or `oob` PIN
   for desktop/headless and CLI; loopback redirect to be tested per installation), `authorize` in the system
   browser (Custom Tabs, no WebView, so CAS/SSO/MFA work), `access_token`. Store token+secret in Android Keystore /
   desktop keyring. On 401 run the flow again. Never ask for or store the password.
3. **Minimal scopes.** Request only what the screens need (`grades`, `studies`, `crstests`, `offline_access`,
   `events` for notifications); show the list to the user before the redirect. Probe `apiref/scopes` at runtime
   to drop unavailable ones, because `request_token` fails on unknown scopes.
4. **Notifications without Google.** Do not depend on `register_fcm_token` (tied to the official Firebase
   project) or `subscribe_event` (needs a public webhook server and a consumer-owned subscription). Use polling:
   WorkManager periodic job on Android (15 min minimum, battery-friendly, with user-chosen interval and
   `ConnectivityType`), a systemd user timer or the headless daemon on desktop. Probes: `events2/latest_notifications`
   (first enable channel 3 "Save in database" for the event types in `events2/user_preferences`; the JSON shape
   still needs one authenticated capture), `grades/latest?days=N`, `crstests/user_points` / `user_grades` diffs,
   `payments/user_payments`, `tt/user` diff for the next 7 days. Keep a local "last seen" state to diff. Local
   notifications are then posted by the app. Optional later: UnifiedPush via a self-hostable relay for people who
   run their own consumer key.
5. **Timetable and widgets.** `tt/upcoming_share` returns a per-user secret URL for `tt/upcoming_ical` which works
   without OAuth: ideal for calendar sync and widgets (needs no token refresh), but the URL is a bearer secret
   and must be stored like a token and re-created on revoke.
6. **Multi-university from day one.** Fetch `apisrv/installations` (public, cacheable) for the dropdown; the
   user can also type a base URL. Force https. For each installation read `apisrv/installation` (version,
   institution name, contact), probe `apiref/method_index` once and cache, and gate features on method
   availability, not on version string. Account for versions 6.5 to 8.0 and path-prefixed bases.
7. **Data modelling.** Datetimes are local time without offset: store them as local date-time plus an installation
   time zone (default `Europe/Warsaw`); term ids are opaque strings with installation-specific formats (UW `2025Z`,
   PolSl `2026/2027-Z`); several terms can be "active" at once, so choose by date; ids are strings; LangDict
   `{pl,en}` may have one side empty; always request explicit `fields=` and use the nested selector syntax
   (`a|b[c|d]`) to cut requests.
8. **Error handling.** Parse the JSON error body; map `invalid_token`/401 to re-login, 400 `method_forbidden` to
   hide the feature, 500 `field_not_found` to a programming bug (log). No documented rate limit, so be gentle
   (batch ids with `course_ids=a|b|c`, conditional diffing, jittered polling, back-off on 5xx/HTML pages during
   maintenance).
9. **Testing.** There is no official sandbox or demo installation I could find (`fake-usos` is only a rough mock
   with no OAuth). Build a local mock around recorded fixtures; use the maintainer's real PolSl account for manual
   checks, and never commit tokens or personal data.
10. **No scraping.** Not in scope: password handling, CAS form automation and parsing USOSweb are fragile and
    risky (section 5.3, 9.6).

### Open questions (need an authenticated run or an e-mail)

- Exact JSON shape and retention of `events2/latest_notifications`; which event types PolSl actually delivers to
  channel 3.
- Numeric lifetimes: PolSl USOSapps `sid`, USOSweb idle timeout, CAS TGC, `offline_access` token (and any purge
  policy). Measure with a real login.
- Whether PolSl restricts scopes or the number of self-service keys, and whether a loopback redirect is allowed.
- Whether the MUCI/UW team and each university accept an open-source app with a published key, and any API
  terms not on the public docs (ask `usos@usos.edu.pl` and `Pomoctechniczna.USOS@polsl.pl`).
- Whether `apisrv/info` is public at other installations (it is not at PolSl).

---

## 9. PolSl specifics (Politechnika Śląska)

All "live" facts below come from unauthenticated requests on 2026-11-04. Nothing was registered or logged in.

### 9.1 Installation facts

| Item | Value |
|---|---|
| API base URL | `https://usosapi.polsl.pl/` (this is also the OAuth host; endpoints are `https://usosapi.polsl.pl/services/...`) |
| Listed in `apisrv/installations` | yes, `base_url` `https://usosapi.polsl.pl/` (list says `http://`; use https) |
| Version | `7.3.1.0-1, 7f9c2e12, dirty (2026-07-14)`; `machine_version` 7.3.1.0-1; schema 7.3.1.0 (UW runs 7.3.1.0-2) |
| `services/apisrv/info` | **not public** (empty / forbidden anonymously). Use `services/apisrv/installation` instead (public, returns the record above) |
| Contact | `Pomoctechniczna.USOS@polsl.pl` (from `contact_emails`) |
| Methods | 575 in `apiref/method_index`; same 26 scopes as UW |
| Active terms | the live list returned `2026/2027`, `2026/2027-Z`, `2026/2027-L` and the following years up to `2029/2030` as active. PolSl term ids use the form `2026/2027-Z`, **not** `2026Z` as at UW. Never hardcode the id format |
| Notification channels | 1 "USOS MAIL", 2 "USOS mobile", 3 "Save in database" (all three present) |
| Enabled event types | applications/{decision_received, decision_to_receive, new_application, status_changed}, crstests/{user_grade, user_point}, events/test_my_fcm, grades/grade, mcard, meetings/meeting, mobility/la, payments/{new_payment, new_remittance, upcoming_deadline}, theses/exam_report_to_sign |
| `events/notifier_status` | public: `{"daemon_running": true, "total_pending_events_count": 0}` (useful health probe) |
| Anonymous method calls | `tt/user` returns `method_forbidden` / `user_missing` ("Access token is required"), `fac/faculties` without params returns the usual missing-parameter error. Public data such as `geo/building_index` (60 buildings) works without a token |

Notable difference from the UW term list: the PolSl installation reports many future years as active at once,
so "current term" cannot be derived from `is_active`; derive it from dates (`terms/term` start/end) and the user's
`courses/user` / `grades/terms`.

### 9.2 Registering a consumer key as a student (step by step; I did NOT do this)

The developers page is open and self-service (same software as UW). Observed: `https://usosapi.polsl.pl/developers/`
serves a form that POSTs to `/developers/submit` with fields `appname`, `appurl` and `email`, plus a CSRF token
(`csrftoken` cookie/field).

1. Open `https://usosapi.polsl.pl/developers/` in a browser (no login needed).
2. Fill the form: application name (shown on the consent screen to users, so choose the real project name),
   optional application URL (project page), and a valid e-mail address (used to revoke the key later; a private
   address is accepted, the page does not demand a `@polsl.pl` address).
3. Submit. The key and secret are displayed immediately on the result page. I saw **no approval or moderation
   step** and no mention of a waiting period in the docs. A confirmation e-mail is sent to the address given.
   Copy both values at once, the secret is not meant to be shown again.
4. Choose the callback: the key itself is not bound to a callback URL in the form; the callback is passed at
   runtime as `oauth_callback` in `request_token`. For Android use a custom scheme or the package-based scheme
   (for example `pl.example.usosclient://oauth`), for desktop use `oob` (PIN shown by the page) or a loopback
   `http://127.0.0.1:<port>/cb` if the installation accepts it (to be tested, not verified).
5. To revoke: the same page has a "revoke key" form which needs the registered e-mail (confirmation e-mail).
6. Caveat: the effective rules (maximum number of keys per person, whether keys can be disabled by the admin,
   and whether non-students may use them) are not published. Ask `Pomoctechniczna.USOS@polsl.pl` before
   shipping a public key; keep a polite contact in the `appurl`.

Allowed scopes: self-service keys can request the normal user scopes (`offline_access`, `grades`, `studies`,
`crstests`, `mobile_numbers`, `email`, `photo`, `events`, `payments`, `cards` etc., 26 scopes listed by `apiref/scopes`).
Administrative scopes and `administrative_only` methods (for example `oauth2/impersonated_token`, `usos` internal
admin modules) require an admin consumer key from the installation admin. Whether PolSl additionally filters
individual scopes for self-service keys is **not visible anonymously**; the first real `request_token` call with
`scopes=...` would show a `scope_not_allowed`-style error if so.

### 9.3 USOSweb and login at PolSl

- USOSweb: `https://usosweb.polsl.pl/` (PHP controller style: `kontroler.php?_action=news/default`, `?_action=dla_stud/...`).
- Login: central CAS at `https://usoscas.polsl.pl/cas/login` (Spring-based CAS; login form fields `username`,
  `password`, `execution`, `_eventId`, and a `geolocation` hidden field). The user types the **USOS login
  (indeks-based or `login`), not the `@student.polsl.pl` mail address**; check the field label on the page.
- OAuth `authorize` at PolSl redirects to
  `https://usoscas.polsl.pl/cas/login?service=https://usosapi.polsl.pl/apps/login?next=...`, i.e. the API host has
  its own session (cookie `sid`, "USOSapps" login) created from a CAS service ticket.
- Cookies observed: USOSweb `PHPSESSID` and `lang`; API/USOSapps `sid`; developers page `csrftoken`; CAS `TGC`
  (ticket granting cookie, path `/cas`).
- Numeric lifetimes (USOSweb idle timeout, CAS TGC TTL, USOSapps `sid`) are **not published**; my web searches
  found nothing, and anonymous requests cannot show them. Typical Spring CAS defaults are 2 h idle / 8 h absolute for
  TGC, and PHP default `session.gc_maxlifetime` is 24 min, but this is a guess, not a PolSl fact. A measurement
  needs a logged-in session (cookie `Expires`/`Max-Age` headers, plus an idle test).

### 9.4 Why the USOSweb session drops quickly (and what survives)

From the MUCI "USOS CAS" guide (2008 PDF, still describing the architecture) and what the login redirects show:

- USOSweb keeps its own PHP session (`PHPSESSID`) with a short idle timeout (the common cause of "logged out
  after a few minutes"). That session is a *cache* of the CAS identity.
- When the USOSweb session expires but the CAS session (`TGC`) is still alive, the next click is redirected to
  CAS, which immediately returns a fresh service ticket without asking for the password; the user notices
  nothing. The user gets the login form only when the **CAS** session is gone too.
- Single logout: logging out in one USOS application (or at CAS) invalidates the others. This also kills OAuth
  tokens tied to that user session where the installation implements that (docs: the access token lasts "about
  2 hours or until the user logs out of USOS"; `offline_access` tokens are meant to survive this).
- Consequences for a client that scrapes with a cookie jar: it must hold the `TGC` plus `PHPSESSID`, and any
  logout or server-side idle expiry means a full CAS re-login with the password (or with a stored session
  cookie that is no longer valid). It cannot do this unattended without storing the password.

### 9.5 PolSl-only apps and quirks

- No PolSl-specific mobile app was found for USOS data; PolSl students use the standard Mobilny USOS and
  mLegitymacja (`cards` module, `mcard` event). The mLegitymacja page warns that the digital ID is tied to
  the USOS account and needs the official app.
- USOSweb at PolSl shows maintenance banners (seen on the news page); the API may be unreachable during those
  windows, so treat `5xx` and HTML error pages as "retry later".
- Student Service Office (COS): `COS@polsl.pl`, +48 32 237-10-79 (listed on the PolSl pages for USOS questions).
  The technical USOS address is `Pomoctechniczna.USOS@polsl.pl`.
- PolSl term ids differ from UW (see 9.1) and the installation runs a patched/dirty build `7.3.1.0-1`, one patch
  level below UW: methods added in `7.3.1.0-2` may be missing. Probe `apiref/method` at runtime.

### 9.6 How the app persists login: OAuth access token vs. USOSweb cookie

| | OAuth 1.0a access token (`offline_access`) | USOSweb session cookie |
|---|---|---|
| What is stored | token + token secret (two short strings) in Android Keystore / desktop keyring | `PHPSESSID` + CAS `TGC` (and ideally the password to renew) |
| Lifetime | Without `offline_access`: about 2 h or until the user logs out of USOS. With `offline_access`: long-lived (months+, until revoked) | USOSweb idle expiry is short and the CAS session is bounded; re-login needs the password |
| Renewal | None needed; if a call returns 401 (`invalid_token`), run the browser flow again once | Re-login against CAS form: fragile (execution token, geolocation field, MFA / captcha possible) |
| Revocation | By the user in the USOSapps Administration Panel (list of authorized apps), by the app with `oauth/revoke_token`, by the consumer owner with `revoke_consumer_key` (kills every user of that key) | Logout anywhere (SLO) |
| Scope / privacy | Only the scopes granted; the app never sees the password | Full account access, password at risk |
| Stability | Documented, versioned API | Undocumented HTML/forms, breaks on UI changes |

Recommendation: request `offline_access` in the `scopes` of `request_token`, persist `oauth_token` and
`oauth_token_secret` in the platform keystore, and on HTTP 401 (or `invalid_token`) show a "log in again" prompt that
re-runs the OAuth flow. Do not store passwords, do not scrape. Lifetime of `offline_access` tokens at PolSl itself
is not published; confirm empirically (store token, retry after 24 h, 7 d, 30 d) and ask the admin whether tokens
are purged at some interval.
