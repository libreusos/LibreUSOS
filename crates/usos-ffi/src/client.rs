//! The session object Kotlin holds: login, whoami, terms, timetable, sync and logout.
//!
//! Threading: see the crate docs. Call everything from `Dispatchers.IO`.

use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use usos_core::auth::consumer::Consumer;
use usos_core::auth::login::{self, Callback};
use usos_core::session::{ConsumerSource, RevokeOutcome};
use usos_core::{Client, RequestToken, Secret, Session, StoredToken, current_term};
use usos_store::{Store, repo};

use crate::error::UsosError;
use crate::model::{
    self, CourseTestsResult, Identity, SyncReport, Term, TermGrades, TermTests, TermsResult,
    Timetable, User,
};
use crate::token::{CoreTokenStore, TokenStore};

type Result<T> = std::result::Result<T, UsosError>;

/// The time zone of the USOS installation this app targets (PolSl). All times USOS reports
/// (timetable start and end, term dates) are naive local times of this zone, and `today`
/// arguments must be computed in it by Kotlin (`LocalDate.now(ZoneId.of(...))`), not in the
/// device's zone.
#[uniffi::export]
pub fn installation_time_zone() -> String {
    "Europe/Warsaw".to_owned()
}

struct Inner {
    session: Session,
    store: Store,
    base_url: String,
    /// The core client without a token, built (TLS setup included) on first use and cloned
    /// for each call.
    http: Mutex<Option<Client>>,
    /// Serializes `sync` (see [`UsosClient::sync`]).
    sync_lock: tokio::sync::Mutex<()>,
}

impl Inner {
    fn load_token(&self) -> Result<Option<StoredToken>> {
        Ok(self.session.token_store().load(&self.base_url)?)
    }

    fn anonymous_client(&self) -> Result<Client> {
        let mut cached = self.http.lock().map_err(|_| poisoned())?;
        if cached.is_none() {
            *cached = Some(self.session.anonymous_client()?);
        }
        Ok(cached.clone().expect("just initialized"))
    }

    /// A client with the stored token; `Unauthorized` when there is none.
    fn client(&self) -> Result<Client> {
        let token = self.load_token()?.ok_or(usos_core::Error::NotLoggedIn)?;
        let mut client = self.anonymous_client()?;
        client.set_access_token(Some(token));
        Ok(client)
    }

    /// The active cache account, asking the server who we are only when none is known yet.
    async fn account(&self, client: &Client) -> Result<String> {
        if let Some(account) = self.store.active_account(&self.base_url)? {
            return Ok(account);
        }
        Ok(repo::refresh_identity(client, &self.store).await?.account)
    }
}

fn poisoned() -> UsosError {
    UsosError::Other {
        detail: "internal state poisoned".into(),
    }
}

fn date_range(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate)> {
    let (from, to) = (
        model::parse_date("from", from)?,
        model::parse_date("to", to)?,
    );
    if from > to {
        return Err(UsosError::invalid("`from` is after `to`"));
    }
    Ok((from, to))
}

/// Whether this build embeds a consumer key for the installation at `base_url`, so that
/// [`UsosClient::with_embedded_key`] works. Never exposes the key.
#[uniffi::export]
pub fn has_embedded_key(base_url: String) -> bool {
    usos_core::auth::consumer::embedded(&base_url).is_some()
}

/// The LibreUSOS client for one USOS installation. Thread-safe; create one and share it.
#[derive(uniffi::Object)]
pub struct UsosClient {
    inner: Arc<Inner>,
}

#[uniffi::export]
impl UsosClient {
    /// `base_url`: the installation's API root (for example `https://usosapi.polsl.pl/`).
    /// `consumer_key` / `consumer_secret`: the application's USOS consumer credentials.
    /// `cache_path`: a plain file path for the offline cache (parent directories are created).
    /// `token_store`: the app's persistent, encrypted token storage.
    #[uniffi::constructor]
    pub fn new(
        base_url: String,
        consumer_key: String,
        consumer_secret: String,
        cache_path: String,
        token_store: Arc<dyn TokenStore>,
    ) -> Result<Arc<Self>> {
        if consumer_key.trim().is_empty() || consumer_secret.trim().is_empty() {
            return Err(UsosError::invalid(
                "the consumer key and secret are required",
            ));
        }
        let consumer = ConsumerSource::Fixed(Consumer {
            key: Secret::new(consumer_key.trim()),
            secret: Secret::new(consumer_secret.trim()),
        });
        Self::build(base_url, consumer, cache_path, token_store)
    }

    /// Like [`new`](Self::new), but uses the consumer key embedded in the build for this
    /// installation (see `usos-core`'s `build.rs`). Fails with `InvalidInput` on the first call
    /// that needs the key when the build has none; check [`has_embedded_key`] first.
    #[uniffi::constructor]
    pub fn with_embedded_key(
        base_url: String,
        cache_path: String,
        token_store: Arc<dyn TokenStore>,
    ) -> Result<Arc<Self>> {
        Self::build(base_url, ConsumerSource::Default, cache_path, token_store)
    }
}

impl UsosClient {
    fn build(
        base_url: String,
        consumer: ConsumerSource,
        cache_path: String,
        token_store: Arc<dyn TokenStore>,
    ) -> Result<Arc<Self>> {
        let session = Session::new(
            Arc::new(CoreTokenStore(token_store)),
            None,
            Some(&base_url),
            consumer,
        )?;
        let store = Store::open(&cache_path)?;
        let base_url = session.base_url().to_owned();
        Ok(Arc::new(Self {
            inner: Arc::new(Inner {
                session,
                store,
                base_url,
                http: Mutex::new(None),
                sync_lock: tokio::sync::Mutex::new(()),
            }),
        }))
    }
}

#[uniffi::export]
impl UsosClient {
    /// The normalized installation URL (also the key passed to the `TokenStore`).
    pub fn base_url(&self) -> String {
        self.inner.base_url.clone()
    }

    /// Whether the token store holds an access token (it is not validated against the server).
    /// Only asks the `TokenStore`; no network or TLS setup.
    pub fn has_token(&self) -> Result<bool> {
        Ok(self.inner.load_token()?.is_some())
    }

    /// Continue a login after the app process was killed while the browser was open, from the
    /// state saved with [`PendingLogin::state`].
    pub fn resume_login(&self, state: PendingLoginState) -> Result<Arc<PendingLogin>> {
        if state.request_token.is_empty() || state.request_secret.is_empty() {
            return Err(UsosError::invalid("the pending login state is incomplete"));
        }
        PendingLogin::new(
            &self.inner,
            state.request_token,
            state.request_secret,
            state.scopes,
        )
    }

    /// The cached timetable for `from..=to` (ISO dates), no network. `NotCached` when no day
    /// of the range was ever fetched. This is what widgets should use.
    pub fn cached_timetable(&self, from: String, to: String) -> Result<Timetable> {
        let (from, to) = date_range(&from, &to)?;
        let identity = repo::cached_identity(&self.inner.store, &self.inner.base_url)?;
        Ok(repo::cached_plan(&self.inner.store, &identity.account, from, to)?.into())
    }

    /// The cached terms and the current one for `today` (ISO date), no network. `NotCached`
    /// when terms were never fetched.
    pub fn cached_terms(&self, today: String) -> Result<TermsResult> {
        let today = model::parse_date("today", &today)?;
        let identity = repo::cached_identity(&self.inner.store, &self.inner.base_url)?;
        let result = repo::cached_terms(&self.inner.store, &identity.account)?;
        let current = current_term(&result.terms, today).map(Term::from);
        Ok(TermsResult {
            terms: result.terms.into_iter().map(Term::from).collect(),
            current,
            source: result.source.into(),
        })
    }

    /// The cached grades of those of `term_ids` that were ever fetched, in that order, no
    /// network. `NotCached` when `term_ids` is not empty and none of them was; an empty list
    /// gives an empty result. Each term carries its own `source`.
    pub fn cached_grades(&self, term_ids: Vec<String>) -> Result<Vec<TermGrades>> {
        let identity = repo::cached_identity(&self.inner.store, &self.inner.base_url)?;
        let cached = repo::cached_term_grades(&self.inner.store, &identity.account, &term_ids)?;
        Ok(cached.into_iter().map(TermGrades::from).collect())
    }

    /// The cached course tests of those of `term_ids` that were ever fetched, in that order,
    /// no network. `NotCached` when `term_ids` is not empty and none of them was; an empty list
    /// gives an empty result. Each term carries its own `source`.
    pub fn cached_course_tests(&self, term_ids: Vec<String>) -> Result<Vec<TermTests>> {
        let identity = repo::cached_identity(&self.inner.store, &self.inner.base_url)?;
        let mut out = Vec::new();
        for id in &term_ids {
            match repo::cached_course_tests(&self.inner.store, &identity.account, id) {
                Ok(r) => out.push(TermTests::from_result(id.clone(), r)),
                Err(usos_store::RepoError::NotCached { .. }) => {}
                Err(e) => return Err(e.into()),
            }
        }
        if out.is_empty() && !term_ids.is_empty() {
            return Err(UsosError::NotCached {
                detail: "nothing cached for tests".into(),
            });
        }
        Ok(out)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl UsosClient {
    /// Start a login. Open [`PendingLogin::authorize_url`] in the browser; the browser then
    /// redirects to `callback_url` (for example `libreusos://oauth-callback`), which the app
    /// passes to [`PendingLogin::complete`]. Persist [`PendingLogin::state`] (encrypted) before
    /// opening the browser: the app may be killed meanwhile.
    pub async fn begin_login(&self, callback_url: String) -> Result<Arc<PendingLogin>> {
        let callback = callback_url.trim();
        if callback.is_empty() {
            return Err(UsosError::invalid("the callback URL is empty"));
        }
        let core =
            login::PendingLogin::begin(&self.inner.session, Callback::Custom(callback.to_owned()))
                .await?;
        let request = core.request_token();
        PendingLogin::new(
            &self.inner,
            request.token.expose().to_owned(),
            request.secret.expose().to_owned(),
            core.scopes().to_vec(),
        )
    }

    /// The logged-in user: from USOS, or from the cache when offline.
    pub async fn whoami(&self) -> Result<Identity> {
        let client = self.inner.client()?;
        Ok(repo::identify_or_cached(&client, &self.inner.store)
            .await?
            .into())
    }

    /// The user's terms and the current one for `today` (ISO date): from USOS, or the cache
    /// when offline.
    pub async fn terms(&self, today: String) -> Result<TermsResult> {
        let today = model::parse_date("today", &today)?;
        let client = self.inner.client()?;
        let account = self.inner.account(&client).await?;
        let result = repo::terms_or_cached(&client, &self.inner.store, &account).await?;
        let current = current_term(&result.terms, today).map(Term::from);
        Ok(TermsResult {
            terms: result.terms.into_iter().map(Term::from).collect(),
            current,
            source: result.source.into(),
        })
    }

    /// The grades of `term_ids` with one live call, stored in the cache per term (a term USOS
    /// knows nothing about comes back empty). Falls back to the cache when offline, so check
    /// each term's `source`. Results are in the order of `term_ids`.
    pub async fn grades(&self, term_ids: Vec<String>) -> Result<Vec<TermGrades>> {
        let client = self.inner.client()?;
        let account = self.inner.account(&client).await?;
        let results =
            repo::term_grades_or_cached(&client, &self.inner.store, &account, &term_ids).await?;
        Ok(results.into_iter().map(TermGrades::from).collect())
    }

    /// The course tests of `term_ids`, one term after another (each costs one request for the
    /// list plus the requests for the trees of its tests), stored in the cache per term. A term
    /// is served from the cache when the call for it fails (offline or otherwise), so check each
    /// term's `source`. A term that failed and has no cache is listed in `failed_terms` and left
    /// out of `terms`; only `Unauthorized` ends the call. `terms` keeps the order of `term_ids`.
    pub async fn course_tests(&self, term_ids: Vec<String>) -> Result<CourseTestsResult> {
        let client = self.inner.client()?;
        let account = self.inner.account(&client).await?;
        let mut terms = Vec::with_capacity(term_ids.len());
        let mut failed_terms = Vec::new();
        for id in term_ids {
            let live =
                repo::course_tests_or_cached(&client, &self.inner.store, &account, &id).await;
            match live {
                Ok(r) => terms.push(TermTests::from_result(id, r)),
                Err(e) => {
                    let e = UsosError::from(e);
                    if matches!(e, UsosError::Unauthorized { .. }) {
                        return Err(e);
                    }
                    match repo::cached_course_tests(&self.inner.store, &account, &id) {
                        Ok(r) => terms.push(TermTests::from_result(id, r)),
                        Err(_) => failed_terms.push(id),
                    }
                }
            }
        }
        Ok(CourseTestsResult {
            terms,
            failed_terms,
        })
    }

    /// The timetable for `from..=to` (ISO dates, at most about a year): from USOS, or the
    /// cache when offline.
    pub async fn timetable(&self, from: String, to: String) -> Result<Timetable> {
        let (from, to) = date_range(&from, &to)?;
        let client = self.inner.client()?;
        let account = self.inner.account(&client).await?;
        Ok(
            repo::plan_or_cached(&client, &self.inner.store, &account, from, to)
                .await?
                .into(),
        )
    }

    /// Refresh the cache: user, terms, the timetable from the Monday of `today`'s week for
    /// `weeks` weeks, and the current term's grades and tests. Needs the network.
    ///
    /// Single-flight: a second `sync` started while one is running waits for it and then runs
    /// itself (it does not join or skip it). Cancelling the call cancels the sync.
    pub async fn sync(&self, today: String, weeks: u32) -> Result<SyncReport> {
        let today = model::parse_date("today", &today)?;
        let _single_flight = self.inner.sync_lock.lock().await;
        let client = self.inner.client()?;
        Ok(repo::sync(&client, &self.inner.store, today, weeks)
            .await?
            .into())
    }

    /// Log out: delete the stored token, and then the cached data of the active account
    /// (best effort). With `revoke`, also ask USOS to invalidate the token (best effort).
    /// Fails only if the token could not be deleted (the user is then still logged in).
    /// Returns warnings (no secrets) for the best-effort parts that failed.
    pub async fn logout(&self, revoke: bool) -> Result<Vec<String>> {
        let outcome = self.inner.session.logout(revoke).await?;
        let mut warnings = Vec::new();
        if matches!(outcome, RevokeOutcome::Failed(_)) {
            warnings.push("could not revoke the token on the server".to_owned());
        }
        if let Err(e) = repo::forget_account(&self.inner.store, &self.inner.base_url) {
            warnings.push(format!(
                "could not clear the offline cache: {}",
                UsosError::from(e)
            ));
        }
        Ok(warnings)
    }
}

/// What is needed to finish a login in a new process. SECRETS: it holds the request-token
/// secret. Never log it or call `toString()` on it (Kotlin's generated one prints every
/// field). Persist it only encrypted (Android Keystore), and delete it when the login is done.
/// Persisting it is required: the OS may kill the app while the browser is open.
#[derive(uniffi::Record)]
pub struct PendingLoginState {
    pub request_token: String,
    pub request_secret: String,
    pub scopes: Vec<String>,
}

/// A login waiting for the browser redirect.
#[derive(uniffi::Object)]
pub struct PendingLogin {
    inner: Arc<Inner>,
    authorize_url: String,
    request_token: String,
    request_secret: String,
    scopes: Vec<String>,
    /// Whether the token exchange succeeded. Held across the exchange, so concurrent
    /// `complete` calls queue.
    done: tokio::sync::Mutex<bool>,
}

impl PendingLogin {
    fn new(
        inner: &Arc<Inner>,
        request_token: String,
        request_secret: String,
        scopes: Vec<String>,
    ) -> Result<Arc<Self>> {
        let request = RequestToken {
            token: Secret::new(request_token.as_str()),
            secret: Secret::new(request_secret.as_str()),
        };
        let authorize_url = inner
            .anonymous_client()?
            .authorize_url(&request)
            .to_string();
        Ok(Arc::new(Self {
            inner: inner.clone(),
            authorize_url,
            request_token,
            request_secret,
            scopes,
            done: tokio::sync::Mutex::new(false),
        }))
    }
}

#[uniffi::export]
impl PendingLogin {
    /// The page to open in the browser.
    pub fn authorize_url(&self) -> String {
        self.authorize_url.clone()
    }

    /// The state to persist (encrypted) before opening the browser, see
    /// [`UsosClient::resume_login`].
    pub fn state(&self) -> PendingLoginState {
        PendingLoginState {
            request_token: self.request_token.clone(),
            request_secret: self.request_secret.clone(),
            scopes: self.scopes.clone(),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl PendingLogin {
    /// Finish the login with the full redirect URL the browser was sent to (the custom-scheme
    /// URL with `oauth_token` and `oauth_verifier`). Stores the token via the `TokenStore` and,
    /// best effort, the identity in the cache.
    ///
    /// Nothing is used up before the token exchange succeeds, so after a `Network` error or a
    /// cancelled call the same redirect can be passed again. Errors: an unparseable URL or a
    /// redirect of another login is `InvalidInput`; a denied authorization is `Unauthorized`.
    /// Once the token is saved this never fails: later problems are in `LoggedIn::warnings`.
    pub async fn complete(&self, redirect_url: String) -> Result<LoggedIn> {
        url::Url::parse(redirect_url.trim())
            .map_err(|_| UsosError::invalid("invalid redirect URL"))?;
        let verifier = login::parse_redirect(&redirect_url, &self.request_token)?;
        let mut done = self.done.lock().await;
        if *done {
            return Err(UsosError::invalid("this login was already completed"));
        }
        let request = RequestToken {
            token: Secret::new(self.request_token.as_str()),
            secret: Secret::new(self.request_secret.as_str()),
        };
        let core = login::PendingLogin::resume(&self.inner.session, request, self.scopes.clone())?;
        let result = core.complete(&verifier).await?;
        *done = true;

        let mut warnings = result.warnings;
        let user = match result.user {
            Some(user) => {
                if let Err(e) =
                    repo::remember_identity(&self.inner.store, &self.inner.base_url, user.clone())
                {
                    warnings.push(format!(
                        "could not cache the identity: {}",
                        UsosError::from(e)
                    ));
                }
                Some(User::from(user))
            }
            None => {
                warnings.push("the user could not be determined; call whoami".to_owned());
                None
            }
        };
        Ok(LoggedIn {
            user,
            granted_scopes: result.granted_scopes,
            warnings,
        })
    }
}

/// A completed login: the token is saved.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LoggedIn {
    /// `None` if the identity could not be read (see `warnings`; `whoami` retries).
    pub user: Option<User>,
    pub granted_scopes: Vec<String>,
    /// Non-fatal problems after the token was stored (no secrets), the core's included.
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::tests::MemoryTokenStore;

    fn client(dir: &tempfile::TempDir, tokens: Arc<MemoryTokenStore>) -> Arc<UsosClient> {
        UsosClient::new(
            "usos.example.test".into(),
            "key".into(),
            "secret".into(),
            dir.path().join("cache.sqlite").to_str().unwrap().into(),
            tokens,
        )
        .unwrap()
    }

    #[test]
    fn embedded_key_matches_the_build() {
        // Same answer as core; the key itself is never exposed. Needs a workspace .env or
        // USOS_CONSUMER_* at build time, else skip.
        let polsl = "https://usosapi.polsl.pl/";
        assert_eq!(
            has_embedded_key(polsl.into()),
            usos_core::auth::consumer::embedded(polsl).is_some()
        );
        assert!(!has_embedded_key("https://other.example.test/".into()));
        let dir = tempfile::tempdir().unwrap();
        UsosClient::with_embedded_key(
            polsl.into(),
            dir.path().join("cache.sqlite").to_str().unwrap().into(),
            Arc::new(MemoryTokenStore::default()),
        )
        .unwrap();
    }

    #[test]
    fn constructs_and_reports_token_state() {
        let dir = tempfile::tempdir().unwrap();
        let tokens = Arc::new(MemoryTokenStore::default());
        let c = client(&dir, tokens.clone());
        assert_eq!(c.base_url(), "https://usos.example.test/");
        assert!(!c.has_token().unwrap());
        tokens.tokens.lock().unwrap().insert(
            "https://usos.example.test/".into(),
            ("t".into(), "s".into()),
        );
        assert!(c.has_token().unwrap());
        *tokens.fail.lock().unwrap() = Some(|| UsosError::Unauthorized {
            detail: "keystore says no".into(),
        });
        match c.has_token() {
            Err(UsosError::Unauthorized { detail }) => assert_eq!(detail, "keystore says no"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn rejects_bad_construction_input() {
        let dir = tempfile::tempdir().unwrap();
        let tokens = Arc::new(MemoryTokenStore::default());
        let path = dir.path().join("c.sqlite").to_str().unwrap().to_owned();
        let make = |url: &str, key: &str, path: &str| {
            UsosClient::new(
                url.into(),
                key.into(),
                "s".into(),
                path.into(),
                tokens.clone(),
            )
            .map(|_| ())
        };
        assert!(matches!(
            make("https://u.test/", " ", &path),
            Err(UsosError::InvalidInput { .. })
        ));
        assert!(matches!(
            make("", "k", &path),
            Err(UsosError::InvalidInput { .. })
        ));
        assert!(matches!(
            make("https://u.test/", "k", ":memory:"),
            Err(UsosError::Store { .. })
        ));
    }

    #[test]
    fn cached_timetable_without_account_is_not_cached() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        assert!(matches!(
            c.cached_timetable("2026-11-02".into(), "2026-11-08".into()),
            Err(UsosError::NotCached { .. })
        ));
        assert!(matches!(
            c.cached_timetable("nope".into(), "2026-11-08".into()),
            Err(UsosError::InvalidInput { .. })
        ));
        assert!(matches!(
            c.cached_timetable("2026-11-09".into(), "2026-11-08".into()),
            Err(UsosError::InvalidInput { .. })
        ));
    }

    fn cache_term_grades(c: &UsosClient, term_id: &str, average: Option<f64>) {
        let user = usos_core::User {
            id: "u1".into(),
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
        };
        repo::remember_identity(&c.inner.store, &c.inner.base_url, user).unwrap();
        let account = c
            .inner
            .store
            .active_account(&c.inner.base_url)
            .unwrap()
            .unwrap();
        let grades = usos_core::TermGrades {
            term_id: term_id.into(),
            courses: Vec::new(),
            average,
            weighted_average: None,
        };
        c.inner
            .store
            .put_term_grades(&account, term_id, &grades)
            .unwrap();
    }

    #[test]
    fn cached_grades_follow_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(matches!(
            c.cached_grades(ids(&["T1"])),
            Err(UsosError::NotCached { .. })
        ));
        assert!(matches!(
            c.cached_terms("2026-11-02".into()),
            Err(UsosError::NotCached { .. })
        ));
        assert!(matches!(
            c.cached_terms("nope".into()),
            Err(UsosError::InvalidInput { .. })
        ));
        cache_term_grades(&c, "T1", Some(4.0));
        // Only the cached terms come back, in the requested order.
        let got = c.cached_grades(ids(&["T9", "T1"])).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].term_id, "T1");
        assert_eq!(got[0].average, Some(4.0));
        assert!(matches!(got[0].source, model::DataSource::Cached { .. }));
        assert!(matches!(
            c.cached_grades(ids(&["T9"])),
            Err(UsosError::NotCached { .. })
        ));
    }

    #[test]
    fn grades_without_a_token_are_unauthorized() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        let err = runtime().block_on(c.grades(vec!["T1".into()])).unwrap_err();
        assert!(matches!(err, UsosError::Unauthorized { .. }), "{err:?}");
    }

    fn cache_term_tests(c: &UsosClient, term_id: &str, course_id: &str) {
        let user = usos_core::User {
            id: "u1".into(),
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
        };
        repo::remember_identity(&c.inner.store, &c.inner.base_url, user).unwrap();
        let account = c
            .inner
            .store
            .active_account(&c.inner.base_url)
            .unwrap()
            .unwrap();
        let tests = usos_core::crstests::CourseTests {
            tests: vec![usos_core::TestRoot {
                course_id: course_id.into(),
                course_name: usos_core::LangDict::default(),
                term_id: term_id.into(),
                limited_to_groups: true,
                root: usos_core::TestNode {
                    id: "R1".into(),
                    kind: usos_core::NodeKind::Root,
                    points: Some(7.5),
                    points_max: Some(20.0),
                    ..usos_core::TestNode::default()
                },
            }],
            skipped: vec!["R2".into()],
        };
        c.inner
            .store
            .put_course_tests(&account, term_id, &tests)
            .unwrap();
    }

    #[test]
    fn cached_course_tests_follow_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(matches!(
            c.cached_course_tests(ids(&["T1"])),
            Err(UsosError::NotCached { .. })
        ));
        cache_term_tests(&c, "T1", "C1");
        // Only the cached terms come back, in the requested order.
        let got = c.cached_course_tests(ids(&["T9", "T1"])).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].term_id, "T1");
        assert_eq!(got[0].skipped, ["R2"]);
        assert!(matches!(got[0].source, model::DataSource::Cached { .. }));
        let test = &got[0].tests[0];
        assert_eq!(test.course_id, "C1");
        assert!(test.limited_to_groups);
        assert_eq!(test.root.points, Some(7.5));
        assert_eq!(test.root.points_max, Some(20.0));
        assert!(matches!(
            c.cached_course_tests(ids(&["T9"])),
            Err(UsosError::NotCached { .. })
        ));
        assert!(c.cached_course_tests(Vec::new()).unwrap().is_empty());
    }

    fn with_token(tokens: &MemoryTokenStore) {
        tokens.tokens.lock().unwrap().insert(
            "https://usos.example.test/".into(),
            ("t".into(), "s".into()),
        );
    }

    #[test]
    fn course_tests_without_a_token_are_unauthorized() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        let err = runtime()
            .block_on(c.course_tests(vec!["T1".into()]))
            .unwrap_err();
        assert!(matches!(err, UsosError::Unauthorized { .. }), "{err:?}");
    }

    // The host name does not resolve, so every live call fails like a lost connection.
    #[test]
    fn course_tests_offline_fall_back_to_the_cache_per_term() {
        let dir = tempfile::tempdir().unwrap();
        let tokens = Arc::new(MemoryTokenStore::default());
        let c = client(&dir, tokens.clone());
        with_token(&tokens);
        cache_term_tests(&c, "T1", "C1");
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let got = runtime().block_on(c.course_tests(ids(&["T1"]))).unwrap();
        assert!(got.failed_terms.is_empty());
        assert_eq!(got.terms.len(), 1);
        assert_eq!(got.terms[0].term_id, "T1");
        assert!(matches!(
            got.terms[0].source,
            model::DataSource::Cached { .. }
        ));
        assert_eq!(got.terms[0].tests[0].course_id, "C1");
    }

    #[test]
    fn one_failed_term_does_not_fail_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let tokens = Arc::new(MemoryTokenStore::default());
        let c = client(&dir, tokens.clone());
        with_token(&tokens);
        cache_term_tests(&c, "T2", "C2");
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // T1 and T3 were never cached, T2 was: the result is partial, not an error.
        let got = runtime()
            .block_on(c.course_tests(ids(&["T1", "T2", "T3"])))
            .unwrap();
        assert_eq!(got.failed_terms, ["T1", "T3"]);
        assert_eq!(got.terms.len(), 1);
        assert_eq!(got.terms[0].term_id, "T2");
        // Nothing cached at all: still a result, with every term failed.
        let none = runtime().block_on(c.course_tests(ids(&["T9"]))).unwrap();
        assert!(none.terms.is_empty());
        assert_eq!(none.failed_terms, ["T9"]);
    }

    fn pending(c: &UsosClient) -> Arc<PendingLogin> {
        c.resume_login(PendingLoginState {
            request_token: "rt".into(),
            request_secret: "rs".into(),
            scopes: vec!["studies".into()],
        })
        .unwrap()
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn resume_and_redirect_validation() {
        let dir = tempfile::tempdir().unwrap();
        let c = client(&dir, Arc::new(MemoryTokenStore::default()));
        let pending = pending(&c);
        let state = pending.state();
        assert_eq!(state.request_token, "rt");
        assert_eq!(state.scopes, ["studies"]);
        assert!(pending.authorize_url().contains("oauth_token=rt"));

        // These are rejected before any network access.
        let rt = runtime();
        let complete = |url: &str| rt.block_on(pending.complete(url.into())).unwrap_err();
        match complete("not a url") {
            UsosError::InvalidInput { detail } => assert_eq!(detail, "invalid redirect URL"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            complete("libreusos://oauth-callback?oauth_token=other&oauth_verifier=v"),
            UsosError::InvalidInput { .. }
        ));
        assert!(matches!(
            complete("libreusos://oauth-callback?oauth_token=rt"),
            UsosError::Unauthorized { .. }
        ));
    }

    #[test]
    fn failed_exchange_leaves_the_login_retryable() {
        let dir = tempfile::tempdir().unwrap();
        let c = UsosClient::new(
            // Nothing listens on port 1, so the exchange fails with a network error.
            "http://127.0.0.1:1/".into(),
            "key".into(),
            "secret".into(),
            dir.path().join("cache.sqlite").to_str().unwrap().into(),
            Arc::new(MemoryTokenStore::default()),
        )
        .unwrap();
        let pending = pending(&c);
        let rt = runtime();
        let url = "libreusos://oauth-callback?oauth_token=rt&oauth_verifier=v";
        for _ in 0..2 {
            let err = rt.block_on(pending.complete(url.into())).unwrap_err();
            assert!(matches!(err, UsosError::Network { .. }), "{err:?}");
        }
    }

    #[test]
    fn time_zone_is_the_installations() {
        assert_eq!(installation_time_zone(), "Europe/Warsaw");
    }
}
