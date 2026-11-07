//! Error types. Nothing in here ever contains a secret or a token.

use std::fmt;

use serde_json::{Map, Value};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("invalid API method name: {0:?}")]
    InvalidMethod(String),
    #[error("network error: {0}")]
    Http(reqwest::Error),
    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("unexpected response from the server: {0}")]
    UnexpectedResponse(String),
    #[error("not logged in (no access token available)")]
    NotLoggedIn,
    #[error(transparent)]
    Store(#[from] crate::store::StoreError),
    #[error(
        "no consumer key/secret for this installation: the build has none embedded (set \
         USOS_CONSUMER_KEY, USOS_CONSUMER_SECRET and USOS_KEY_BASE_URL when building, or put \
         CONSUMER_KEY and CONSUMER_SECRET in the workspace .env), or set USOS_CONSUMER_KEY and \
         USOS_CONSUMER_SECRET at runtime"
    )]
    ConsumerNotConfigured,
    /// The browser part of the login failed.
    #[error(transparent)]
    Login(#[from] LoginError),
}

/// Why the browser part of the login failed.
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("binding a loopback port: {0}")]
    Bind(std::io::Error),
    #[error("callback server failed: {0}")]
    CallbackServer(std::io::Error),
    #[error("callback server stopped unexpectedly")]
    CallbackServerStopped,
    #[error("authorization was denied or no verifier was returned")]
    Denied,
    #[error("timed out after {} waiting for the browser callback", humantime::format_duration(*.0))]
    Timeout(std::time::Duration),
    #[error("the redirect carries a different request token than the pending login")]
    TokenMismatch,
    #[error("not waiting on a loopback callback")]
    NotLoopback,
}

impl From<reqwest::Error> for Error {
    /// Drops the URL, which can carry request parameters.
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.without_url())
    }
}

/// An error reported by the USOS API (HTTP 400, 401 or 500 with a JSON body,
/// or a plain-text OAuth failure).
#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    /// The `error` code, e.g. `method_forbidden`, `param_missing`.
    pub code: Option<String>,
    /// The `reason` for `*_forbidden` errors, e.g. `scope_missing`.
    pub reason: Option<String>,
    /// The developer-facing `message`, or a truncated plain-text body.
    pub message: String,
    /// All remaining keys of the JSON body (`param_name`, `missing_scopes`, ...).
    pub extra: Map<String, Value>,
}

impl ApiError {
    /// Maximum number of characters kept from a non-JSON error body.
    const MAX_PLAIN_BODY: usize = 300;

    /// Parse an error response body. `scrub` removes secrets from raw text first.
    pub fn from_response(status: u16, body: &str, scrub: impl Fn(&str) -> String) -> Self {
        let body = scrub(body);
        match serde_json::from_str::<Value>(&body) {
            Ok(Value::Object(mut map)) => {
                let take_string = |map: &mut Map<String, Value>, key: &str| match map.remove(key) {
                    Some(Value::String(s)) => Some(s),
                    Some(other) => {
                        // Keep unexpected shapes visible instead of dropping them.
                        map.insert(key.to_owned(), other);
                        None
                    }
                    None => None,
                };
                let message = take_string(&mut map, "message").unwrap_or_default();
                let code = take_string(&mut map, "error");
                let reason = take_string(&mut map, "reason");
                Self {
                    status,
                    code,
                    reason,
                    message,
                    extra: map,
                }
            }
            _ => {
                let message: String = body.trim().chars().take(Self::MAX_PLAIN_BODY).collect();
                Self {
                    status,
                    code: None,
                    reason: None,
                    message,
                    extra: Map::new(),
                }
            }
        }
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == 401
    }

    pub fn is_scope_missing(&self) -> bool {
        self.reason.as_deref() == Some("scope_missing")
    }

    /// Scopes the call needed but the token lacks (for `reason = scope_missing`).
    pub fn missing_scopes(&self) -> Vec<String> {
        match self.extra.get("missing_scopes") {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "USOS API error (HTTP {}", self.status)?;
        if let Some(code) = &self.code {
            write!(f, ", {code}")?;
        }
        if let Some(reason) = &self.reason {
            write!(f, ", {reason}")?;
        }
        write!(f, ")")?;
        if !self.message.is_empty() {
            write!(f, ": {}", self.message)?;
        }
        let missing = self.missing_scopes();
        if !missing.is_empty() {
            write!(f, " [missing scopes: {}]", missing.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_error() {
        let body = r#"{"message":"Access to method denied.","error":"method_forbidden","reason":"scope_missing","missing_scopes":["grades"]}"#;
        let e = ApiError::from_response(400, body, str::to_owned);
        assert_eq!(e.status, 400);
        assert_eq!(e.code.as_deref(), Some("method_forbidden"));
        assert!(e.is_scope_missing());
        assert_eq!(e.missing_scopes(), vec!["grades"]);
        assert_eq!(
            e.to_string(),
            "USOS API error (HTTP 400, method_forbidden, scope_missing): Access to method denied. [missing scopes: grades]"
        );
    }

    #[test]
    fn parses_plain_text_and_scrubs() {
        let e = ApiError::from_response(401, "bad signature for SECRET123", |s| {
            s.replace("SECRET123", "***")
        });
        assert!(e.is_unauthorized());
        assert_eq!(e.message, "bad signature for ***");
    }

    #[test]
    fn keeps_extra_keys() {
        let body = r#"{"message":"m","error":"param_missing","param_name":"fields"}"#;
        let e = ApiError::from_response(400, body, str::to_owned);
        assert_eq!(
            e.extra.get("param_name").and_then(Value::as_str),
            Some("fields")
        );
    }
}
