//! Whole-result snapshots (terms, user, grades and tests per term) with change detection.
//!
//! The typed wrappers are concrete on purpose: one small struct per kind
//! (`CachedTerms { terms, fetched_at }`, ...) instead of a generic `Cached<T>`, so the public
//! edge stays free of generics and can be wrapped by UniFFI. The generic code is private.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use usos_core::crstests::CourseTests;
use usos_core::{Term, TermGrades, User};

use crate::error::Result;
use crate::store::{Store, parse_ts, ts};

const TERMS: &str = "terms";
const USER: &str = "user";
const GRADES: &str = "grades";
const TESTS: &str = "crstests";

/// What a `put_*` call found. A future sync layer uses it to decide whether to notify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutOutcome {
    /// The new value differs from the stored one (always `true` the first time).
    pub changed: bool,
    /// When the previous value was fetched; `None` if there was none.
    pub previous_fetched_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedTerms {
    pub terms: Vec<Term>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedUser {
    pub user: User,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedTermGrades {
    pub grades: TermGrades,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedCourseTests {
    pub tests: CourseTests,
    pub fetched_at: DateTime<Utc>,
}

fn hash_hex(json: &str) -> String {
    Sha256::digest(json.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Store {
    /// Store `value` as the snapshot `(account, kind, key)`. The hash is that of the JSON text,
    /// which is deterministic because the models contain no unordered maps. `fetched_at` is
    /// refreshed even when nothing changed ("checked now").
    fn put_snapshot<T: Serialize>(
        &self,
        account: &str,
        kind: &str,
        key: &str,
        value: &T,
    ) -> Result<PutOutcome> {
        let json = serde_json::to_string(value)?;
        let hash = hash_hex(&json);
        let mut conn = self.lock();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<(String, String)> = tx
            .query_row(
                "SELECT hash, fetched_at FROM snapshots WHERE account = ?1 AND kind = ?2 AND key = ?3",
                params![account, kind, key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        tx.execute(
            "INSERT INTO snapshots (account, kind, key, json, hash, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (account, kind, key) DO UPDATE SET
                 json = excluded.json, hash = excluded.hash, fetched_at = excluded.fetched_at",
            params![account, kind, key, json, hash, ts(Utc::now())],
        )?;
        tx.commit()?;
        Ok(match previous {
            None => PutOutcome {
                changed: true,
                previous_fetched_at: None,
            },
            Some((old_hash, at)) => PutOutcome {
                changed: old_hash != hash,
                previous_fetched_at: Some(parse_ts(&at)?),
            },
        })
    }

    /// The snapshot, or `None` when missing or when it no longer deserializes (written by an
    /// incompatible older version; treated as a cache miss so the next fetch repairs it).
    fn get_snapshot<T: DeserializeOwned>(
        &self,
        account: &str,
        kind: &str,
        key: &str,
    ) -> Result<Option<(T, DateTime<Utc>)>> {
        let row: Option<(String, String)> = self
            .lock()
            .query_row(
                "SELECT json, fetched_at FROM snapshots WHERE account = ?1 AND kind = ?2 AND key = ?3",
                params![account, kind, key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((json, at)) = row else {
            return Ok(None);
        };
        let fetched_at = parse_ts(&at)?;
        Ok(serde_json::from_str(&json)
            .ok()
            .map(|value| (value, fetched_at)))
    }

    /// The user's terms.
    pub fn put_terms(&self, account: &str, terms: &[Term]) -> Result<PutOutcome> {
        self.put_snapshot(account, TERMS, "", &terms)
    }

    pub fn get_terms(&self, account: &str) -> Result<Option<CachedTerms>> {
        Ok(self
            .get_snapshot(account, TERMS, "")?
            .map(|(terms, fetched_at)| CachedTerms { terms, fetched_at }))
    }

    pub fn put_user(&self, account: &str, user: &User) -> Result<PutOutcome> {
        self.put_snapshot(account, USER, "", user)
    }

    pub fn get_user(&self, account: &str) -> Result<Option<CachedUser>> {
        Ok(self
            .get_snapshot(account, USER, "")?
            .map(|(user, fetched_at)| CachedUser { user, fetched_at }))
    }

    /// The grades of one term, keyed by `term_id`.
    pub fn put_term_grades(
        &self,
        account: &str,
        term_id: &str,
        grades: &TermGrades,
    ) -> Result<PutOutcome> {
        self.put_snapshot(account, GRADES, term_id, grades)
    }

    pub fn get_term_grades(
        &self,
        account: &str,
        term_id: &str,
    ) -> Result<Option<CachedTermGrades>> {
        Ok(self
            .get_snapshot(account, GRADES, term_id)?
            .map(|(grades, fetched_at)| CachedTermGrades { grades, fetched_at }))
    }

    /// The course tests of one term, keyed by `term_id`.
    pub fn put_course_tests(
        &self,
        account: &str,
        term_id: &str,
        tests: &CourseTests,
    ) -> Result<PutOutcome> {
        self.put_snapshot(account, TESTS, term_id, tests)
    }

    pub fn get_course_tests(
        &self,
        account: &str,
        term_id: &str,
    ) -> Result<Option<CachedCourseTests>> {
        Ok(self
            .get_snapshot(account, TESTS, term_id)?
            .map(|(tests, fetched_at)| CachedCourseTests { tests, fetched_at }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usos_core::LangDict;
    use usos_core::crstests::{NodeKind, TestNode, TestRoot};
    use usos_core::grades::{CourseGrades, Grade};

    fn term(id: &str) -> Term {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": {"pl": "Semestr", "en": "Semester"},
            "start_date": "2026-10-01", "end_date": "2027-02-28"
        }))
        .unwrap()
    }

    fn grades(term_id: &str, course: &str) -> TermGrades {
        TermGrades {
            term_id: term_id.into(),
            courses: vec![CourseGrades {
                course_id: course.into(),
                name: LangDict {
                    pl: Some("Analiza".into()),
                    en: Some("Analysis".into()),
                },
                term_id: term_id.into(),
                ects: Some(6.0),
                grades: vec![Grade {
                    session: 1,
                    symbol: Some("4,5".into()),
                    counts_into_average: true,
                    ..Grade::default()
                }],
                units: Vec::new(),
            }],
            average: Some(4.5),
            weighted_average: Some(4.5),
        }
    }

    fn tests(name: &str) -> CourseTests {
        CourseTests {
            tests: vec![TestRoot {
                course_id: "C1".into(),
                course_name: LangDict::default(),
                term_id: "T1".into(),
                limited_to_groups: false,
                root: TestNode {
                    id: "1".into(),
                    kind: NodeKind::Folder,
                    name: LangDict {
                        pl: Some(name.into()),
                        en: None,
                    },
                    points: Some(7.5),
                    points_max: Some(10.0),
                    children: vec![TestNode::default()],
                    ..TestNode::default()
                },
            }],
            skipped: vec!["9".into()],
        }
    }

    #[test]
    fn change_detection() {
        let s = Store::open_in_memory().unwrap();
        let first = s.put_terms("a", &[term("T1")]).unwrap();
        assert_eq!(
            first,
            PutOutcome {
                changed: true,
                previous_fetched_at: None
            }
        );
        let t1 = s.get_terms("a").unwrap().unwrap().fetched_at;
        let same = s.put_terms("a", &[term("T1")]).unwrap();
        assert!(!same.changed);
        assert_eq!(same.previous_fetched_at, Some(t1));
        let diff = s.put_terms("a", &[term("T1"), term("T2")]).unwrap();
        assert!(diff.changed);
        assert!(diff.previous_fetched_at.is_some());
        assert_eq!(s.get_terms("a").unwrap().unwrap().terms.len(), 2);
        // fetched_at moves forward even without a change.
        assert!(s.get_terms("a").unwrap().unwrap().fetched_at >= t1);
    }

    #[test]
    fn round_trips_every_kind() {
        let s = Store::open_in_memory().unwrap();
        let user = User {
            id: "7".into(),
            first_name: "A".into(),
            last_name: "B".into(),
        };
        s.put_user("a", &user).unwrap();
        assert_eq!(s.get_user("a").unwrap().unwrap().user, user);

        let terms = vec![term("T1")];
        s.put_terms("a", &terms).unwrap();
        assert_eq!(s.get_terms("a").unwrap().unwrap().terms, terms);

        let g = grades("T1", "C1");
        s.put_term_grades("a", "T1", &g).unwrap();
        assert_eq!(s.get_term_grades("a", "T1").unwrap().unwrap().grades, g);
        assert!(s.get_term_grades("a", "T2").unwrap().is_none());

        let t = tests("Kolokwium");
        s.put_course_tests("a", "T1", &t).unwrap();
        assert_eq!(s.get_course_tests("a", "T1").unwrap().unwrap().tests, t);
        // Different term key, same kind: separate rows.
        assert!(
            !s.put_term_grades("a", "T2", &g)
                .unwrap()
                .previous_fetched_at
                .is_some()
        );
    }

    #[test]
    fn accounts_and_kinds_are_isolated() {
        let s = Store::open_in_memory().unwrap();
        s.put_terms("a", &[term("T1")]).unwrap();
        assert!(s.get_terms("b").unwrap().is_none());
        assert!(
            s.put_terms("b", &[term("T1")])
                .unwrap()
                .previous_fetched_at
                .is_none()
        );
        assert!(s.get_user("a").unwrap().is_none());
    }

    #[test]
    fn undecodable_snapshot_is_a_miss() {
        let s = Store::open_in_memory().unwrap();
        s.put_terms("a", &[term("T1")]).unwrap();
        s.lock()
            .execute("UPDATE snapshots SET json = '{\"not\":\"terms\"}'", [])
            .unwrap();
        assert!(s.get_terms("a").unwrap().is_none());
    }
}
