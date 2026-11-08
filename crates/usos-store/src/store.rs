//! The database handle, schema migrations and the account helpers.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

use crate::error::{Result, StoreError};

/// Schema migrations. Entry `i` upgrades `user_version` from `i` to `i + 1`; never edit a
/// released entry, append a new one.
const MIGRATIONS: &[&str] = &[
    // 1: initial schema
    "
    CREATE TABLE meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) WITHOUT ROWID;

    -- One row per timetable entry. `date` is the day it starts on (YYYY-MM-DD); `json` is the
    -- core `Activity`; the other columns only serve queries.
    CREATE TABLE activities (
        id        INTEGER PRIMARY KEY,
        account   TEXT NOT NULL,
        starts_at TEXT NOT NULL,
        ends_at   TEXT NOT NULL,
        date      TEXT NOT NULL,
        course_id TEXT,
        kind      TEXT NOT NULL,
        json      TEXT NOT NULL
    );
    CREATE INDEX activities_by_date ON activities (account, date, starts_at);

    -- Which days of the timetable were fetched, and when. A day without a row was never fetched
    -- (as opposed to fetched and empty).
    CREATE TABLE activity_days (
        account    TEXT NOT NULL,
        date       TEXT NOT NULL,
        fetched_at TEXT NOT NULL,
        PRIMARY KEY (account, date)
    ) WITHOUT ROWID;

    -- Whole API results kept as JSON: terms, user, grades per term, tests per term.
    CREATE TABLE snapshots (
        account    TEXT NOT NULL,
        kind       TEXT NOT NULL,
        key        TEXT NOT NULL,
        json       TEXT NOT NULL,
        hash       TEXT NOT NULL,
        fetched_at TEXT NOT NULL,
        PRIMARY KEY (account, kind, key)
    ) WITHOUT ROWID;
    ",
];

/// The account key for rows of one user on one USOS installation: `<base_url>#<user_id>`.
/// Trailing slashes of the base URL are ignored.
pub fn account_key(base_url: &str, user_id: &str) -> String {
    format!(
        "{}#{}",
        base_url.trim().trim_end_matches('/'),
        user_id.trim()
    )
}

// shortcut: no retention. Activities and snapshots of past terms and of old accounts stay until
// `delete_account`; a few KB per week is not worth a pruning policy yet.
/// A handle on one cache database. Cheap to share between threads (`Arc<Store>`); calls are
/// serialized by an internal mutex.
#[derive(Debug)]
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Open (creating if needed) the database file at `path` and migrate it to the current
    /// schema. Missing parent directories are created (mode 0700 on Unix) and a new file gets
    /// mode 0600, because the cache holds personal data. The caller chooses the path.
    ///
    /// `path` must be a plain file path: `:memory:`, the empty string and `file:` URIs are
    /// rejected (use [`Store::open_in_memory`]). An existing file and its directory are
    /// tightened to 0600 / 0700 when they are ours to change.
    pub fn open(path: &str) -> Result<Self> {
        if path.is_empty() || path == ":memory:" || path.starts_with("file:") {
            return Err(StoreError::InvalidPath(path.to_owned()));
        }
        prepare_file(Path::new(path))?;
        // No SQLITE_OPEN_URI: the path is never interpreted as a URI.
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        let store = Self::from_connection(conn, true)?;
        // SQLite creates -wal and -shm with the mode of the database file; make sure.
        for suffix in ["-wal", "-shm"] {
            tighten(Path::new(&format!("{path}{suffix}")), 0o600)?;
        }
        Ok(store)
    }

    /// A private in-memory database (tests, previews).
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?, false)
    }

    fn from_connection(mut conn: Connection, wal: bool) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Switching a fresh file to WAL needs an exclusive lock and SQLite does not always wait
        // for it (it returns BUSY to avoid a deadlock), so a second process opening the same new
        // file at the same moment retries. Every step is safe to repeat.
        let mut attempt = 0;
        loop {
            let setup = (|| -> Result<()> {
                if wal {
                    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
                    conn.pragma_update(None, "synchronous", "NORMAL")?;
                }
                migrate(&mut conn)
            })();
            match setup {
                Err(StoreError::Sqlite(rusqlite::Error::SqliteFailure(e, _)))
                    if e.code == rusqlite::ErrorCode::DatabaseBusy && attempt < 100 =>
                {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                other => break other?,
            }
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// The schema version of the open database (`PRAGMA user_version`).
    pub fn schema_version(&self) -> Result<u32> {
        user_version(&self.lock())
    }

    /// Remember which account is the active one for `base_url`, so that offline code can find
    /// its data without asking the server who the token belongs to.
    pub fn set_active_account(&self, base_url: &str, account: &str) -> Result<()> {
        self.lock().execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![active_key(base_url), account],
        )?;
        Ok(())
    }

    /// The account last passed to [`Store::set_active_account`] for `base_url`.
    pub fn active_account(&self, base_url: &str) -> Result<Option<String>> {
        Ok(self
            .lock()
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![active_key(base_url)],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Delete everything cached for `account` (for example on logout).
    pub fn delete_account(&self, account: &str) -> Result<()> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        delete_rows(&tx, account)?;
        tx.commit()?;
        Ok(())
    }

    /// How many times [`Store::forget_active_account`] ran for `base_url`. Read before a fetch
    /// and compared when its result is stored, it tells whether a logout happened in between.
    pub fn session_epoch(&self, base_url: &str) -> Result<u64> {
        read_epoch(&self.lock(), base_url)
    }

    /// Logout: in one transaction, bump the [`Store::session_epoch`] of `base_url`, delete
    /// everything cached for its active account and forget that it was active. Returns that
    /// account, if there was one.
    pub fn forget_active_account(&self, base_url: &str) -> Result<Option<String>> {
        let mut conn = self.lock();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let next = read_epoch(&tx, base_url)? + 1;
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![epoch_key(base_url), next.to_string()],
        )?;
        let account: Option<String> = tx
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![active_key(base_url)],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(account) = &account {
            delete_rows(&tx, account)?;
        }
        tx.commit()?;
        Ok(account)
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic in another thread cannot leave the connection half-written: every write is
        // a transaction. So a poisoned lock is safe to reuse.
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(crate) fn active_key(base_url: &str) -> String {
    format!("active_account:{}", base_url.trim().trim_end_matches('/'))
}

fn epoch_key(base_url: &str) -> String {
    format!("session_epoch:{}", base_url.trim().trim_end_matches('/'))
}

pub(crate) fn read_epoch(conn: &Connection, base_url: &str) -> Result<u64> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![epoch_key(base_url)],
            |r| r.get(0),
        )
        .optional()?;
    value.map_or(Ok(0), |v| {
        v.parse()
            .map_err(|_| StoreError::Corrupt(format!("session epoch {v:?}")))
    })
}

/// `Err(LoggedOut)` unless `account` is the active account of some installation. Cache writes
/// of a fetch call it in their own transaction: after a logout the account is no longer active,
/// so a fetch that was running then cannot fill the cache again.
pub(crate) fn ensure_active(conn: &Connection, account: &str) -> Result<()> {
    let active: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM meta
                        WHERE substr(key, 1, 15) = 'active_account:' AND value = ?1)",
        params![account],
        |r| r.get(0),
    )?;
    if active {
        Ok(())
    } else {
        Err(StoreError::LoggedOut)
    }
}

fn delete_rows(conn: &Connection, account: &str) -> Result<()> {
    for table in ["activities", "activity_days", "snapshots"] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE account = ?1"),
            params![account],
        )?;
    }
    conn.execute("DELETE FROM meta WHERE value = ?1", params![account])?;
    Ok(())
}

fn user_version(conn: &Connection) -> Result<u32> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
}

/// Bring the schema up to date. Each step runs in its own write transaction that re-reads
/// `user_version` first, so two processes opening the same new file cannot both run a step: the
/// loser waits for the lock, sees the new version and does nothing.
fn migrate(conn: &mut Connection) -> Result<()> {
    let supported = MIGRATIONS.len() as u32;
    loop {
        // `user_version` lives in the file header and is rolled back with the transaction.
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let found = user_version(&tx)?;
        if found > supported {
            return Err(StoreError::SchemaTooNew { found, supported });
        }
        if found == supported {
            return Ok(());
        }
        tx.execute_batch(MIGRATIONS[found as usize])?;
        tx.pragma_update(None, "user_version", found + 1)?;
        tx.commit()?;
    }
}

/// Make `path` private to its owner if it exists and its mode is wider. A file we do not own
/// cannot be changed and is left as it is.
#[cfg(unix)]
fn tighten(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(());
    };
    if meta.permissions().mode() & 0o777 & !mode == 0 {
        return Ok(());
    }
    match std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(()),
        other => Ok(other?),
    }
}

#[cfg(not(unix))]
fn tighten(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

/// Create the parent directory (0700) and the file (0600) before SQLite does, so the
/// database, and the `-wal`/`-shm` files that copy its mode, are never world-readable.
fn prepare_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs::{DirBuilder, OpenOptions};
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
            tighten(dir, 0o700)?;
        }
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)?;
        tighten(path, 0o600)?;
    }
    #[cfg(not(unix))]
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

/// Timestamps are stored as UTC RFC 3339 with milliseconds, which sorts as text.
pub(crate) fn ts(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn parse_ts(text: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StoreError::Corrupt(format!("timestamp {text:?}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_key_format() {
        assert_eq!(
            account_key("https://usosapps.polsl.pl/", "123"),
            "https://usosapps.polsl.pl#123"
        );
        assert_eq!(account_key("https://x.pl", " 7 "), "https://x.pl#7");
    }

    #[test]
    fn migrates_from_empty_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.sqlite");
        let path = path.to_str().unwrap();
        {
            let store = Store::open(path).unwrap();
            assert_eq!(store.schema_version().unwrap(), MIGRATIONS.len() as u32);
            let tables: Vec<String> = store
                .lock()
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(tables, ["activities", "activity_days", "meta", "snapshots"]);
            let mode: String = store
                .lock()
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap();
            assert_eq!(mode, "wal");
            store
                .set_active_account("https://x.pl/", "https://x.pl#1")
                .unwrap();
        }
        let again = Store::open(path).unwrap();
        assert_eq!(again.schema_version().unwrap(), MIGRATIONS.len() as u32);
        // Data survives reopening.
        assert_eq!(
            again.active_account("https://x.pl").unwrap().as_deref(),
            Some("https://x.pl#1")
        );
    }

    #[test]
    fn refuses_a_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.sqlite");
        let path = path.to_str().unwrap();
        Store::open(path).unwrap();
        Connection::open(path)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        match Store::open(path) {
            Err(StoreError::SchemaTooNew { found: 99, .. }) => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn file_directory_and_sidecars_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("a").join("b");
        let path = sub.join("cache.sqlite");
        let store = Store::open(path.to_str().unwrap()).unwrap();
        store.set_active_account("u", "a").unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&sub), 0o700);
        assert_eq!(mode(&dir.path().join("a")), 0o700);
        // The connection is open, so the WAL sidecars exist.
        for suffix in ["-wal", "-shm"] {
            let side = sub.join(format!("cache.sqlite{suffix}"));
            assert_eq!(mode(&side), 0o600, "{suffix}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn existing_wide_file_and_directory_are_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.sqlite");
        std::fs::write(&path, b"").unwrap();
        let set =
            |p: &Path, m| std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).unwrap();
        set(&path, 0o644);
        set(dir.path(), 0o755);
        let _store = Store::open(path.to_str().unwrap()).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(dir.path()), 0o700);
    }

    #[test]
    fn only_plain_file_paths_are_accepted() {
        for bad in [
            "",
            ":memory:",
            "file:cache.sqlite?mode=memory",
            "file::memory:",
        ] {
            assert!(
                matches!(Store::open(bad), Err(StoreError::InvalidPath(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn concurrent_first_opens_migrate_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("race.sqlite").to_str().unwrap().to_owned();
        for round in 0..5 {
            let path = format!("{path}{round}");
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let threads: Vec<_> = (0..2)
                .map(|_| {
                    let (path, barrier) = (path.clone(), barrier.clone());
                    std::thread::spawn(move || {
                        barrier.wait();
                        Store::open(&path).map(|s| s.schema_version().unwrap())
                    })
                })
                .collect();
            for t in threads {
                assert_eq!(t.join().unwrap().unwrap(), MIGRATIONS.len() as u32);
            }
        }
    }

    #[test]
    fn delete_account_removes_only_that_account() {
        let store = Store::open_in_memory().unwrap();
        for a in ["a", "b"] {
            store.set_active_account(&format!("u-{a}"), a).unwrap();
        }
        store.delete_account("a").unwrap();
        assert_eq!(store.active_account("u-a").unwrap(), None);
        assert_eq!(store.active_account("u-b").unwrap().as_deref(), Some("b"));
    }

    #[test]
    fn store_is_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<Store>();
    }
}
