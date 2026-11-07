//! A UI-agnostic OAuth 1.0a login flow.
//!
//! 1. [`PendingLogin::begin`] asks the server for a request token and returns a [`PendingLogin`].
//! 2. The front end shows [`PendingLogin::authorize_url`] to the user and obtains the verifier:
//!    from [`PendingLogin::wait_loopback`] (feature `loopback`), a PIN the user types
//!    (`Callback::Oob`), or a custom-URL-scheme redirect the app handles itself
//!    (`Callback::Custom`).
//! 3. [`PendingLogin::complete`] exchanges the verifier, stores the token and metadata and
//!    checks the identity.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::scopes::{ScopeSelection, select_scopes};
use crate::client::{Client, RequestToken};
#[cfg(feature = "loopback")]
use crate::error::{ApiError, Error};
use crate::error::{LoginError, Result};
use crate::session::Session;
use crate::store::{Backend, ConfigStore, SessionMeta, TokenStore};
use crate::users::User;

/// How the browser hands the verifier back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    /// A one-shot `http://127.0.0.1:<port>/callback` server; falls back to
    /// [`CallbackMode::Oob`] if the server rejects the callback URL.
    #[cfg(feature = "loopback")]
    Loopback,
    /// The user copies a PIN from the authorize page.
    Oob,
    /// The app's own redirect URL (e.g. a custom scheme); the app passes the verifier in.
    Custom(String),
}

/// The callback mode actually in use (after any fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackMode {
    Loopback,
    Oob,
    Custom,
}

impl CallbackMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CallbackMode::Loopback => "loopback",
            CallbackMode::Oob => "oob",
            CallbackMode::Custom => "custom",
        }
    }
}

impl fmt::Display for CallbackMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What starting a login produced, before it becomes a [`PendingLogin`].
struct Started {
    token: RequestToken,
    mode: CallbackMode,
    fallback_reason: Option<String>,
    #[cfg(feature = "loopback")]
    listener: Option<tokio::net::TcpListener>,
}

impl Started {
    fn new(token: RequestToken, mode: CallbackMode) -> Self {
        Self {
            token,
            mode,
            fallback_reason: None,
            #[cfg(feature = "loopback")]
            listener: None,
        }
    }
}

/// Bind a loopback port and ask for a callback there; fall back to `oob` if the server
/// refuses the callback URL.
#[cfg(feature = "loopback")]
async fn start_loopback(client: &Client, scopes: &[String]) -> Result<Started> {
    let bound = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(LoginError::Bind)?;
    let port = bound.local_addr().map_err(LoginError::Bind)?.port();
    let url = format!("http://127.0.0.1:{port}/callback");
    match client.request_token(&url, scopes).await {
        Ok(token) => Ok(Started {
            listener: Some(bound),
            ..Started::new(token, CallbackMode::Loopback)
        }),
        Err(Error::Api(e)) if rejects_callback(&e) => {
            let token = client.request_token("oob", scopes).await?;
            Ok(Started {
                fallback_reason: Some(e.to_string()),
                ..Started::new(token, CallbackMode::Oob)
            })
        }
        Err(e) => Err(e),
    }
}

/// A login waiting for the user's approval.
pub struct PendingLogin {
    client: Client,
    tokens: Arc<dyn TokenStore>,
    config: Option<ConfigStore>,
    base_url: String,
    request_token: RequestToken,
    selection: ScopeSelection,
    mode: CallbackMode,
    fallback_reason: Option<String>,
    #[cfg(feature = "loopback")]
    listener: Option<tokio::net::TcpListener>,
    /// The browser connection that delivered the verifier, answered in `complete`.
    #[cfg(feature = "loopback")]
    browser: Option<crate::auth::loopback::Pending>,
}

impl PendingLogin {
    /// Pick the scopes the installation defines, then fetch a request token for `callback`.
    pub async fn begin(session: &Session, callback: Callback) -> Result<PendingLogin> {
        let client = session.anonymous_client()?;
        let selection = select_scopes(&client).await;
        let scope_refs = selection.scopes.clone();
        let started = match callback {
            #[cfg(feature = "loopback")]
            Callback::Loopback => start_loopback(&client, &scope_refs).await?,
            Callback::Custom(url) => Started::new(
                client.request_token(&url, &scope_refs).await?,
                CallbackMode::Custom,
            ),
            Callback::Oob => Started::new(
                client.request_token("oob", &scope_refs).await?,
                CallbackMode::Oob,
            ),
        };
        Ok(PendingLogin {
            client,
            tokens: session.token_store().clone(),
            config: session.config_store().cloned(),
            base_url: session.base_url().to_owned(),
            request_token: started.token,
            selection,
            mode: started.mode,
            fallback_reason: started.fallback_reason,
            #[cfg(feature = "loopback")]
            listener: started.listener,
            #[cfg(feature = "loopback")]
            browser: None,
        })
    }

    /// Continue a login begun with [`Callback::Custom`] in an earlier process (the OS may
    /// kill the app while the browser is open). Persist [`PendingLogin::request_token`] and
    /// [`PendingLogin::scopes`] before opening the browser, then pass them back here.
    pub fn resume(
        session: &Session,
        request_token: RequestToken,
        scopes: Vec<String>,
    ) -> Result<PendingLogin> {
        Ok(PendingLogin {
            client: session.anonymous_client()?,
            tokens: session.token_store().clone(),
            config: session.config_store().cloned(),
            base_url: session.base_url().to_owned(),
            request_token,
            selection: ScopeSelection {
                scopes,
                dropped: Vec::new(),
                list_error: None,
            },
            mode: CallbackMode::Custom,
            fallback_reason: None,
            #[cfg(feature = "loopback")]
            listener: None,
            #[cfg(feature = "loopback")]
            browser: None,
        })
    }

    /// The request token, to persist for [`PendingLogin::resume`]. Its secret is a
    /// [`crate::secret::Secret`]; store it like a credential.
    pub fn request_token(&self) -> &RequestToken {
        &self.request_token
    }

    /// The page the user must open to approve access.
    pub fn authorize_url(&self) -> Url {
        self.client.authorize_url(&self.request_token)
    }

    pub fn callback_mode(&self) -> CallbackMode {
        self.mode
    }

    /// The scopes requested.
    pub fn scopes(&self) -> &[String] {
        &self.selection.scopes
    }

    /// Requested scopes the installation does not define (not requested).
    pub fn dropped_scopes(&self) -> &[String] {
        &self.selection.dropped
    }

    /// Why the installation's scope list was unavailable (all scopes were requested then).
    pub fn scope_list_error(&self) -> Option<&str> {
        self.selection.list_error.as_deref()
    }

    /// Why a loopback callback was abandoned for the PIN flow, if it was.
    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    /// Wait up to `timeout` for the browser to deliver the verifier to the loopback server.
    /// Only valid in [`CallbackMode::Loopback`].
    #[cfg(feature = "loopback")]
    pub async fn wait_loopback(&mut self, timeout: std::time::Duration) -> Result<String> {
        let listener = self.listener.take().ok_or(LoginError::NotLoopback)?;
        let pending = crate::auth::loopback::wait_for_verifier(
            listener,
            self.request_token.token.expose(),
            timeout,
        )
        .await?;
        let verifier = pending.verifier().to_owned();
        self.browser = Some(pending);
        Ok(verifier)
    }

    /// Exchange the verifier for an access token, store it and verify the identity.
    ///
    /// Failing to read the user or the granted scopes, or to save the metadata, is not an
    /// error: the login succeeded, and the problem is listed in [`LoggedIn::warnings`].
    pub async fn complete(mut self, verifier: &str) -> Result<LoggedIn> {
        let access = self
            .client
            .access_token(&self.request_token, verifier)
            .await;
        #[cfg(feature = "loopback")]
        if let Some(browser) = self.browser.take() {
            browser.finish(access.is_ok()).await;
        }
        let access = access?;
        let saved = self.tokens.save(&self.base_url, &access)?;
        self.client.set_access_token(Some(access));

        let mut warnings: Vec<String> = saved.cleanup_warning.iter().cloned().collect();
        let user = match self.client.current_user().await {
            Ok(user) => Some(user),
            Err(e) => {
                warnings.push(format!("logged in, but fetching the user failed: {e}"));
                None
            }
        };
        let granted_scopes = match self.client.token_scopes().await {
            Ok(scopes) => scopes,
            Err(e) => {
                warnings.push(format!("could not read the granted scopes ({e})"));
                Vec::new()
            }
        };
        let meta = SessionMeta {
            user_id: user.as_ref().map(|u| u.id.clone()),
            scopes: granted_scopes.clone(),
            backend: Some(saved.backend),
            saved_at_unix: Some(crate::oauth::unix_timestamp()),
        };
        if let Some(config) = &self.config {
            if let Err(e) = config.record_session(&self.base_url, meta) {
                warnings.push(format!("could not save session metadata: {e}"));
            }
            if let Err(e) = config.set_default_base_url(&self.base_url) {
                warnings.push(format!("could not save the default base URL: {e}"));
            }
        }
        Ok(LoggedIn {
            user,
            granted_scopes,
            storage_backend: saved.backend,
            keyring_error: saved.keyring_error,
            warnings,
        })
    }
}

/// The result of a completed login.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggedIn {
    /// The verified identity; `None` if reading it failed (see `warnings`).
    pub user: Option<User>,
    /// The scopes the server says the token has.
    pub granted_scopes: Vec<String>,
    pub storage_backend: Backend,
    /// Why the system keyring was not used, when the file fallback was taken.
    pub keyring_error: Option<String>,
    /// Non-fatal problems after the token was stored.
    pub warnings: Vec<String>,
}

/// Read the verifier from the redirect URL the browser was sent to (a custom-scheme redirect),
/// checking that it belongs to the pending login's request token.
pub fn parse_redirect(url: &str, expected_token: &str) -> Result<String, LoginError> {
    let url = Url::parse(url).map_err(|_| LoginError::TokenMismatch)?;
    let get = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    if get("oauth_token").as_deref() != Some(expected_token) {
        return Err(LoginError::TokenMismatch);
    }
    match get("oauth_verifier") {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(LoginError::Denied),
    }
}

/// Whether the server refused the `oauth_callback` itself (not, say, the consumer key).
#[cfg(feature = "loopback")]
fn rejects_callback(e: &ApiError) -> bool {
    e.status == 400
        && matches!(e.code.as_deref(), Some("param_invalid" | "param_forbidden"))
        && (e.message.contains("oauth_callback")
            || e.extra.get("param_name").and_then(|v| v.as_str()) == Some("oauth_callback"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "loopback")]
    fn api(status: u16, body: &str) -> ApiError {
        ApiError::from_response(status, body, |s| s.to_owned())
    }

    #[cfg(feature = "loopback")]
    #[test]
    fn detects_rejected_callback_only() {
        let by_name = api(
            400,
            r#"{"error":"param_invalid","param_name":"oauth_callback"}"#,
        );
        assert!(rejects_callback(&by_name));
        let by_message = api(
            400,
            r#"{"error":"param_forbidden","message":"bad oauth_callback value"}"#,
        );
        assert!(rejects_callback(&by_message));
        let other_param = api(400, r#"{"error":"param_invalid","param_name":"scopes"}"#);
        assert!(!rejects_callback(&other_param));
        assert!(!rejects_callback(&api(401, "Invalid consumer key")));
    }

    #[test]
    fn logged_in_roundtrips_through_json() {
        let done = LoggedIn {
            user: Some(User {
                id: "1".into(),
                first_name: "Jan".into(),
                last_name: "Kowalski".into(),
            }),
            granted_scopes: vec!["studies".into(), "grades".into()],
            storage_backend: Backend::Custom,
            keyring_error: Some("no keyring".into()),
            warnings: vec!["w".into()],
        };
        let json = serde_json::to_string(&done).unwrap();
        assert!(json.contains("\"storage_backend\":\"custom\""));
        assert_eq!(serde_json::from_str::<LoggedIn>(&json).unwrap(), done);
    }

    #[test]
    fn parses_redirect() {
        let ok = "libreusos://cb?oauth_token=abc&oauth_verifier=123";
        assert_eq!(parse_redirect(ok, "abc").unwrap(), "123");
        assert!(matches!(
            parse_redirect(ok, "other"),
            Err(LoginError::TokenMismatch)
        ));
        assert!(matches!(
            parse_redirect("libreusos://cb?oauth_token=abc", "abc"),
            Err(LoginError::Denied)
        ));
        assert!(matches!(
            parse_redirect("libreusos://cb?oauth_token=abc&oauth_verifier=", "abc"),
            Err(LoginError::Denied)
        ));
        assert!(matches!(
            parse_redirect("not a url", "abc"),
            Err(LoginError::TokenMismatch)
        ));
    }

    #[test]
    fn callback_mode_names() {
        assert_eq!(CallbackMode::Loopback.to_string(), "loopback");
        assert_eq!(CallbackMode::Oob.to_string(), "oob");
        assert_eq!(CallbackMode::Custom.to_string(), "custom");
    }
}
