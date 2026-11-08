//! Access-token storage and non-secret session metadata.
//!
//! Tokens are kept behind the [`TokenStore`] trait so a platform can plug in its own secure
//! storage (e.g. the Android Keystore). The default implementation, [`KeyringFileStore`], uses
//! the system keyring (Secret Service on Linux) when it works, otherwise
//! `~/.config/libreusos/credentials.json` (mode 0600, directory 0700).
//!
//! Metadata (base URL, user id, granted scopes, time) is separate and optional: [`ConfigStore`]
//! writes `~/.config/libreusos/config.toml`; it never contains tokens. A session built without a
//! [`ConfigStore`] simply skips it.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::client::AccessToken;

/// A stored access token: the token and its secret, both [`crate::secret::Secret`]s
/// (redacted `Debug`).
pub type StoredToken = AccessToken;

#[cfg(target_os = "linux")]
const KEYRING_SERVICE: &str = "libreusos";

/// Where a token is (or was just) stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Keyring,
    File,
    /// A [`TokenStore`] supplied by the host application.
    Custom,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Backend::Keyring => "system keyring (Secret Service)",
            Backend::File => "credentials file (mode 0600)",
            Backend::Custom => "application-provided token store",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not determine the user config directory")]
    NoConfigDir,
    #[error("{context}: {source}")]
    Io { context: String, source: io::Error },
    #[error("{context}: {message}")]
    Format { context: String, message: String },
    /// A failure reported by a host-provided [`TokenStore`] (never put a secret in it).
    #[error("token store: {0}")]
    Other(String),
    /// The system keyring failed in a way that would leave a token behind.
    #[error("system keyring: {0}")]
    Keyring(String),
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;
type Result<T> = StoreResult<T>;

fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> StoreError {
    let context = context.into();
    move |source| StoreError::Io { context, source }
}

/// Result of [`TokenStore::save`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveOutcome {
    pub backend: Backend,
    /// Why the keyring was not used, when the file fallback was taken.
    pub keyring_error: Option<String>,
    /// A non-fatal problem while cleaning up the other backend.
    pub cleanup_warning: Option<String>,
}

/// Non-secret facts about a stored session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub user_id: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub backend: Option<Backend>,
    /// Seconds since the Unix epoch when the token was saved.
    pub saved_at_unix: Option<u64>,
}

/// Contents of `config.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub default_base_url: Option<String>,
    #[serde(default)]
    pub sessions: BTreeMap<String, SessionMeta>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialsFile {
    #[serde(default)]
    tokens: BTreeMap<String, AccessToken>,
}

/// Where access tokens live. Implementations must be safe to share between threads; calls are
/// blocking and short. Errors must not contain the token.
pub trait TokenStore: Send + Sync {
    /// The token stored for `base_url` (a normalized base URL), if any.
    fn load(&self, base_url: &str) -> StoreResult<Option<StoredToken>>;
    /// Store (or replace) the token for `base_url`.
    fn save(&self, base_url: &str, token: &StoredToken) -> StoreResult<SaveOutcome>;
    /// Delete the token for `base_url`. A missing token is not an error.
    fn delete(&self, base_url: &str) -> StoreResult<()>;
}

fn ensure_dir(dir: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
        .create(dir)
        .map_err(io_err(format!("creating {}", dir.display())))?;
    // `create_dir_all` leaves a pre-existing directory's mode alone.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(io_err(format!("setting permissions on {}", dir.display())))?;
    }
    Ok(())
}

fn default_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or(StoreError::NoConfigDir)?
        .join("libreusos"))
}

/// The keyring calls [`KeyringFileStore`] makes; a trait so tests can use a fake keyring.
trait Keyring: fmt::Debug + Send + Sync {
    fn save(&self, base_url: &str, token: &AccessToken) -> std::result::Result<(), String>;
    /// `None` when there is no entry or it cannot be read.
    fn load(&self, base_url: &str) -> Option<AccessToken>;
    /// Delete the entry. A missing entry, or no keyring service in this session, is success.
    fn delete(&self, base_url: &str) -> std::result::Result<(), String>;
}

#[derive(Debug)]
struct SystemKeyring;

impl Keyring for SystemKeyring {
    fn save(&self, base_url: &str, token: &AccessToken) -> std::result::Result<(), String> {
        keyring_save(base_url, token)
    }
    fn load(&self, base_url: &str) -> Option<AccessToken> {
        keyring_load(base_url)
    }
    fn delete(&self, base_url: &str) -> std::result::Result<(), String> {
        keyring_delete(base_url)
    }
}

/// The default [`TokenStore`]: the system keyring, with a 0600 credentials file as fallback.
#[derive(Debug, Clone)]
pub struct KeyringFileStore {
    dir: PathBuf,
    keyring: Option<Arc<dyn Keyring>>,
}

impl KeyringFileStore {
    /// The default store: `<config dir>/libreusos`, keyring enabled.
    pub fn new() -> Result<Self> {
        Ok(Self::with_dir(default_dir()?, true))
    }

    /// A store in `dir`; `use_keyring = false` forces the file backend (used by tests).
    pub fn with_dir(dir: impl Into<PathBuf>, use_keyring: bool) -> Self {
        Self {
            dir: dir.into(),
            keyring: use_keyring.then(|| Arc::new(SystemKeyring) as Arc<dyn Keyring>),
        }
    }

    #[cfg(test)]
    fn with_keyring(dir: impl Into<PathBuf>, keyring: Arc<dyn Keyring>) -> Self {
        Self {
            dir: dir.into(),
            keyring: Some(keyring),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The metadata store that lives next to this one.
    pub fn config_store(&self) -> ConfigStore {
        ConfigStore::with_dir(self.dir.clone())
    }

    fn credentials_path(&self) -> PathBuf {
        self.dir.join("credentials.json")
    }

    // ---- tokens ----------------------------------------------------------

    /// Store `token` for `base_url`. Tries the keyring first, then the file. Whichever backend
    /// gets the token, the other one must not keep an older token that `load` could return.
    fn save_token(&self, base_url: &str, token: &AccessToken) -> Result<SaveOutcome> {
        let Some(keyring) = &self.keyring else {
            self.file_save(base_url, token)?;
            return Ok(SaveOutcome {
                backend: Backend::File,
                keyring_error: None,
                cleanup_warning: None,
            });
        };
        match keyring.save(base_url, token) {
            Ok(()) => {
                // `load` prefers the keyring, so a stale file entry only matters if the keyring
                // later becomes unreadable; a failure to remove it is a warning.
                let cleanup_warning = self
                    .file_remove(base_url)
                    .err()
                    .map(|e| format!("could not remove the old credentials file entry: {e}"));
                Ok(SaveOutcome {
                    backend: Backend::Keyring,
                    keyring_error: None,
                    cleanup_warning,
                })
            }
            Err(keyring_error) => {
                // `load` prefers the keyring: an older token left there would win over the
                // new file token, so it has to go before the file is written.
                keyring.delete(base_url).map_err(|e| {
                    StoreError::Keyring(format!(
                        "saving failed ({keyring_error}), and an older entry could not be \
                         removed: {e}"
                    ))
                })?;
                self.file_save(base_url, token)?;
                Ok(SaveOutcome {
                    backend: Backend::File,
                    keyring_error: Some(keyring_error),
                    cleanup_warning: None,
                })
            }
        }
    }

    /// Load the token for `base_url` from the keyring, then the file.
    fn load_token(&self, base_url: &str) -> Result<Option<AccessToken>> {
        if let Some(keyring) = &self.keyring
            && let Some(token) = keyring.load(base_url)
        {
            return Ok(Some(token));
        }
        self.file_load(base_url)
    }

    /// Delete the token for `base_url` from every backend. Missing entries are not an error;
    /// a keyring entry that could not be deleted is.
    fn delete_token(&self, base_url: &str) -> Result<()> {
        let keyring = match &self.keyring {
            Some(keyring) => keyring.delete(base_url).map_err(|e| {
                StoreError::Keyring(format!("could not delete the stored token: {e}"))
            }),
            None => Ok(()),
        };
        // Remove the file copy even when the keyring failed.
        self.file_remove(base_url)?;
        keyring
    }

    fn read_credentials(&self) -> Result<CredentialsFile> {
        let path = self.credentials_path();
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|_| StoreError::Format {
                context: format!("reading {}", path.display()),
                // The parser message may quote file content; keep it out.
                message: "file is not valid credentials JSON".into(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(CredentialsFile::default()),
            Err(e) => Err(io_err(format!("reading {}", path.display()))(e)),
        }
    }

    fn write_credentials(&self, file: &CredentialsFile) -> Result<()> {
        use io::Write as _;
        ensure_dir(&self.dir)?;
        let path = self.credentials_path();
        let tmp = self.dir.join("credentials.json.tmp");
        let json = serde_json::to_vec_pretty(file).expect("credentials serialize to JSON");
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut handle = options
            .open(&tmp)
            .map_err(io_err(format!("creating {}", tmp.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            handle
                .set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(io_err(format!("setting permissions on {}", tmp.display())))?;
        }
        handle
            .write_all(&json)
            .map_err(io_err(format!("writing {}", tmp.display())))?;
        handle
            .sync_all()
            .map_err(io_err(format!("syncing {}", tmp.display())))?;
        drop(handle);
        fs::rename(&tmp, &path).map_err(io_err(format!("replacing {}", path.display())))
    }

    fn file_save(&self, base_url: &str, token: &AccessToken) -> Result<()> {
        let mut file = self.read_credentials()?;
        file.tokens.insert(base_url.to_owned(), token.clone());
        self.write_credentials(&file)
    }

    fn file_load(&self, base_url: &str) -> Result<Option<AccessToken>> {
        Ok(self.read_credentials()?.tokens.remove(base_url))
    }

    fn file_remove(&self, base_url: &str) -> Result<()> {
        if !self.credentials_path().exists() {
            return Ok(());
        }
        let mut file = self.read_credentials()?;
        if file.tokens.remove(base_url).is_some() {
            self.write_credentials(&file)?;
        }
        Ok(())
    }
}

impl TokenStore for KeyringFileStore {
    fn load(&self, base_url: &str) -> Result<Option<StoredToken>> {
        self.load_token(base_url)
    }

    fn save(&self, base_url: &str, token: &StoredToken) -> Result<SaveOutcome> {
        self.save_token(base_url, token)
    }

    fn delete(&self, base_url: &str) -> Result<()> {
        self.delete_token(base_url)
    }
}

/// The non-secret metadata file (`config.toml`): default base URL and per-installation session
/// facts. Optional: a platform with its own storage can run a [`crate::Session`] without it.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    dir: PathBuf,
}

impl ConfigStore {
    /// `<config dir>/libreusos`.
    pub fn new() -> Result<Self> {
        Ok(Self {
            dir: default_dir()?,
        })
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn config_path(&self) -> PathBuf {
        self.dir.join("config.toml")
    }

    pub fn load_config(&self) -> Result<Config> {
        let path = self.config_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| StoreError::Format {
                context: format!("reading {}", path.display()),
                message: e.message().to_owned(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(io_err(format!("reading {}", path.display()))(e)),
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        ensure_dir(&self.dir)?;
        let path = self.config_path();
        let text = toml::to_string_pretty(config).expect("config serializes to TOML");
        fs::write(&path, text).map_err(io_err(format!("writing {}", path.display())))
    }

    /// Record (or replace) the metadata of the session for `base_url`.
    pub fn record_session(&self, base_url: &str, meta: SessionMeta) -> Result<()> {
        let mut config = self.load_config()?;
        config.sessions.insert(base_url.to_owned(), meta);
        self.save_config(&config)
    }

    /// Remember `base_url` as the default for later commands.
    pub fn set_default_base_url(&self, base_url: &str) -> Result<()> {
        let mut config = self.load_config()?;
        if config.default_base_url.as_deref() != Some(base_url) {
            config.default_base_url = Some(base_url.to_owned());
            self.save_config(&config)?;
        }
        Ok(())
    }

    /// Forget the metadata of the session for `base_url`.
    pub fn forget_session(&self, base_url: &str) -> Result<()> {
        let mut config = self.load_config()?;
        if config.sessions.remove(base_url).is_some() {
            self.save_config(&config)?;
        }
        Ok(())
    }
}

// ---- keyring (Linux only; elsewhere the file backend is always used) --------

#[cfg(target_os = "linux")]
fn keyring_save(base_url: &str, token: &AccessToken) -> std::result::Result<(), String> {
    let json = serde_json::to_string(token).map_err(|_| "token serialization failed".to_owned())?;
    keyring::Entry::new(KEYRING_SERVICE, base_url)
        .and_then(|entry| entry.set_password(&json))
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn keyring_load(base_url: &str) -> Option<AccessToken> {
    let json = keyring::Entry::new(KEYRING_SERVICE, base_url)
        .ok()?
        .get_password()
        .ok()?;
    serde_json::from_str(&json).ok()
}

#[cfg(target_os = "linux")]
fn keyring_delete(base_url: &str) -> std::result::Result<(), String> {
    match keyring::Entry::new(KEYRING_SERVICE, base_url).and_then(|e| e.delete_credential()) {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        // No Secret Service reachable from this session (e.g. headless): nothing here can
        // hold or read a keyring token, and saves go to the file.
        Err(keyring::Error::PlatformFailure(e))
            if matches!(
                e.downcast_ref::<secret_service::Error>(),
                Some(secret_service::Error::Unavailable)
            ) =>
        {
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(target_os = "linux"))]
fn keyring_save(_base_url: &str, _token: &AccessToken) -> std::result::Result<(), String> {
    Err("no keyring backend on this platform".to_owned())
}

#[cfg(not(target_os = "linux"))]
fn keyring_load(_base_url: &str) -> Option<AccessToken> {
    None
}

#[cfg(not(target_os = "linux"))]
fn keyring_delete(_base_url: &str) -> std::result::Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::Secret;
    use std::sync::Arc;

    const URL: &str = "https://example.com/";

    fn token() -> AccessToken {
        AccessToken {
            token: Secret::new("tok"),
            secret: Secret::new("sec"),
        }
    }

    #[test]
    fn file_roundtrip_and_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let store = KeyringFileStore::with_dir(tmp.path().join("libreusos"), false);
        assert!(store.load(URL).unwrap().is_none());

        let outcome = store.save(URL, &token()).unwrap();
        assert_eq!(outcome.backend, Backend::File);
        assert_eq!(store.load(URL).unwrap(), Some(token()));
        assert!(
            store
                .load_token("https://other.example/")
                .unwrap()
                .is_none()
        );

        store.delete(URL).unwrap();
        assert!(store.load(URL).unwrap().is_none());
        store.delete(URL).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_and_dir_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let store = KeyringFileStore::with_dir(tmp.path().join("libreusos"), false);
        store.save(URL, &token()).unwrap();
        let dir_mode = fs::metadata(store.dir()).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(store.dir().join("credentials.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[test]
    fn works_as_trait_object() {
        let tmp = tempfile::tempdir().unwrap();
        let store: Arc<dyn TokenStore> = Arc::new(KeyringFileStore::with_dir(
            tmp.path().join("libreusos"),
            false,
        ));
        store.save(URL, &token()).unwrap();
        assert_eq!(store.load(URL).unwrap(), Some(token()));
        store.delete(URL).unwrap();
        assert_eq!(store.load(URL).unwrap(), None);
    }

    #[test]
    fn config_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::with_dir(tmp.path().join("libreusos"));
        assert_eq!(store.load_config().unwrap(), Config::default());
        let meta = SessionMeta {
            user_id: Some("123".into()),
            scopes: vec!["studies".into(), "grades".into()],
            backend: Some(Backend::File),
            saved_at_unix: Some(1_700_000_000),
        };
        store.record_session(URL, meta.clone()).unwrap();
        assert_eq!(store.load_config().unwrap().sessions.get(URL), Some(&meta));
        store.forget_session(URL).unwrap();
        assert!(store.load_config().unwrap().sessions.is_empty());
        let text = fs::read_to_string(tmp.path().join("libreusos/config.toml")).unwrap();
        assert!(!text.contains("tok") && !text.contains("sec"));
    }

    /// Needs a running Secret Service; run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "requires a Secret Service (gnome-keyring) session"]
    fn keyring_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = KeyringFileStore::with_dir(tmp.path().join("libreusos"), true);
        let url = "https://keyring-test.invalid/";
        let outcome = store.save(url, &token()).unwrap();
        assert_eq!(
            outcome.backend,
            Backend::Keyring,
            "keyring error: {:?}",
            outcome.keyring_error
        );
        assert_eq!(store.load(url).unwrap(), Some(token()));
        store.delete(url).unwrap();
        assert!(store.load(url).unwrap().is_none());
    }

    /// An in-memory keyring whose save and delete can be made to fail.
    #[derive(Debug, Default)]
    struct FakeKeyring {
        tokens: std::sync::Mutex<BTreeMap<String, AccessToken>>,
        fail_save: std::sync::atomic::AtomicBool,
        fail_delete: std::sync::atomic::AtomicBool,
    }

    impl FakeKeyring {
        fn set_failures(&self, save: bool, delete: bool) {
            use std::sync::atomic::Ordering::SeqCst;
            self.fail_save.store(save, SeqCst);
            self.fail_delete.store(delete, SeqCst);
        }
        fn get(&self, base_url: &str) -> Option<AccessToken> {
            self.tokens.lock().unwrap().get(base_url).cloned()
        }
    }

    impl Keyring for FakeKeyring {
        fn save(&self, base_url: &str, token: &AccessToken) -> std::result::Result<(), String> {
            if self.fail_save.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("keyring locked".into());
            }
            self.tokens
                .lock()
                .unwrap()
                .insert(base_url.to_owned(), token.clone());
            Ok(())
        }
        fn load(&self, base_url: &str) -> Option<AccessToken> {
            self.get(base_url)
        }
        fn delete(&self, base_url: &str) -> std::result::Result<(), String> {
            if self.fail_delete.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("keyring locked".into());
            }
            self.tokens.lock().unwrap().remove(base_url);
            Ok(())
        }
    }

    fn other_token() -> AccessToken {
        AccessToken {
            token: Secret::new("tok2"),
            secret: Secret::new("sec2"),
        }
    }

    fn fake_store(tmp: &tempfile::TempDir) -> (KeyringFileStore, Arc<FakeKeyring>) {
        let keyring = Arc::new(FakeKeyring::default());
        let store = KeyringFileStore::with_keyring(tmp.path().join("libreusos"), keyring.clone());
        (store, keyring)
    }

    #[test]
    fn delete_reports_a_keyring_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let (store, keyring) = fake_store(&tmp);
        // A missing entry is fine.
        store.delete(URL).unwrap();

        store.save(URL, &token()).unwrap();
        keyring.set_failures(false, true);
        let err = store.delete(URL).unwrap_err();
        assert!(matches!(err, StoreError::Keyring(_)), "{err:?}");
        assert_eq!(store.load(URL).unwrap(), Some(token()));

        keyring.set_failures(false, false);
        store.delete(URL).unwrap();
        assert_eq!(store.load(URL).unwrap(), None);
    }

    #[test]
    fn delete_removes_the_file_copy_even_if_the_keyring_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let (store, keyring) = fake_store(&tmp);
        keyring.set_failures(true, false);
        assert_eq!(store.save(URL, &token()).unwrap().backend, Backend::File);
        keyring.set_failures(false, true);
        assert!(store.delete(URL).is_err());
        assert_eq!(store.file_load(URL).unwrap(), None);
    }

    #[test]
    fn file_fallback_removes_an_older_keyring_token() {
        let tmp = tempfile::tempdir().unwrap();
        let (store, keyring) = fake_store(&tmp);
        assert_eq!(store.save(URL, &token()).unwrap().backend, Backend::Keyring);

        keyring.set_failures(true, false);
        let outcome = store.save(URL, &other_token()).unwrap();
        assert_eq!(outcome.backend, Backend::File);
        assert!(outcome.keyring_error.is_some());
        assert_eq!(keyring.get(URL), None);
        assert_eq!(store.load(URL).unwrap(), Some(other_token()));
    }

    #[test]
    fn file_fallback_fails_if_the_older_keyring_token_stays() {
        let tmp = tempfile::tempdir().unwrap();
        let (store, keyring) = fake_store(&tmp);
        store.save(URL, &token()).unwrap();

        keyring.set_failures(true, true);
        let err = store.save(URL, &other_token()).unwrap_err();
        assert!(matches!(err, StoreError::Keyring(_)), "{err:?}");
        assert!(!err.to_string().contains("tok2") && !err.to_string().contains("sec2"));
        // Nothing was written that `load` would not return.
        assert_eq!(store.file_load(URL).unwrap(), None);
        assert_eq!(store.load(URL).unwrap(), Some(token()));
    }

    #[test]
    fn keyring_save_removes_the_file_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let (store, keyring) = fake_store(&tmp);
        keyring.set_failures(true, false);
        store.save(URL, &token()).unwrap();
        assert_eq!(store.file_load(URL).unwrap(), Some(token()));

        keyring.set_failures(false, false);
        let outcome = store.save(URL, &other_token()).unwrap();
        assert_eq!(outcome.backend, Backend::Keyring);
        assert_eq!(outcome.cleanup_warning, None);
        assert_eq!(store.file_load(URL).unwrap(), None);
        assert_eq!(store.load(URL).unwrap(), Some(other_token()));
    }

    #[test]
    fn corrupt_credentials_error_does_not_quote_content() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("libreusos");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("credentials.json"), "{\"tokens\": hunter2-secret").unwrap();
        let store = KeyringFileStore::with_dir(&dir, false);
        let err = store.load(URL).unwrap_err().to_string();
        assert!(!err.contains("hunter2"));
    }
}
