//! The scopes LibreUSOS asks for at login.

use crate::client::Client;

/// Everything the app may need, requested in one go. Scopes the installation
/// does not define are dropped before the request (see [`filter_defined`]).
pub const REQUESTED_SCOPES: &[&str] = &[
    "studies",
    "grades",
    "crstests",
    "cards",
    "events",
    "email",
    "photo",
    "student_exams",
    "surveys_filling",
    "mailclient",
    "placement_tests",
    "adm_documents",
    "offline_access",
];

/// Split `requested` into (defined by the installation, not defined), keeping order.
pub fn filter_defined<'a>(
    requested: &[&'a str],
    defined: &[String],
) -> (Vec<&'a str>, Vec<&'a str>) {
    requested
        .iter()
        .partition(|s| defined.iter().any(|d| d == **s))
}

/// The scopes to request from one installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeSelection {
    /// Scopes to request, in [`REQUESTED_SCOPES`] order.
    pub scopes: Vec<String>,
    /// Requested scopes the installation does not define (dropped).
    pub dropped: Vec<String>,
    /// Why the installation's scope list could not be fetched; then every scope is requested.
    pub list_error: Option<String>,
}

/// [`REQUESTED_SCOPES`] minus those the installation does not define (`apiref/scopes`).
pub async fn select_scopes(client: &Client) -> ScopeSelection {
    match client.defined_scopes().await {
        Ok(defined) => {
            let (kept, dropped) = filter_defined(REQUESTED_SCOPES, &defined);
            ScopeSelection {
                scopes: kept.into_iter().map(str::to_owned).collect(),
                dropped: dropped.into_iter().map(str::to_owned).collect(),
                list_error: None,
            }
        }
        Err(e) => ScopeSelection {
            scopes: REQUESTED_SCOPES.iter().map(|s| (*s).to_owned()).collect(),
            dropped: Vec::new(),
            list_error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_undefined_scopes() {
        let defined = vec!["studies".to_owned(), "grades".to_owned()];
        let (kept, dropped) = filter_defined(&["studies", "photo", "grades"], &defined);
        assert_eq!(kept, ["studies", "grades"]);
        assert_eq!(dropped, ["photo"]);
    }

    #[test]
    fn requested_list_has_no_duplicates() {
        let mut v = REQUESTED_SCOPES.to_vec();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), REQUESTED_SCOPES.len());
    }
}
