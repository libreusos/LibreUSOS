//! The offline cache as the commands see it: open the store, find the account, and serve data
//! online with a fall-back to the cache. All of the logic is in `usos-store`; this is wiring and
//! the one-line cache note.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use chrono::{Local, NaiveDate};
use usos_core::{Client, Session, current_term};
use usos_store::repo::{
    self, CourseTestsResult, Identity, PlanResult, Source, TermGradesResult, TermsResult,
};
use usos_store::{RepoError, Store};

use crate::explain_repo;

/// The store, the account, and a client when the server is to be asked at all.
pub struct Data {
    pub store: Store,
    pub identity: Identity,
    client: Option<Client>,
}

/// `<data dir>/libreusos/cache.sqlite`, unless `--cache` / `LIBREUSOS_CACHE` says otherwise.
fn cache_path(custom: Option<PathBuf>) -> Result<String> {
    let path = match custom {
        Some(p) => p,
        None => dirs::data_dir()
            .context("cannot find the user data directory; use --cache PATH")?
            .join("libreusos")
            .join("cache.sqlite"),
    };
    path.into_os_string()
        .into_string()
        .map_err(|p| anyhow::anyhow!("cache path {p:?} is not valid UTF-8"))
}

pub fn open_store(custom: Option<PathBuf>) -> Result<Store> {
    Ok(Store::open(&cache_path(custom)?)?)
}

impl Data {
    /// With `offline` nothing is fetched and `client()` is `None`. Otherwise the account is the
    /// one `login` or `sync` last recorded (no `users/user` call); only when there is none, the
    /// server is asked who the token belongs to.
    pub async fn open(ctx: &Session, custom: Option<PathBuf>, offline: bool) -> Result<Self> {
        let store = open_store(custom)?;
        let cached = repo::cached_identity(&store, ctx.base_url());
        if offline {
            let identity = cached.map_err(|e| match e {
                RepoError::NotCached { .. } => anyhow::anyhow!(
                    "no cached account (not logged in, or logged out); run `usos login` and `usos sync` online first"
                ),
                e => explain_repo(e, "your account"),
            })?;
            return Ok(Self {
                store,
                identity,
                client: None,
            });
        }
        let client = ctx.client()?;
        if !client.is_logged_in() {
            bail!(
                "not logged in to {}; run `usos login` first",
                ctx.base_url()
            );
        }
        let identity = match cached {
            Ok(identity) => identity,
            Err(_) => repo::identify_or_cached(&client, &store)
                .await
                .map_err(|e| explain_repo(e, "your account"))?,
        };
        Ok(Self {
            store,
            identity,
            client: Some(client),
        })
    }

    /// The client, when online.
    pub fn client(&self) -> Option<&Client> {
        self.client.as_ref()
    }

    fn account(&self) -> &str {
        &self.identity.account
    }

    /// One line on stderr when `source` is the cache; `partial` adds "N of M days cached".
    fn note(&self, source: Source, partial: Option<(u32, u32)>) {
        if let Source::Cached { fetched_at } = source {
            let when = fetched_at.with_timezone(&Local).format("%Y-%m-%d %H:%M");
            match partial {
                Some((have, total)) if have != total => {
                    eprintln!("(offline, cached {when}; {have} of {total} days cached)")
                }
                _ => eprintln!("(offline, cached {when})"),
            }
        }
    }

    pub async fn plan(&self, from: NaiveDate, to: NaiveDate) -> Result<PlanResult> {
        let result = match &self.client {
            Some(c) => repo::plan_or_cached(c, &self.store, self.account(), from, to).await,
            None => repo::cached_plan(&self.store, self.account(), from, to),
        }
        .map_err(|e| explain_repo(e, "the timetable"))?;
        self.note(result.source, Some((result.days_cached, result.days_total)));
        Ok(result)
    }

    pub async fn terms(&self) -> Result<TermsResult> {
        let result = self.terms_quiet().await?;
        self.note(result.source, None);
        Ok(result)
    }

    async fn terms_quiet(&self) -> Result<TermsResult> {
        match &self.client {
            Some(c) => repo::terms_or_cached(c, &self.store, self.account()).await,
            None => repo::cached_terms(&self.store, self.account()),
        }
        .map_err(|e| explain_repo(e, "terms"))
    }

    /// The id of the current term among the (cached) terms.
    pub async fn current_term_id(&self, hint: &str) -> Result<String> {
        // No cache note: the terms only pick the term whose data is shown next.
        let terms = self.terms_quiet().await?;
        let today = Local::now().date_naive();
        match current_term(&terms.terms, today) {
            Some(t) => Ok(t.id),
            None => bail!("no current term among your terms; {hint}"),
        }
    }

    /// The grades of `term_ids` in that order, fetched with one call when online. Offline, only
    /// the terms that were ever synced are returned.
    pub async fn term_grades(&self, term_ids: &[String]) -> Result<Vec<TermGradesResult>> {
        let results = match &self.client {
            Some(c) => repo::term_grades_or_cached(c, &self.store, self.account(), term_ids).await,
            None => repo::cached_term_grades(&self.store, self.account(), term_ids),
        }
        .map_err(|e| explain_repo(e, "grades"))?;
        // One note for the stalest cached term.
        let stalest = results
            .iter()
            .filter_map(|r| match r.source {
                Source::Cached { fetched_at } => Some(fetched_at),
                Source::Live => None,
            })
            .min();
        if let Some(fetched_at) = stalest {
            self.note(Source::Cached { fetched_at }, None);
        }
        Ok(results)
    }

    pub async fn course_tests(&self, term_id: &str) -> Result<CourseTestsResult> {
        let result = match &self.client {
            Some(c) => repo::course_tests_or_cached(c, &self.store, self.account(), term_id).await,
            None => repo::cached_course_tests(&self.store, self.account(), term_id),
        }
        .map_err(|e| explain_repo(e, "tests"))?;
        self.note(result.source, None);
        Ok(result)
    }
}
