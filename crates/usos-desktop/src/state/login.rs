//! The browser login as a background task (SYSTEM.md section 11, "Sign-in").
//!
//! The task owns the core's `PendingLogin`. The UI talks to it through two channels:
//! [`LoginCmd`] in, [`LoginEvent`] out. The first attempt uses the loopback callback. A PIN
//! cannot be delivered to a loopback login (USOS shows no PIN for it), so "use a PIN" restarts
//! the login in the `oob` mode with a new address to open.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use usos_core::auth::login::{Callback, CallbackMode, LoggedIn, PendingLogin};
use usos_core::error::LoginError;
use usos_core::session::{ConsumerSource, Session};
use usos_core::store::KeyringFileStore;
use usos_core::{Error as CoreError, auth::consumer};

/// The lifetime of a request token, which is also how long the app waits (SYSTEM.md section 11).
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The university shipped with a consumer key.
pub const POLSL_URL: &str = "https://usosapi.polsl.pl/";

/// A university the login screen lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub name: &'static str,
    pub base_url: &'static str,
}

/// The universities with a consumer key in the build. Others go through the URL entry and need
/// the user's own key (see `has_consumer_key`).
pub const PRESETS: [Preset; 1] = [Preset {
    name: "Politechnika Śląska",
    base_url: POLSL_URL,
}];

/// The display name of the installation at `base_url`: the preset's name, else its host.
pub fn university_name(base_url: &str) -> String {
    let normalized = usos_core::normalize_base_url(base_url).ok();
    if let Some(preset) = PRESETS.iter().find(|p| {
        usos_core::normalize_base_url(p.base_url).ok() == normalized && normalized.is_some()
    }) {
        return preset.name.to_owned();
    }
    normalized
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| base_url.to_owned())
}

/// What the UI asks the task to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginCmd {
    /// Restart in PIN mode (when the browser cannot reach the loopback address).
    UsePin,
    /// Complete with the PIN the user read off the USOS page.
    Pin(String),
    Cancel,
}

/// How a login ended without success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginFailure {
    /// The user pressed Anuluj.
    Cancelled,
    /// Nothing came back within [`LOGIN_TIMEOUT`].
    Timeout,
    /// The user refused access, or no verifier was returned.
    Denied,
    /// This installation needs a consumer key the build does not have.
    NeedsKey,
    /// The server could not be reached.
    Network,
    /// Anything else. The text is shown behind "Szczegóły" only.
    Other(String),
}

/// What the task reports.
#[derive(Debug, Clone)]
pub enum LoginEvent {
    /// The page the user has to open, and how the answer will come back.
    Waiting {
        authorize_url: String,
        mode: CallbackMode,
    },
    /// The verifier arrived; exchanging it for a token.
    Completing,
    Done(Box<LoggedIn>),
    Failed(LoginFailure),
}

/// Sort a core error into the states the login screen has.
pub fn failure_of(error: &CoreError) -> LoginFailure {
    match error {
        CoreError::Login(LoginError::Timeout(_)) => LoginFailure::Timeout,
        CoreError::Login(LoginError::Denied) => LoginFailure::Denied,
        CoreError::ConsumerNotConfigured => LoginFailure::NeedsKey,
        CoreError::Http(_) => LoginFailure::Network,
        other => LoginFailure::Other(other.to_string()),
    }
}

/// Whether the build can sign in to `base_url` without the user bringing their own key.
pub fn has_consumer_key(base_url: &str) -> bool {
    consumer::resolve(base_url).is_ok()
}

/// A session over the default stores for `base_url`. Blocking (it may talk to the keyring).
pub fn open_session(base_url: &str) -> Result<Session, CoreError> {
    let store = KeyringFileStore::new()?;
    let config = store.config_store();
    Session::new(
        Arc::new(store),
        Some(config),
        Some(base_url),
        ConsumerSource::Default,
    )
}

/// Run one login to its end and report the result on `events`.
pub async fn run(
    base_url: String,
    mut cmds: UnboundedReceiver<LoginCmd>,
    events: UnboundedSender<LoginEvent>,
) {
    let outcome = flow(&base_url, &mut cmds, &events).await;
    let _ = events.send(match outcome {
        Ok(done) => LoginEvent::Done(Box::new(done)),
        Err(failure) => LoginEvent::Failed(failure),
    });
}

async fn flow(
    base_url: &str,
    cmds: &mut UnboundedReceiver<LoginCmd>,
    events: &UnboundedSender<LoginEvent>,
) -> Result<LoggedIn, LoginFailure> {
    if !has_consumer_key(base_url) {
        return Err(LoginFailure::NeedsKey);
    }
    let url = base_url.to_owned();
    let session = tokio::task::spawn_blocking(move || open_session(&url))
        .await
        .map_err(|e| LoginFailure::Other(e.to_string()))?
        .map_err(|e| failure_of(&e))?;

    let mut callback = Callback::Loopback;
    loop {
        let mut pending = PendingLogin::begin(&session, callback.clone())
            .await
            .map_err(|e| failure_of(&e))?;
        let mode = pending.callback_mode();
        let _ = events.send(LoginEvent::Waiting {
            authorize_url: pending.authorize_url().to_string(),
            mode,
        });

        if mode == CallbackMode::Loopback {
            let verifier = tokio::select! {
                verifier = pending.wait_loopback(LOGIN_TIMEOUT) => {
                    verifier.map_err(|e| failure_of(&e))?
                }
                cmd = cmds.recv() => match cmd {
                    Some(LoginCmd::UsePin | LoginCmd::Pin(_)) => {
                        callback = Callback::Oob;
                        continue;
                    }
                    Some(LoginCmd::Cancel) | None => return Err(LoginFailure::Cancelled),
                },
            };
            let _ = events.send(LoginEvent::Completing);
            return pending
                .complete(&verifier)
                .await
                .map_err(|e| failure_of(&e));
        }

        // PIN mode: wait for the code.
        loop {
            match tokio::time::timeout(LOGIN_TIMEOUT, cmds.recv()).await {
                Err(_) => return Err(LoginFailure::Timeout),
                Ok(Some(LoginCmd::Cancel) | None) => return Err(LoginFailure::Cancelled),
                Ok(Some(LoginCmd::UsePin)) => {}
                Ok(Some(LoginCmd::Pin(pin))) => {
                    let pin = pin.trim().to_owned();
                    if pin.is_empty() {
                        continue;
                    }
                    let _ = events.send(LoginEvent::Completing);
                    return pending.complete(&pin).await.map_err(|e| failure_of(&e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn university_names() {
        assert_eq!(
            university_name("https://usosapi.polsl.pl"),
            "Politechnika Śląska"
        );
        assert_eq!(
            university_name("https://usosapi.polsl.pl/"),
            "Politechnika Śląska"
        );
        assert_eq!(
            university_name("https://usosapps.uw.edu.pl/"),
            "usosapps.uw.edu.pl"
        );
        assert_eq!(university_name("not a url"), "not a url");
    }

    #[test]
    fn core_errors_map_to_login_states() {
        assert_eq!(
            failure_of(&CoreError::Login(LoginError::Timeout(LOGIN_TIMEOUT))),
            LoginFailure::Timeout
        );
        assert_eq!(
            failure_of(&CoreError::Login(LoginError::Denied)),
            LoginFailure::Denied
        );
        assert_eq!(
            failure_of(&CoreError::ConsumerNotConfigured),
            LoginFailure::NeedsKey
        );
        assert!(matches!(
            failure_of(&CoreError::NotLoggedIn),
            LoginFailure::Other(_)
        ));
    }
}
