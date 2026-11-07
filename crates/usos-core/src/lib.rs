//! LibreUSOS core: a GUI-agnostic client for the USOS API (unofficial).
//!
//! - [`oauth`]: in-house OAuth 1.0a HMAC-SHA1 signer (pure functions).
//! - [`client`]: signed requests, the three-legged login flow, generic calls.
//! - [`store`]: the [`store::TokenStore`] trait for access tokens (default: [`store::KeyringFileStore`],
//!   system keyring with a 0600 file fallback) and the optional metadata [`store::ConfigStore`].
//! - [`auth`]: consumer credentials, scope selection, the login flow, the loopback callback server.
//! - [`session`]: stored config + token -> an authenticated [`Client`]; logout.
//! - [`users`]: the logged-in [`users::User`].
//! - [`terms`]: academic terms and the current-term rule.
//! - [`timetable`]: the user's activities (`tt/user`), range fetching, grouping by day.
//! - [`grades`]: the user's grades per term (`grades/terms2`), the latest grades, and the Polish-scale average.
//! - [`crstests`]: the user's course tests (points and grades per test node).
//! - [`models`]: small shared data types such as [`models::LangDict`].
//!
//! Secrets (consumer secret, tokens) are wrapped in [`secret::Secret`], whose
//! `Debug` output is redacted, and are scrubbed from error messages.

pub mod auth;
pub mod client;
pub mod crstests;
pub mod error;
pub mod grades;
mod lenient;
pub mod models;
pub mod oauth;
pub mod secret;
pub mod session;
pub mod store;
pub mod terms;
pub mod timetable;
pub mod users;

pub use client::{AccessToken, Client, RequestToken, normalize_base_url};
pub use crstests::{NodeGrade, NodeKind, TestNode, TestRoot};
pub use error::{ApiError, Error, Result};
pub use grades::{CourseGrades, Grade, LatestGrade, TermGrades, UnitGrades, average};
pub use models::{Lang, LangDict};
pub use secret::Secret;
pub use session::Session;
pub use store::{StoredToken, TokenStore};
pub use terms::{Term, current_term};
pub use timetable::{Activity, ActivityKind, DayPlan, Frequency};
pub use users::User;
