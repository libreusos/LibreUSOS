//! Authentication: consumer credentials, scopes, the login flow and its callback server.

pub mod consumer;
pub mod login;
#[cfg(feature = "loopback")]
pub mod loopback;
pub mod scopes;
