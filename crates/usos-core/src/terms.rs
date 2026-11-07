//! Academic terms (`services/terms/*`) and the "current term" rule.

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::LangDict;

/// How far ahead (in days) a term that has not started yet still counts as "current".
pub const UPCOMING_WINDOW_DAYS: u64 = 30;

/// An academic term, e.g. `2026/2027-Z` (semester) or `2026/2027` (academic year).
///
/// Dates are installation-local calendar dates (USOS sends no offset).
/// `courses/user` returns a reduced shape, so `finish_date` and `is_active` are optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: LangDict,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    #[serde(default)]
    pub finish_date: Option<NaiveDate>,
    #[serde(default)]
    pub order_key: Option<f64>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Treat an explicit JSON `null` like a missing field.
fn null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

impl Term {
    /// Length of the term in days; semesters are shorter than academic years.
    fn span_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days()
    }

    /// End of the exam session: `finish_date` when present, else `end_date`.
    fn effective_end(&self) -> NaiveDate {
        self.finish_date.unwrap_or(self.end_date)
    }

    fn contains(&self, day: NaiveDate) -> bool {
        self.start_date <= day && day <= self.end_date
    }
}

/// Pick the current term.
///
/// 1. Among terms containing `today`, the shortest span (semester over academic year),
///    ties broken by the latest start.
/// 2. Otherwise the nearest term starting within [`UPCOMING_WINDOW_DAYS`] days, unless a term
///    is still running its exam session (`today <= finish_date`), which wins.
/// 3. Otherwise the most recently ended term (by `finish_date`, falling back to `end_date`).
///    Ties go to the shorter span (semester over year), then the later start.
pub fn current_term(terms: &[Term], today: NaiveDate) -> Option<Term> {
    pick_current(terms, today).cloned()
}

fn pick_current(terms: &[Term], today: NaiveDate) -> Option<&Term> {
    let containing = terms
        .iter()
        .filter(|t| t.contains(today))
        .min_by_key(|t| (t.span_days(), std::cmp::Reverse(t.start_date)));
    if containing.is_some() {
        return containing;
    }
    // Most recently ended term; a term in its exam session (after `end_date`, up to
    // `finish_date`) still counts as running and outranks an upcoming one.
    let recent = terms.iter().filter(|t| t.end_date < today).max_by_key(|t| {
        (
            t.effective_end(),
            std::cmp::Reverse(t.span_days()),
            t.start_date,
        )
    });
    if let Some(t) = recent
        && t.effective_end() >= today
    {
        return Some(t);
    }
    let horizon = today.checked_add_days(Days::new(UPCOMING_WINDOW_DAYS));
    let upcoming = horizon.and_then(|horizon| {
        terms
            .iter()
            .filter(|t| t.start_date > today && t.start_date <= horizon)
            .min_by_key(|t| (t.start_date, t.span_days()))
    });
    upcoming.or(recent)
}

/// Parse a JSON array of terms, skipping entries without usable ids or dates.
fn parse_terms(value: serde_json::Value) -> Result<Vec<Term>> {
    let serde_json::Value::Array(items) = value else {
        return Err(Error::UnexpectedResponse(
            "unexpected terms shape (not an array)".to_owned(),
        ));
    };
    Ok(items
        .into_iter()
        .filter_map(|item| serde_json::from_value(item).ok())
        .collect())
}

impl Client {
    /// `services/terms/terms_index`: every term known to the installation.
    pub async fn terms_index(&self) -> Result<Vec<Term>> {
        parse_terms(self.call("terms/terms_index", &[]).await?)
    }

    /// `services/terms/terms`: the given terms.
    pub async fn terms(&self, ids: &[String]) -> Result<Vec<Term>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let params = [("term_ids".to_owned(), ids.join("|"))];
        parse_terms(self.call("terms/terms", &params).await?)
    }

    /// The terms the logged-in user has course editions in (`courses/user`, field `terms`), ordered by start date.
    pub async fn user_terms(&self) -> Result<Vec<Term>> {
        let value = self
            .call_authenticated("courses/user", &[("fields".to_owned(), "terms".to_owned())])
            .await?;
        let terms = value.get("terms").cloned().ok_or_else(|| {
            Error::UnexpectedResponse("courses/user response has no `terms`".to_owned())
        })?;
        let mut terms = parse_terms(terms)?;
        terms.sort_by(|a, b| a.start_date.cmp(&b.start_date).then(a.id.cmp(&b.id)));
        Ok(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    /// Shapes copied from `terms_index` (PolSl).
    fn index() -> Vec<Term> {
        serde_json::from_str(
            r#"[
            {"end_date":"2026-09-30","finish_date":"2026-09-30","id":"2025/2026","is_active":false,
             "name":{"en":"Academic year 2025/2026","pl":"Rok akademicki 2025/2026"},"order_key":91.0,"start_date":"2025-10-01","extra":1},
            {"end_date":"2026-02-28","finish_date":"2026-02-28","id":"2025/2026-Z","is_active":false,
             "name":{"en":"Winter semester 2025/2026","pl":"Semestr zimowy 2025/2026"},"order_key":92.0,"start_date":"2025-10-01"},
            {"end_date":"2026-09-30","finish_date":"2026-09-30","id":"2025/2026-L","is_active":true,
             "name":{"en":"Summer semester 2025/2026","pl":"Semestr letni 2025/2026"},"order_key":93.0,"start_date":"2026-03-01"},
            {"end_date":"2027-09-30","finish_date":"2027-09-30","id":"2026/2027","is_active":true,
             "name":{"en":"Academic year 2026/2027","pl":"Rok akademicki 2026/2027"},"order_key":94.0,"start_date":"2026-10-01"},
            {"end_date":"2027-02-28","finish_date":"2027-02-28","id":"2026/2027-Z","is_active":true,
             "name":{"en":"Winter semester 2026/2027","pl":"Semestr zimowy 2026/2027"},"order_key":95.0,"start_date":"2026-10-01"},
            {"end_date":"2027-09-30","finish_date":"2027-09-30","id":"2026/2027-L","is_active":true,
             "name":{"en":"Summer semester 2026/2027","pl":"Semestr letni 2026/2027"},"order_key":96.0,"start_date":"2027-03-01"}
        ]"#,
        )
        .unwrap()
    }

    fn pick(terms: &[Term], today: &str) -> Option<String> {
        current_term(terms, d(today)).map(|t| t.id.clone())
    }

    #[test]
    fn semester_beats_year() {
        let t = index();
        assert_eq!(pick(&t, "2026-11-04").as_deref(), Some("2026/2027-Z"));
        assert_eq!(pick(&t, "2027-04-10").as_deref(), Some("2026/2027-L"));
    }

    #[test]
    fn boundaries_are_inclusive() {
        let t = index();
        assert_eq!(pick(&t, "2026-10-01").as_deref(), Some("2026/2027-Z"));
        assert_eq!(pick(&t, "2027-02-28").as_deref(), Some("2026/2027-Z"));
    }

    #[test]
    fn upcoming_within_window() {
        // Only the semesters of 2026/2027 are in the user's list; 2026-09-15 is before the first.
        let t: Vec<Term> = index().into_iter().skip(4).collect();
        assert_eq!(pick(&t, "2026-09-15").as_deref(), Some("2026/2027-Z"));
        assert_eq!(pick(&t, "2026-08-15"), None);
    }

    #[test]
    fn gap_between_semesters_with_user_terms_only() {
        // The user only has winter 2025/26 and summer 2026/27; between them there is a gap.
        let user: Vec<Term> = index()
            .into_iter()
            .filter(|t| t.id == "2025/2026-Z" || t.id == "2026/2027-L")
            .collect();
        assert_eq!(pick(&user, "2026-09-01").as_deref(), Some("2025/2026-Z"));
        assert_eq!(pick(&user, "2027-02-15").as_deref(), Some("2026/2027-L"));
    }

    #[test]
    fn falls_back_to_most_recent_past() {
        let t = index();
        assert_eq!(pick(&t, "2030-01-01").as_deref(), Some("2026/2027-L"));
        assert_eq!(pick(&[], "2026-11-04"), None);
    }

    #[test]
    fn tie_on_span_prefers_latest_start() {
        let t = vec![
            mk("a", "2026-10-01", "2027-01-31", None),
            mk("b", "2026-10-15", "2027-02-14", None),
        ];
        assert_eq!(pick(&t, "2026-11-04").as_deref(), Some("b"));
    }

    #[test]
    fn roundtrips_through_json() {
        let terms = index();
        let json = serde_json::to_string(&terms).unwrap();
        let back: Vec<Term> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, terms);
        let reduced = mk("a", "2026-10-01", "2027-01-31", None);
        let json = serde_json::to_string(&reduced).unwrap();
        assert_eq!(serde_json::from_str::<Term>(&json).unwrap(), reduced);
    }

    #[test]
    fn parses_reduced_user_shape() {
        let t: Vec<Term> = serde_json::from_str(
            r#"[{"end_date":"2027-02-28","id":"2026/2027-Z","name":{"en":"Winter semester 2026/2027","pl":"Semestr zimowy 2026/2027"},"order_key":95,"start_date":"2026-10-01"}]"#,
        )
        .unwrap();
        assert_eq!(t[0].finish_date, None);
        assert_eq!(t[0].is_active, None);
    }

    fn mk(id: &str, start: &str, end: &str, finish: Option<&str>) -> Term {
        Term {
            id: id.into(),
            name: LangDict::default(),
            start_date: d(start),
            end_date: d(end),
            finish_date: finish.map(d),
            order_key: None,
            is_active: None,
        }
    }

    #[test]
    fn day_before_start_and_after_end() {
        let t = vec![mk("a", "2026-10-01", "2027-01-31", None)];
        // Before the start: upcoming within the window, otherwise nothing.
        assert_eq!(pick(&t, "2026-09-30").as_deref(), Some("a"));
        assert_eq!(pick(&t, "2026-08-31"), None);
        // After the end: still the most recently ended term.
        assert_eq!(pick(&t, "2027-02-01").as_deref(), Some("a"));
    }

    #[test]
    fn exam_session_keeps_old_term() {
        let t = vec![
            mk("old", "2026-10-01", "2027-01-31", Some("2027-02-20")),
            mk("new", "2027-03-01", "2027-06-30", Some("2027-07-15")),
        ];
        // 2027-03-01 is within 30 days of 2027-02-05, but the exam session is still on.
        assert_eq!(pick(&t, "2027-02-05").as_deref(), Some("old"));
        assert_eq!(pick(&t, "2027-02-20").as_deref(), Some("old"));
        // Once the session is over the upcoming term wins.
        assert_eq!(pick(&t, "2027-02-21").as_deref(), Some("new"));
    }

    #[test]
    fn upcoming_beats_ended_term() {
        let t = vec![
            mk("old", "2026-02-01", "2026-06-30", Some("2026-07-15")),
            mk("new", "2026-10-01", "2027-01-31", None),
        ];
        assert_eq!(pick(&t, "2026-09-15").as_deref(), Some("new"));
        // Outside the window the ended term is the answer.
        assert_eq!(pick(&t, "2026-08-15").as_deref(), Some("old"));
    }

    #[test]
    fn recent_tie_prefers_semester() {
        let t = vec![
            mk("year", "2025-10-01", "2026-09-30", None),
            mk("sem", "2026-03-01", "2026-09-30", None),
        ];
        assert_eq!(pick(&t, "2026-12-01").as_deref(), Some("sem"));
    }

    #[test]
    fn explicit_null_fields_parse() {
        let t: Vec<Term> = serde_json::from_str(
            r#"[{"id":"x","name":null,"start_date":"2026-10-01","end_date":"2027-01-31",
                 "finish_date":null,"order_key":null,"is_active":null}]"#,
        )
        .unwrap();
        assert_eq!(t[0].finish_date, None);
        assert_eq!(t[0].order_key, None);
        assert_eq!(t[0].is_active, None);
        assert_eq!(t[0].name, LangDict::default());
    }

    #[test]
    fn lenient_list_skips_bad_terms() {
        let v: serde_json::Value = serde_json::from_str(
            r#"[{"id":"ok","start_date":"2026-10-01","end_date":"2027-01-31"},
                {"id":"no_end","start_date":"2026-10-01"},
                {"id":"null_start","start_date":null,"end_date":"2027-01-31"},
                {"id":"bad","start_date":"garbage","end_date":"2027-01-31"}]"#,
        )
        .unwrap();
        let t = parse_terms(v).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].id, "ok");
        assert!(parse_terms(serde_json::json!({"a": 1})).is_err());
    }

    #[tokio::test]
    async fn empty_ids_make_no_request() {
        // The host does not resolve; any request would fail.
        let c = Client::new("https://usos.invalid", "k", "s").unwrap();
        assert!(c.terms(&[]).await.unwrap().is_empty());
    }
}
