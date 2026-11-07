//! A session: stored configuration and token for one installation, ready to build clients.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::auth::consumer::{self, Consumer};
use crate::client::{Client, normalize_base_url};
use crate::error::{Error, Result};
use crate::store::{ConfigStore, KeyringFileStore, TokenStore};

/// The installation used when neither the caller nor the stored config names one.
pub const DEFAULT_BASE_URL: &str = "https://usosapi.polsl.pl/";

/// Where the consumer key and secret come from.
#[derive(Debug, Clone)]
pub enum ConsumerSource {
    /// Default resolution: runtime environment variables (`USOS_CONSUMER_KEY`/`SECRET`, legacy
    /// `CONSUMER_KEY`/`SECRET`), then the key embedded at compile time for this installation
    /// (see `auth::consumer::embedded`).
    Default,
    /// Supplied explicitly (e.g. the user's own key); wins over everything else.
    Fixed(Consumer),
}

/// Everything needed to talk to one installation on behalf of the stored user.
#[derive(Clone)]
pub struct Session {
    tokens: Arc<dyn TokenStore>,
    config: Option<ConfigStore>,
    base_url: String,
    consumer: ConsumerSource,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("base_url", &self.base_url)
            .field("consumer", &self.consumer)
            .field("has_config", &self.config.is_some())
            .finish_non_exhaustive()
    }
}

/// What `logout(revoke = true)` did on the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevokeOutcome {
    NotRequested,
    Revoked,
    /// Revocation was requested but no token was stored.
    NoToken,
    /// The server call failed; the local token is deleted anyway.
    Failed(String),
}

impl Session {
    /// Open the default stores (keyring with a file fallback, plus `config.toml`). The base URL
    /// is `base_url` if given, else the one saved by the last login, else [`DEFAULT_BASE_URL`].
    /// The consumer key uses [`ConsumerSource::Default`].
    pub fn open(base_url: Option<&str>) -> Result<Self> {
        let store = KeyringFileStore::new()?;
        let config = store.config_store();
        Self::new(
            Arc::new(store),
            Some(config),
            base_url,
            ConsumerSource::Default,
        )
    }

    /// A session with explicit storage. `config` is the optional non-secret metadata file
    /// (default base URL, session facts); pass `None` when the platform keeps no such file, and
    /// give `base_url` explicitly then.
    pub fn new(
        tokens: Arc<dyn TokenStore>,
        config: Option<ConfigStore>,
        base_url: Option<&str>,
        consumer: ConsumerSource,
    ) -> Result<Self> {
        let configured = match base_url {
            Some(url) => Some(url.to_owned()),
            None => config
                .as_ref()
                .and_then(|c| c.load_config().ok())
                .and_then(|c| c.default_base_url),
        };
        let raw = configured.as_deref().unwrap_or(DEFAULT_BASE_URL);
        let base_url = normalize_base_url(raw)?.to_string();
        Ok(Self {
            tokens,
            config,
            base_url,
            consumer,
        })
    }

    /// The normalized base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The token store.
    pub fn token_store(&self) -> &Arc<dyn TokenStore> {
        &self.tokens
    }

    /// The metadata file, if this session keeps one.
    pub fn config_store(&self) -> Option<&ConfigStore> {
        self.config.as_ref()
    }

    fn consumer(&self) -> Result<Consumer> {
        match &self.consumer {
            ConsumerSource::Default => consumer::resolve(&self.base_url),
            ConsumerSource::Fixed(c) => Ok(c.clone()),
        }
    }

    /// A consumer-signed client without an access token (for the login flow).
    pub fn anonymous_client(&self) -> Result<Client> {
        let consumer = self.consumer()?;
        Client::new(
            &self.base_url,
            consumer.key.expose(),
            consumer.secret.expose(),
        )
    }

    /// A client carrying the stored access token, if there is one
    /// (check [`Client::is_logged_in`]).
    pub fn client(&self) -> Result<Client> {
        let mut client = self.anonymous_client()?;
        client.set_access_token(self.tokens.load(&self.base_url)?);
        Ok(client)
    }

    /// Delete the local token and session metadata; with `revoke`, first revoke the token
    /// on the server (a failure there is reported, not fatal).
    pub async fn logout(&self, revoke: bool) -> Result<RevokeOutcome> {
        let outcome = if revoke {
            match self.client() {
                Ok(client) if client.is_logged_in() => match client.revoke_token().await {
                    Ok(()) => RevokeOutcome::Revoked,
                    Err(e) => RevokeOutcome::Failed(e.to_string()),
                },
                Ok(_) => RevokeOutcome::NoToken,
                Err(e) => RevokeOutcome::Failed(e.to_string()),
            }
        } else {
            RevokeOutcome::NotRequested
        };
        self.tokens.delete(&self.base_url)?;
        if let Some(config) = &self.config {
            config.forget_session(&self.base_url)?;
        }
        Ok(outcome)
    }

    /// The client of a logged-in user, or [`Error::NotLoggedIn`].
    pub fn logged_in_client(&self) -> Result<Client> {
        let client = self.client()?;
        if client.is_logged_in() {
            Ok(client)
        } else {
            Err(Error::NotLoggedIn)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::secret::Secret;
    use crate::store::{Backend, SaveOutcome, StoreResult, StoredToken};

    /// A host-provided store that keeps tokens in memory.
    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, StoredToken>>);

    impl TokenStore for MemoryStore {
        fn load(&self, base_url: &str) -> StoreResult<Option<StoredToken>> {
            Ok(self.0.lock().unwrap().get(base_url).cloned())
        }
        fn save(&self, base_url: &str, token: &StoredToken) -> StoreResult<SaveOutcome> {
            self.0
                .lock()
                .unwrap()
                .insert(base_url.to_owned(), token.clone());
            Ok(SaveOutcome {
                backend: Backend::Custom,
                keyring_error: None,
                cleanup_warning: None,
            })
        }
        fn delete(&self, base_url: &str) -> StoreResult<()> {
            self.0.lock().unwrap().remove(base_url);
            Ok(())
        }
    }

    fn session(store: Arc<MemoryStore>) -> Session {
        let consumer = ConsumerSource::Fixed(Consumer {
            key: Secret::new("k"),
            secret: Secret::new("s"),
        });
        Session::new(store, None, Some("https://usos.example/"), consumer).unwrap()
    }

    #[tokio::test]
    async fn custom_store_without_config_file() {
        let store = Arc::new(MemoryStore::default());
        let session = session(store.clone());
        assert_eq!(session.base_url(), "https://usos.example/");
        assert!(session.config_store().is_none());
        assert!(!session.client().unwrap().is_logged_in());
        assert!(matches!(
            session.logged_in_client(),
            Err(Error::NotLoggedIn)
        ));

        let token = StoredToken {
            token: Secret::new("t"),
            secret: Secret::new("ts"),
        };
        session
            .token_store()
            .save(session.base_url(), &token)
            .unwrap();
        assert!(session.client().unwrap().is_logged_in());

        let outcome = session.logout(false).await.unwrap();
        assert_eq!(outcome, RevokeOutcome::NotRequested);
        assert!(store.0.lock().unwrap().is_empty());
        assert!(!session.client().unwrap().is_logged_in());
    }
}
