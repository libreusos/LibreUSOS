//! The app's state, a thin layer over `usos-core` and `usos-store`.
//!
//! One [`AppState`] entity (see [`AppState::global`]) holds the sign-in state, the sync state
//! and a [`Snapshot`] of the user's data. Views observe it (`cx.observe(&state, ..)`) and read
//! the public fields. Screen view-models are pure functions of [`Snapshot`], written next to
//! each screen; nothing here knows what a screen shows.
//!
//! All network and disk work runs on a tokio runtime that lives on its own threads
//! ([`runtime`]); results come back to the UI thread through `cx.spawn`.
//!
//! - [`data`]: the snapshot, the sync status, the fetch (`*_or_cached` through `usos-store`).
//! - [`login`]: the browser login task and its channels.

pub mod data;
pub mod login;
pub mod term_data;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use gpui::{App, AppContext, Context, Entity, Global};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use usos_core::Session;
use usos_core::auth::login::CallbackMode;
use usos_store::{Store, repo};

pub use data::{Failure, Outcome, Snapshot, SyncStatus};
pub use login::{LoginCmd, LoginFailure};
pub use term_data::{TermData, TermLoad};

/// How often the data is refreshed while the app is open.
const REFRESH_EVERY: Duration = Duration::from_secs(15 * 60);

/// Who is signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    /// The session is being opened.
    Starting,
    /// No token. `error` says why the session could not be opened, when it could not.
    SignedOut {
        error: Option<String>,
    },
    SignedIn {
        base_url: String,
    },
}

/// The login screen's progress.
#[derive(Debug, Clone)]
pub enum LoginStatus {
    Idle,
    /// Asking the university for a request token.
    Starting {
        base_url: String,
    },
    /// The browser is open; waiting for it to come back (or for a PIN).
    Waiting {
        base_url: String,
        authorize_url: String,
        mode: CallbackMode,
    },
    /// The verifier arrived; fetching the token.
    Completing {
        base_url: String,
    },
    Failed {
        base_url: String,
        failure: LoginFailure,
    },
}

/// The pieces that exist once a session could be opened.
struct Core {
    session: Session,
    store: Arc<Store>,
    base_url: String,
}

pub struct AppState {
    pub auth: Auth,
    pub sync: SyncStatus,
    pub data: Snapshot,
    pub login: LoginStatus,
    /// The university to preselect on the login screen (set by "Zmień uczelnię lub konto").
    pub preselect_url: Option<String>,
    /// Grades and tests of the terms the picker visited (the current term lives in `data`).
    pub term_cache: HashMap<String, TermData>,
    core: Option<Core>,
    runtime: Arc<Runtime>,
    login_cmds: Option<UnboundedSender<LoginCmd>>,
    /// Bumped by every `begin_login`, so a late event of a replaced login is ignored.
    login_gen: u64,
    /// Bumped by every logout and login, so a fetch of an earlier session is discarded.
    session_gen: u64,
}

struct StateHandle(Entity<AppState>);

impl Global for StateHandle {}

impl AppState {
    /// Create the state and start opening the session. Call once at start-up.
    pub fn init(cx: &mut App) -> Entity<AppState> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("usos-io")
                .enable_all()
                .build()
                .expect("the tokio runtime must start"),
        );
        let state = cx.new(|cx| {
            let mut state = AppState {
                auth: Auth::Starting,
                sync: SyncStatus::Idle,
                data: Snapshot::default(),
                login: LoginStatus::Idle,
                preselect_url: None,
                term_cache: HashMap::new(),
                core: None,
                runtime,
                login_cmds: None,
                login_gen: 0,
                session_gen: 0,
            };
            state.start(cx);
            state
        });
        cx.set_global(StateHandle(state.clone()));
        state
    }

    /// The app-wide state entity.
    pub fn global(cx: &App) -> Entity<AppState> {
        cx.global::<StateHandle>().0.clone()
    }

    /// The signed-in installation's base URL.
    pub fn base_url(&self) -> Option<&str> {
        match &self.auth {
            Auth::SignedIn { base_url } => Some(base_url),
            _ => None,
        }
    }

    /// The university's display name (empty when signed out).
    pub fn university(&self) -> String {
        self.base_url()
            .map(login::university_name)
            .unwrap_or_default()
    }

    /// The user's full name, when known.
    pub fn user_name(&self) -> Option<String> {
        self.data
            .user
            .as_ref()
            .map(|u| u.full_name())
            .filter(|name| !name.is_empty())
    }

    /// Open the session and the cache on the I/O threads, show what is cached, then sync.
    fn start(&mut self, cx: &mut Context<Self>) {
        let runtime = self.runtime.clone();
        cx.spawn(async move |this, cx| {
            let opened = runtime
                .spawn_blocking(|| {
                    let session = Session::open(None).map_err(|e| e.to_string())?;
                    let store = open_store().map_err(|e| e.to_string())?;
                    let signed_in = session
                        .client()
                        .map(|c| c.is_logged_in())
                        .map_err(|e| e.to_string())?;
                    let base_url = session.base_url().to_owned();
                    let cached = signed_in
                        .then(|| data::load_cached(&store, &base_url, Local::now().date_naive()));
                    Ok::<_, String>((session, Arc::new(store), base_url, signed_in, cached))
                })
                .await;
            this.update(cx, |this, cx| {
                match opened {
                    Ok(Ok((session, store, base_url, signed_in, cached))) => {
                        this.core = Some(Core {
                            session,
                            store,
                            base_url: base_url.clone(),
                        });
                        if signed_in {
                            this.auth = Auth::SignedIn { base_url };
                            this.data = cached.unwrap_or_default();
                            this.sync(cx);
                        } else {
                            this.auth = Auth::SignedOut { error: None };
                        }
                    }
                    Ok(Err(error)) => this.auth = Auth::SignedOut { error: Some(error) },
                    Err(join) => {
                        this.auth = Auth::SignedOut {
                            error: Some(join.to_string()),
                        }
                    }
                }
                cx.notify();
            })
            .ok();
            // Keep the data fresh while the app is open.
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                if this.update(cx, |this, cx| this.sync(cx)).is_err() {
                    return;
                }
            }
        })
        .detach();
    }

    /// Refresh everything (the sidebar's refresh button, Ctrl R, and the timer). Does nothing
    /// when signed out or already syncing.
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.auth, Auth::SignedIn { .. }) || self.sync == SyncStatus::Syncing {
            return;
        }
        let Some(core) = &self.core else { return };
        let session = core.session.clone();
        let store = core.store.clone();
        let previous = self.data.clone();
        let generation = self.session_gen;
        self.sync = SyncStatus::Syncing;
        cx.notify();
        let runtime = self.runtime.clone();
        cx.spawn(async move |this, cx| {
            let work = runtime.spawn(async move {
                let today = Local::now().date_naive();
                let client = tokio::task::spawn_blocking(move || session.client()).await;
                match client {
                    Ok(Ok(client)) if client.is_logged_in() => {
                        data::fetch_all(client, store, today, previous).await
                    }
                    Ok(Ok(_)) => data::Fetched {
                        snapshot: previous,
                        outcomes: vec![Outcome::Failed(Failure::Unauthorized)],
                    },
                    Ok(Err(e)) => data::Fetched {
                        snapshot: previous,
                        outcomes: vec![Outcome::Failed(Failure::Other(e.to_string()))],
                    },
                    Err(e) => data::Fetched {
                        snapshot: previous,
                        outcomes: vec![Outcome::Failed(Failure::Other(e.to_string()))],
                    },
                }
            });
            let fetched = work.await;
            this.update(cx, |this, cx| {
                // A logout (or another login) while the fetch ran wins.
                if this.session_gen != generation || !matches!(this.auth, Auth::SignedIn { .. }) {
                    return;
                }
                match fetched {
                    Ok(fetched) => {
                        this.sync = data::derive_status(&fetched.outcomes, Local::now());
                        this.data = fetched.snapshot;
                        // Visited terms stay on screen; their next visit fetches them again.
                        for entry in this.term_cache.values_mut() {
                            entry.stale = entry.load != TermLoad::Loading;
                        }
                    }
                    Err(e) => {
                        this.sync = SyncStatus::Error {
                            details: e.to_string(),
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Fetch the grades and tests of `term_id` for the term picker, unless that was already
    /// done or is under way. Does nothing when signed out. A refresh (`sync`) marks the loaded
    /// terms stale: they stay on screen and the next call fetches them again.
    pub fn load_term(&mut self, term_id: &str, cx: &mut Context<Self>) {
        if self.term_cache.get(term_id).is_some_and(|e| !e.stale)
            || !matches!(self.auth, Auth::SignedIn { .. })
        {
            return;
        }
        let (Some(core), Some(account)) = (&self.core, self.data.account.clone()) else {
            return;
        };
        let session = core.session.clone();
        let store = core.store.clone();
        let term_id = term_id.to_owned();
        let generation = self.session_gen;
        match self.term_cache.get_mut(term_id.as_str()) {
            Some(entry) => entry.stale = false,
            None => {
                self.term_cache.insert(term_id.clone(), TermData::loading());
            }
        }
        cx.notify();
        let runtime = self.runtime.clone();
        cx.spawn(async move |this, cx| {
            let key = term_id.clone();
            let work = runtime.spawn(async move {
                let client = tokio::task::spawn_blocking(move || session.client()).await;
                match client {
                    Ok(Ok(client)) => term_data::fetch_term(client, store, account, term_id).await,
                    _ => TermData {
                        load: TermLoad::Failed,
                        ..TermData::loading()
                    },
                }
            });
            let data = work.await.unwrap_or(TermData {
                load: TermLoad::Failed,
                ..TermData::loading()
            });
            this.update(cx, |this, cx| {
                if this.session_gen == generation && matches!(this.auth, Auth::SignedIn { .. }) {
                    // A failed refresh keeps what the earlier load found.
                    let keep_old = data.load == TermLoad::Failed
                        && this
                            .term_cache
                            .get(&key)
                            .is_some_and(|old| old.load == TermLoad::Loaded);
                    if !keep_old {
                        this.term_cache.insert(key, data);
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Delete the local token and cache (with a server-side revoke when possible). With
    /// `preselect`, the login screen opens with the current university chosen.
    pub fn logout(&mut self, preselect: bool, cx: &mut Context<Self>) {
        // While a login opens its session there is no core yet; the token still has to go.
        let core = self
            .core
            .as_ref()
            .map(|c| (c.session.clone(), c.store.clone()));
        let Some(base_url) = self
            .core
            .as_ref()
            .map(|c| c.base_url.clone())
            .or_else(|| self.base_url().map(str::to_owned))
        else {
            return;
        };
        let again = preselect.then(|| base_url.clone());
        let runtime = self.runtime.clone();
        // Signed out at once: the screens must stop showing the data while the cleanup runs.
        // A running login must not sign the user back in.
        self.cancel_login();
        self.login_gen += 1;
        self.session_gen += 1;
        self.auth = Auth::SignedOut { error: None };
        self.sync = SyncStatus::Idle;
        self.data = Snapshot::default();
        self.term_cache.clear();
        self.login = LoginStatus::Idle;
        self.preselect_url = again;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let (session, store) = match core {
                        Some(core) => core,
                        None => {
                            let url = base_url.clone();
                            let opened = tokio::task::spawn_blocking(move || {
                                let session =
                                    login::open_session(&url).map_err(|e| e.to_string())?;
                                let store = open_store().map_err(|e| e.to_string())?;
                                Ok::<_, String>((session, Arc::new(store)))
                            })
                            .await;
                            match opened {
                                Ok(Ok(core)) => core,
                                Ok(Err(e)) => return Some(e),
                                Err(e) => return Some(e.to_string()),
                            }
                        }
                    };
                    let revoked = session.logout(true).await;
                    let forgotten = repo::forget_account(&store, &base_url);
                    revoked
                        .err()
                        .map(|e| e.to_string())
                        .or_else(|| forgotten.err().map(|e| e.to_string()))
                })
                .await;
            let error = match result {
                Ok(error) => error,
                Err(e) => Some(e.to_string()),
            };
            if let Some(error) = error {
                this.update(cx, |this, cx| {
                    if matches!(this.auth, Auth::SignedOut { .. }) {
                        this.auth = Auth::SignedOut { error: Some(error) };
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    /// Start the browser login for the installation at `base_url`.
    pub fn begin_login(&mut self, base_url: String, cx: &mut Context<Self>) {
        self.cancel_login();
        self.login_gen += 1;
        let generation = self.login_gen;
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let (event_tx, mut event_rx) = unbounded_channel();
        self.login_cmds = Some(cmd_tx);
        self.login = LoginStatus::Starting {
            base_url: base_url.clone(),
        };
        cx.notify();
        self.runtime
            .spawn(login::run(base_url.clone(), cmd_rx, event_tx));
        cx.spawn(async move |this, cx| {
            while let Some(event) = event_rx.recv().await {
                let finished = matches!(
                    event,
                    login::LoginEvent::Done(_) | login::LoginEvent::Failed(_)
                );
                let base_url = base_url.clone();
                let alive = this
                    .update(cx, |this, cx| {
                        if this.login_gen == generation {
                            this.on_login_event(base_url, event, cx);
                        } else if matches!(event, login::LoginEvent::Done(_)) {
                            // Cancelled after the token was saved: the copy promises nothing changed.
                            this.runtime.spawn(async move {
                                let session = tokio::task::spawn_blocking(move || {
                                    login::open_session(&base_url)
                                })
                                .await;
                                if let Ok(Ok(session)) = session {
                                    let _ = session.logout(true).await;
                                }
                            });
                        }
                    })
                    .is_ok();
                if finished || !alive {
                    return;
                }
            }
        })
        .detach();
    }

    fn on_login_event(
        &mut self,
        base_url: String,
        event: login::LoginEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            login::LoginEvent::Waiting {
                authorize_url,
                mode,
            } => {
                cx.open_url(&authorize_url);
                self.login = LoginStatus::Waiting {
                    base_url,
                    authorize_url,
                    mode,
                };
            }
            login::LoginEvent::Completing => {
                self.login = LoginStatus::Completing { base_url };
            }
            login::LoginEvent::Done(done) => {
                self.login_cmds = None;
                self.login = LoginStatus::Idle;
                self.preselect_url = None;
                self.term_cache.clear();
                self.session_gen += 1;
                self.sync = SyncStatus::Idle;
                self.data = Snapshot {
                    user: done.user,
                    ..Snapshot::default()
                };
                // The session was opened for the previous installation; open one for this.
                let runtime = self.runtime.clone();
                let url = base_url.clone();
                let generation = self.session_gen;
                let existing_store = self.core.as_ref().map(|c| c.store.clone());
                // The old session belongs to the previous installation: nothing may use it now.
                self.core = None;
                self.auth = Auth::SignedIn {
                    base_url: base_url.clone(),
                };
                cx.spawn(async move |this, cx| {
                    let opened = runtime
                        .spawn_blocking(move || {
                            let session = login::open_session(&url).map_err(|e| e.to_string())?;
                            let store = match existing_store {
                                Some(store) => store,
                                None => Arc::new(open_store().map_err(|e| e.to_string())?),
                            };
                            Ok::<_, String>((session, store, url))
                        })
                        .await;
                    this.update(cx, |this, cx| {
                        // Another login or a logout replaced this one while the session opened.
                        if this.session_gen != generation {
                            return;
                        }
                        match opened {
                            Ok(Ok((session, store, base_url))) => {
                                this.core = Some(Core {
                                    session,
                                    store,
                                    base_url,
                                });
                                this.sync(cx);
                            }
                            Ok(Err(error)) => this.auth = Auth::SignedOut { error: Some(error) },
                            Err(e) => {
                                this.auth = Auth::SignedOut {
                                    error: Some(e.to_string()),
                                }
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            login::LoginEvent::Failed(failure) => {
                self.login_cmds = None;
                self.login = LoginStatus::Failed { base_url, failure };
            }
        }
        cx.notify();
    }

    /// Send a command to the running login (PIN, switch to PIN mode, cancel).
    pub fn login_command(&mut self, cmd: LoginCmd) {
        if let Some(tx) = &self.login_cmds {
            let _ = tx.send(cmd);
        }
    }

    /// Stop a running login. What the user sees is [`LoginFailure::Cancelled`].
    pub fn cancel_login(&mut self) {
        if let Some(tx) = self.login_cmds.take() {
            let _ = tx.send(LoginCmd::Cancel);
        }
    }

    /// Leave a finished or failed login and show the entry form again.
    pub fn reset_login(&mut self, cx: &mut Context<Self>) {
        self.cancel_login();
        // Late events of the cancelled task must not reopen the browser or sign in.
        self.login_gen += 1;
        self.login = LoginStatus::Idle;
        cx.notify();
    }
}

/// `<data dir>/libreusos/cache.sqlite`, the same file the CLI uses.
fn open_store() -> anyhow::Result<Store> {
    use anyhow::Context as _;
    let path = dirs::data_dir()
        .context("cannot find the user data directory")?
        .join("libreusos")
        .join("cache.sqlite");
    let path = path
        .into_os_string()
        .into_string()
        .map_err(|p| anyhow::anyhow!("cache path {p:?} is not valid UTF-8"))?;
    Ok(Store::open(&path)?)
}
