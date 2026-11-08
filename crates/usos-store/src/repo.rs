//! Online-or-cached orchestration over [`usos_core::Client`] and a [`Store`].
//!
//! For each kind of data there are three functions:
//! - `refresh_*`: fetch from USOS and store the result; any failure propagates. A logout
//!   ([`forget_account`]) while the fetch runs wins: the result is not stored and the call fails
//!   with [`StoreError::LoggedOut`](crate::StoreError::LoggedOut).
//! - `cached_*`: read the store only (fails with [`RepoError::NotCached`] when empty).
//! - `*_or_cached`: `refresh_*`, and only when the network is unreachable (see
//!   [`is_network_error`]) fall back to `cached_*`. Authorization (401) and other API errors
//!   propagate: they mean something the cache cannot fix, and stale data would hide them. If the
//!   cache is empty too, the original network error is returned.
//!
//! All results say where they came from ([`Source`]) and, for live data, whether it differs from
//! what was cached before (`changed`).

use chrono::{DateTime, Days, NaiveDate, Utc};
use usos_core::crstests::CourseTests;
use usos_core::timetable::week_of;
use usos_core::{Activity, Client, Error, Term, TermGrades, User, current_term};

use crate::activities::starts_in;
use crate::error::RepoError;
use crate::snapshots::PutOutcome;
use crate::store::{Store, account_key};

type Result<T> = std::result::Result<T, RepoError>;

/// Where a result came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Just fetched from USOS (and stored).
    Live,
    /// Served from the cache, fetched at `fetched_at`.
    Cached { fetched_at: DateTime<Utc> },
}

/// Whether `error` means "the server could not be reached": a transport failure or timeout.
pub fn is_network_error(error: &Error) -> bool {
    matches!(error, Error::Http(_))
}

/// The logged-in user and the account key their cache rows use.
#[derive(Debug, Clone, PartialEq)]
pub struct Identity {
    pub account: String,
    pub user: User,
    pub source: Source,
    /// Live only: the user differs from the cached one.
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanResult {
    /// Activities starting on a day of the requested range, sorted by start time.
    pub activities: Vec<Activity>,
    pub source: Source,
    pub changed: bool,
    /// Days of the requested range that are covered (all of them when live).
    pub days_cached: u32,
    /// Days in the requested range.
    pub days_total: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TermsResult {
    pub terms: Vec<Term>,
    pub source: Source,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TermGradesResult {
    pub grades: TermGrades,
    pub source: Source,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CourseTestsResult {
    pub tests: CourseTests,
    pub source: Source,
    pub changed: bool,
}

/// Try `live`; when it fails because the network is down, try `cached`.
async fn live_or_cached<T>(
    live: impl Future<Output = Result<T>>,
    cached: impl FnOnce() -> Result<T>,
) -> Result<T> {
    match live.await {
        Err(RepoError::Core(e)) if is_network_error(&e) => cached().map_err(|_| RepoError::Core(e)),
        other => other,
    }
}

fn days_in(from: NaiveDate, to: NaiveDate) -> u32 {
    ((to - from).num_days() + 1).max(0) as u32
}

fn not_cached(what: &str) -> RepoError {
    RepoError::NotCached { what: what.into() }
}

fn live(outcome: PutOutcome) -> (Source, bool) {
    (Source::Live, outcome.changed)
}

fn from_cache(fetched_at: DateTime<Utc>) -> (Source, bool) {
    (Source::Cached { fetched_at }, false)
}

// ---- identity ----

/// Ask the server who the token belongs to (`users/user`), store the user and make the account
/// the active one for this installation. A logout while the request runs wins: nothing is
/// stored and the result is [`StoreError::LoggedOut`](crate::StoreError::LoggedOut).
pub async fn refresh_identity(client: &Client, store: &Store) -> Result<Identity> {
    let base_url = client.base_url().as_str();
    let epoch = store.session_epoch(base_url)?;
    let user = client.current_user().await?;
    remember(store, base_url, user, Some(epoch))
}

/// Make `user` the active account for `base_url` and cache them (what a login does with the
/// user it learned). Rows of other accounts are left alone.
pub fn remember_identity(store: &Store, base_url: &str, user: User) -> Result<Identity> {
    remember(store, base_url, user, None)
}

fn remember(store: &Store, base_url: &str, user: User, epoch: Option<u64>) -> Result<Identity> {
    let account = account_key(base_url, &user.id);
    let outcome = store.remember_user(base_url, &account, &user, epoch)?;
    Ok(Identity {
        account,
        user,
        source: Source::Live,
        changed: outcome.changed,
    })
}

/// Logout: delete everything cached for the active account of `base_url` and forget that it
/// was active. Returns whether there was an active account. A fetch still running for it
/// stores nothing afterwards (see [`Store::forget_active_account`]).
pub fn forget_account(store: &Store, base_url: &str) -> Result<bool> {
    Ok(store.forget_active_account(base_url)?.is_some())
}

/// The last active account for `base_url` and its cached user, without any network access.
pub fn cached_identity(store: &Store, base_url: &str) -> Result<Identity> {
    let account = store
        .active_account(base_url)?
        .ok_or_else(|| not_cached("account"))?;
    let cached = store
        .get_user(&account)?
        .ok_or_else(|| not_cached("user"))?;
    Ok(Identity {
        account,
        user: cached.user,
        source: Source::Cached {
            fetched_at: cached.fetched_at,
        },
        changed: false,
    })
}

pub async fn identify_or_cached(client: &Client, store: &Store) -> Result<Identity> {
    live_or_cached(refresh_identity(client, store), || {
        cached_identity(store, client.base_url().as_str())
    })
    .await
}

// ---- terms ----

pub async fn refresh_terms(client: &Client, store: &Store, account: &str) -> Result<TermsResult> {
    let terms = client.user_terms().await?;
    let (source, changed) = live(store.put_terms_if_active(account, &terms)?);
    Ok(TermsResult {
        terms,
        source,
        changed,
    })
}

pub fn cached_terms(store: &Store, account: &str) -> Result<TermsResult> {
    let c = store
        .get_terms(account)?
        .ok_or_else(|| not_cached("terms"))?;
    let (source, changed) = from_cache(c.fetched_at);
    Ok(TermsResult {
        terms: c.terms,
        source,
        changed,
    })
}

pub async fn terms_or_cached(client: &Client, store: &Store, account: &str) -> Result<TermsResult> {
    live_or_cached(refresh_terms(client, store, account), || {
        cached_terms(store, account)
    })
    .await
}

// ---- plan ----

/// Fetch the timetable for `from..=to` and replace that range in the store.
pub async fn refresh_plan(
    client: &Client,
    store: &Store,
    account: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<PlanResult> {
    let mut activities = client.timetable_range(from, to).await?;
    // The same predicate the store applies, so live and cached results agree.
    activities.retain(|a| starts_in(a, from, to));
    let outcome = store.replace_activities_if_active(account, from, to, &activities)?;
    let days = days_in(from, to);
    Ok(PlanResult {
        activities,
        source: Source::Live,
        changed: outcome.changed,
        days_cached: days,
        days_total: days,
    })
}

/// The cached timetable for `from..=to`. `NotCached` when no day of the range was ever fetched;
/// otherwise the source time is that of the stalest cached day (see
/// [`Store::activity_coverage`] to learn whether the whole range is covered).
pub fn cached_plan(
    store: &Store,
    account: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<PlanResult> {
    let coverage = store.activity_coverage(account, from, to)?;
    let fetched_at = coverage
        .oldest_fetched_at
        .ok_or_else(|| not_cached("plan"))?;
    let (source, changed) = from_cache(fetched_at);
    Ok(PlanResult {
        activities: store.activities(account, from, to)?,
        source,
        changed,
        days_cached: coverage.days_cached,
        days_total: coverage.days_total,
    })
}

pub async fn plan_or_cached(
    client: &Client,
    store: &Store,
    account: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<PlanResult> {
    live_or_cached(refresh_plan(client, store, account, from, to), || {
        cached_plan(store, account, from, to)
    })
    .await
}

// ---- grades ----

/// Fetch the grades of `term_ids` with one live call and store each term separately (a term
/// USOS knows nothing about gets an empty `TermGrades`). Results are in the order of `term_ids`.
pub async fn refresh_term_grades(
    client: &Client,
    store: &Store,
    account: &str,
    term_ids: &[String],
) -> Result<Vec<TermGradesResult>> {
    let mut fetched = client.term_grades(term_ids).await?;
    let mut out = Vec::with_capacity(term_ids.len());
    for id in term_ids {
        let grades = match fetched.iter().position(|g| g.term_id == *id) {
            Some(i) => fetched.swap_remove(i),
            None => TermGrades {
                term_id: id.clone(),
                courses: Vec::new(),
                average: None,
                weighted_average: None,
            },
        };
        let (source, changed) = live(store.put_term_grades_if_active(account, id, &grades)?);
        out.push(TermGradesResult {
            grades,
            source,
            changed,
        });
    }
    Ok(out)
}

/// The cached grades of those of `term_ids` that were ever fetched, in that order. `NotCached`
/// when none of them was.
pub fn cached_term_grades(
    store: &Store,
    account: &str,
    term_ids: &[String],
) -> Result<Vec<TermGradesResult>> {
    let mut out = Vec::new();
    for id in term_ids {
        if let Some(c) = store.get_term_grades(account, id)? {
            let (source, changed) = from_cache(c.fetched_at);
            out.push(TermGradesResult {
                grades: c.grades,
                source,
                changed,
            });
        }
    }
    if out.is_empty() && !term_ids.is_empty() {
        return Err(not_cached("grades"));
    }
    Ok(out)
}

/// Grades of several terms: one live call, or the cache when the network is down.
pub async fn term_grades_or_cached(
    client: &Client,
    store: &Store,
    account: &str,
    term_ids: &[String],
) -> Result<Vec<TermGradesResult>> {
    live_or_cached(
        refresh_term_grades(client, store, account, term_ids),
        || cached_term_grades(store, account, term_ids),
    )
    .await
}

// ---- course tests ----

/// Fetch the course tests of `term_id` and store them. A test USOS refused this time (listed in
/// `skipped`) keeps its cached copy instead of vanishing: see [`merge_skipped`].
pub async fn refresh_course_tests(
    client: &Client,
    store: &Store,
    account: &str,
    term_id: &str,
) -> Result<CourseTestsResult> {
    let mut tests = client.course_tests(&[term_id.to_owned()]).await?;
    if !tests.skipped.is_empty()
        && let Some(cached) = store.get_course_tests(account, term_id)?
    {
        tests = merge_skipped(tests, cached.tests);
    }
    let (source, changed) = live(store.put_course_tests_if_active(account, term_id, &tests)?);
    Ok(CourseTestsResult {
        tests,
        source,
        changed,
    })
}

/// `live` plus the tests of `cached` whose root node id is among `live.skipped`.
fn merge_skipped(mut live: CourseTests, cached: CourseTests) -> CourseTests {
    let skipped = live.skipped.clone();
    live.tests.extend(
        cached
            .tests
            .into_iter()
            .filter(|t| skipped.contains(&t.root.id)),
    );
    live
}

pub fn cached_course_tests(
    store: &Store,
    account: &str,
    term_id: &str,
) -> Result<CourseTestsResult> {
    let c = store
        .get_course_tests(account, term_id)?
        .ok_or_else(|| not_cached("tests"))?;
    let (source, changed) = from_cache(c.fetched_at);
    Ok(CourseTestsResult {
        tests: c.tests,
        source,
        changed,
    })
}

pub async fn course_tests_or_cached(
    client: &Client,
    store: &Store,
    account: &str,
    term_id: &str,
) -> Result<CourseTestsResult> {
    live_or_cached(
        refresh_course_tests(client, store, account, term_id),
        || cached_course_tests(store, account, term_id),
    )
    .await
}

// ---- sync ----

/// What [`sync`] fetched. Counts and change flags only.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncReport {
    pub account: String,
    pub user_changed: bool,
    pub terms: u32,
    pub terms_changed: bool,
    pub plan_from: NaiveDate,
    pub plan_to: NaiveDate,
    pub activities: u32,
    pub plan_changed: bool,
    /// The current term; the fields below are zero/false when there is none.
    pub term_id: Option<String>,
    pub courses: u32,
    pub grades_changed: bool,
    pub tests: u32,
    /// Tests USOS refused (see [`refresh_course_tests`]).
    pub tests_skipped: u32,
    pub tests_changed: bool,
}

/// Refresh everything the offline views need: the user, the terms, the timetable from the Monday
/// of `today`'s week for `weeks` weeks (at least 1), and the grades and tests of the current
/// term. Any failure aborts the sync (what was stored before it stays).
pub async fn sync(
    client: &Client,
    store: &Store,
    today: NaiveDate,
    weeks: u32,
) -> Result<SyncReport> {
    let weeks = weeks.max(1);
    let identity = refresh_identity(client, store).await?;
    let account = identity.account.as_str();
    let terms = refresh_terms(client, store, account).await?;

    let plan_from = week_of(today)?.from;
    let plan_to = plan_from
        .checked_add_days(Days::new(u64::from(weeks) * 7 - 1))
        .ok_or_else(|| Error::InvalidArgument("the sync range is out of the date limits".into()))?;
    let plan = refresh_plan(client, store, account, plan_from, plan_to).await?;

    let mut report = SyncReport {
        account: identity.account.clone(),
        user_changed: identity.changed,
        terms: terms.terms.len() as u32,
        terms_changed: terms.changed,
        plan_from,
        plan_to,
        activities: plan.activities.len() as u32,
        plan_changed: plan.changed,
        term_id: None,
        courses: 0,
        grades_changed: false,
        tests: 0,
        tests_skipped: 0,
        tests_changed: false,
    };
    if let Some(term) = current_term(&terms.terms, today) {
        let grades = refresh_term_grades(client, store, account, std::slice::from_ref(&term.id))
            .await?
            .remove(0);
        let tests = refresh_course_tests(client, store, account, &term.id).await?;
        report.courses = grades.grades.courses.len() as u32;
        report.grades_changed = grades.changed;
        report.tests = tests.tests.tests.len() as u32;
        report.tests_skipped = tests.tests.skipped.len() as u32;
        report.tests_changed = tests.changed;
        report.term_id = Some(term.id);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use usos_core::AccessToken;
    use usos_core::Secret;

    fn assert_send<T: Send>(_: T) {}

    /// Every repo future must be `Send`, so a host can run them on a multi-threaded runtime.
    /// Compile-time only: nothing is awaited.
    #[test]
    fn repo_futures_are_send() {
        let client = Client::new("https://example.invalid/", "k", "s").unwrap();
        let store = Store::open_in_memory().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let ids = vec!["1".to_owned()];
        assert_send(sync(&client, &store, day, 2));
        assert_send(refresh_identity(&client, &store));
        assert_send(identify_or_cached(&client, &store));
        assert_send(refresh_terms(&client, &store, "a"));
        assert_send(terms_or_cached(&client, &store, "a"));
        assert_send(refresh_plan(&client, &store, "a", day, day));
        assert_send(plan_or_cached(&client, &store, "a", day, day));
        assert_send(refresh_term_grades(&client, &store, "a", &ids));
        assert_send(term_grades_or_cached(&client, &store, "a", &ids));
        assert_send(refresh_course_tests(&client, &store, "a", "1"));
        assert_send(course_tests_or_cached(&client, &store, "a", "1"));
    }

    /// A tiny HTTP server: `handler(whole request) -> (status, JSON body)`.
    async fn serve(
        handler: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    return;
                };
                let handler = handler.clone();
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    let (head_end, len) = loop {
                        let n = sock.read(&mut chunk).await.unwrap_or(0);
                        if n == 0 {
                            return;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
                            let len = head
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            break (pos + 4, len);
                        }
                    };
                    while buf.len() < head_end + len {
                        let n = sock.read(&mut chunk).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    let request = String::from_utf8_lossy(&buf);
                    let (status, body) = handler(&request);
                    let response = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(response.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        (url, task)
    }

    fn ok_handler(line: &str) -> (u16, String) {
        let body = if line.contains("users/user") {
            r#"{"id":"42","first_name":"A","last_name":"B"}"#
        } else if line.contains("tt/user") {
            r#"[{"type":"classgroup","start_time":"2026-11-04 08:00:00","end_time":"2026-11-04 09:30:00",
                 "course_id":"C1","course_name":{"en":"Maths","pl":"Matematyka"},"classtype_id":"CW"},
                {"type":"classgroup","start_time":"2026-12-30 08:00:00","end_time":"2026-12-30 09:30:00",
                 "course_id":"C1","course_name":{"en":"Outside","pl":"Poza"},"classtype_id":"CW"}]"#
        } else if line.contains("courses/user") && line.contains("fields=terms") {
            r#"{"terms":[{"id":"T1","name":{"en":"Winter"},"start_date":"2026-10-01","end_date":"2027-02-28"}]}"#
        } else {
            return (400, r#"{"error":"nope","message":"unknown method"}"#.into());
        };
        (200, body.into())
    }

    fn client(base: &str) -> Client {
        Client::new(base, "key", "secret")
            .unwrap()
            .with_access_token(AccessToken {
                token: Secret::new("t"),
                secret: Secret::new("s"),
            })
    }

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[tokio::test]
    async fn live_then_cached_when_the_network_is_down() {
        let store = Store::open_in_memory().unwrap();
        let (base, server) = serve(ok_handler).await;
        let c = client(&base);
        let (from, to) = (d("2026-11-02"), d("2026-11-08"));

        let id = identify_or_cached(&c, &store).await.unwrap();
        assert_eq!(id.source, Source::Live);
        assert_eq!(id.account, format!("{}#42", base.trim_end_matches('/')));

        let first = plan_or_cached(&c, &store, &id.account, from, to)
            .await
            .unwrap();
        assert_eq!(
            (first.source, first.changed, first.activities.len()),
            (Source::Live, true, 1)
        );
        let again = plan_or_cached(&c, &store, &id.account, from, to)
            .await
            .unwrap();
        assert!(!again.changed);
        let terms = terms_or_cached(&c, &store, &id.account).await.unwrap();
        assert_eq!(terms.terms.len(), 1);

        // Switch the server off: the connection is refused (a network error).
        server.abort();
        let _ = server.await;

        let offline = plan_or_cached(&c, &store, &id.account, from, to)
            .await
            .unwrap();
        assert!(matches!(offline.source, Source::Cached { .. }));
        assert_eq!(offline.activities, again.activities);
        assert_eq!((offline.days_cached, offline.days_total), (7, 7));
        // A wider range is only partly covered.
        let wide = cached_plan(&store, &id.account, from, d("2026-11-15")).unwrap();
        assert_eq!((wide.days_cached, wide.days_total), (7, 14));
        assert!(!offline.changed);
        let terms = terms_or_cached(&c, &store, &id.account).await.unwrap();
        assert!(matches!(terms.source, Source::Cached { .. }));
        let who = identify_or_cached(&c, &store).await.unwrap();
        assert_eq!(
            (who.account.as_str(), &who.user),
            (id.account.as_str(), &id.user)
        );
        assert!(matches!(who.source, Source::Cached { .. }));

        // Nothing cached for grades: the original network error comes back.
        match term_grades_or_cached(&c, &store, &id.account, &["T1".to_owned()]).await {
            Err(RepoError::Core(e)) => assert!(is_network_error(&e)),
            other => panic!("unexpected {other:?}"),
        }
        // Cache-only access reports the miss as such.
        assert!(matches!(
            cached_course_tests(&store, &id.account, "T1"),
            Err(RepoError::NotCached { .. })
        ));
        // A range never fetched is a miss too.
        assert!(matches!(
            cached_plan(&store, &id.account, d("2027-01-01"), d("2027-01-07")),
            Err(RepoError::NotCached { .. })
        ));
    }

    #[tokio::test]
    async fn api_errors_do_not_fall_back_to_the_cache() {
        let store = Store::open_in_memory().unwrap();
        let (base, server) = serve(ok_handler).await;
        let c = client(&base);
        let (from, to) = (d("2026-11-02"), d("2026-11-08"));
        let id = identify_or_cached(&c, &store).await.unwrap();
        plan_or_cached(&c, &store, &id.account, from, to)
            .await
            .unwrap();
        server.abort();
        let _ = server.await;

        let (base401, _server) = serve(|_| (401, r#"{"message":"Invalid token"}"#.into())).await;
        let c401 = client(&base401);
        match plan_or_cached(&c401, &store, &id.account, from, to).await {
            Err(RepoError::Core(Error::Api(api))) => assert!(api.is_unauthorized()),
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            identify_or_cached(&c401, &store).await,
            Err(RepoError::Core(Error::Api(_)))
        ));
    }

    #[tokio::test]
    async fn sync_stores_everything_for_the_current_term() {
        let store = Store::open_in_memory().unwrap();
        let (base, _server) = serve(|line| {
            if line.contains("courses/user") && line.contains("course_editions") {
                (200, r#"{"course_editions":{}}"#.into())
            } else if line.contains("grades/terms2") {
                (
                    200,
                    r#"{"T1":{"course_grades":{},"course_units_grades":{}}}"#.into(),
                )
            } else if line.contains("crstests/participant2") {
                (200, "[]".into())
            } else if line.contains("courses/user_ects_points") {
                (200, "{}".into())
            } else {
                ok_handler(line)
            }
        })
        .await;
        let c = client(&base);
        let r = sync(&c, &store, d("2026-11-04"), 3).await.unwrap();
        assert_eq!((r.plan_from, r.plan_to), (d("2026-11-02"), d("2026-11-22")));
        // The fake server answers each of the 3 weekly chunks with the same activity.
        assert_eq!((r.terms, r.activities), (1, 3));
        assert_eq!(r.term_id.as_deref(), Some("T1"));
        assert!(r.plan_changed && r.terms_changed && r.user_changed);
        // Everything is now readable without the network.
        assert_eq!(
            cached_plan(&store, &r.account, r.plan_from, r.plan_to)
                .unwrap()
                .activities
                .len(),
            3
        );
        assert_eq!(
            cached_term_grades(&store, &r.account, &["T1".to_owned()])
                .unwrap()
                .len(),
            1
        );
        assert!(cached_course_tests(&store, &r.account, "T1").is_ok());
        let again = sync(&c, &store, d("2026-11-04"), 3).await.unwrap();
        assert!(!again.plan_changed && !again.terms_changed && !again.user_changed);
    }

    fn test(name: &str, root_id: &str) -> usos_core::TestRoot {
        usos_core::TestRoot {
            course_id: "C".into(),
            course_name: Default::default(),
            term_id: "T1".into(),
            limited_to_groups: false,
            root: usos_core::TestNode {
                id: root_id.into(),
                name: usos_core::LangDict {
                    pl: Some(name.into()),
                    en: None,
                },
                ..Default::default()
            },
        }
    }

    fn names(t: &CourseTests) -> Vec<String> {
        t.tests
            .iter()
            .map(|t| t.root.name.pl.clone().unwrap())
            .collect()
    }

    #[test]
    fn skipped_tests_keep_their_cached_copy() {
        let live = CourseTests {
            tests: vec![test("fresh-1", "1")],
            skipped: vec!["2".into()],
        };
        let cached = CourseTests {
            tests: vec![test("old-1", "1"), test("old-2", "2"), test("old-3", "3")],
            skipped: vec![],
        };
        let merged = merge_skipped(live, cached);
        // 1 replaced, 2 kept (it was skipped), 3 dropped (not skipped, not returned: gone).
        assert_eq!(names(&merged), ["fresh-1", "old-2"]);
        assert_eq!(merged.skipped, ["2"]);
    }

    #[tokio::test]
    async fn refresh_merges_skipped_tests_with_the_cache() {
        let store = Store::open_in_memory().unwrap();
        store.set_active_account("https://u", "a").unwrap();
        let cached = CourseTests {
            tests: vec![test("old-2", "2")],
            skipped: vec![],
        };
        store.put_course_tests("a", "T1", &cached).unwrap();
        // participant2 lists one test whose node call is refused with an API error.
        let (base, _server) = serve(|req| {
            if req.contains("crstests/participant2") {
                (
                    200,
                    r#"[{"course_edition":{"course":{"id":"C","name":{"pl":"X"}},"term":{"id":"T1"}},
                         "root":{"id":2,"name":{"pl":"X"}}}]"#
                        .into(),
                )
            } else {
                (400, r#"{"error":"param_invalid","message":"no such node"}"#.into())
            }
        })
        .await;
        let r = refresh_course_tests(&client(&base), &store, "a", "T1")
            .await
            .unwrap();
        // Whatever the live call produced for the refused test, the cached copy survives.
        assert!(
            r.tests.tests.iter().any(|t| t.root.id == "2"),
            "{:?}",
            r.tests
        );
        assert!(
            store
                .get_course_tests("a", "T1")
                .unwrap()
                .unwrap()
                .tests
                .tests
                .iter()
                .any(|t| t.root.id == "2")
        );
    }

    #[test]
    fn logout_forgets_only_the_active_account() {
        let store = Store::open_in_memory().unwrap();
        let user = |id: &str| User {
            id: id.into(),
            first_name: "A".into(),
            last_name: "B".into(),
        };
        let base = "https://usos.example/";
        let one = remember_identity(&store, base, user("1")).unwrap();
        store
            .replace_activities(&one.account, d("2026-11-01"), d("2026-11-02"), &[])
            .unwrap();
        // A second account of the same installation, not active.
        store
            .put_user("https://usos.example#2", &user("2"))
            .unwrap();

        assert!(forget_account(&store, base).unwrap());
        assert!(matches!(
            cached_identity(&store, base),
            Err(RepoError::NotCached { .. })
        ));
        assert!(store.get_user(&one.account).unwrap().is_none());
        assert_eq!(
            store
                .activity_coverage(&one.account, d("2026-11-01"), d("2026-11-02"))
                .unwrap()
                .days_cached,
            0
        );
        assert!(store.get_user("https://usos.example#2").unwrap().is_some());
        // Nothing active any more: forgetting again is a no-op.
        assert!(!forget_account(&store, base).unwrap());
    }

    /// A server that runs `forget_account` (a logout from another thread or process) while it
    /// answers the first request that contains `during`, and answers like [`ok_handler`].
    async fn serve_with_logout(
        store: Arc<Store>,
        during: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use std::sync::atomic::{AtomicBool, Ordering};
        let base = Arc::new(std::sync::OnceLock::<String>::new());
        let done = AtomicBool::new(false);
        let seen = base.clone();
        let (url, server) = serve(move |req| {
            if req.contains(during) && !done.swap(true, Ordering::SeqCst) {
                forget_account(&store, seen.get().unwrap()).unwrap();
            }
            ok_handler(req)
        })
        .await;
        base.set(url.clone()).unwrap();
        (url, server)
    }

    #[tokio::test]
    async fn logout_during_identity_refresh_stores_nothing() {
        let store = Arc::new(Store::open_in_memory().unwrap());
        let (base, _server) = serve_with_logout(store.clone(), "users/user").await;
        let c = client(&base);

        assert!(matches!(
            refresh_identity(&c, &store).await,
            Err(RepoError::Store(crate::StoreError::LoggedOut))
        ));
        assert_eq!(store.active_account(&base).unwrap(), None);
        let account = account_key(&base, "42");
        assert!(store.get_user(&account).unwrap().is_none());

        // A refresh that starts after the logout is not affected.
        let id = refresh_identity(&c, &store).await.unwrap();
        assert_eq!(id.account, account);
        assert_eq!(store.active_account(&base).unwrap(), Some(account));
    }

    #[tokio::test]
    async fn logout_during_sync_does_not_refill_the_cache() {
        let store = Arc::new(Store::open_in_memory().unwrap());
        // The identity is stored, then the logout comes while the terms are fetched.
        let (base, _server) = serve_with_logout(store.clone(), "fields=terms").await;
        let c = client(&base);

        assert!(matches!(
            sync(&c, &store, d("2026-11-04"), 1).await,
            Err(RepoError::Store(crate::StoreError::LoggedOut))
        ));
        let account = account_key(&base, "42");
        assert_eq!(store.active_account(&base).unwrap(), None);
        assert!(store.get_user(&account).unwrap().is_none());
        assert!(store.get_terms(&account).unwrap().is_none());

        // Later writes of fetches that began before the logout are refused too.
        assert!(matches!(
            refresh_plan(&c, &store, &account, d("2026-11-02"), d("2026-11-08")).await,
            Err(RepoError::Store(crate::StoreError::LoggedOut))
        ));
        assert_eq!(
            store
                .activity_coverage(&account, d("2026-11-02"), d("2026-11-08"))
                .unwrap()
                .days_cached,
            0
        );
    }

    #[test]
    fn logout_bumps_the_session_epoch() {
        let store = Store::open_in_memory().unwrap();
        let base = "https://usos.example/";
        assert_eq!(store.session_epoch(base).unwrap(), 0);
        // Even without an active account: a first identity refresh may be running.
        assert!(!forget_account(&store, base).unwrap());
        assert_eq!(store.session_epoch("https://usos.example").unwrap(), 1);
        assert_eq!(store.session_epoch("https://other.example/").unwrap(), 0);
    }

    #[test]
    fn switching_accounts_keeps_both_data_sets() {
        let store = Store::open_in_memory().unwrap();
        let user = |id: &str, name: &str| User {
            id: id.into(),
            first_name: name.into(),
            last_name: String::new(),
        };
        let base = "https://usos.example";
        let one = remember_identity(&store, base, user("1", "One")).unwrap();
        store.put_terms(&one.account, &[]).unwrap();
        let two = remember_identity(&store, &format!("{base}/"), user("2", "Two")).unwrap();
        assert_ne!(one.account, two.account);
        assert_eq!(
            cached_identity(&store, base).unwrap().user.first_name,
            "Two"
        );
        assert!(cached_terms(&store, &two.account).is_err());
        assert!(cached_terms(&store, &one.account).is_ok());
        // Switching back is just remembering the old user again.
        remember_identity(&store, base, user("1", "One")).unwrap();
        assert_eq!(
            cached_identity(&store, base).unwrap().user.first_name,
            "One"
        );
        assert!(cached_terms(&store, &one.account).is_ok());
    }

    #[tokio::test]
    async fn grades_of_many_terms_take_one_live_call() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let (base, server) = serve(move |req| {
            if req.contains("courses/user") && req.contains("course_editions") {
                (200, r#"{"course_editions":{}}"#.into())
            } else if req.contains("grades/terms2") {
                seen.fetch_add(1, Ordering::SeqCst);
                (
                    200,
                    r#"{"T1":{"course_grades":{},"course_units_grades":{}},"T2":null}"#.into(),
                )
            } else if req.contains("courses/user_ects_points") {
                (200, "{}".into())
            } else {
                ok_handler(req)
            }
        })
        .await;
        let store = Store::open_in_memory().unwrap();
        store.set_active_account("https://u", "a").unwrap();
        let c = client(&base);
        let ids = ["T1".to_owned(), "T2".to_owned()];
        let live = term_grades_or_cached(&c, &store, "a", &ids).await.unwrap();
        assert_eq!(live.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(live[1].grades.term_id, "T2");
        server.abort();
        let _ = server.await;
        let offline = term_grades_or_cached(&c, &store, "a", &ids).await.unwrap();
        assert_eq!(offline.len(), 2);
        assert!(matches!(offline[0].source, Source::Cached { .. }));
        // Some terms cached is fine; none is a miss.
        let mixed = ["T1".to_owned(), "T9".to_owned()];
        assert_eq!(cached_term_grades(&store, "a", &mixed).unwrap().len(), 1);
        assert!(cached_term_grades(&store, "a", &["T9".to_owned()]).is_err());
    }
}
