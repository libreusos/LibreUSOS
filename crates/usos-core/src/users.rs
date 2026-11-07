//! Users (`services/users/*`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::Client;
use crate::error::{Error, Result};

/// A USOS user, reduced to what identifies them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
}

impl User {
    /// `"First Last"`, trimmed; empty when both names are missing.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
            .trim()
            .to_owned()
    }

    /// Parse a `users/user` object requested with `id|first_name|last_name`.
    fn from_json(value: &Value) -> Result<Self> {
        let id = match value.get("id") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => {
                return Err(Error::UnexpectedResponse(
                    "users/user response has no `id`".to_owned(),
                ));
            }
        };
        let name = |k: &str| {
            value
                .get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        Ok(Self {
            id,
            first_name: name("first_name"),
            last_name: name("last_name"),
        })
    }
}

impl Client {
    /// The logged-in user (`users/user` without `user_id`).
    pub async fn current_user(&self) -> Result<User> {
        User::from_json(&self.users_user("id|first_name|last_name").await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_string_and_numeric_ids() {
        let u = User::from_json(
            &serde_json::json!({"id":"123","first_name":" Jan ","last_name":"Kowalski"}),
        )
        .unwrap();
        assert_eq!(u.id, "123");
        assert_eq!(u.full_name(), "Jan Kowalski");
        let u = User::from_json(&serde_json::json!({"id":7,"first_name":null})).unwrap();
        assert_eq!((u.id.as_str(), u.full_name().as_str()), ("7", ""));
        assert!(User::from_json(&serde_json::json!({"first_name":"x"})).is_err());
    }

    #[test]
    fn roundtrips_through_json() {
        let u = User {
            id: "123".into(),
            first_name: "Jan".into(),
            last_name: "Kowalski".into(),
        };
        let json = serde_json::to_string(&u).unwrap();
        assert_eq!(
            json,
            r#"{"id":"123","first_name":"Jan","last_name":"Kowalski"}"#
        );
        assert_eq!(serde_json::from_str::<User>(&json).unwrap(), u);
    }
}
