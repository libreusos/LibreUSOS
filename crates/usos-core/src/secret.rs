//! A string wrapper that never shows its contents in `Debug` and wipes memory on drop.

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// A secret string value (consumer key/secret, OAuth tokens).
///
/// `Debug` prints `Secret(***)`. There is deliberately no `Display` impl; the
/// value is only reachable through [`Secret::expose`].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value. Never log or print the result.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s:?}"), "Secret(***)");
        assert_eq!(format!("{:#?}", Some(&s)), "Some(\n    Secret(***),\n)");
        assert_eq!(s.expose(), "hunter2");
    }
}
