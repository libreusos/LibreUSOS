//! FFI-side mirrors of the core and store data types. See the crate docs for the date and
//! time conventions.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use usos_core::timetable::clashes;
use usos_core::{ActivityKind as CoreKind, Frequency as CoreFrequency, LangDict};
use usos_store::repo::{self, Source};

use crate::error::UsosError;

const DATE_FORMAT: &str = "%Y-%m-%d";
const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

pub(crate) fn parse_date(what: &str, s: &str) -> Result<NaiveDate, UsosError> {
    NaiveDate::parse_from_str(s.trim(), DATE_FORMAT)
        .map_err(|_| UsosError::invalid(format!("{what} must be a date like 2026-11-04")))
}

pub(crate) fn fmt_date(d: NaiveDate) -> String {
    d.format(DATE_FORMAT).to_string()
}

fn fmt_datetime(d: NaiveDateTime) -> String {
    d.format(DATETIME_FORMAT).to_string()
}

fn epoch(t: DateTime<Utc>) -> i64 {
    t.timestamp()
}

/// A text in both languages; either side may be absent.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LangText {
    pub pl: Option<String>,
    pub en: Option<String>,
}

impl From<LangDict> for LangText {
    fn from(d: LangDict) -> Self {
        Self { pl: d.pl, en: d.en }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
}

impl From<usos_core::User> for User {
    fn from(u: usos_core::User) -> Self {
        Self {
            id: u.id,
            first_name: u.first_name,
            last_name: u.last_name,
        }
    }
}

/// Where a result came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DataSource {
    /// Just fetched from USOS (and cached).
    Live,
    /// Served from the offline cache; `fetched_at` is epoch seconds (UTC).
    Cached { fetched_at: i64 },
}

impl From<Source> for DataSource {
    fn from(s: Source) -> Self {
        match s {
            Source::Live => Self::Live,
            Source::Cached { fetched_at } => Self::Cached {
                fetched_at: epoch(fetched_at),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Identity {
    pub user: User,
    pub source: DataSource,
}

impl From<repo::Identity> for Identity {
    fn from(i: repo::Identity) -> Self {
        Self {
            user: i.user.into(),
            source: i.source.into(),
        }
    }
}

/// An academic term such as `2026/2027-Z`. Dates are ISO dates.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Term {
    pub id: String,
    pub name: LangText,
    pub start_date: String,
    pub end_date: String,
    pub finish_date: Option<String>,
    pub order_key: Option<f64>,
    pub is_active: Option<bool>,
}

impl From<usos_core::Term> for Term {
    fn from(t: usos_core::Term) -> Self {
        Self {
            id: t.id,
            name: t.name.into(),
            start_date: fmt_date(t.start_date),
            end_date: fmt_date(t.end_date),
            finish_date: t.finish_date.map(fmt_date),
            order_key: t.order_key,
            is_active: t.is_active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TermsResult {
    /// The user's terms.
    pub terms: Vec<Term>,
    /// The one the core considers current for the `today` passed in, if any.
    pub current: Option<Term>,
    pub source: DataSource,
}

/// The `type` of a timetable entry. New values USOS adds arrive as `Other`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ActivityKind {
    Classgroup,
    Classgroup2,
    Meeting,
    Exam,
    Other { raw: String },
}

impl From<CoreKind> for ActivityKind {
    fn from(k: CoreKind) -> Self {
        match k {
            CoreKind::Classgroup => Self::Classgroup,
            CoreKind::Classgroup2 => Self::Classgroup2,
            CoreKind::Meeting => Self::Meeting,
            CoreKind::Exam => Self::Exam,
            CoreKind::Other(raw) => Self::Other { raw },
        }
    }
}

/// How often a class group meets (`frequency` of `tt/user`). USOS has no week parity: every entry
/// is already one dated occurrence, so this only describes the pattern. New codes arrive as `Other`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Frequency {
    EveryWeek,
    EveryFortnight,
    FortnightOdd,
    FortnightEven,
    EveryMonth,
    EveryWorkingDay,
    EveryWeekend,
    EveryDay,
    Once,
    Irregular,
    Other { raw: String },
}

impl From<CoreFrequency> for Frequency {
    fn from(f: CoreFrequency) -> Self {
        match f {
            CoreFrequency::EveryWeek => Self::EveryWeek,
            CoreFrequency::EveryFortnight => Self::EveryFortnight,
            CoreFrequency::FortnightOdd => Self::FortnightOdd,
            CoreFrequency::FortnightEven => Self::FortnightEven,
            CoreFrequency::EveryMonth => Self::EveryMonth,
            CoreFrequency::EveryWorkingDay => Self::EveryWorkingDay,
            CoreFrequency::EveryWeekend => Self::EveryWeekend,
            CoreFrequency::EveryDay => Self::EveryDay,
            CoreFrequency::Once => Self::Once,
            CoreFrequency::Irregular => Self::Irregular,
            CoreFrequency::Other(raw) => Self::Other { raw },
        }
    }
}

/// One timetable entry. `start` / `end` are local date-times, `date` is the start day.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Activity {
    pub kind: ActivityKind,
    pub start: String,
    pub end: String,
    pub date: String,
    pub name: LangText,
    pub course_id: Option<String>,
    pub course_name: LangText,
    pub classtype_id: Option<String>,
    pub classtype_name: LangText,
    pub group_number: Option<u32>,
    pub building_id: Option<String>,
    pub building_name: LangText,
    pub room_id: Option<String>,
    pub room_number: Option<String>,
    pub lecturer_ids: Vec<String>,
    pub unit_id: Option<String>,
    pub frequency: Option<Frequency>,
    pub url: Option<String>,
    /// Overlaps another activity of the same day in this result.
    pub clashes: bool,
}

impl Activity {
    fn from_core(a: usos_core::Activity, clashes: bool) -> Self {
        Self {
            date: fmt_date(a.date()),
            kind: a.kind.into(),
            start: fmt_datetime(a.start),
            end: fmt_datetime(a.end),
            name: a.name.into(),
            course_id: a.course_id,
            course_name: a.course_name.into(),
            classtype_id: a.classtype_id,
            classtype_name: a.classtype_name.into(),
            group_number: a.group_number,
            building_id: a.building_id,
            building_name: a.building_name.into(),
            room_id: a.room_id,
            room_number: a.room_number,
            lecturer_ids: a.lecturer_ids,
            unit_id: a.unit_id,
            frequency: a.frequency.map(Into::into),
            url: a.url,
            clashes,
        }
    }
}

pub(crate) fn activities(list: Vec<usos_core::Activity>) -> Vec<Activity> {
    let flags = clashes(&list);
    list.into_iter()
        .zip(flags)
        .map(|(a, c)| Activity::from_core(a, c))
        .collect()
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Timetable {
    /// Sorted by start time.
    pub activities: Vec<Activity>,
    pub source: DataSource,
    /// Days of the requested range present in the cache (equals `days_total` when live).
    pub days_cached: u32,
    pub days_total: u32,
}

impl From<repo::PlanResult> for Timetable {
    fn from(p: repo::PlanResult) -> Self {
        Self {
            activities: activities(p.activities),
            source: p.source.into(),
            days_cached: p.days_cached,
            days_total: p.days_total,
        }
    }
}

/// One grade for one exam session of an exam report.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Grade {
    /// Exam report id; sessions of the same report share it.
    pub exam_id: Option<String>,
    /// 1 for the first attempt, 2 for the first retake, and so on.
    pub session: u32,
    /// The symbol as students see it: `4,5`, `5!`, `ZAL`, `NZAL`, ...
    pub symbol: Option<String>,
    pub description: LangText,
    /// Whether the grade passes; `None` when USOS does not say.
    pub passes: Option<bool>,
    pub counts_into_average: bool,
    pub comment: Option<String>,
    /// Local date-time (installation zone) of the last modification.
    pub date_modified: Option<String>,
    /// Local date-time (installation zone) the grade was acquired.
    pub date_acquisition: Option<String>,
    /// The core's numeric value on the Polish scale (2 to 5.5); `None` for `ZAL`, `NZAL`, etc.
    pub numeric_value: Option<f64>,
}

impl From<usos_core::Grade> for Grade {
    fn from(g: usos_core::Grade) -> Self {
        let numeric_value = g.numeric_value();
        Self {
            exam_id: g.exam_id,
            session: g.session,
            symbol: g.symbol,
            description: g.description.into(),
            passes: g.passes,
            counts_into_average: g.counts_into_average,
            comment: g.comment,
            date_modified: g.date_modified.map(fmt_datetime),
            date_acquisition: g.date_acquisition.map(fmt_datetime),
            numeric_value,
        }
    }
}

/// Grades of one course unit (one class type, for example a lecture or a lab).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UnitGrades {
    pub unit_id: String,
    pub classtype_id: Option<String>,
    pub classtype_name: LangText,
    /// Sorted by exam report, then session.
    pub grades: Vec<Grade>,
}

impl From<usos_core::UnitGrades> for UnitGrades {
    fn from(u: usos_core::UnitGrades) -> Self {
        Self {
            unit_id: u.unit_id,
            classtype_id: u.classtype_id,
            classtype_name: u.classtype_name.into(),
            grades: u.grades.into_iter().map(Grade::from).collect(),
        }
    }
}

/// A course edition with its grades.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseGrades {
    pub course_id: String,
    /// Empty when the course is not among the user's course editions.
    pub name: LangText,
    pub term_id: String,
    pub ects: Option<f64>,
    /// Course-level (final) grades, sorted by exam report, then session.
    pub grades: Vec<Grade>,
    pub units: Vec<UnitGrades>,
    /// The core's [`course_grade`](usos_core::grades::course_grade): the value this course
    /// counts with in averages; `None` when no report counts.
    pub course_grade: Option<f64>,
}

impl From<usos_core::CourseGrades> for CourseGrades {
    fn from(c: usos_core::CourseGrades) -> Self {
        let course_grade = usos_core::grades::course_grade(&c.grades);
        Self {
            course_id: c.course_id,
            name: c.name.into(),
            term_id: c.term_id,
            ects: c.ects,
            grades: c.grades.into_iter().map(Grade::from).collect(),
            units: c.units.into_iter().map(UnitGrades::from).collect(),
            course_grade,
        }
    }
}

/// The grades of one term, with the averages as the core computes them.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TermGrades {
    pub term_id: String,
    /// Sorted by course id.
    pub courses: Vec<CourseGrades>,
    /// The core's unweighted average of the courses' grades; `None` when none counts.
    pub average: Option<f64>,
    /// The core's ECTS-weighted average; `None` when nothing is left to weigh.
    pub weighted_average: Option<f64>,
    pub source: DataSource,
}

impl From<repo::TermGradesResult> for TermGrades {
    fn from(r: repo::TermGradesResult) -> Self {
        Self {
            term_id: r.grades.term_id,
            courses: r
                .grades
                .courses
                .into_iter()
                .map(CourseGrades::from)
                .collect(),
            average: r.grades.average,
            weighted_average: r.grades.weighted_average,
            source: r.source.into(),
        }
    }
}

/// The type of a test node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TestNodeKind {
    Root,
    /// Groups other nodes; the core totals its points from the subtree.
    Folder,
    /// Gives points.
    Task,
    /// Gives a grade.
    Grade,
    /// A type this client does not know.
    Unknown,
}

impl From<usos_core::NodeKind> for TestNodeKind {
    fn from(k: usos_core::NodeKind) -> Self {
        match k {
            usos_core::NodeKind::Root => Self::Root,
            usos_core::NodeKind::Folder => Self::Folder,
            usos_core::NodeKind::Task => Self::Task,
            usos_core::NodeKind::Grade => Self::Grade,
            usos_core::NodeKind::Unknown => Self::Unknown,
        }
    }
}

/// The user's grade in a grade node.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct NodeGrade {
    /// As students see it: `4,5`, `ZAL`, ...
    pub symbol: Option<String>,
    pub description: LangText,
    /// Whether the grade passes; `None` when USOS does not say.
    pub passes: Option<bool>,
    /// The numeric value, when the grade has one.
    pub value: Option<f64>,
}

impl From<usos_core::NodeGrade> for NodeGrade {
    fn from(g: usos_core::NodeGrade) -> Self {
        Self {
            symbol: g.symbol,
            description: g.description.into(),
            passes: g.passes,
            value: g.value,
        }
    }
}

/// One node of a test tree: the root, a folder, a task or a grade.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TestNode {
    pub id: String,
    pub kind: TestNodeKind,
    pub name: LangText,
    pub description: LangText,
    /// Whether the lecturer made the results of this node visible to students; `None` when
    /// USOS does not say.
    pub results_visible: Option<bool>,
    /// The lecturer hid the results of this node. A hidden node stays in the tree but is left
    /// out of every total.
    pub hidden: bool,
    /// The user's points: for a task what the lecturer gave, for a folder or the root the
    /// core's total of the visible children. `None` when there is nothing to sum.
    pub points: Option<f64>,
    /// The most points the node gives, as the core settles it (see the core's docs).
    pub points_max: Option<f64>,
    pub points_min: Option<f64>,
    /// Grade nodes only, and only when a grade was given.
    pub grade: Option<NodeGrade>,
    /// The lecturer's comment on the points or the grade.
    pub comment: Option<String>,
    /// Local date-time (installation zone) the points or grade last changed; for a folder
    /// or the root the latest change below.
    pub last_changed: Option<String>,
    /// In the lecturer's order.
    pub children: Vec<TestNode>,
}

impl From<usos_core::TestNode> for TestNode {
    fn from(n: usos_core::TestNode) -> Self {
        Self {
            id: n.id,
            kind: n.kind.into(),
            name: n.name.into(),
            description: n.description.into(),
            results_visible: n.results_visible,
            hidden: n.hidden,
            points: n.points,
            points_max: n.points_max,
            points_min: n.points_min,
            grade: n.grade.map(NodeGrade::from),
            comment: n.comment,
            last_changed: n.last_changed.map(fmt_datetime),
            children: n.children.into_iter().map(TestNode::from).collect(),
        }
    }
}

/// The test of one course edition.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseTest {
    pub course_id: String,
    /// Empty when USOS did not say.
    pub course_name: LangText,
    pub term_id: String,
    /// The test is only for some class groups.
    pub limited_to_groups: bool,
    /// Its `children` are the first level of the test; `points` and `points_max` are the
    /// core's totals.
    pub root: TestNode,
}

impl From<usos_core::TestRoot> for CourseTest {
    fn from(t: usos_core::TestRoot) -> Self {
        Self {
            course_id: t.course_id,
            course_name: t.course_name.into(),
            term_id: t.term_id,
            limited_to_groups: t.limited_to_groups,
            root: t.root.into(),
        }
    }
}

/// The tests of one term.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TermTests {
    pub term_id: String,
    pub tests: Vec<CourseTest>,
    /// Root ids of the tests USOS refused or returned unreadable this time. A live result
    /// keeps the cached copy of such a test in `tests`.
    pub skipped: Vec<String>,
    pub source: DataSource,
}

/// The result of `course_tests`: the terms that came back and the ones that could not be loaded.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseTestsResult {
    /// In the order requested; each term carries its own `source`.
    pub terms: Vec<TermTests>,
    /// Terms whose call failed (other than `Unauthorized`) and that have no cache to fall back on.
    pub failed_terms: Vec<String>,
}

impl TermTests {
    pub(crate) fn from_result(term_id: String, r: repo::CourseTestsResult) -> Self {
        Self {
            term_id,
            tests: r.tests.tests.into_iter().map(CourseTest::from).collect(),
            skipped: r.tests.skipped,
            source: r.source.into(),
        }
    }
}

/// What `sync` fetched: counts and change flags.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SyncReport {
    pub user_changed: bool,
    pub terms: u32,
    pub terms_changed: bool,
    pub plan_from: String,
    pub plan_to: String,
    pub activities: u32,
    pub plan_changed: bool,
    pub term_id: Option<String>,
    pub courses: u32,
    pub grades_changed: bool,
    pub tests: u32,
    pub tests_skipped: u32,
    pub tests_changed: bool,
}

impl From<repo::SyncReport> for SyncReport {
    fn from(r: repo::SyncReport) -> Self {
        Self {
            user_changed: r.user_changed,
            terms: r.terms,
            terms_changed: r.terms_changed,
            plan_from: fmt_date(r.plan_from),
            plan_to: fmt_date(r.plan_to),
            activities: r.activities,
            plan_changed: r.plan_changed,
            term_id: r.term_id,
            courses: r.courses,
            grades_changed: r.grades_changed,
            tests: r.tests,
            tests_skipped: r.tests_skipped,
            tests_changed: r.tests_changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn activity(start: &str, end: &str) -> usos_core::Activity {
        let parse = |s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap();
        usos_core::Activity {
            kind: CoreKind::from("lab"),
            start: parse(start),
            end: parse(end),
            name: LangDict {
                pl: Some("Fizyka".into()),
                en: None,
            },
            course_id: Some("C1".into()),
            course_name: LangDict::default(),
            classtype_id: None,
            classtype_name: LangDict::default(),
            group_number: Some(2),
            building_id: None,
            building_name: LangDict::default(),
            room_id: None,
            room_number: Some("101".into()),
            lecturer_ids: vec!["7".into()],
            unit_id: None,
            frequency: None,
            url: None,
        }
    }

    #[test]
    fn dates_round_trip_and_reject_garbage() {
        let d = parse_date("from", "2026-11-04").unwrap();
        assert_eq!(fmt_date(d), "2026-11-04");
        assert!(matches!(
            parse_date("from", "04.11.2026"),
            Err(UsosError::InvalidInput { .. })
        ));
    }

    #[test]
    fn activities_carry_iso_times_and_clash_flags() {
        let list = activities(vec![
            activity("2026-11-04 08:00:00", "2026-11-04 09:30:00"),
            activity("2026-11-04 09:00:00", "2026-11-04 10:00:00"),
            activity("2026-11-04 10:00:00", "2026-11-04 11:00:00"),
        ]);
        assert_eq!(list[0].start, "2026-11-04T08:00:00");
        assert_eq!(list[0].date, "2026-11-04");
        assert_eq!(list[0].name.pl.as_deref(), Some("Fizyka"));
        assert_eq!(list[0].kind, ActivityKind::Other { raw: "lab".into() });
        assert_eq!(
            list.iter().map(|a| a.clashes).collect::<Vec<_>>(),
            [true, true, false]
        );
    }

    #[test]
    fn source_uses_epoch_seconds() {
        let t = Utc.with_ymd_and_hms(2026, 11, 4, 12, 0, 0).unwrap();
        assert_eq!(
            DataSource::from(Source::Cached { fetched_at: t }),
            DataSource::Cached {
                fetched_at: 1_793_793_600
            }
        );
        assert_eq!(DataSource::from(Source::Live), DataSource::Live);
    }

    #[test]
    fn frequency_is_typed() {
        let mut a = activity("2026-11-04 08:00:00", "2026-11-04 09:30:00");
        a.frequency = Some(CoreFrequency::FortnightOdd);
        assert_eq!(
            activities(vec![a.clone()])[0].frequency,
            Some(Frequency::FortnightOdd)
        );
        a.frequency = Some(CoreFrequency::Other("every_leap_day".into()));
        assert_eq!(
            activities(vec![a])[0].frequency,
            Some(Frequency::Other {
                raw: "every_leap_day".into()
            })
        );
    }

    #[test]
    fn grades_keep_symbols_flags_and_the_core_values() {
        let grade = |session, symbol: &str, passes, counts| usos_core::Grade {
            exam_id: Some("E1".into()),
            session,
            symbol: Some(symbol.into()),
            passes: Some(passes),
            counts_into_average: counts,
            date_modified: NaiveDateTime::parse_from_str(
                "2026-02-03 10:20:30",
                "%Y-%m-%d %H:%M:%S",
            )
            .ok(),
            ..usos_core::Grade::default()
        };
        let course = usos_core::CourseGrades {
            course_id: "C1".into(),
            name: LangDict {
                pl: Some("Analiza".into()),
                en: None,
            },
            term_id: "T1".into(),
            ects: Some(6.0),
            grades: vec![grade(1, "2", false, true), grade(2, "4,5", true, true)],
            units: vec![usos_core::UnitGrades {
                unit_id: "U1".into(),
                classtype_id: Some("LAB".into()),
                classtype_name: LangDict::default(),
                grades: vec![grade(1, "ZAL", true, false)],
            }],
        };
        let term = usos_core::TermGrades {
            term_id: "T1".into(),
            courses: vec![course],
            average: Some(4.5),
            weighted_average: None,
        };
        let out = TermGrades::from(repo::TermGradesResult {
            grades: term,
            source: Source::Live,
            changed: false,
        });
        assert_eq!(out.source, DataSource::Live);
        assert_eq!(out.average, Some(4.5));
        assert_eq!(out.weighted_average, None);
        let c = &out.courses[0];
        assert_eq!(c.course_grade, Some(4.5));
        assert_eq!(c.name.pl.as_deref(), Some("Analiza"));
        assert_eq!(c.grades[0].symbol.as_deref(), Some("2"));
        assert_eq!(c.grades[0].passes, Some(false));
        assert_eq!(c.grades[1].session, 2);
        assert_eq!(c.grades[1].numeric_value, Some(4.5));
        assert_eq!(
            c.grades[1].date_modified.as_deref(),
            Some("2026-02-03T10:20:30")
        );
        let unit = &c.units[0];
        assert_eq!(unit.classtype_id.as_deref(), Some("LAB"));
        assert_eq!(unit.grades[0].numeric_value, None);
        assert!(!unit.grades[0].counts_into_average);
    }

    #[test]
    fn test_trees_keep_every_node_field() {
        use usos_core::crstests::{CourseTests, NodeGrade as CoreGrade, NodeKind, TestNode as N};
        let task = N {
            id: "N2".into(),
            kind: NodeKind::Task,
            name: LangDict {
                pl: Some("Zadanie 1".into()),
                en: None,
            },
            results_visible: Some(true),
            points: Some(4.5),
            points_max: Some(10.0),
            points_min: Some(0.0),
            comment: Some("ok".into()),
            last_changed: NaiveDateTime::parse_from_str("2026-03-04 05:06:07", "%Y-%m-%d %H:%M:%S")
                .ok(),
            ..N::default()
        };
        let grade = N {
            id: "N3".into(),
            kind: NodeKind::Grade,
            hidden: true,
            results_visible: Some(false),
            grade: Some(CoreGrade {
                symbol: Some("4,5".into()),
                passes: Some(true),
                value: Some(4.5),
                ..CoreGrade::default()
            }),
            ..N::default()
        };
        let folder = N {
            id: "N1".into(),
            kind: NodeKind::Folder,
            points: Some(4.5),
            points_max: Some(10.0),
            children: vec![task, grade],
            ..N::default()
        };
        let root = N {
            id: "R".into(),
            kind: NodeKind::Root,
            children: vec![folder],
            ..N::default()
        };
        let out = TermTests::from_result(
            "T1".into(),
            repo::CourseTestsResult {
                tests: CourseTests {
                    tests: vec![usos_core::TestRoot {
                        course_id: "C1".into(),
                        course_name: LangDict {
                            pl: None,
                            en: Some("Analysis".into()),
                        },
                        term_id: "T1".into(),
                        limited_to_groups: true,
                        root,
                    }],
                    skipped: vec!["R9".into()],
                },
                source: Source::Live,
                changed: false,
            },
        );
        assert_eq!(out.source, DataSource::Live);
        assert_eq!(out.skipped, ["R9"]);
        let t = &out.tests[0];
        assert_eq!(t.course_name.en.as_deref(), Some("Analysis"));
        assert!(t.limited_to_groups);
        assert_eq!(t.root.kind, TestNodeKind::Root);
        let folder = &t.root.children[0];
        assert_eq!(folder.kind, TestNodeKind::Folder);
        assert_eq!(folder.points_max, Some(10.0));
        let task = &folder.children[0];
        assert_eq!(task.kind, TestNodeKind::Task);
        assert_eq!(task.name.pl.as_deref(), Some("Zadanie 1"));
        assert_eq!(task.points, Some(4.5));
        assert_eq!(task.points_min, Some(0.0));
        assert_eq!(task.results_visible, Some(true));
        assert!(!task.hidden);
        assert_eq!(task.comment.as_deref(), Some("ok"));
        assert_eq!(task.last_changed.as_deref(), Some("2026-03-04T05:06:07"));
        let grade = &folder.children[1];
        assert!(grade.hidden);
        assert_eq!(grade.results_visible, Some(false));
        let g = grade.grade.as_ref().unwrap();
        assert_eq!(g.symbol.as_deref(), Some("4,5"));
        assert_eq!(g.passes, Some(true));
        assert_eq!(g.value, Some(4.5));
    }
}
