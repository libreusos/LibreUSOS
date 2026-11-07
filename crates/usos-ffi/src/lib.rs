//! UniFFI bindings of the LibreUSOS core (`usos-core` + `usos-store`) for Kotlin.
//!
//! Entry point: [`UsosClient`]. The app supplies the installation URL, the consumer key and
//! secret (plain strings), a cache database path and a [`TokenStore`] it implements itself
//! (Android Keystore). Nothing here logs or prints a secret.
//!
//! # Async and threading
//!
//! Network calls are `async` exports (Kotlin `suspend` functions) on UniFFI's tokio support.
//! The core is already `async` on tokio/reqwest, so there is no runtime of our own and no
//! blocking wrapper. UniFFI polls these futures on the caller's coroutine dispatcher, and
//! everything that is not network I/O runs there too: SQLite cache access and the (synchronous)
//! calls into the app's [`TokenStore`]. Kotlin must therefore call every function of this crate
//! from `Dispatchers.IO` (never `Main`).
//!
//! # Data conventions
//!
//! - Calendar dates are ISO strings `YYYY-MM-DD`; local date-times (the installation's civil
//!   time, no offset, as USOS reports them) are `YYYY-MM-DDTHH:MM:SS`. Both parse with
//!   `java.time.LocalDate.parse` / `LocalDateTime.parse`. Date arguments use the same format.
//! - Real instants (`fetched_at`) are `i64` seconds since the Unix epoch, UTC.
//! - USOS times carry no offset: they are the installation's civil time, see
//!   [`installation_time_zone`]. Kotlin must compute `today` in that zone, not the device's.
//! - [`LangText`] keeps both languages; the UI picks one and falls back to the other.

mod client;
mod error;
mod model;
mod token;

pub use client::{LoggedIn, PendingLogin, PendingLoginState, UsosClient, installation_time_zone};
pub use error::UsosError;
pub use model::*;
pub use token::{TokenRecord, TokenStore};

uniffi::setup_scaffolding!();
