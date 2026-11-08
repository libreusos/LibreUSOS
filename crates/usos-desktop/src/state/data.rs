//! What the app knows about the user's data and how fresh it is. Everything here is plain data
//! and pure functions (no GPUI, no I/O) except [`fetch_all`], which only talks to the public
//! `usos-store` repo functions.

use std::sync::Arc;

use chrono::{DateTime, Days, Local, NaiveDate, Utc};
use usos_core::timetable::week_of;
use usos_core::{Client, Error as CoreError, Term, User, current_term};
use usos_store::repo::{
    self, CourseTestsResult, Identity, PlanResult, Source, TermGradesResult, TermsResult,
};
use usos_store::{RepoError, Store};

/// How many weeks of timetable are kept ready: the current week and the ones after it.
pub const PLAN_WEEKS: u32 = 4;

/// Everything the screens draw from. Each part is `None` until it was fetched or cached once.
/// Screen view-models are pure functions of this struct.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub user: Option<User>,
    /// The cache key of the account (`<base_url>#<user id>`).
    pub account: Option<String>,
    /// The timetable from `plan_from` to `plan_to` (inclusive).
    pub plan: Option<PlanResult>,
    pub plan_from: Option<NaiveDate>,
    pub plan_to: Option<NaiveDate>,
    pub terms: Option<TermsResult>,
    /// The current term, picked from `terms`.
    pub term: Option<Term>,
    /// Grades of the current term.
    pub grades: Option<TermGradesResult>,
    /// Course tests of the current term.
    pub tests: Option<CourseTestsResult>,
}

/// The sync state the sidebar footer and the banners show.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Nothing to sync (signed out).
    Idle,
    Syncing,
    /// Everything came from USOS at `at`.
    Fresh {
        at: DateTime<Local>,
    },
    /// The network is down; the data is from the cache (from `cached_at`, when there is any).
    Offline {
        cached_at: Option<DateTime<Utc>>,
    },
    /// USOS rejected the token: the user has to sign in again.
    Expired,
    /// Something else failed. The text is for the "Szczegóły" disclosure only.
    Error {
        details: String,
    },
}

/// Why one fetch failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Unauthorized,
    Network,
    Other(String),
}

/// Sort an error of the repo layer.
pub fn classify(error: &RepoError) -> Failure {
    match error {
        RepoError::Core(CoreError::NotLoggedIn) => Failure::Unauthorized,
        RepoError::Core(CoreError::Api(api)) if api.is_unauthorized() => Failure::Unauthorized,
        RepoError::Core(e) if repo::is_network_error(e) => Failure::Network,
        other => Failure::Other(other.to_string()),
    }
}

/// How one part of a fetch ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Live,
    Cached(DateTime<Utc>),
    Failed(Failure),
}

/// Fold the outcomes of a fetch into one status. An expired session beats any other error,
/// which beats "offline", which beats "fresh".
pub fn derive_status(outcomes: &[Outcome], now: DateTime<Local>) -> SyncStatus {
    let mut expired = false;
    let mut other: Option<&str> = None;
    let mut offline = false;
    let mut oldest: Option<DateTime<Utc>> = None;
    for outcome in outcomes {
        match outcome {
            Outcome::Live => {}
            Outcome::Cached(at) => {
                offline = true;
                oldest = Some(oldest.map_or(*at, |o| o.min(*at)));
            }
            Outcome::Failed(Failure::Unauthorized) => expired = true,
            Outcome::Failed(Failure::Network) => offline = true,
            Outcome::Failed(Failure::Other(text)) => other = other.or(Some(text)),
        }
    }
    if expired {
        SyncStatus::Expired
    } else if let Some(details) = other {
        SyncStatus::Error {
            details: details.to_owned(),
        }
    } else if offline {
        SyncStatus::Offline { cached_at: oldest }
    } else {
        SyncStatus::Fresh { at: now }
    }
}

/// The timetable range kept ready: the Monday of `today`'s week, for [`PLAN_WEEKS`] weeks.
pub fn plan_range(today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let from = week_of(today).ok()?.from;
    let to = from.checked_add_days(Days::new(u64::from(PLAN_WEEKS) * 7 - 1))?;
    Some((from, to))
}

/// The result of [`fetch_all`]: the new snapshot and how each part ended.
pub struct Fetched {
    pub snapshot: Snapshot,
    pub outcomes: Vec<Outcome>,
}

/// One part: online, or the cache when the network is down. When the live call failed for any
/// other reason (an expired token, say) the cache is still read for display, but the failure is
/// what gets reported.
async fn part<T>(
    live: impl Future<Output = Result<T, RepoError>>,
    cached: impl FnOnce() -> Result<T, RepoError>,
    source: impl Fn(&T) -> Source,
) -> (Option<T>, Outcome) {
    match live.await {
        Ok(value) => {
            let outcome = match source(&value) {
                Source::Live => Outcome::Live,
                Source::Cached { fetched_at } => Outcome::Cached(fetched_at),
            };
            (Some(value), outcome)
        }
        Err(error) => {
            let failure = classify(&error);
            (cached().ok(), Outcome::Failed(failure))
        }
    }
}

/// Fetch (or read from the cache) everything the screens need, using `previous` for parts that
/// fail and have no cache either.
pub async fn fetch_all(
    client: Client,
    store: Arc<Store>,
    today: NaiveDate,
    previous: Snapshot,
) -> Fetched {
    let mut outcomes = Vec::new();
    let mut snapshot = previous;

    let (identity, outcome) = part(
        repo::identify_or_cached(&client, &store),
        || repo::cached_identity(&store, client.base_url().as_str()),
        |r: &Identity| r.source,
    )
    .await;
    // Without an account there is no cache key, so nothing else can be asked.
    let Some(identity) = identity else {
        outcomes.push(outcome);
        return Fetched { snapshot, outcomes };
    };
    outcomes.push(outcome);
    let account = identity.account.clone();
    snapshot.user = Some(identity.user);
    snapshot.account = Some(identity.account);

    let range = plan_range(today);
    let (terms_part, plan_part) = tokio::join!(
        part(
            repo::terms_or_cached(&client, &store, &account),
            || repo::cached_terms(&store, &account),
            |r: &TermsResult| r.source,
        ),
        async {
            match range {
                Some((from, to)) => Some(
                    part(
                        repo::plan_or_cached(&client, &store, &account, from, to),
                        || repo::cached_plan(&store, &account, from, to),
                        |r: &PlanResult| r.source,
                    )
                    .await,
                ),
                None => None,
            }
        }
    );

    let (terms, outcome) = terms_part;
    outcomes.push(outcome);
    if terms.is_some() {
        snapshot.terms = terms;
    }
    if let Some((plan, outcome)) = plan_part {
        outcomes.push(outcome);
        if plan.is_some() {
            snapshot.plan = plan;
            snapshot.plan_from = range.map(|r| r.0);
            snapshot.plan_to = range.map(|r| r.1);
        }
    }

    pick_current_term(&mut snapshot, today);
    if let Some(term) = snapshot.term.clone() {
        let ids = vec![term.id.clone()];
        let (grades_part, tests_part) = tokio::join!(
            part(
                async {
                    repo::term_grades_or_cached(&client, &store, &account, &ids)
                        .await
                        .and_then(first_term)
                },
                || repo::cached_term_grades(&store, &account, &ids).and_then(first_term),
                |r: &TermGradesResult| r.source,
            ),
            part(
                repo::course_tests_or_cached(&client, &store, &account, &term.id),
                || repo::cached_course_tests(&store, &account, &term.id),
                |r: &CourseTestsResult| r.source,
            )
        );
        let (grades, outcome) = grades_part;
        outcomes.push(outcome);
        if grades.is_some() {
            snapshot.grades = grades;
        }
        let (tests, outcome) = tests_part;
        outcomes.push(outcome);
        if tests.is_some() {
            snapshot.tests = tests;
        }
    }
    Fetched { snapshot, outcomes }
}

/// Pick the current term from `snapshot.terms`. Grades and tests of a different term are
/// dropped, so they cannot pass as the new term's when its fetch fails.
fn pick_current_term(snapshot: &mut Snapshot, today: NaiveDate) {
    let previous = snapshot.term.as_ref().map(|t| t.id.clone());
    snapshot.term = snapshot
        .terms
        .as_ref()
        .and_then(|t| current_term(&t.terms, today));
    if snapshot.term.as_ref().map(|t| &t.id) != previous.as_ref() {
        snapshot.grades = None;
        snapshot.tests = None;
    }
}

fn first_term(mut results: Vec<TermGradesResult>) -> Result<TermGradesResult, RepoError> {
    if results.is_empty() {
        Err(RepoError::NotCached {
            what: "grades".into(),
        })
    } else {
        Ok(results.remove(0))
    }
}

/// What is on disk, without touching the network: shown at once while the first sync runs.
pub fn load_cached(store: &Store, base_url: &str, today: NaiveDate) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let Ok(identity) = repo::cached_identity(store, base_url) else {
        return snapshot;
    };
    let account = identity.account;
    snapshot.user = Some(identity.user);
    if let Some((from, to)) = plan_range(today)
        && let Ok(plan) = repo::cached_plan(store, &account, from, to)
    {
        snapshot.plan = Some(plan);
        snapshot.plan_from = Some(from);
        snapshot.plan_to = Some(to);
    }
    snapshot.terms = repo::cached_terms(store, &account).ok();
    snapshot.term = snapshot
        .terms
        .as_ref()
        .and_then(|t| current_term(&t.terms, today));
    if let Some(term) = &snapshot.term {
        let ids = vec![term.id.clone()];
        snapshot.grades = repo::cached_term_grades(store, &account, &ids)
            .and_then(first_term)
            .ok();
        snapshot.tests = repo::cached_course_tests(store, &account, &term.id).ok();
    }
    snapshot.account = Some(account);
    snapshot
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 10, 10, 52, 0).unwrap()
    }

    fn cached(h: u32) -> Outcome {
        Outcome::Cached(Utc.with_ymd_and_hms(2026, 11, 9, h, 0, 0).unwrap())
    }

    #[test]
    fn all_live_is_fresh() {
        assert_eq!(
            derive_status(&[Outcome::Live, Outcome::Live], now()),
            SyncStatus::Fresh { at: now() }
        );
        assert_eq!(derive_status(&[], now()), SyncStatus::Fresh { at: now() });
    }

    #[test]
    fn cached_parts_mean_offline_with_the_oldest_time() {
        let status = derive_status(&[Outcome::Live, cached(21), cached(8)], now());
        assert_eq!(
            status,
            SyncStatus::Offline {
                cached_at: Some(Utc.with_ymd_and_hms(2026, 11, 9, 8, 0, 0).unwrap())
            }
        );
        assert_eq!(
            derive_status(&[Outcome::Failed(Failure::Network)], now()),
            SyncStatus::Offline { cached_at: None }
        );
    }

    #[test]
    fn expired_beats_other_errors_beats_offline() {
        let other = Outcome::Failed(Failure::Other("boom".into()));
        let expired = Outcome::Failed(Failure::Unauthorized);
        assert_eq!(
            derive_status(&[cached(8), other.clone(), expired], now()),
            SyncStatus::Expired
        );
        assert_eq!(
            derive_status(&[cached(8), other], now()),
            SyncStatus::Error {
                details: "boom".into()
            }
        );
    }

    #[test]
    fn errors_are_classified() {
        use usos_core::ApiError;
        let unauthorized = RepoError::Core(CoreError::Api(ApiError::from_response(
            401,
            "bad token",
            str::to_owned,
        )));
        assert_eq!(classify(&unauthorized), Failure::Unauthorized);
        let forbidden = RepoError::Core(CoreError::Api(ApiError::from_response(
            400,
            r#"{"error":"method_forbidden"}"#,
            str::to_owned,
        )));
        assert!(matches!(classify(&forbidden), Failure::Other(_)));
        assert_eq!(
            classify(&RepoError::Core(CoreError::NotLoggedIn)),
            Failure::Unauthorized
        );
        assert!(matches!(
            classify(&RepoError::NotCached {
                what: "plan".into()
            }),
            Failure::Other(_)
        ));
    }

    #[test]
    fn plan_range_starts_on_monday() {
        // 2026-11-10 is a Tuesday.
        let today = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let (from, to) = plan_range(today).unwrap();
        assert_eq!(from, NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());
        assert_eq!((to - from).num_days(), i64::from(PLAN_WEEKS) * 7 - 1);
    }

    #[test]
    fn empty_store_gives_an_empty_snapshot() {
        let store = Store::open_in_memory().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        assert_eq!(
            load_cached(&store, "https://usos.example/", today),
            Snapshot::default()
        );
    }

    fn term(id: &str, start: (i32, u32, u32), end: (i32, u32, u32)) -> usos_core::terms::Term {
        usos_core::terms::Term {
            id: id.into(),
            name: Default::default(),
            start_date: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            end_date: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            finish_date: None,
            order_key: None,
            is_active: None,
        }
    }

    #[test]
    fn a_new_current_term_drops_the_old_terms_results() {
        let winter = term("2026/2027-Z", (2026, 10, 1), (2027, 2, 20));
        let summer = term("2026/2027-L", (2027, 2, 21), (2027, 9, 30));
        let mut snapshot = Snapshot {
            terms: Some(TermsResult {
                terms: vec![winter.clone(), summer.clone()],
                source: Source::Live,
                changed: false,
            }),
            term: Some(winter.clone()),
            grades: Some(TermGradesResult {
                grades: usos_core::grades::TermGrades {
                    term_id: winter.id.clone(),
                    courses: vec![],
                    average: None,
                    weighted_average: None,
                },
                source: Source::Live,
                changed: false,
            }),
            tests: Some(CourseTestsResult {
                tests: Default::default(),
                source: Source::Live,
                changed: false,
            }),
            ..Snapshot::default()
        };

        // Same term: results stay.
        pick_current_term(&mut snapshot, NaiveDate::from_ymd_opt(2027, 1, 10).unwrap());
        assert_eq!(snapshot.term.as_ref().map(|t| &t.id), Some(&winter.id));
        assert!(snapshot.grades.is_some() && snapshot.tests.is_some());

        // The summer term starts: the winter results go.
        pick_current_term(&mut snapshot, NaiveDate::from_ymd_opt(2027, 3, 1).unwrap());
        assert_eq!(snapshot.term.as_ref().map(|t| &t.id), Some(&summer.id));
        assert!(snapshot.grades.is_none() && snapshot.tests.is_none());
    }
}
