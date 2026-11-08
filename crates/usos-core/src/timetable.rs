//! The user's timetable (`services/tt/user`).
//!
//! `tt/user` is the right method for a student: it joins `tt/student` and `tt/staff`, so it also
//! works for a user who is both. It accepts at most 7 days per call, so [`Client::timetable_range`]
//! splits a longer range into several requests.
//!
//! Times are installation-local (USOS sends no UTC offset), hence [`NaiveDateTime`].

use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::lenient::{lenient, lenient_or_default, opt_string, string_list};
use crate::models::LangDict;

/// The most days `tt/user` returns per call.
pub const MAX_DAYS_PER_REQUEST: u8 = 7;
/// Longest range [`Client::timetable_range`] accepts (a sanity cap on the number of requests).
pub const MAX_RANGE_DAYS: u64 = 400;

/// Fields requested from `tt/user`. Every one exists at PolSl (an unknown field is a 400/500).
/// `slot_number` and `related_user_ids` (exam / meeting extras) are rejected there, so they are left out.
const FIELDS: &str = "type|start_time|end_time|name|url|course_id|course_name|classtype_id|\
classtype_name|group_number|building_id|building_name|room_id|room_number|lecturer_ids|unit_id|\
frequency|classgroup_profile_url";

const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// The `type` of an activity. Unknown values are kept, as the API says new ones may appear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActivityKind {
    /// Regular class generated from a class group schedule.
    #[default]
    Classgroup,
    /// A confirmed single meeting of a class group.
    Classgroup2,
    Meeting,
    Exam,
    Other(String),
}

impl ActivityKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Classgroup => "classgroup",
            Self::Classgroup2 => "classgroup2",
            Self::Meeting => "meeting",
            Self::Exam => "exam",
            Self::Other(s) => s,
        }
    }
}

impl From<&str> for ActivityKind {
    fn from(s: &str) -> Self {
        match s {
            "classgroup" => Self::Classgroup,
            "classgroup2" => Self::Classgroup2,
            "meeting" => Self::Meeting,
            "exam" => Self::Exam,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// How often a class group meets (the `frequency` field of `tt/user`).
///
/// Serialized as the raw USOS code, so cached JSON is the same as before this type existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frequency {
    /// `every_week`
    EveryWeek,
    /// `every_fortnight`
    EveryFortnight,
    /// `every_fortnight_odd`
    FortnightOdd,
    /// `every_fortnight_even`
    FortnightEven,
    /// `every_month`
    EveryMonth,
    /// `every_working_day`
    EveryWorkingDay,
    /// `every_weekend`
    EveryWeekend,
    /// `every_day`
    EveryDay,
    /// `once`: no specific date can be derived.
    Once,
    /// `other`: irregular; no specific date can be derived.
    Irregular,
    /// Any code not listed above (USOS may add codes).
    Other(String),
}

impl Frequency {
    /// The raw USOS code.
    pub fn as_str(&self) -> &str {
        match self {
            Self::EveryWeek => "every_week",
            Self::EveryFortnight => "every_fortnight",
            Self::FortnightOdd => "every_fortnight_odd",
            Self::FortnightEven => "every_fortnight_even",
            Self::EveryMonth => "every_month",
            Self::EveryWorkingDay => "every_working_day",
            Self::EveryWeekend => "every_weekend",
            Self::EveryDay => "every_day",
            Self::Once => "once",
            Self::Irregular => "other",
            Self::Other(s) => s,
        }
    }

    /// Parses a raw USOS code; unknown codes become [`Frequency::Other`].
    pub fn from_raw(s: &str) -> Self {
        match s {
            "every_week" => Self::EveryWeek,
            "every_fortnight" => Self::EveryFortnight,
            "every_fortnight_odd" => Self::FortnightOdd,
            "every_fortnight_even" => Self::FortnightEven,
            "every_month" => Self::EveryMonth,
            "every_working_day" => Self::EveryWorkingDay,
            "every_weekend" => Self::EveryWeekend,
            "every_day" => Self::EveryDay,
            "once" => Self::Once,
            "other" => Self::Irregular,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl Serialize for Frequency {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Frequency {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Ok(Self::from_raw(&String::deserialize(d)?))
    }
}

impl Serialize for ActivityKind {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ActivityKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Ok(Option::<String>::deserialize(d)?
            .as_deref()
            .map(Self::from)
            .unwrap_or_default())
    }
}

/// One timetable entry. Optional fields are `None` when USOS omits them, sends `null` or an
/// empty string; unknown API keys are ignored (see `parse_activities`). Serializes with the field
/// names below and reads them back (the API shape is parsed by a private type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    pub kind: ActivityKind,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// Generic name, e.g. "Mathematics - lecture".
    #[serde(default)]
    pub name: LangDict,
    pub course_id: Option<String>,
    #[serde(default)]
    pub course_name: LangDict,
    /// Short class type code, e.g. `WYK`, `CW`, `LAB`.
    pub classtype_id: Option<String>,
    #[serde(default)]
    pub classtype_name: LangDict,
    pub group_number: Option<u32>,
    pub building_id: Option<String>,
    #[serde(default)]
    pub building_name: LangDict,
    pub room_id: Option<String>,
    pub room_number: Option<String>,
    /// Staff user ids (the API sends integers; kept as strings like every other id).
    #[serde(default)]
    pub lecturer_ids: Vec<String>,
    pub unit_id: Option<String>,
    /// How often the class meets; absent for `classgroup2`.
    ///
    /// Each `tt/user` entry is already one concrete dated occurrence, so "does this class meet in
    /// week X" is answered by looking for its activities in that week. USOS exposes no term or week
    /// parity field, so do not derive odd/even weeks from [`Frequency::FortnightOdd`] and friends.
    pub frequency: Option<Frequency>,
    pub url: Option<String>,
}

/// API shape of one `tt/user` entry (private; [`Activity`] is the public view).
#[derive(Deserialize)]
struct RawActivity {
    #[serde(default, rename = "type")]
    kind: ActivityKind,
    #[serde(rename = "start_time", deserialize_with = "datetime")]
    start: NaiveDateTime,
    #[serde(rename = "end_time", deserialize_with = "datetime")]
    end: NaiveDateTime,
    /// Generic name, e.g. "Mathematics - lecture".
    #[serde(default, deserialize_with = "lenient_or_default")]
    name: LangDict,
    #[serde(default, deserialize_with = "opt_string")]
    course_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    course_name: LangDict,
    /// Short class type code, e.g. `WYK`, `CW`, `LAB`.
    #[serde(default, deserialize_with = "opt_string")]
    classtype_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    classtype_name: LangDict,
    #[serde(default, deserialize_with = "lenient")]
    group_number: Option<u32>,
    #[serde(default, deserialize_with = "opt_string")]
    building_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    building_name: LangDict,
    #[serde(default, deserialize_with = "opt_string")]
    room_id: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    room_number: Option<String>,
    /// Staff user ids (the API sends integers; kept as strings like every other id).
    #[serde(default, deserialize_with = "string_list")]
    lecturer_ids: Vec<String>,
    #[serde(default, deserialize_with = "opt_string")]
    unit_id: Option<String>,
    /// Frequency code such as `every_week`; absent for `classgroup2`.
    #[serde(default, deserialize_with = "opt_string")]
    frequency: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    url: Option<String>,
}

impl From<RawActivity> for Activity {
    fn from(r: RawActivity) -> Self {
        Self {
            kind: r.kind,
            start: r.start,
            end: r.end,
            name: r.name,
            course_id: r.course_id,
            course_name: r.course_name,
            classtype_id: r.classtype_id,
            classtype_name: r.classtype_name,
            group_number: r.group_number,
            building_id: r.building_id,
            building_name: r.building_name,
            room_id: r.room_id,
            room_number: r.room_number,
            lecturer_ids: r.lecturer_ids,
            unit_id: r.unit_id,
            frequency: r.frequency.as_deref().map(Frequency::from_raw),
            url: r.url,
        }
    }
}

impl Activity {
    /// The day the activity starts on.
    pub fn date(&self) -> NaiveDate {
        self.start.date()
    }
}

fn datetime<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<NaiveDateTime, D::Error> {
    let s = String::deserialize(d)?;
    NaiveDateTime::parse_from_str(&s, DATETIME_FORMAT).map_err(serde::de::Error::custom)
}

/// Parse a `tt/user` response, skipping entries without usable start and end times.
pub fn parse_activities(value: Value) -> Result<Vec<Activity>> {
    let Value::Array(items) = value else {
        return Err(Error::UnexpectedResponse(
            "unexpected timetable shape (not an array)".to_owned(),
        ));
    };
    Ok(items
        .into_iter()
        .filter_map(|item| serde_json::from_value::<RawActivity>(item).ok())
        .map(Activity::from)
        .collect())
}

/// Sort by start time, then end time, then course id (a stable, deterministic order).
fn sort_activities(list: &mut [Activity]) {
    list.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.end.cmp(&b.end))
            .then_with(|| a.course_id.cmp(&b.course_id))
    });
}

/// An inclusive range of calendar days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Monday and Sunday of the ISO week containing `day`.
/// Fails with [`Error::InvalidArgument`] at the edge of the supported date range.
pub fn week_of(day: NaiveDate) -> Result<DateRange> {
    let back = Days::new(u64::from(day.weekday().num_days_from_monday()));
    let monday = day
        .checked_sub_days(back)
        .ok_or_else(|| out_of_range(day))?;
    let sunday = monday
        .checked_add_days(Days::new(6))
        .ok_or_else(|| out_of_range(day))?;
    Ok(DateRange {
        from: monday,
        to: sunday,
    })
}

fn out_of_range(day: NaiveDate) -> Error {
    Error::InvalidArgument(format!("date {day} is too close to the supported limit"))
}

/// For each activity, whether it overlaps another one starting on the same day (input order).
/// Back-to-back activities (one ends exactly when the next starts) do not clash.
pub fn clashes(activities: &[Activity]) -> Vec<bool> {
    let mut days: BTreeMap<NaiveDate, Vec<usize>> = BTreeMap::new();
    for (i, a) in activities.iter().enumerate() {
        days.entry(a.date()).or_default().push(i);
    }
    let mut out = vec![false; activities.len()];
    for ids in days.values() {
        clashes_in_day(activities, ids, &mut out);
    }
    out
}

/// Sweep over the activities `ids` that start on one day: O(n log n), same predicate as the
/// pairwise test `a.start < b.end && b.start < a.end`.
fn clashes_in_day(activities: &[Activity], ids: &[usize], out: &mut [bool]) {
    // Activities that end before they start are malformed; they only clash with well-formed
    // activities that strictly contain the reversed interval, so they are handled apart.
    let (mut well, mut inverted): (Vec<usize>, Vec<usize>) = ids
        .iter()
        .partition(|&&i| activities[i].end >= activities[i].start);
    well.sort_by_key(|&i| activities[i].start);
    // Well-formed: a clash is an earlier start that ends after this start, a later start before
    // this end, or another non-empty activity with the same start (empty ones touch nothing at
    // their own start).
    let mut max_end: Option<NaiveDateTime> = None;
    let mut at = 0;
    while at < well.len() {
        let start = activities[well[at]].start;
        let stop = at + well[at..].partition_point(|&i| activities[i].start == start);
        let run = &well[at..stop];
        let next_start = well.get(stop).map(|&k| activities[k].start);
        let non_empty = run.iter().filter(|&&i| activities[i].end > start).count();
        for &i in run {
            let end = activities[i].end;
            out[i] = max_end.is_some_and(|e| e > start)
                || next_start.is_some_and(|n| n < end)
                || (end > start && non_empty > 1);
        }
        let run_end = run.iter().map(|&i| activities[i].end).max();
        max_end = max_end.max(run_end);
        at = stop;
    }
    if inverted.is_empty() {
        return;
    }
    // Inverted `a` (reversed interval lo..hi) clashes with well-formed `b` iff b.start < lo && hi < b.end.
    let lo = |i: usize| activities[i].end;
    let hi = |i: usize| activities[i].start;
    let mut prefix_max_end = Vec::with_capacity(well.len());
    for &i in &well {
        let e = activities[i].end;
        prefix_max_end.push(
            prefix_max_end
                .last()
                .map_or(e, |&m: &NaiveDateTime| m.max(e)),
        );
    }
    for &a in &inverted {
        let n = well.partition_point(|&b| activities[b].start < lo(a));
        out[a] = n > 0 && prefix_max_end[n - 1] > hi(a);
    }
    inverted.sort_by_key(|&a| lo(a));
    let mut suffix_min_hi = vec![None; inverted.len() + 1];
    for k in (0..inverted.len()).rev() {
        let h = hi(inverted[k]);
        suffix_min_hi[k] = Some(suffix_min_hi[k + 1].map_or(h, |m: NaiveDateTime| m.min(h)));
    }
    for &b in &well {
        let n = inverted.partition_point(|&a| lo(a) <= activities[b].start);
        if suffix_min_hi[n].is_some_and(|h| h < activities[b].end) {
            out[b] = true;
        }
    }
}

/// The activities starting on one day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayPlan {
    pub date: NaiveDate,
    pub activities: Vec<Activity>,
    /// Same length and order as `activities`: whether each one overlaps another that day.
    pub clashes: Vec<bool>,
}

/// Activities grouped by their start day, days in ascending order, days without activities omitted.
/// Within a day the input order is kept (sort first, e.g. with [`Client::timetable_range`]).
pub fn group_by_day(activities: &[Activity]) -> Vec<DayPlan> {
    let mut days: BTreeMap<NaiveDate, Vec<Activity>> = BTreeMap::new();
    for a in activities {
        days.entry(a.date()).or_default().push(a.clone());
    }
    days.into_iter()
        .map(|(date, activities)| DayPlan {
            date,
            clashes: clashes(&activities),
            activities,
        })
        .collect()
}

impl Client {
    /// `services/tt/user`: activities from `start` for `days` days (1..=7), sorted by start time.
    pub async fn user_timetable(&self, start: NaiveDate, days: u8) -> Result<Vec<Activity>> {
        if !(1..=MAX_DAYS_PER_REQUEST).contains(&days) {
            return Err(Error::InvalidArgument(format!(
                "days must be between 1 and {MAX_DAYS_PER_REQUEST}, got {days}"
            )));
        }
        let params = [
            ("start".to_owned(), start.format("%Y-%m-%d").to_string()),
            ("days".to_owned(), days.to_string()),
            ("fields".to_owned(), FIELDS.to_owned()),
        ];
        let value = self.call_authenticated("tt/user", &params).await?;
        let mut list = parse_activities(value)?;
        sort_activities(&mut list);
        Ok(list)
    }

    /// Activities for `from..=to` (both inclusive), fetched in chunks of at most 7 days and
    /// sorted by start time. Ranges longer than [`MAX_RANGE_DAYS`] are rejected.
    pub async fn timetable_range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Activity>> {
        let chunks = range_chunks(from, to)?;
        let mut all = Vec::new();
        for (start, days) in chunks {
            all.extend(self.user_timetable(start, days).await?);
        }
        sort_activities(&mut all);
        Ok(all)
    }
}

/// Split `from..=to` into `(start, days)` requests of at most 7 days.
fn range_chunks(from: NaiveDate, to: NaiveDate) -> Result<Vec<(NaiveDate, u8)>> {
    if to < from {
        return Err(Error::InvalidArgument(format!(
            "range end {to} is before its start {from}"
        )));
    }
    let total = (to - from).num_days() as u64 + 1;
    if total > MAX_RANGE_DAYS {
        return Err(Error::InvalidArgument(format!(
            "range of {total} days is too long (max {MAX_RANGE_DAYS})"
        )));
    }
    let step = u64::from(MAX_DAYS_PER_REQUEST);
    (0..total)
        .step_by(step as usize)
        .map(|offset| {
            let days = (total - offset).min(step) as u8;
            let start = from
                .checked_add_days(Days::new(offset))
                .ok_or_else(|| out_of_range(from))?;
            Ok((start, days))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Lang;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    /// Shapes copied from live `tt/user` responses (PolSl); ids and room data are not personal.
    fn live() -> Value {
        serde_json::json!([
            {"type":"classgroup","start_time":"2026-11-02 10:30:00","end_time":"2026-11-02 12:45:00",
             "name":{"en":"Mathematics - exercise classes","pl":"Matematyka - ćwiczenia"},
             "course_id":"SIAu>SI1-Mat-26","course_name":{"en":"Mathematics","pl":"Matematyka"},
             "classtype_id":"CW","classtype_name":{"en":"exercise classes","pl":"ćwiczenia"},
             "group_number":1,"building_id":"BU/0022",
             "building_name":{"en":"","pl":"Wydział Automatyki, Elektroniki i Informatyki"},
             "room_id":1459,"room_number":"101","lecturer_ids":[111111],"unit_id":152006,
             "frequency":"every_week","url":"https://example.invalid/u","new_future_key":[1,2]},
            {"type":"classgroup","start_time":"2026-11-02 08:00:00","end_time":"2026-11-02 10:15:00",
             "name":{"en":"Mathematics - lecture","pl":"Matematyka - wykład"},
             "course_id":"SIAu>SI1-Mat-26","course_name":{"en":"Mathematics","pl":"Matematyka"},
             "classtype_id":"WYK","classtype_name":{"en":"lecture","pl":"wykład"},
             "group_number":1,"building_id":"BU/0022",
             "building_name":{"en":"","pl":"Wydział Automatyki, Elektroniki i Informatyki"},
             "room_id":1458,"room_number":"aula c","lecturer_ids":[111111],"unit_id":152005,
             "frequency":"every_week"},
            {"type":"classgroup2","start_time":"2026-11-03 08:30:00","end_time":"2026-11-03 10:00:00",
             "name":{"en":"Physical Education - exercise classes","pl":"Wychowanie fizyczne - ćwiczenia"},
             "course_id":"PSWF>S1sem_1","classtype_id":"CW","group_number":60,
             "building_id":"BU/0122","building_name":{"en":"","pl":"Ośrodek Sportu"},
             "room_id":1222,"room_number":"Sala Judo","lecturer_ids":[222222],"unit_id":164246,
             "sm_id":62681304}
        ])
    }

    #[test]
    fn frequency_raw_round_trip() {
        let raws = [
            "every_week",
            "every_fortnight",
            "every_fortnight_odd",
            "every_fortnight_even",
            "every_month",
            "every_working_day",
            "every_weekend",
            "every_day",
            "once",
            "other",
        ];
        for raw in raws {
            let f = Frequency::from_raw(raw);
            assert!(!matches!(f, Frequency::Other(_)), "{raw}");
            assert_eq!(f.as_str(), raw);
            assert_eq!(serde_json::to_string(&f).unwrap(), format!("\"{raw}\""));
            assert_eq!(
                serde_json::from_str::<Frequency>(&format!("\"{raw}\"")).unwrap(),
                f
            );
        }
        let unknown = Frequency::from_raw("every_leap_day");
        assert_eq!(unknown, Frequency::Other("every_leap_day".into()));
        assert_eq!(unknown.as_str(), "every_leap_day");
        assert_eq!(
            serde_json::to_string(&unknown).unwrap(),
            "\"every_leap_day\""
        );
    }

    #[test]
    fn cached_activity_json_is_unchanged() {
        let old = serde_json::json!({
            "kind":"classgroup","start":"2026-11-02T08:00:00","end":"2026-11-02T10:15:00",
            "name":{"en":"","pl":""},"frequency":"every_fortnight_odd"
        });
        let a: Activity = serde_json::from_value(old).unwrap();
        assert_eq!(a.frequency, Some(Frequency::FortnightOdd));
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["frequency"], "every_fortnight_odd");
        let back: Activity = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }

    #[test]
    fn parses_live_shape() {
        let mut list = parse_activities(live()).unwrap();
        sort_activities(&mut list);
        assert_eq!(list.len(), 3);
        let a = &list[0];
        assert_eq!(a.kind, ActivityKind::Classgroup);
        assert_eq!(a.start.to_string(), "2026-11-02 08:00:00");
        assert_eq!(a.end.to_string(), "2026-11-02 10:15:00");
        assert_eq!(a.classtype_id.as_deref(), Some("WYK"));
        assert_eq!(a.course_name.get(Lang::En), Some("Mathematics"));
        assert_eq!(a.group_number, Some(1));
        assert_eq!(a.room_number.as_deref(), Some("aula c"));
        assert_eq!(a.room_id.as_deref(), Some("1458"));
        assert_eq!(a.lecturer_ids, ["111111"]);
        assert_eq!(a.frequency, Some(Frequency::EveryWeek));
        // English building name is empty at PolSl: falls back to Polish.
        assert_eq!(
            a.building_name.get(Lang::En),
            Some("Wydział Automatyki, Elektroniki i Informatyki")
        );
        let pe = &list[2];
        assert_eq!(pe.kind, ActivityKind::Classgroup2);
        assert_eq!(pe.frequency, None);
        assert_eq!(pe.course_name, LangDict::default());
    }

    #[test]
    fn lenient_with_nulls_and_odd_types() {
        let v = serde_json::json!([
            {"type":"exam","start_time":"2026-11-05 09:00:00","end_time":"2026-11-05 10:00:00",
             "name":null,"course_name":[],"room_number":"","room_id":null,"building_id":null,
             "lecturer_ids":null,"group_number":"x","url":null},
            {"start_time":"2026-11-05 11:00:00","end_time":"2026-11-05 12:00:00"},
            {"type":"hologram","start_time":"2026-11-05 13:00:00","end_time":"2026-11-05 14:00:00"},
            {"type":"classgroup","start_time":null,"end_time":"2026-11-05 14:00:00"},
            {"type":"classgroup","start_time":"garbage","end_time":"2026-11-05 14:00:00"},
            {"type":"classgroup","end_time":"2026-11-05 14:00:00"}
        ]);
        let list = parse_activities(v).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].kind, ActivityKind::Exam);
        assert_eq!(list[0].room_number, None);
        assert_eq!(list[0].group_number, None);
        assert!(list[0].lecturer_ids.is_empty());
        assert_eq!(list[0].name, LangDict::default());
        assert_eq!(list[1].kind, ActivityKind::Classgroup);
        assert_eq!(list[2].kind, ActivityKind::Other("hologram".into()));
        assert!(parse_activities(serde_json::json!({"a": 1})).is_err());
        assert!(parse_activities(serde_json::json!([])).unwrap().is_empty());
    }

    #[test]
    fn serializes_kind_as_api_string() {
        let list = parse_activities(live()).unwrap();
        let json = serde_json::to_value(&list[2]).unwrap();
        assert_eq!(json["kind"], "classgroup2");
        assert_eq!(json["start"], "2026-11-03T08:30:00");
    }

    #[test]
    fn groups_by_day_in_order() {
        let mut list = parse_activities(live()).unwrap();
        sort_activities(&mut list);
        let days = group_by_day(&list);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date, d("2026-11-02"));
        assert_eq!(days[0].activities.len(), 2);
        assert_eq!(days[0].clashes.len(), 2);
        assert_eq!(days[0].activities[0].classtype_id.as_deref(), Some("WYK"));
        assert_eq!(days[1].date, d("2026-11-03"));
        assert!(group_by_day(&[]).is_empty());
    }

    #[test]
    fn week_bounds() {
        // 2026-11-04 is a Wednesday.
        let w = |s: &str| {
            let r = week_of(d(s)).unwrap();
            (r.from, r.to)
        };
        assert_eq!(w("2026-11-04"), (d("2026-11-02"), d("2026-11-08")));
        assert_eq!(w("2026-11-02"), (d("2026-11-02"), d("2026-11-08")));
        assert_eq!(w("2026-11-08"), (d("2026-11-02"), d("2026-11-08")));
        // Across a month and year boundary.
        assert_eq!(w("2027-01-01"), (d("2026-12-28"), d("2027-01-03")));
        // Extreme dates give an error, not a panic.
        assert!(week_of(NaiveDate::MAX).is_err());
    }

    #[test]
    fn chunks_split_into_weeks() {
        let c = range_chunks(d("2026-11-02"), d("2026-11-08")).unwrap();
        assert_eq!(c, [(d("2026-11-02"), 7)]);
        let c = range_chunks(d("2026-11-02"), d("2026-11-09")).unwrap();
        assert_eq!(c, [(d("2026-11-02"), 7), (d("2026-11-09"), 1)]);
        let c = range_chunks(d("2026-11-02"), d("2026-11-16")).unwrap();
        assert_eq!(
            c,
            [
                (d("2026-11-02"), 7),
                (d("2026-11-09"), 7),
                (d("2026-11-16"), 1)
            ]
        );
        assert_eq!(
            range_chunks(d("2026-11-04"), d("2026-11-04")).unwrap(),
            [(d("2026-11-04"), 1)]
        );
        assert!(range_chunks(d("2026-11-05"), d("2026-11-04")).is_err());
        assert!(range_chunks(d("2026-01-01"), d("2028-01-01")).is_err());
    }

    #[test]
    fn chunks_across_year_boundary() {
        assert_eq!(
            range_chunks(d("2026-12-28"), d("2027-01-10")).unwrap(),
            [(d("2026-12-28"), 7), (d("2027-01-04"), 7)]
        );
    }

    #[test]
    fn range_length_bound() {
        let from = d("2026-01-01");
        let end = |n: u64| from + Days::new(n - 1);
        assert_eq!(range_chunks(from, end(400)).unwrap().len(), 58);
        assert!(matches!(
            range_chunks(from, end(401)),
            Err(Error::InvalidArgument(_))
        ));
        assert!(range_chunks(NaiveDate::MAX - Days::new(3), NaiveDate::MAX).is_ok());
    }

    fn act(start: &str, end: &str) -> Activity {
        parse_activities(serde_json::json!([
            {"type":"classgroup","start_time":start,"end_time":end}
        ]))
        .unwrap()
        .remove(0)
    }

    fn naive_clashes(activities: &[Activity]) -> Vec<bool> {
        activities
            .iter()
            .enumerate()
            .map(|(i, a)| {
                activities.iter().enumerate().any(|(j, b)| {
                    i != j && a.date() == b.date() && a.start < b.end && b.start < a.end
                })
            })
            .collect()
    }

    fn span(start: &str, end: &str) -> Activity {
        let mut a = parse_activities(serde_json::json!([{
            "type": "classgroup", "start_time": "2026-11-02 08:00:00", "end_time": "2026-11-02 09:00:00"
        }]))
        .unwrap()
        .remove(0);
        a.start = start.parse().unwrap();
        a.end = end.parse().unwrap();
        a
    }

    #[test]
    fn clashes_match_the_pairwise_definition() {
        let cases: Vec<Vec<(&str, &str)>> = vec![
            vec![
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T09:00:00", "2026-11-02T10:00:00"),
            ],
            vec![
                ("2026-11-02T08:00:00", "2026-11-02T12:00:00"),
                ("2026-11-02T09:00:00", "2026-11-02T10:00:00"),
            ],
            vec![
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
            ],
            vec![
                ("2026-11-02T08:00:00", "2026-11-02T08:00:00"),
                ("2026-11-02T08:00:00", "2026-11-02T08:00:00"),
            ],
            vec![
                ("2026-11-02T08:00:00", "2026-11-02T08:00:00"),
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
            ],
            vec![
                ("2026-11-02T08:30:00", "2026-11-02T08:30:00"),
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
            ],
            vec![
                ("2026-11-02T09:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T08:00:00", "2026-11-02T09:00:00"),
            ],
            // Spans midnight; compared only with activities starting the same day.
            vec![
                ("2026-11-02T22:00:00", "2026-11-03T02:00:00"),
                ("2026-11-03T01:00:00", "2026-11-03T03:00:00"),
                ("2026-11-02T23:00:00", "2026-11-03T00:30:00"),
            ],
            // Multi-day spans.
            vec![
                ("2026-11-02T08:00:00", "2026-11-04T08:00:00"),
                ("2026-11-02T20:00:00", "2026-11-05T00:00:00"),
                ("2026-11-03T08:00:00", "2026-11-03T09:00:00"),
            ],
            // Inverted (end before start).
            vec![
                ("2026-11-02T10:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T08:00:00", "2026-11-02T12:00:00"),
            ],
            vec![
                ("2026-11-02T10:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T09:00:00", "2026-11-02T12:00:00"),
            ],
            vec![
                ("2026-11-02T10:00:00", "2026-11-02T09:00:00"),
                ("2026-11-02T09:30:00", "2026-11-02T09:45:00"),
                ("2026-11-02T10:00:00", "2026-11-02T08:00:00"),
            ],
        ];
        for case in cases {
            let list: Vec<Activity> = case.iter().map(|(s, e)| span(s, e)).collect();
            assert_eq!(clashes(&list), naive_clashes(&list), "{case:?}");
        }
    }

    #[test]
    fn clashes_match_the_pairwise_definition_on_pseudo_random_input() {
        let mut state = 12345_u64;
        let mut next = move |m: u64| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) % m
        };
        let base: NaiveDateTime = "2026-11-02T00:00:00".parse().unwrap();
        for _ in 0..500 {
            let n = next(12) as usize;
            let list: Vec<Activity> = (0..n)
                .map(|_| {
                    let start = base + chrono::Duration::minutes(next(40) as i64 * 30);
                    let end = base + chrono::Duration::minutes(next(40) as i64 * 30);
                    span(
                        &start.to_string().replace(' ', "T"),
                        &end.to_string().replace(' ', "T"),
                    )
                })
                .collect();
            assert_eq!(clashes(&list), naive_clashes(&list), "{list:?}");
        }
    }

    #[test]
    fn detects_clashes() {
        let list = [
            act("2026-11-05 08:00:00", "2026-11-05 10:00:00"),
            act("2026-11-05 09:30:00", "2026-11-05 11:00:00"),
            act("2026-11-05 11:00:00", "2026-11-05 12:00:00"), // back-to-back: no clash
            act("2026-11-06 08:30:00", "2026-11-06 09:00:00"), // other day
        ];
        assert_eq!(clashes(&list), [true, true, false, false]);
        assert!(clashes(&[]).is_empty());
        let days = group_by_day(&list);
        assert_eq!(days[0].clashes, [true, true, false]);
        assert_eq!(days[1].clashes, [false]);
    }

    #[test]
    fn models_roundtrip_through_json() {
        let mut list = parse_activities(live()).unwrap();
        sort_activities(&mut list);
        let json = serde_json::to_string(&list).unwrap();
        let back: Vec<Activity> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, list);
        let days = group_by_day(&list);
        let json = serde_json::to_string(&days).unwrap();
        assert_eq!(serde_json::from_str::<Vec<DayPlan>>(&json).unwrap(), days);
        // Other kinds survive too.
        let other = act("2026-11-05 08:00:00", "2026-11-05 09:00:00");
        let other = Activity {
            kind: ActivityKind::Other("hologram".into()),
            ..other
        };
        let json = serde_json::to_string(&other).unwrap();
        assert_eq!(serde_json::from_str::<Activity>(&json).unwrap(), other);
    }

    #[test]
    fn overnight_activity_groups_under_start_day() {
        let list = [act("2026-11-05 22:00:00", "2026-11-06 01:00:00")];
        let days = group_by_day(&list);
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].date, d("2026-11-05"));
    }

    #[tokio::test]
    async fn invalid_days_make_no_request() {
        let c = Client::new("https://usos.invalid", "k", "s").unwrap();
        for days in [0, 8] {
            let err = c.user_timetable(d("2026-11-04"), days).await.unwrap_err();
            assert!(matches!(err, Error::InvalidArgument(_)));
        }
        let err = c.timetable_range(d("2026-11-05"), d("2026-11-04")).await;
        assert!(matches!(err, Err(Error::InvalidArgument(_))));
    }
}
