//! The timetable cache: activities by date range plus per-day fetch times.

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{TransactionBehavior, params};
use usos_core::Activity;

use crate::error::{Result, StoreError};
use crate::store::{Store, ensure_active, parse_ts, ts};

const DATE: &str = "%Y-%m-%d";
const DATETIME: &str = "%Y-%m-%dT%H:%M:%S";

/// What [`Store::replace_activities`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceOutcome {
    /// Rows inserted (activities starting inside the range).
    pub stored: u32,
    /// Rows deleted from the range.
    pub removed: u32,
    /// Whether the range's contents differ from before. `true` the first time.
    pub changed: bool,
}

/// How much of a date range was ever fetched, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityCoverage {
    /// Days in the range.
    pub days_total: u32,
    /// Days of the range that were fetched at some point (empty days count).
    pub days_cached: u32,
    /// The fetch time of the stalest cached day; `None` when no day is cached.
    pub oldest_fetched_at: Option<DateTime<Utc>>,
}

/// Whether `activity` belongs to the range `from..=to`: it starts on a day inside it. This is
/// the one predicate for what the store keeps and what live results show.
pub(crate) fn starts_in(activity: &Activity, from: NaiveDate, to: NaiveDate) -> bool {
    (from..=to).contains(&activity.date())
}

fn check_range(from: NaiveDate, to: NaiveDate) -> Result<()> {
    if from > to {
        return Err(StoreError::InvalidRange { from, to });
    }
    Ok(())
}

impl Store {
    /// Replace the cached activities of `account` that start on a day in `from..=to` (both
    /// inclusive) with `activities`, atomically, and mark each of those days as fetched now.
    /// Activities that start outside the range are ignored (they belong to another fetch).
    pub fn replace_activities(
        &self,
        account: &str,
        from: NaiveDate,
        to: NaiveDate,
        activities: &[Activity],
    ) -> Result<ReplaceOutcome> {
        self.replace(account, from, to, activities, false)
    }

    /// [`Store::replace_activities`], only while `account` is active.
    pub(crate) fn replace_activities_if_active(
        &self,
        account: &str,
        from: NaiveDate,
        to: NaiveDate,
        activities: &[Activity],
    ) -> Result<ReplaceOutcome> {
        self.replace(account, from, to, activities, true)
    }

    fn replace(
        &self,
        account: &str,
        from: NaiveDate,
        to: NaiveDate,
        activities: &[Activity],
        only_if_active: bool,
    ) -> Result<ReplaceOutcome> {
        check_range(from, to)?;
        let now = ts(Utc::now());
        let (from_s, to_s) = (from.format(DATE).to_string(), to.format(DATE).to_string());

        // (starts_at, ends_at, date, course_id, kind, json), sorted so that it compares with
        // what the database returns for the same query.
        let mut fresh = Vec::with_capacity(activities.len());
        for a in activities
            .iter()
            .filter(|a| (from..=to).contains(&a.date()))
        {
            fresh.push((
                a.start.format(DATETIME).to_string(),
                a.end.format(DATETIME).to_string(),
                a.date().format(DATE).to_string(),
                a.course_id.as_deref(),
                a.kind.as_str(),
                serde_json::to_string(a)?,
            ));
        }
        fresh.sort_by(|x, y| (&x.0, &x.5).cmp(&(&y.0, &y.5)));

        let mut conn = self.lock();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if only_if_active {
            ensure_active(&tx, account)?;
        }
        let old: Vec<String> = tx
            .prepare_cached(
                "SELECT json FROM activities WHERE account = ?1 AND date BETWEEN ?2 AND ?3
                 ORDER BY starts_at, json",
            )?
            .query_map(params![account, from_s, to_s], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let changed = old.len() != fresh.len() || old.iter().zip(&fresh).any(|(o, f)| *o != f.5);
        tx.execute(
            "DELETE FROM activities WHERE account = ?1 AND date BETWEEN ?2 AND ?3",
            params![account, from_s, to_s],
        )?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO activities (account, starts_at, ends_at, date, course_id, kind, json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for f in &fresh {
                insert.execute(params![account, f.0, f.1, f.2, f.3, f.4, f.5])?;
            }
            let mut mark = tx.prepare_cached(
                "INSERT INTO activity_days (account, date, fetched_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (account, date) DO UPDATE SET fetched_at = excluded.fetched_at",
            )?;
            for day in from.iter_days().take_while(|d| *d <= to) {
                mark.execute(params![account, day.format(DATE).to_string(), now])?;
            }
        }
        tx.commit()?;
        Ok(ReplaceOutcome {
            stored: fresh.len() as u32,
            removed: old.len() as u32,
            changed,
        })
    }

    /// The cached activities of `account` that start on a day in `from..=to` (both inclusive),
    /// sorted by start time. Rows that no longer deserialize (written by an incompatible older
    /// version) are skipped.
    pub fn activities(
        &self,
        account: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<Activity>> {
        check_range(from, to)?;
        let conn = self.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT json FROM activities WHERE account = ?1 AND date BETWEEN ?2 AND ?3
             ORDER BY starts_at, id",
        )?;
        let rows = stmt.query_map(
            params![
                account,
                from.format(DATE).to_string(),
                to.format(DATE).to_string()
            ],
            |r| r.get::<_, String>(0),
        )?;
        let mut out = Vec::new();
        for json in rows {
            if let Ok(a) = serde_json::from_str(&json?) {
                out.push(a);
            }
        }
        Ok(out)
    }

    /// When the days of `from..=to` were last fetched; see [`ActivityCoverage`].
    pub fn activity_coverage(
        &self,
        account: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<ActivityCoverage> {
        check_range(from, to)?;
        let (cached, oldest): (u32, Option<String>) = self.lock().query_row(
            "SELECT COUNT(*), MIN(fetched_at) FROM activity_days
             WHERE account = ?1 AND date BETWEEN ?2 AND ?3",
            params![
                account,
                from.format(DATE).to_string(),
                to.format(DATE).to_string()
            ],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(ActivityCoverage {
            days_total: ((to - from).num_days() + 1) as u32,
            days_cached: cached,
            oldest_fetched_at: oldest.as_deref().map(parse_ts).transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usos_core::timetable::parse_activities;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn acts(items: &[(&str, &str)]) -> Vec<Activity> {
        // (start, course name)
        let json: Vec<_> = items
            .iter()
            .map(|(start, name)| {
                let end = format!("{} 23:30:00", &start[..10]);
                serde_json::json!({"type":"classgroup","start_time":start,"end_time":end,
                    "course_id":"C1","course_name":{"en":name,"pl":name},"classtype_id":"CW"})
            })
            .collect();
        parse_activities(serde_json::Value::Array(json)).unwrap()
    }

    fn names(list: &[Activity]) -> Vec<String> {
        list.iter()
            .map(|a| a.course_name.en.clone().unwrap())
            .collect()
    }

    #[test]
    fn replaces_only_the_range_with_inclusive_boundaries() {
        let s = Store::open_in_memory().unwrap();
        let all = acts(&[
            ("2026-11-01 08:00:00", "before"),
            ("2026-11-02 08:00:00", "first-day"),
            ("2026-11-04 23:00:00", "last-day-late"),
            ("2026-11-05 00:00:00", "after"),
        ]);
        let out = s
            .replace_activities("a", d("2026-11-01"), d("2026-11-05"), &all)
            .unwrap();
        assert_eq!((out.stored, out.removed, out.changed), (4, 0, true));

        // Replace 11-02..=11-04: boundaries are replaced, outside rows stay.
        let fresh = acts(&[
            ("2026-11-04 09:00:00", "new-b"),
            ("2026-11-02 09:00:00", "new-a"),
            ("2026-11-06 09:00:00", "ignored-outside-range"),
        ]);
        let out = s
            .replace_activities("a", d("2026-11-02"), d("2026-11-04"), &fresh)
            .unwrap();
        assert_eq!((out.stored, out.removed, out.changed), (2, 2, true));
        let got = s.activities("a", d("2026-10-01"), d("2026-12-01")).unwrap();
        assert_eq!(names(&got), ["before", "new-a", "new-b", "after"]);

        // Same content again: unchanged.
        let out = s
            .replace_activities("a", d("2026-11-02"), d("2026-11-04"), &fresh)
            .unwrap();
        assert!(!out.changed);
        // Empty replacement clears the range and reports a change.
        let out = s
            .replace_activities("a", d("2026-11-02"), d("2026-11-04"), &[])
            .unwrap();
        assert_eq!((out.stored, out.removed, out.changed), (0, 2, true));
        let got = s.activities("a", d("2026-11-01"), d("2026-11-05")).unwrap();
        assert_eq!(names(&got), ["before", "after"]);
    }

    #[test]
    fn query_boundaries_are_inclusive_and_sorted() {
        let s = Store::open_in_memory().unwrap();
        let all = acts(&[
            ("2026-11-03 10:00:00", "c"),
            ("2026-11-02 12:00:00", "b"),
            ("2026-11-02 08:00:00", "a"),
            ("2026-11-04 08:00:00", "d"),
        ]);
        s.replace_activities("x", d("2026-11-01"), d("2026-11-30"), &all)
            .unwrap();
        let got = s.activities("x", d("2026-11-02"), d("2026-11-03")).unwrap();
        assert_eq!(names(&got), ["a", "b", "c"]);
        assert_eq!(
            got[0],
            all.iter()
                .find(|a| a.course_name.en.as_deref() == Some("a"))
                .unwrap()
                .clone()
        );
        assert!(
            s.activities("x", d("2026-11-05"), d("2026-11-09"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn accounts_are_isolated() {
        let s = Store::open_in_memory().unwrap();
        let (r1, r2) = (d("2026-11-01"), d("2026-11-30"));
        s.replace_activities("a", r1, r2, &acts(&[("2026-11-02 08:00:00", "mine")]))
            .unwrap();
        s.replace_activities("b", r1, r2, &acts(&[("2026-11-02 08:00:00", "theirs")]))
            .unwrap();
        // Replacing b's range does not touch a.
        s.replace_activities("b", r1, r2, &[]).unwrap();
        assert_eq!(names(&s.activities("a", r1, r2).unwrap()), ["mine"]);
        assert!(s.activities("b", r1, r2).unwrap().is_empty());
        assert_eq!(s.activity_coverage("c", r1, r2).unwrap().days_cached, 0);
    }

    #[test]
    fn coverage_tracks_fetched_days() {
        let s = Store::open_in_memory().unwrap();
        let none = s
            .activity_coverage("a", d("2026-11-01"), d("2026-11-07"))
            .unwrap();
        assert_eq!((none.days_total, none.days_cached), (7, 0));
        assert!(none.oldest_fetched_at.is_none());
        // An empty fetch still marks its days as fetched.
        s.replace_activities("a", d("2026-11-03"), d("2026-11-05"), &[])
            .unwrap();
        let cov = s
            .activity_coverage("a", d("2026-11-01"), d("2026-11-07"))
            .unwrap();
        assert_eq!((cov.days_total, cov.days_cached), (7, 3));
        assert!(cov.oldest_fetched_at.unwrap() <= Utc::now());
    }

    #[test]
    fn rejects_backwards_ranges() {
        let s = Store::open_in_memory().unwrap();
        assert!(matches!(
            s.replace_activities("a", d("2026-11-02"), d("2026-11-01"), &[]),
            Err(StoreError::InvalidRange { .. })
        ));
        assert!(s.activities("a", d("2026-11-02"), d("2026-11-01")).is_err());
    }
}
