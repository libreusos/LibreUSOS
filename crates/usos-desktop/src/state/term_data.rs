//! Grades and tests of terms other than the current one (the term picker of Oceny and
//! Sprawdziany). The snapshot only carries the current term; anything else is fetched on demand
//! and kept here until the user signs out.

use std::sync::Arc;

use usos_core::Client;
use usos_store::Store;
use usos_store::repo::{self, CourseTestsResult, TermGradesResult};

/// How far a term's load got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermLoad {
    Loading,
    Loaded,
    /// Neither USOS nor the cache could provide anything.
    Failed,
}

/// What is known about one term.
#[derive(Debug, Clone, PartialEq)]
pub struct TermData {
    pub grades: Option<TermGradesResult>,
    pub tests: Option<CourseTestsResult>,
    pub load: TermLoad,
    /// A sync finished after this was fetched: still shown, but fetched again on the next visit.
    pub stale: bool,
}

impl TermData {
    pub fn loading() -> Self {
        Self {
            grades: None,
            tests: None,
            load: TermLoad::Loading,
            stale: false,
        }
    }
}

/// Fetch the grades and the tests of `term_id`; when the network is down the cache answers, and a
/// part that cannot be had stays `None`.
pub async fn fetch_term(
    client: Client,
    store: Arc<Store>,
    account: String,
    term_id: String,
) -> TermData {
    let ids = vec![term_id.clone()];
    let (grades, tests) = tokio::join!(
        async {
            match repo::term_grades_or_cached(&client, &store, &account, &ids).await {
                Ok(results) => results.into_iter().next(),
                Err(_) => repo::cached_term_grades(&store, &account, &ids)
                    .ok()
                    .and_then(|results| results.into_iter().next()),
            }
        },
        async {
            match repo::course_tests_or_cached(&client, &store, &account, &term_id).await {
                Ok(result) => Some(result),
                Err(_) => repo::cached_course_tests(&store, &account, &term_id).ok(),
            }
        }
    );
    let load = if grades.is_some() || tests.is_some() {
        TermLoad::Loaded
    } else {
        TermLoad::Failed
    };
    TermData {
        grades,
        tests,
        load,
        stale: false,
    }
}
