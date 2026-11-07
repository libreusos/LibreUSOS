//! LibreUSOS offline cache: a SQLite file with the user's timetable and snapshots of the other
//! data, plus thin online-or-cached orchestration over [`usos_core::Client`].
//!
//! - [`Store`]: a thread-safe handle on one database file (`Send + Sync`, synchronous API).
//!   Every row belongs to an `account` (see [`account_key`]), so several users or installations
//!   can share one file.
//! - [`Store::replace_activities`] / [`Store::activities`]: the timetable by date range, which
//!   is what widgets query. [`Store::activity_coverage`] tells when a range was fetched.
//! - [`Store::put_terms`], [`Store::put_term_grades`], [`Store::put_course_tests`],
//!   [`Store::put_user`] and their `get_*` twins: snapshots with change detection
//!   ([`PutOutcome`]) and a `fetched_at` time ([`CachedTerms`] and friends).
//! - [`repo`]: `refresh_*` (online, then store), `cached_*` (store only) and `*_or_cached`
//!   (online, and only on a network failure the store), plus [`repo::sync`].
//!
//! The public edge uses owned types and concrete structs only (no generics, lifetimes or
//! `impl Trait`) so that it can be wrapped by UniFFI. Nothing here prints or guesses paths.

mod activities;
mod error;
pub mod repo;
mod snapshots;
mod store;

pub use activities::{ActivityCoverage, ReplaceOutcome};
pub use error::{RepoError, Result, StoreError};
pub use snapshots::{CachedCourseTests, CachedTermGrades, CachedTerms, CachedUser, PutOutcome};
pub use store::{Store, account_key};
