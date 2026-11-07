//! The error type seen by Kotlin. Messages never contain secrets or URLs with tokens: the
//! core and the store already scrub theirs, and request URLs are dropped there.

use usos_core::error::LoginError;
use usos_core::{Error, store::StoreError as TokenStoreError};
use usos_store::{RepoError, StoreError};

/// Kotlin: `UsosException` with one subclass per variant. Every variant carries `detail`, a
/// short human-readable (English, developer-facing) text.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UsosError {
    /// The server could not be reached (transport failure or timeout).
    #[error("network error: {detail}")]
    Network { detail: String },
    /// No usable token, the token was rejected (HTTP 401), or the user denied the login.
    #[error("unauthorized: {detail}")]
    Unauthorized { detail: String },
    /// USOS answered with an error other than 401. `detail` is the server's message, or else
    /// the reason or code (never empty).
    #[error("USOS API error (HTTP {status}): {detail}")]
    Api {
        status: u16,
        code: Option<String>,
        reason: Option<String>,
        /// Scopes the call needed but the token lacks (`reason` is `scope_missing`).
        missing_scopes: Vec<String>,
        detail: String,
    },
    /// Offline, and the cache holds nothing for the request.
    #[error("not cached: {detail}")]
    NotCached { detail: String },
    /// The local cache or the app's token store failed.
    #[error("storage error: {detail}")]
    Store { detail: String },
    /// An argument or a redirect URL is malformed.
    #[error("invalid input: {detail}")]
    InvalidInput { detail: String },
    /// Anything else (unexpected server response, internal failure).
    #[error("{detail}")]
    Other { detail: String },
}

impl UsosError {
    pub(crate) fn invalid(detail: impl Into<String>) -> Self {
        Self::InvalidInput {
            detail: detail.into(),
        }
    }
}

/// Carries an error from the app's own `TokenStore` through the core unchanged (inside an
/// `io::Error`, the one `StoreError` variant that can hold an arbitrary error) so that it comes
/// out of the FFI as the same `UsosError` Kotlin threw.
#[derive(Debug)]
pub(crate) struct ForeignFailure(pub UsosError);

impl std::fmt::Display for ForeignFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ForeignFailure {}

/// An exception thrown by the app's own `TokenStore` implementation.
impl From<uniffi::UnexpectedUniFFICallbackError> for UsosError {
    fn from(e: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Store {
            detail: format!("token store callback failed: {}", e.reason),
        }
    }
}

impl From<Error> for UsosError {
    fn from(e: Error) -> Self {
        let detail = e.to_string();
        match e {
            Error::Http(_) => Self::Network { detail },
            Error::Api(api) if api.is_unauthorized() => Self::Unauthorized { detail },
            Error::Api(api) => {
                let missing_scopes = api.missing_scopes();
                let detail = [Some(&api.message), api.reason.as_ref(), api.code.as_ref()]
                    .into_iter()
                    .flatten()
                    .find(|s| !s.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("HTTP {}", api.status));
                Self::Api {
                    status: api.status,
                    code: api.code,
                    reason: api.reason,
                    missing_scopes,
                    detail,
                }
            }
            Error::NotLoggedIn => Self::Unauthorized { detail },
            Error::InvalidBaseUrl(_)
            | Error::InvalidMethod(_)
            | Error::InvalidArgument(_)
            | Error::ConsumerNotConfigured => Self::InvalidInput { detail },
            Error::Store(e) => e.into(),
            Error::Login(LoginError::Denied) => Self::Unauthorized { detail },
            Error::Login(LoginError::TokenMismatch) => Self::InvalidInput { detail },
            Error::Login(_) | Error::UnexpectedResponse(_) => Self::Other { detail },
        }
    }
}

impl From<LoginError> for UsosError {
    fn from(e: LoginError) -> Self {
        Error::Login(e).into()
    }
}

impl From<StoreError> for UsosError {
    fn from(e: StoreError) -> Self {
        Self::Store {
            detail: e.to_string(),
        }
    }
}

impl From<TokenStoreError> for UsosError {
    fn from(e: TokenStoreError) -> Self {
        let detail = e.to_string();
        if let TokenStoreError::Io { source, .. } = e
            && let Some(inner) = source.into_inner()
            && let Ok(foreign) = inner.downcast::<ForeignFailure>()
        {
            return foreign.0;
        }
        Self::Store { detail }
    }
}

impl From<RepoError> for UsosError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Core(e) => e.into(),
            RepoError::Store(e) => e.into(),
            RepoError::NotCached { what } => Self::NotCached {
                detail: format!("nothing cached for {what}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usos_core::ApiError;

    fn api(status: u16, message: &str) -> Error {
        Error::Api(ApiError {
            status,
            code: Some("some_code".into()),
            reason: None,
            message: message.into(),
            extra: Default::default(),
        })
    }

    #[test]
    fn maps_core_errors() {
        assert!(matches!(
            UsosError::from(api(401, "bad")),
            UsosError::Unauthorized { .. }
        ));
        match UsosError::from(api(400, "oops")) {
            UsosError::Api {
                status,
                code,
                detail,
                ..
            } => {
                assert_eq!(status, 400);
                assert_eq!(code.as_deref(), Some("some_code"));
                assert_eq!(detail, "oops");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            UsosError::from(Error::NotLoggedIn),
            UsosError::Unauthorized { .. }
        ));
        assert!(matches!(
            UsosError::from(Error::InvalidBaseUrl("x".into())),
            UsosError::InvalidInput { .. }
        ));
        assert!(matches!(
            UsosError::from(Error::UnexpectedResponse("x".into())),
            UsosError::Other { .. }
        ));
    }

    #[test]
    fn api_error_keeps_reason_scopes_and_never_has_an_empty_detail() {
        let mut e = ApiError {
            status: 400,
            code: Some("method_forbidden".into()),
            reason: Some("scope_missing".into()),
            message: String::new(),
            extra: Default::default(),
        };
        e.extra
            .insert("missing_scopes".into(), serde_json::json!(["grades"]));
        match UsosError::from(Error::Api(e.clone())) {
            UsosError::Api {
                reason,
                missing_scopes,
                detail,
                ..
            } => {
                assert_eq!(reason.as_deref(), Some("scope_missing"));
                assert_eq!(missing_scopes, ["grades"]);
                assert_eq!(detail, "scope_missing");
            }
            other => panic!("unexpected {other:?}"),
        }
        e.reason = None;
        assert!(matches!(
            UsosError::from(Error::Api(e.clone())),
            UsosError::Api { detail, .. } if detail == "method_forbidden"
        ));
        e.code = None;
        assert!(matches!(
            UsosError::from(Error::Api(e)),
            UsosError::Api { detail, .. } if detail == "HTTP 400"
        ));
    }

    #[test]
    fn foreign_errors_pass_through_unchanged() {
        let io = std::io::Error::other(ForeignFailure(UsosError::Unauthorized {
            detail: "keystore says no".into(),
        }));
        let wrapped = Error::Store(TokenStoreError::Io {
            context: "token store".into(),
            source: io,
        });
        match UsosError::from(wrapped) {
            UsosError::Unauthorized { detail } => assert_eq!(detail, "keystore says no"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn maps_login_errors() {
        assert!(matches!(
            UsosError::from(LoginError::Denied),
            UsosError::Unauthorized { .. }
        ));
        assert!(matches!(
            UsosError::from(LoginError::TokenMismatch),
            UsosError::InvalidInput { .. }
        ));
    }

    #[test]
    fn maps_repo_errors() {
        match UsosError::from(RepoError::NotCached {
            what: "plan".into(),
        }) {
            UsosError::NotCached { detail } => assert!(detail.contains("plan")),
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            UsosError::from(RepoError::Store(StoreError::InvalidPath("x".into()))),
            UsosError::Store { .. }
        ));
        assert!(matches!(
            UsosError::from(RepoError::Core(Error::NotLoggedIn)),
            UsosError::Unauthorized { .. }
        ));
    }
}
