//! The user's grades (`services/grades/*`).
//!
//! - [`Client::term_grades`] uses `grades/terms2`: per term and course edition, the course-level
//!   grades (`course_grades`) and the grades of each course unit (`course_units_grades`, keyed by
//!   unit id). `terms2` knows neither course names nor class types, so one `courses/user` call
//!   supplies them (unit id -> class type, course id -> name).
//! - [`Client::latest_grades`] uses `grades/latest` (recently modified grades, newest first), with
//!   the course edition and unit inlined.
//! - [`average`] and [`weighted_average`] are pure helpers for the Polish scale. ECTS points per
//!   course come from `courses/user_ects_points`.
//!
//! Each exam report holds up to several sessions (1 = first attempt, 2 = retake, ...). Sessions
//! without a grade (`null` in the API) are dropped, so a course without grades has empty lists.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::lenient::{lenient, lenient_or_default, opt_datetime, opt_string};
use crate::models::LangDict;

/// Primary `grade` fields requested from `terms2` and `latest`. `private_comment` needs an
/// administrative consumer key, so it is left out.
const GRADE_FIELDS: &str = "value_symbol|passes|value_description|exam_id|exam_session_number|\
counts_into_average|comment|grade_type_id|date_modified|date_acquisition|modification_author";

/// Extra `latest` fields: the course edition and unit the grade belongs to.
const LATEST_EXTRA_FIELDS: &str =
    "course_edition[course_id|term_id|course_name]|unit[id|classtype_id]";

/// Course editions of the user with their groups (to name courses and classify units).
const COURSES_FIELDS: &str = "course_editions[course_id|course_name|term_id|\
user_groups[course_unit_id|class_type_id|class_type]]";

/// The highest value on the Polish scale (`5!` and `5,5`).
const MAX_SCALE_VALUE: f64 = 5.5;
/// The lowest value on the Polish scale (a fail).
const MIN_SCALE_VALUE: f64 = 2.0;

/// One grade for one exam session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Grade {
    /// Exam report id; sessions of the same report share it.
    pub exam_id: Option<String>,
    /// 1 for the first attempt, 2 for the first retake, and so on.
    pub session: u32,
    /// Symbol as shown to students: `4,5`, `5!`, `ZAL`, `NZAL`, ...
    pub symbol: Option<String>,
    pub description: LangDict,
    /// Whether the grade passes the exam report.
    pub passes: Option<bool>,
    /// Whether it counts into the average (absent in the response counts as `false`).
    pub counts_into_average: bool,
    /// Comment visible to the student.
    pub comment: Option<String>,
    pub grade_type_id: Option<String>,
    pub date_modified: Option<NaiveDateTime>,
    pub date_acquisition: Option<NaiveDateTime>,
    /// Id of the user who last edited the grade.
    pub modification_author: Option<String>,
}

impl Grade {
    /// The numeric value on the Polish scale (2 to 5.5), or `None` for non-numeric grades such as
    /// `ZAL` / `NZAL` or ECTS letters. Accepts comma decimals (`3,5`); `5!` is 5.5.
    pub fn numeric_value(&self) -> Option<f64> {
        let symbol = self.symbol.as_deref()?.trim();
        if symbol == "5!" {
            return Some(MAX_SCALE_VALUE);
        }
        let value: f64 = symbol.replace(',', ".").parse().ok()?;
        (value.is_finite() && (MIN_SCALE_VALUE..=MAX_SCALE_VALUE).contains(&value)).then_some(value)
    }
}

/// The grade a course counts with: course-level exam reports only. Per report (grades sharing an
/// `exam_id`; grades without one are each their own report) take the latest session; it counts when
/// it has `counts_into_average` and a numeric value. With several counting reports the highest wins.
/// `None` when no report counts.
pub fn course_grade(grades: &[Grade]) -> Option<f64> {
    let mut latest: BTreeMap<&str, &Grade> = BTreeMap::new();
    let mut considered: Vec<&Grade> = Vec::new();
    for g in grades {
        match g.exam_id.as_deref() {
            Some(id) => {
                let slot = latest.entry(id).or_insert(g);
                if g.session > slot.session {
                    *slot = g;
                }
            }
            None => considered.push(g),
        }
    }
    considered.extend(latest.into_values());
    considered
        .into_iter()
        .filter(|g| g.counts_into_average)
        .filter_map(|g| g.numeric_value())
        .max_by(f64::total_cmp)
}

/// The unweighted average of the courses' [`course_grade`]s: every course counts once. `None`
/// when no course has a counting grade.
pub fn average(courses: &[CourseGrades]) -> Option<f64> {
    let values: Vec<f64> = courses
        .iter()
        .filter_map(|c| course_grade(&c.grades))
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// The ECTS-weighted average of the courses' [`course_grade`]s. Courses without a counting grade,
/// or with a missing or non-positive `ects`, are skipped. `None` when nothing is left.
pub fn weighted_average(courses: &[CourseGrades]) -> Option<f64> {
    let (sum, weights) = courses
        .iter()
        .filter_map(|c| Some((course_grade(&c.grades)?, c.ects.filter(|w| *w > 0.0)?)))
        .fold((0.0, 0.0), |(s, w), (grade, ects)| {
            (s + grade * ects, w + ects)
        });
    (weights > 0.0).then(|| sum / weights)
}

/// Grades of one course unit (one class type, e.g. a lecture or a lab).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitGrades {
    pub unit_id: String,
    pub classtype_id: Option<String>,
    pub classtype_name: LangDict,
    /// Sorted by exam report, then session.
    pub grades: Vec<Grade>,
}

/// A course edition with its grades.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CourseGrades {
    pub course_id: String,
    /// Empty when the course is not among the user's course editions.
    pub name: LangDict,
    pub term_id: String,
    /// ECTS points of the course for the user; `None` when `user_ects_points` does not list it
    /// (or could not be fetched).
    pub ects: Option<f64>,
    /// Course-level (final) grades, sorted by exam report, then session.
    pub grades: Vec<Grade>,
    /// Per-unit grades, sorted by class type id, then unit id. Only units with a grade report.
    pub units: Vec<UnitGrades>,
}

impl CourseGrades {
    /// Whether any session has a grade.
    pub fn has_grades(&self) -> bool {
        !self.grades.is_empty() || self.units.iter().any(|u| !u.grades.is_empty())
    }
}

/// All course editions of one term that have a grade report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermGrades {
    pub term_id: String,
    /// Sorted by course id.
    pub courses: Vec<CourseGrades>,
    /// [`average`] of the term's courses (unweighted).
    pub average: Option<f64>,
    /// [`weighted_average`] of the term's courses.
    pub weighted_average: Option<f64>,
}

/// A grade from `grades/latest` with the course edition and unit it belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatestGrade {
    pub course_id: Option<String>,
    pub term_id: Option<String>,
    pub course_name: LangDict,
    pub unit_id: Option<String>,
    pub classtype_id: Option<String>,
    #[serde(flatten)]
    pub grade: Grade,
}

/// API shape of a grade (private; [`Grade`] is the public view).
#[derive(Deserialize)]
struct RawGrade {
    #[serde(default, deserialize_with = "opt_string")]
    value_symbol: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    value_description: LangDict,
    #[serde(default, deserialize_with = "lenient")]
    passes: Option<bool>,
    #[serde(default, deserialize_with = "opt_string")]
    exam_id: Option<String>,
    #[serde(default, deserialize_with = "opt_u32")]
    exam_session_number: Option<u32>,
    #[serde(default, deserialize_with = "lenient")]
    counts_into_average: Option<bool>,
    #[serde(default, deserialize_with = "opt_string")]
    comment: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    grade_type_id: Option<String>,
    #[serde(default, deserialize_with = "opt_datetime")]
    date_modified: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "opt_datetime")]
    date_acquisition: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "opt_string")]
    modification_author: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    course_edition: Option<RawEdition>,
    #[serde(default, deserialize_with = "lenient")]
    unit: Option<RawUnit>,
}

#[derive(Deserialize)]
struct RawEdition {
    #[serde(default, deserialize_with = "opt_string")]
    course_id: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    term_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    course_name: LangDict,
}

#[derive(Deserialize)]
struct RawUnit {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    classtype_id: Option<String>,
}

impl RawGrade {
    /// `fallback_session` is used when the grade does not carry its session number.
    fn into_grade(self, fallback_session: Option<u32>) -> Option<Grade> {
        Some(Grade {
            exam_id: self.exam_id,
            session: self.exam_session_number.or(fallback_session)?,
            symbol: self.value_symbol,
            description: self.value_description,
            passes: self.passes,
            counts_into_average: self.counts_into_average.unwrap_or(false),
            comment: self.comment,
            grade_type_id: self.grade_type_id,
            date_modified: self.date_modified,
            date_acquisition: self.date_acquisition,
            modification_author: self.modification_author,
        })
    }
}

fn opt_u32<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<u32>, D::Error> {
    Ok(opt_string(d)?.and_then(|s| s.trim().parse().ok()))
}

/// Exam ids compare numerically when both are numbers, else as strings; no id sorts first.
fn cmp_exam_ids(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => a.cmp(b),
        },
        _ => a.cmp(b),
    }
}

fn sort_grades(grades: &mut [Grade]) {
    grades.sort_by(|a, b| cmp_exam_ids(&a.exam_id, &b.exam_id).then(a.session.cmp(&b.session)));
}

/// One exam report: an object mapping the session number to a grade or `null`. `position` (the
/// report's index in its list) becomes the exam id of sessions that lack one, so they still group.
fn parse_exam(value: &Value, position: usize, out: &mut Vec<Grade>) {
    let Value::Object(sessions) = value else {
        return;
    };
    let start = out.len();
    for (key, v) in sessions {
        if !v.is_object() {
            continue;
        }
        if let Some(grade) = serde_json::from_value::<RawGrade>(v.clone())
            .ok()
            .and_then(|raw| raw.into_grade(key.parse().ok()))
        {
            out.push(grade);
        }
    }
    let exam_id = out[start..]
        .iter()
        .find_map(|g| g.exam_id.clone())
        .unwrap_or_else(|| format!("#{position}"));
    for g in &mut out[start..] {
        g.exam_id.get_or_insert_with(|| exam_id.clone());
    }
}

/// An array of exam reports (or a single one, which the docs of `terms2` describe).
fn parse_exams(value: &Value) -> Vec<Grade> {
    let mut out = Vec::new();
    match value {
        Value::Array(exams) => exams
            .iter()
            .enumerate()
            .for_each(|(i, e)| parse_exam(e, i, &mut out)),
        other => parse_exam(other, 0, &mut out),
    }
    sort_grades(&mut out);
    out
}

/// ECTS points from `courses/user_ects_points`: (term id, course id) -> points. Entries that are
/// `null` or not numbers are left out.
fn parse_ects(value: &Value) -> HashMap<(String, String), f64> {
    let mut out = HashMap::new();
    let Value::Object(by_term) = value else {
        return out;
    };
    for (term, courses) in by_term {
        for (course, points) in courses.as_object().into_iter().flatten() {
            let points = match points {
                Value::String(s) => s.trim().replace(',', ".").parse::<f64>().ok(),
                Value::Number(n) => n.as_f64(),
                _ => None,
            };
            if let Some(p) = points.filter(|p| p.is_finite()) {
                out.insert((term.clone(), course.clone()), p);
            }
        }
    }
    out
}

/// Course names and unit class types from `courses/user`.
#[derive(Default)]
struct Directory {
    courses: HashMap<String, LangDict>,
    /// Unit id -> (class type id, class type name).
    units: HashMap<String, (Option<String>, LangDict)>,
}

#[derive(Deserialize)]
struct RawDirEdition {
    #[serde(deserialize_with = "opt_string")]
    course_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    course_name: LangDict,
    #[serde(default, deserialize_with = "lenient_or_default")]
    user_groups: Vec<RawDirGroup>,
}

#[derive(Deserialize)]
struct RawDirGroup {
    #[serde(default, deserialize_with = "opt_string")]
    course_unit_id: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    class_type_id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    class_type: LangDict,
}

impl Directory {
    /// Parse a `courses/user` response; malformed editions are skipped.
    fn parse(value: &Value) -> Result<Self> {
        let Some(Value::Object(by_term)) = value.get("course_editions") else {
            return Err(Error::UnexpectedResponse(
                "courses/user response has no `course_editions`".to_owned(),
            ));
        };
        let mut dir = Self::default();
        for edition in by_term.values().filter_map(Value::as_array).flatten() {
            let Ok(raw) = serde_json::from_value::<RawDirEdition>(edition.clone()) else {
                continue;
            };
            if let Some(id) = raw.course_id {
                dir.courses.entry(id).or_insert(raw.course_name);
            }
            for g in raw.user_groups {
                if let Some(unit) = g.course_unit_id {
                    dir.units.insert(unit, (g.class_type_id, g.class_type));
                }
            }
        }
        Ok(dir)
    }
}

/// Parse a `terms2` response for `term_ids` (in that order). Unknown terms (`null`) and terms
/// without courses give an empty `courses` list; malformed courses are skipped.
fn parse_term_grades(
    value: &Value,
    term_ids: &[String],
    dir: &Directory,
    ects: &HashMap<(String, String), f64>,
) -> Result<Vec<TermGrades>> {
    let Value::Object(by_term) = value else {
        return Err(Error::UnexpectedResponse(
            "unexpected grades shape (not an object)".to_owned(),
        ));
    };
    Ok(term_ids
        .iter()
        .map(|term_id| {
            let mut courses: Vec<CourseGrades> = by_term
                .get(term_id.as_str())
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(course_id, edition)| parse_course(term_id, course_id, edition, dir, ects))
                .collect();
            courses.sort_by(|a, b| a.course_id.cmp(&b.course_id));
            TermGrades {
                term_id: term_id.to_owned(),
                average: average(&courses),
                weighted_average: weighted_average(&courses),
                courses,
            }
        })
        .collect())
}

fn parse_course(
    term_id: &str,
    course_id: &str,
    edition: &Value,
    dir: &Directory,
    ects: &HashMap<(String, String), f64>,
) -> CourseGrades {
    let grades = edition
        .get("course_grades")
        .map(parse_exams)
        .unwrap_or_default();
    let mut units: Vec<UnitGrades> = edition
        .get("course_units_grades")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(unit_id, exams)| {
            let (classtype_id, classtype_name) =
                dir.units.get(unit_id).cloned().unwrap_or_default();
            UnitGrades {
                unit_id: unit_id.clone(),
                classtype_id,
                classtype_name,
                grades: parse_exams(exams),
            }
        })
        .collect();
    units.sort_by(|a, b| {
        a.classtype_id
            .cmp(&b.classtype_id)
            .then(a.unit_id.cmp(&b.unit_id))
    });
    CourseGrades {
        course_id: course_id.to_owned(),
        name: dir.courses.get(course_id).cloned().unwrap_or_default(),
        term_id: term_id.to_owned(),
        ects: ects
            .get(&(term_id.to_owned(), course_id.to_owned()))
            .copied(),
        grades,
        units,
    }
}

/// Parse a `grades/latest` response (newest first). Grades without a session number are skipped.
fn parse_latest(value: Value) -> Result<Vec<LatestGrade>> {
    let Value::Array(items) = value else {
        return Err(Error::UnexpectedResponse(
            "unexpected latest grades shape (not an array)".to_owned(),
        ));
    };
    let mut out: Vec<LatestGrade> = items
        .into_iter()
        .filter_map(|item| serde_json::from_value::<RawGrade>(item).ok())
        .filter_map(|mut raw| {
            let edition = raw.course_edition.take();
            let unit = raw.unit.take();
            let grade = raw.into_grade(None)?;
            let (course_id, term_id, course_name) = match edition {
                Some(e) => (e.course_id, e.term_id, e.course_name),
                None => (None, None, LangDict::default()),
            };
            let (unit_id, classtype_id) = unit.map_or((None, None), |u| (u.id, u.classtype_id));
            Some(LatestGrade {
                course_id,
                term_id,
                course_name,
                unit_id,
                classtype_id,
                grade,
            })
        })
        .collect();
    out.sort_by_key(|l| std::cmp::Reverse(l.grade.date_modified));
    Ok(out)
}

/// The ECTS map of a `user_ects_points` response. A failure of the API itself leaves it empty
/// (the points are optional), but a network failure propagates, so that a flaky connection cannot
/// pass off grades without weights as a complete result (callers cache them).
fn ects_or_empty(response: Result<Value>) -> Result<HashMap<(String, String), f64>> {
    match response {
        Ok(v) => Ok(parse_ects(&v)),
        Err(e @ Error::Http(_)) => Err(e),
        Err(_) => Ok(HashMap::new()),
    }
}

impl Client {
    /// `services/grades/terms2` for `term_ids`, in the given order, with course names and class
    /// types from `courses/user` and ECTS points from `courses/user_ects_points` (best effort). An unknown term, or one without grade reports, has no courses.
    pub async fn term_grades(&self, term_ids: &[String]) -> Result<Vec<TermGrades>> {
        if term_ids.is_empty() {
            return Ok(Vec::new());
        }
        let courses = self
            .call_authenticated(
                "courses/user",
                &[("fields".to_owned(), COURSES_FIELDS.to_owned())],
            )
            .await?;
        let dir = Directory::parse(&courses)?;
        let params = [
            ("term_ids".to_owned(), term_ids.join("|")),
            ("fields".to_owned(), GRADE_FIELDS.to_owned()),
        ];
        let value = self.call_authenticated("grades/terms2", &params).await?;
        // ECTS points are optional: without them the weighted average is just absent.
        let ects = ects_or_empty(
            self.call_authenticated("courses/user_ects_points", &[])
                .await,
        )?;
        parse_term_grades(&value, term_ids, &dir, &ects)
    }

    /// `services/grades/latest`: grades modified in the last `days` days (at least 1), newest first.
    pub async fn latest_grades(&self, days: u32) -> Result<Vec<LatestGrade>> {
        if days == 0 {
            return Err(Error::InvalidArgument("days must be at least 1".to_owned()));
        }
        let params = [
            ("days".to_owned(), days.to_string()),
            (
                "fields".to_owned(),
                format!("{GRADE_FIELDS}|{LATEST_EXTRA_FIELDS}"),
            ),
        ];
        parse_latest(self.call_authenticated("grades/latest", &params).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ects_failures() {
        // A network error propagates; any other error leaves the weights out.
        let network = reqwest::Client::new().get("not a url").build().unwrap_err();
        assert!(matches!(
            ects_or_empty(Err(Error::from(network))),
            Err(Error::Http(_))
        ));
        assert!(ects_or_empty(Err(Error::NotLoggedIn)).unwrap().is_empty());
        let ok = ects_or_empty(Ok(json!({"2026/2027-Z": {"C1": "6.00"}}))).unwrap();
        assert_eq!(ok.get(&("2026/2027-Z".into(), "C1".into())), Some(&6.0));
    }

    fn grade(symbol: &str, exam: Option<&str>, session: u32, counts: bool) -> Grade {
        Grade {
            exam_id: exam.map(str::to_owned),
            session,
            symbol: Some(symbol.to_owned()),
            counts_into_average: counts,
            ..Grade::default()
        }
    }

    fn course(id: &str, ects: Option<f64>, grades: Vec<Grade>) -> CourseGrades {
        CourseGrades {
            course_id: id.to_owned(),
            name: LangDict::default(),
            term_id: "T".to_owned(),
            ects,
            grades,
            units: vec![],
        }
    }

    fn no_ects() -> HashMap<(String, String), f64> {
        HashMap::new()
    }

    #[test]
    fn numeric_values() {
        let v = |s: &str| grade(s, None, 1, true).numeric_value();
        assert_eq!(v("2"), Some(2.0));
        assert_eq!(v("3,5"), Some(3.5));
        assert_eq!(v("3.5"), Some(3.5));
        assert_eq!(v(" 4 "), Some(4.0));
        assert_eq!(v("5,5"), Some(5.5));
        assert_eq!(v("5!"), Some(5.5));
        for bad in ["ZAL", "NZAL", "A", "", "1", "6", "NaN", "inf"] {
            assert_eq!(v(bad), None, "{bad}");
        }
        assert_eq!(Grade::default().numeric_value(), None);
    }

    #[test]
    fn course_grade_rules() {
        let cg = |g: &[Grade]| course_grade(g);
        assert_eq!(cg(&[]), None);
        // Only numeric counting grades.
        assert_eq!(cg(&[grade("ZAL", Some("3"), 1, true)]), None);
        assert_eq!(cg(&[grade("2", Some("4"), 1, false)]), None);
        // Latest session per report; input order does not matter.
        let g = [
            grade("2", Some("7"), 1, true),
            grade("4", Some("7"), 2, true),
        ];
        assert_eq!(cg(&g), Some(4.0));
        assert_eq!(cg(&[g[1].clone(), g[0].clone()]), Some(4.0));
        // A later session that does not count removes the report.
        let g = [
            grade("2", Some("7"), 1, true),
            grade("ZAL", Some("7"), 2, false),
        ];
        assert_eq!(cg(&g), None);
        // Several reports: max of the per-report latest grades.
        let g = [
            grade("5", Some("1"), 1, true),
            grade("3", Some("2"), 1, true),
            grade("4", Some("2"), 2, true),
        ];
        assert_eq!(cg(&g), Some(5.0));
    }

    #[test]
    fn average_is_per_course() {
        let two_reports = course(
            "A",
            None,
            vec![
                grade("5", Some("1"), 1, true),
                grade("3", Some("2"), 1, true),
            ],
        );
        let one = course("B", None, vec![grade("3", Some("9"), 1, true)]);
        let none = course("C", None, vec![grade("ZAL", Some("8"), 1, true)]);
        // The course with two reports counts once (5), not twice.
        assert_eq!(average(&[two_reports, one, none]), Some(4.0));
        assert_eq!(average(&[]), None);
    }

    #[test]
    fn weighted_average_uses_ects() {
        let c = [
            course("A", Some(6.0), vec![grade("5", Some("1"), 1, true)]),
            course("B", Some(2.0), vec![grade("3", Some("2"), 1, true)]),
            // Skipped: zero weight, missing weight, no counting grade.
            course("PE", Some(0.0), vec![grade("2", Some("3"), 1, true)]),
            course("N", None, vec![grade("2", Some("4"), 1, true)]),
            course("Z", Some(4.0), vec![grade("ZAL", Some("5"), 1, true)]),
        ];
        assert_eq!(weighted_average(&c), Some(4.5));
        assert_eq!(average(&c), Some((5.0 + 3.0 + 2.0 + 2.0) / 4.0));
        assert_eq!(weighted_average(&c[2..4]), None);
        assert_eq!(weighted_average(&[]), None);
    }

    #[test]
    fn parses_ects_points() {
        let m = parse_ects(&json!({
            "T": {"A": "6.00", "B": null, "PE": "0.00", "N": 3, "bad": "x"},
            "U": null
        }));
        let get = |t: &str, c: &str| m.get(&(t.to_owned(), c.to_owned())).copied();
        assert_eq!(get("T", "A"), Some(6.0));
        assert_eq!(get("T", "PE"), Some(0.0));
        assert_eq!(get("T", "N"), Some(3.0));
        assert_eq!(get("T", "B"), None);
        assert_eq!(get("T", "bad"), None);
        assert!(parse_ects(&json!([])).is_empty());
    }

    #[test]
    fn missing_exam_id_gets_position_key_and_dedupes() {
        let v = json!({"t": {"c": {"course_grades": [
            {"1": {"value_symbol": "2", "counts_into_average": true},
             "2": {"value_symbol": "4", "counts_into_average": true}},
            {"1": {"value_symbol": "3", "counts_into_average": true}}
        ]}}});
        let t =
            parse_term_grades(&v, &["t".to_owned()], &Directory::default(), &no_ects()).unwrap();
        let g = &t[0].courses[0].grades;
        let ids: Vec<_> = g.iter().map(|g| g.exam_id.as_deref().unwrap()).collect();
        assert_eq!(ids, ["#0", "#0", "#1"]);
        // Report 0 counts with its second session (4); the course takes the max (4).
        assert_eq!(t[0].average, Some(4.0));
    }

    #[test]
    fn exam_ids_sort_numerically() {
        let mut g = vec![
            grade("3", Some("10"), 1, true),
            grade("3", Some("9"), 2, true),
            grade("3", Some("9"), 1, true),
            grade("3", Some("#0"), 1, true),
            grade("3", None, 1, true),
        ];
        sort_grades(&mut g);
        let order: Vec<_> = g
            .iter()
            .map(|g| (g.exam_id.as_deref(), g.session))
            .collect();
        assert_eq!(
            order,
            [
                (None, 1),
                (Some("#0"), 1),
                (Some("9"), 1),
                (Some("9"), 2),
                (Some("10"), 1)
            ]
        );
    }

    /// Shapes from the `terms2` / `exam` / `grade` docs, with the PolSl key layout.
    fn terms2() -> Value {
        json!({
            "2026/2027-Z": {
                "CRS>B": {
                    "course_grades": [
                        {"1": {"value_symbol": "2", "passes": false,
                               "value_description": {"pl": "niedostateczny", "en": "fail"},
                               "exam_id": 900, "exam_session_number": 1,
                               "counts_into_average": true, "comment": "retake",
                               "grade_type_id": "PL", "date_modified": "2027-02-01 10:15:00",
                               "date_acquisition": "2027-01-30", "modification_author": 77},
                         "2": {"value_symbol": "4,5", "passes": true,
                               "value_description": {"pl": "dobry plus", "en": "good plus"},
                               "exam_id": "900", "counts_into_average": true,
                               "date_modified": "2027-02-15 09:00:00", "date_acquisition": null},
                         "3": null}
                    ],
                    "course_units_grades": {
                        "152016": [{"1": {"value_symbol": "ZAL", "passes": true, "exam_id": "901",
                                           "exam_session_number": 1, "counts_into_average": false}}],
                        "152014": [{"1": null}],
                        "152015": []
                    }
                },
                "CRS>A": {"course_grades": [{"1": null}], "course_units_grades": {}}
            },
            "2025/2026-Z": {},
            "bogus": null
        })
    }

    fn dir() -> Directory {
        Directory::parse(&json!({"course_editions": {"2026/2027-Z": [
            {"course_id": "CRS>B", "course_name": {"pl": "Kurs B", "en": "Course B"}, "term_id": "2026/2027-Z",
             "user_groups": [
                {"course_unit_id": "152016", "class_type_id": "LAB", "class_type": {"pl": "laboratorium", "en": "laboratory"}},
                {"course_unit_id": 152014, "class_type_id": "WYK", "class_type": {"pl": "wykład", "en": "lecture"}},
                {"class_type_id": "CW"}]},
            {"course_name": "no id"},
            "junk"
        ]}}))
        .unwrap()
    }

    #[test]
    fn parses_term_grades() {
        let ids = ["2026/2027-Z", "2025/2026-Z", "bogus", "missing"].map(String::from);
        let ects = parse_ects(&json!({"2026/2027-Z": {"CRS>B": "5.00"}}));
        let terms = parse_term_grades(&terms2(), &ids, &dir(), &ects).unwrap();
        assert_eq!(terms.len(), 4);
        assert!(terms[1..].iter().all(|t| t.courses.is_empty()));
        assert!(terms[1..].iter().all(|t| t.average.is_none()));
        assert!(terms[1..].iter().all(|t| t.weighted_average.is_none()));
        assert_eq!(terms[3].term_id, "missing");

        let t = &terms[0];
        let ids: Vec<_> = t.courses.iter().map(|c| c.course_id.as_str()).collect();
        assert_eq!(ids, ["CRS>A", "CRS>B"]);
        let (a, b) = (&t.courses[0], &t.courses[1]);
        assert!(!a.has_grades());
        assert_eq!(a.name, LangDict::default());
        assert!(b.has_grades());
        assert_eq!(b.name.pl.as_deref(), Some("Kurs B"));

        assert_eq!(b.grades.len(), 2);
        let (first, second) = (&b.grades[0], &b.grades[1]);
        assert_eq!((first.session, second.session), (1, 2));
        assert_eq!(first.exam_id.as_deref(), Some("900"));
        assert_eq!(first.passes, Some(false));
        assert_eq!(first.comment.as_deref(), Some("retake"));
        assert_eq!(first.modification_author.as_deref(), Some("77"));
        assert_eq!(first.description.pl.as_deref(), Some("niedostateczny"));
        assert_eq!(
            first.date_modified.unwrap().to_string(),
            "2027-02-01 10:15:00"
        );
        assert_eq!(
            first.date_acquisition.unwrap().to_string(),
            "2027-01-30 00:00:00"
        );
        // Session from the dict key when the grade omits it.
        assert_eq!(second.symbol.as_deref(), Some("4,5"));
        assert_eq!(second.date_acquisition, None);

        // Units sorted by class type; the empty ones stay (they are report slots).
        let kinds: Vec<_> = b
            .units
            .iter()
            .map(|u| {
                (
                    u.unit_id.as_str(),
                    u.classtype_id.as_deref(),
                    u.grades.len(),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            [
                ("152015", None, 0),
                ("152016", Some("LAB"), 1),
                ("152014", Some("WYK"), 0)
            ]
        );
        assert_eq!(b.units[1].classtype_name.en.as_deref(), Some("laboratory"));
        assert!(!b.units[1].grades[0].counts_into_average);

        // Average over the course-level grades only; session 2 replaces session 1.
        assert_eq!(t.average, Some(4.5));
        assert_eq!(t.weighted_average, Some(4.5));
        assert_eq!((a.ects, b.ects), (None, Some(5.0)));
    }

    #[test]
    fn empty_and_bad_input() {
        let empty = parse_term_grades(
            &json!({"2026/2027-Z": {}}),
            &["2026/2027-Z".to_owned()],
            &dir(),
            &no_ects(),
        );
        assert!(empty.unwrap()[0].courses.is_empty());
        assert!(parse_term_grades(&json!([]), &["x".to_owned()], &dir(), &no_ects()).is_err());
        assert!(Directory::parse(&json!({})).is_err());
        // Course-level grades as a single exam dict instead of a list.
        let single = json!({"t": {"c": {"course_grades": {"1": {"value_symbol": "3"}}}}});
        let t = parse_term_grades(
            &single,
            &["t".to_owned()],
            &Directory::default(),
            &no_ects(),
        )
        .unwrap();
        assert_eq!(t[0].courses[0].grades[0].symbol.as_deref(), Some("3"));
    }

    #[test]
    fn models_roundtrip_through_json() {
        let ids = ["2026/2027-Z".to_owned()];
        let ects = parse_ects(&json!({"2026/2027-Z": {"CRS>B": "5.00"}}));
        let terms = parse_term_grades(&terms2(), &ids, &dir(), &ects).unwrap();
        assert!(terms[0].courses.iter().any(CourseGrades::has_grades));
        let json = serde_json::to_string(&terms).unwrap();
        assert_eq!(
            serde_json::from_str::<Vec<TermGrades>>(&json).unwrap(),
            terms
        );

        let latest = parse_latest(json!([
            {"value_symbol": "4", "exam_session_number": 1, "counts_into_average": true,
             "date_modified": "2026-12-01 08:00:00",
             "course_edition": {"course_id": "CRS>B", "term_id": "2026/2027-Z",
                                "course_name": {"pl": "Kurs B", "en": "Course B"}},
             "unit": {"id": 152016, "classtype_id": "LAB"}}
        ]))
        .unwrap();
        let json = serde_json::to_string(&latest).unwrap();
        assert_eq!(
            serde_json::from_str::<Vec<LatestGrade>>(&json).unwrap(),
            latest
        );
    }

    #[test]
    fn parses_latest_newest_first() {
        let list = parse_latest(json!([
            {"value_symbol": "4", "exam_session_number": 1, "counts_into_average": true,
             "date_modified": "2026-12-01 08:00:00",
             "course_edition": {"course_id": "CRS>B", "term_id": "2026/2027-Z",
                                "course_name": {"pl": "Kurs B", "en": "Course B"}},
             "unit": {"id": 152016, "classtype_id": "LAB"}},
            {"value_symbol": "5", "exam_session_number": 1,
             "date_modified": "2026-12-20 12:00:00", "course_edition": null, "unit": null},
            {"value_symbol": "3"}
        ]))
        .unwrap();
        // The grade without a session number is dropped.
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].grade.symbol.as_deref(), Some("5"));
        assert_eq!(list[0].course_id, None);
        let b = &list[1];
        assert_eq!(b.course_id.as_deref(), Some("CRS>B"));
        assert_eq!(b.unit_id.as_deref(), Some("152016"));
        assert_eq!(b.classtype_id.as_deref(), Some("LAB"));
        assert_eq!(b.course_name.en.as_deref(), Some("Course B"));

        let v = serde_json::to_value(b).unwrap();
        assert_eq!(v["symbol"], "4");
        assert_eq!(v["session"], 1);
        assert_eq!(v["course_id"], "CRS>B");
        assert!(parse_latest(json!([])).unwrap().is_empty());
        assert!(parse_latest(json!({})).is_err());
    }

    #[tokio::test]
    async fn no_request_for_no_terms_or_zero_days() {
        // The host does not resolve; any request would fail.
        let c = Client::new("https://usos.invalid", "k", "s").unwrap();
        assert!(c.term_grades(&[]).await.unwrap().is_empty());
        assert!(matches!(
            c.latest_grades(0).await,
            Err(Error::InvalidArgument(_))
        ));
    }
}
