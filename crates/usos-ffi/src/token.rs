//! The app-implemented token store (Android: Keystore-backed) and its adapter to the core.

use std::sync::Arc;

use usos_core::store::{Backend, SaveOutcome, StoreError as CoreStoreError, StoreResult};
use usos_core::{Secret, StoredToken};

use crate::error::{ForeignFailure, UsosError};

/// An OAuth access token and its secret. SECRETS: never log this object or call `toString()`
/// on it (Kotlin's generated one prints both fields), and persist it only encrypted.
#[derive(uniffi::Record)]
pub struct TokenRecord {
    pub token: String,
    pub secret: String,
}

/// Persistent storage for the access token, implemented in Kotlin. `base_url` is the
/// normalized installation URL (the key). Implementations may block briefly; they are called
/// from the library's worker threads. Throwing is allowed and surfaces as a store error.
#[uniffi::export(with_foreign)]
pub trait TokenStore: Send + Sync {
    fn load(&self, base_url: String) -> Result<Option<TokenRecord>, UsosError>;
    fn save(&self, base_url: String, token: TokenRecord) -> Result<(), UsosError>;
    fn delete(&self, base_url: String) -> Result<(), UsosError>;
}

/// Lets the core use the app's store.
pub(crate) struct CoreTokenStore(pub Arc<dyn TokenStore>);

/// The app's error travels through the core as-is and is unwrapped again in
/// `From<StoreError> for UsosError`, so an exception such as `Unauthorized` keeps its type.
fn to_core(e: UsosError) -> CoreStoreError {
    CoreStoreError::Io {
        context: "token store".into(),
        source: std::io::Error::other(ForeignFailure(e)),
    }
}

impl usos_core::TokenStore for CoreTokenStore {
    fn load(&self, base_url: &str) -> StoreResult<Option<StoredToken>> {
        Ok(self
            .0
            .load(base_url.to_owned())
            .map_err(to_core)?
            .map(|r| StoredToken {
                token: Secret::new(r.token),
                secret: Secret::new(r.secret),
            }))
    }

    fn save(&self, base_url: &str, token: &StoredToken) -> StoreResult<SaveOutcome> {
        self.0
            .save(
                base_url.to_owned(),
                TokenRecord {
                    token: token.token.expose().to_owned(),
                    secret: token.secret.expose().to_owned(),
                },
            )
            .map_err(to_core)?;
        Ok(SaveOutcome {
            backend: Backend::Custom,
            keyring_error: None,
            cleanup_warning: None,
        })
    }

    fn delete(&self, base_url: &str) -> StoreResult<()> {
        self.0.delete(base_url.to_owned()).map_err(to_core)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use usos_core::TokenStore as _;

    use super::*;

    /// A Rust implementation of the foreign trait, as a stand-in for the Kotlin one.
    #[derive(Default)]
    pub(crate) struct MemoryTokenStore {
        pub tokens: Mutex<HashMap<String, (String, String)>>,
        pub fail: Mutex<Option<fn() -> UsosError>>,
    }

    impl TokenStore for MemoryTokenStore {
        fn load(&self, base_url: String) -> Result<Option<TokenRecord>, UsosError> {
            if let Some(fail) = *self.fail.lock().unwrap() {
                return Err(fail());
            }
            Ok(self
                .tokens
                .lock()
                .unwrap()
                .get(&base_url)
                .map(|(t, s)| TokenRecord {
                    token: t.clone(),
                    secret: s.clone(),
                }))
        }
        fn save(&self, base_url: String, token: TokenRecord) -> Result<(), UsosError> {
            self.tokens
                .lock()
                .unwrap()
                .insert(base_url, (token.token, token.secret));
            Ok(())
        }
        fn delete(&self, base_url: String) -> Result<(), UsosError> {
            self.tokens.lock().unwrap().remove(&base_url);
            Ok(())
        }
    }

    #[test]
    fn adapter_round_trips_through_the_foreign_store() {
        let inner = Arc::new(MemoryTokenStore::default());
        let store = CoreTokenStore(inner.clone());
        assert!(store.load("https://u/").unwrap().is_none());

        let token = StoredToken {
            token: Secret::new("tok"),
            secret: Secret::new("sec"),
        };
        let outcome = store.save("https://u/", &token).unwrap();
        assert_eq!(outcome.backend, Backend::Custom);
        assert_eq!(store.load("https://u/").unwrap(), Some(token));
        assert_eq!(
            inner.tokens.lock().unwrap()["https://u/"],
            ("tok".to_owned(), "sec".to_owned())
        );

        store.delete("https://u/").unwrap();
        assert!(store.load("https://u/").unwrap().is_none());
    }

    #[test]
    fn foreign_failure_becomes_a_store_error() {
        let inner = Arc::new(MemoryTokenStore::default());
        *inner.fail.lock().unwrap() = Some(|| UsosError::Store {
            detail: "keystore locked".into(),
        });
        let err = CoreTokenStore(inner.clone())
            .load("https://u/")
            .unwrap_err();
        match UsosError::from(err) {
            UsosError::Store { detail } => assert_eq!(detail, "keystore locked"),
            other => panic!("unexpected {other:?}"),
        }
        *inner.fail.lock().unwrap() = Some(|| UsosError::Unauthorized {
            detail: "locked".into(),
        });
        let err = CoreTokenStore(inner).load("https://u/").unwrap_err();
        assert!(matches!(
            UsosError::from(err),
            UsosError::Unauthorized { .. }
        ));
    }
}
