//! Error types of the store and of the orchestration layer.

use chrono::NaiveDate;

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// A failure of the local database. Never contains cached data.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("cannot prepare the cache file: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot encode the data for the cache: {0}")]
    Json(#[from] serde_json::Error),
    /// The file was written by a newer version of the app.
    #[error(
        "the cache has schema version {found}, but this build only understands up to {supported}"
    )]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("invalid cached value: {0}")]
    Corrupt(String),
    #[error("invalid cache path {0:?}: use a plain file path")]
    InvalidPath(String),
    #[error("invalid date range: {from} is after {to}")]
    InvalidRange { from: NaiveDate, to: NaiveDate },
    /// A logout happened while the data was being fetched; it was not stored.
    #[error("the account was logged out while its data was being fetched")]
    LoggedOut,
}

/// A failure of an online-or-cached operation in [`crate::repo`].
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    /// The USOS call failed (and, for `*_or_cached`, so would the cache have, or the error is not
    /// a network failure).
    #[error(transparent)]
    Core(#[from] usos_core::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Cache-only access found nothing for `what` (for example `"plan"`).
    #[error("nothing cached for {what}")]
    NotCached { what: String },
}
