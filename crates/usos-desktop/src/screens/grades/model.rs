//! The view-model of Oceny: pure functions from a term's grades to the rows, the summary and the
//! detail of one course. Grade symbols are shown exactly as USOS gives them; only the average
//! and the ECTS sums are computed here, with the same rules `usos-core` documents.

use std::collections::BTreeMap;

use usos_core::grades::{CourseGrades, Grade, TermGrades, UnitGrades};

use crate::components::TileKind;
use crate::i18n::{Lang, fixed, number};
use crate::theme::Kind;
use crate::view_util::{pick, roman, short_date};

/// The class form of a unit ("WYK", "ĆW", "LAB") and its colour class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormTag {
    pub code: String,
    pub kind: Kind,
}

/// The tag for a USOS class type id. `CW` is shown with its Polish diacritic.
pub fn form_tag(classtype_id: &str) -> Option<FormTag> {
    let id = classtype_id.trim();
    if id.is_empty() {
        return None;
    }
    let upper = id.to_uppercase();
    let kind = match upper.as_str() {
        "WYK" => Kind::Lecture,
        "LAB" => Kind::Lab,
        "EGZ" | "KOL" => Kind::Exam,
        _ => Kind::Other,
    };
    let code = if upper == "CW" {
        "ĆW".to_owned()
    } else {
        upper
    };
    Some(FormTag { code, kind })
}

/// Whether `grade` passes: USOS says so when it can, else the scale decides (3,0 and up).
pub fn is_passing(grade: &Grade) -> bool {
    match grade.passes {
        Some(passes) => passes,
        None => grade.numeric_value().is_some_and(|v| v >= 3.0),
    }
}

/// The latest session of every report (grades sharing an `exam_id`; one without an id is its
/// own report), in a stable order.
fn latest_per_report(grades: &[Grade]) -> Vec<&Grade> {
    let mut latest: BTreeMap<&str, &Grade> = BTreeMap::new();
    let mut out: Vec<&Grade> = Vec::new();
    for grade in grades {
        match grade.exam_id.as_deref() {
            Some(id) => {
                let slot = latest.entry(id).or_insert(grade);
                if grade.session > slot.session {
                    *slot = grade;
                }
            }
            None => out.push(grade),
        }
    }
    out.extend(latest.into_values());
    out
}

/// The grade a course shows: the counting numeric grade `course_grade` would average (the
/// highest of the reports' latest sessions); else the latest grade with a symbol, preferring a
/// passing one (a "ZAL").
pub fn shown_grade(grades: &[Grade]) -> Option<&Grade> {
    let reports: Vec<&Grade> = latest_per_report(grades)
        .into_iter()
        .filter(|g| g.symbol.as_deref().is_some_and(|s| !s.trim().is_empty()))
        .collect();
    let counting = reports
        .iter()
        .filter(|g| g.counts_into_average)
        .filter_map(|g| Some((*g, g.numeric_value()?)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(g, _)| g);
    counting.or_else(|| {
        let key = |g: &&Grade| (is_passing(g), g.date_acquisition, g.session);
        reports.into_iter().max_by_key(key)
    })
}

/// The earlier session of the same report as `shown`, when there was one.
pub fn earlier_session<'a>(grades: &'a [Grade], shown: &Grade) -> Option<&'a Grade> {
    let id = shown.exam_id.as_deref()?;
    grades
        .iter()
        .filter(|g| g.exam_id.as_deref() == Some(id) && g.session < shown.session)
        .max_by_key(|g| g.session)
}

/// "I termin" / "Attempt 1".
pub fn session_label(session: u32, lang: Lang) -> String {
    match lang {
        Lang::Pl => format!("{} termin", roman(session.max(1))),
        Lang::En => format!("Attempt {}", session.max(1)),
    }
}

fn tile_for(grade: &Grade) -> TileKind {
    if is_passing(grade) {
        TileKind::Normal
    } else {
        TileKind::Fail
    }
}

fn symbol_of(grade: &Grade) -> String {
    grade
        .symbol
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The grade shown in a table row.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownGrade {
    pub symbol: String,
    pub tile: TileKind,
    /// The earlier session whose grade differs, as (label, symbol): ("I termin", "2,0"). The
    /// symbol is drawn struck through.
    pub earlier: Option<(String, String)>,
}

/// One row of the course table.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseRow {
    pub course_id: String,
    pub name: String,
    pub ects: Option<f64>,
    pub ects_text: String,
    pub forms: Vec<FormTag>,
    pub grade: Option<ShownGrade>,
    /// The shown grade passes (for the ECTS sum).
    pub passed: bool,
}

/// The numbers above the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub average: Option<String>,
    pub weighted: Option<String>,
    pub ects_earned: f64,
    pub ects_total: f64,
}

/// Everything the course table draws.
#[derive(Debug, Clone, PartialEq)]
pub struct GradesView {
    pub summary: Summary,
    pub rows: Vec<CourseRow>,
    /// Rows with a grade.
    pub graded: usize,
}

/// The unique class forms of a course, in the order USOS lists its units.
fn forms_of(course: &CourseGrades) -> Vec<FormTag> {
    let mut forms: Vec<FormTag> = Vec::new();
    for unit in &course.units {
        if let Some(tag) = unit.classtype_id.as_deref().and_then(form_tag)
            && !forms.contains(&tag)
        {
            forms.push(tag);
        }
    }
    forms
}

fn row(course: &CourseGrades, lang: Lang) -> CourseRow {
    let shown = shown_grade(&course.grades);
    let grade = shown.map(|g| ShownGrade {
        symbol: symbol_of(g),
        tile: tile_for(g),
        earlier: earlier_session(&course.grades, g)
            .filter(|e| symbol_of(e) != symbol_of(g))
            .map(|e| (session_label(e.session, lang), symbol_of(e))),
    });
    let name = {
        let name = pick(&course.name, lang);
        if name.is_empty() {
            course.course_id.clone()
        } else {
            name
        }
    };
    CourseRow {
        course_id: course.course_id.clone(),
        name,
        ects: course.ects,
        ects_text: course
            .ects
            .map_or_else(|| "\u{2014}".to_owned(), |e| number(e, 1, lang)),
        forms: forms_of(course),
        grade,
        passed: shown.is_some_and(is_passing),
    }
}

/// The table and the summary of one term. The averages come from `usos-core` (the same
/// functions the CLI and the Android app use).
pub fn build(term: &TermGrades, lang: Lang) -> GradesView {
    let mut rows: Vec<CourseRow> = term.courses.iter().map(|c| row(c, lang)).collect();
    mark_best(&mut rows);
    let average = term
        .average
        .or_else(|| usos_core::grades::average(&term.courses));
    let weighted = term
        .weighted_average
        .or_else(|| usos_core::grades::weighted_average(&term.courses));
    let ects_total = rows.iter().filter_map(|r| r.ects).sum();
    let ects_earned = rows
        .iter()
        .filter(|r| r.passed)
        .filter_map(|r| r.ects)
        .sum();
    let graded = rows.iter().filter(|r| r.grade.is_some()).count();
    GradesView {
        summary: Summary {
            average: average.map(|v| fixed(v, 2, lang)),
            weighted: weighted.map(|v| fixed(v, 2, lang)),
            ects_earned,
            ects_total,
        },
        rows,
        graded,
    }
}

/// The highest numeric grade of the term gets the "best" tile, unless every grade is equal.
fn mark_best(rows: &mut [CourseRow]) {
    let numeric = |row: &CourseRow| -> Option<f64> {
        let grade = row.grade.as_ref()?;
        if grade.tile == TileKind::Fail {
            return None;
        }
        if grade.symbol == "5!" {
            return Some(5.5);
        }
        grade.symbol.replace(',', ".").parse::<f64>().ok()
    };
    let values: Vec<Option<f64>> = rows.iter().map(numeric).collect();
    let max = values.iter().flatten().copied().max_by(f64::total_cmp);
    let min = values.iter().flatten().copied().min_by(f64::total_cmp);
    let (Some(max), Some(min)) = (max, min) else {
        return;
    };
    if (max - min).abs() < f64::EPSILON {
        return;
    }
    for (row, value) in rows.iter_mut().zip(values) {
        if value == Some(max)
            && let Some(grade) = &mut row.grade
        {
            grade.tile = TileKind::Best;
        }
    }
}

/// One grade line of the detail pane.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub label: String,
    pub meta: String,
    pub symbol: String,
    pub tile: TileKind,
}

/// A block of the detail pane: the course-level grades, or the grades of one unit.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailGroup {
    pub tag: Option<FormTag>,
    pub title: String,
    pub entries: Vec<Entry>,
}

/// The right-hand pane.
#[derive(Debug, Clone, PartialEq)]
pub struct Detail {
    pub name: String,
    /// "2025/26-L · 5 ECTS · RI-2-FIZ"
    pub meta: String,
    pub big: Option<(String, TileKind)>,
    /// "Ocena końcowa · dostateczny"
    pub caption: String,
    /// When the shown grade was given.
    pub sub: Option<String>,
    /// Why the shown grade is not the first one.
    pub note: Option<String>,
    pub groups: Vec<DetailGroup>,
}

fn entry(grade: &Grade, label: String, lang: Lang) -> Entry {
    let mut meta: Vec<String> = Vec::new();
    if let Some(date) = grade.date_acquisition {
        meta.push(short_date(date.date(), lang));
    }
    if let Some(comment) = grade.comment.as_deref().map(str::trim)
        && !comment.is_empty()
    {
        meta.push(comment.to_owned());
    }
    Entry {
        label,
        meta: meta.join(" \u{b7} "),
        symbol: symbol_of(grade),
        tile: tile_for(grade),
    }
}

fn unit_group(unit: &UnitGrades, lang: Lang) -> Option<DetailGroup> {
    let entries: Vec<Entry> = unit
        .grades
        .iter()
        .filter(|g| g.symbol.as_deref().is_some_and(|s| !s.trim().is_empty()))
        .map(|g| {
            let description = pick(&g.description, lang);
            let label = if description.is_empty() {
                session_label(g.session, lang)
            } else {
                description
            };
            entry(g, label, lang)
        })
        .collect();
    if entries.is_empty() {
        return None;
    }
    let tag = unit.classtype_id.as_deref().and_then(form_tag);
    let name = pick(&unit.classtype_name, lang);
    let title = if name.is_empty() {
        tag.as_ref().map(|t| t.code.clone()).unwrap_or_default()
    } else {
        name
    };
    Some(DetailGroup {
        tag,
        title,
        entries,
    })
}

const FINAL: [&str; 2] = ["Ocena końcowa", "Final grade"];
const NO_FINAL: [&str; 2] = ["Brak oceny końcowej", "No final grade yet"];
const GROUP_COURSE: [&str; 2] = ["Oceny z przedmiotu", "Course grades"];
const NOT_COUNTED: [&str; 2] = ["Nie wlicza się do średniej", "Not counted in the average"];

fn t(pair: [&'static str; 2], lang: Lang) -> &'static str {
    match lang {
        Lang::Pl => pair[0],
        Lang::En => pair[1],
    }
}

/// The note under the big grade when the shown grade is a later session.
fn retake_note(earlier: &Grade, shown: &Grade, lang: Lang) -> String {
    let label = session_label(earlier.session, lang);
    let (before, now) = (symbol_of(earlier), symbol_of(shown));
    match (lang, is_passing(earlier), shown.counts_into_average) {
        (Lang::Pl, false, true) => {
            format!("{label} niezaliczony ({before}). Do średniej liczy się ocena końcowa {now}.")
        }
        (Lang::Pl, false, false) => format!("{label} niezaliczony ({before})."),
        (Lang::Pl, true, _) => format!("{label}: {before}. Ocena końcowa to {now}."),
        (Lang::En, false, true) => format!(
            "{label} was not passed ({before}). The final grade {now} counts into the average."
        ),
        (Lang::En, false, false) => format!("{label} was not passed ({before})."),
        (Lang::En, true, _) => format!("{label}: {before}. The final grade is {now}."),
    }
}

/// The detail pane for `course` in the term `term_id`.
pub fn detail(course: &CourseGrades, term_id: &str, lang: Lang) -> Detail {
    let shown = shown_grade(&course.grades);
    let row = row(course, lang);
    let mut meta: Vec<String> = vec![term_id.to_owned()];
    if let Some(ects) = course.ects {
        meta.push(format!("{} ECTS", number(ects, 1, lang)));
    }
    meta.push(course.course_id.clone());

    let big = shown.map(|g| (symbol_of(g), tile_for(g)));
    let description = shown
        .map(|g| pick(&g.description, lang))
        .unwrap_or_default();
    let caption = match shown {
        Some(_) if description.is_empty() => t(FINAL, lang).to_owned(),
        Some(_) => format!("{} \u{b7} {description}", t(FINAL, lang)),
        None => t(NO_FINAL, lang).to_owned(),
    };
    let sub = shown.and_then(|g| {
        let mut parts: Vec<String> = Vec::new();
        if let Some(date) = g.date_acquisition {
            parts.push(short_date(date.date(), lang));
        }
        if !g.counts_into_average && g.numeric_value().is_some() {
            parts.push(t(NOT_COUNTED, lang).to_owned());
        }
        (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
    });
    let note = shown.and_then(|g| {
        earlier_session(&course.grades, g)
            .filter(|e| symbol_of(e) != symbol_of(g))
            .map(|e| retake_note(e, g, lang))
    });

    let mut groups: Vec<DetailGroup> = Vec::new();
    let mut course_entries: Vec<&Grade> = course
        .grades
        .iter()
        .filter(|g| g.symbol.as_deref().is_some_and(|s| !s.trim().is_empty()))
        .collect();
    course_entries.sort_by_key(|g| (g.exam_id.clone(), g.session));
    if !course_entries.is_empty() {
        groups.push(DetailGroup {
            tag: None,
            title: t(GROUP_COURSE, lang).to_owned(),
            entries: course_entries
                .into_iter()
                .map(|g| entry(g, session_label(g.session, lang), lang))
                .collect(),
        });
    }
    groups.extend(course.units.iter().filter_map(|u| unit_group(u, lang)));

    Detail {
        name: row.name,
        meta: meta.join(" \u{b7} "),
        big,
        caption,
        sub,
        note,
        groups,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use usos_core::LangDict;
    use usos_core::grades::course_grade;

    use super::*;

    fn counted_value(course: &CourseGrades) -> Option<f64> {
        course_grade(&course.grades)
    }

    fn dict(pl: &str) -> LangDict {
        LangDict {
            pl: Some(pl.into()),
            en: None,
        }
    }

    fn grade(symbol: &str, exam: Option<&str>, session: u32, counts: bool) -> Grade {
        Grade {
            exam_id: exam.map(str::to_owned),
            session,
            symbol: Some(symbol.into()),
            counts_into_average: counts,
            passes: Some(
                symbol
                    .replace(',', ".")
                    .parse::<f64>()
                    .map_or(matches!(symbol, "ZAL" | "5!"), |v| v >= 3.0),
            ),
            ..Grade::default()
        }
    }

    fn course(id: &str, name: &str, ects: Option<f64>, grades: Vec<Grade>) -> CourseGrades {
        CourseGrades {
            course_id: id.into(),
            name: dict(name),
            term_id: "2025/26-L".into(),
            ects,
            grades,
            units: Vec::new(),
        }
    }

    fn term(courses: Vec<CourseGrades>) -> TermGrades {
        TermGrades {
            term_id: "2025/26-L".into(),
            average: usos_core::grades::average(&courses),
            weighted_average: usos_core::grades::weighted_average(&courses),
            courses,
        }
    }

    #[test]
    fn the_latest_session_of_a_report_is_shown_with_the_earlier_one() {
        let grades = vec![
            grade("2,0", Some("e1"), 1, true),
            grade("3,0", Some("e1"), 2, true),
        ];
        let shown = shown_grade(&grades).unwrap();
        assert_eq!(shown.symbol.as_deref(), Some("3,0"));
        let earlier = earlier_session(&grades, shown).unwrap();
        assert_eq!(earlier.symbol.as_deref(), Some("2,0"));
        // The row agrees with what `usos-core` averages.
        let c = course("X", "Fizyka", Some(5.0), grades);
        assert_eq!(counted_value(&c), Some(3.0));
        let view = build(&term(vec![c]), Lang::Pl);
        let g = view.rows[0].grade.as_ref().unwrap();
        assert_eq!(g.symbol, "3,0");
        assert_eq!(g.earlier, Some(("I termin".into(), "2,0".into())));
    }

    #[test]
    fn an_unchanged_retake_has_no_struck_through_grade() {
        let grades = vec![
            grade("4,0", Some("e1"), 1, true),
            grade("4,0", Some("e1"), 2, true),
        ];
        let view = build(&term(vec![course("X", "A", Some(2.0), grades)]), Lang::Pl);
        assert_eq!(view.rows[0].grade.as_ref().unwrap().earlier, None);
    }

    #[test]
    fn the_highest_counting_report_wins() {
        let grades = vec![
            grade("3,5", Some("a"), 1, true),
            grade("4,5", Some("b"), 1, true),
            grade("5,0", Some("c"), 1, false),
        ];
        assert_eq!(shown_grade(&grades).unwrap().symbol.as_deref(), Some("4,5"));
    }

    #[test]
    fn a_pass_word_is_shown_but_not_averaged() {
        let grades = vec![grade("ZAL", None, 1, false)];
        let shown = shown_grade(&grades).unwrap();
        assert_eq!(shown.symbol.as_deref(), Some("ZAL"));
        let c = course("WF", "Wychowanie fizyczne", Some(0.0), grades);
        let view = build(&term(vec![c]), Lang::Pl);
        assert_eq!(view.rows[0].grade.as_ref().unwrap().symbol, "ZAL");
        assert_eq!(view.summary.average, None);
        assert!(view.rows[0].passed);
    }

    #[test]
    fn a_passing_word_beats_a_failing_one() {
        let grades = vec![
            grade("NZAL", Some("a"), 1, false),
            grade("ZAL", Some("b"), 1, false),
        ];
        assert_eq!(shown_grade(&grades).unwrap().symbol.as_deref(), Some("ZAL"));
    }

    #[test]
    fn a_course_without_grades_has_no_tile() {
        let view = build(
            &term(vec![course("X", "Pusty", Some(3.0), vec![])]),
            Lang::Pl,
        );
        assert!(view.rows[0].grade.is_none());
        assert!(!view.rows[0].passed);
        assert_eq!(view.graded, 0);
        assert_eq!(view.summary.ects_total, 3.0);
        assert_eq!(view.summary.ects_earned, 0.0);
    }

    #[test]
    fn failing_grades_get_the_fail_tile() {
        let grades = vec![grade("2,0", Some("a"), 1, true)];
        let view = build(&term(vec![course("X", "A", Some(4.0), grades)]), Lang::Pl);
        let g = view.rows[0].grade.as_ref().unwrap();
        assert_eq!(g.tile, TileKind::Fail);
        assert!(!view.rows[0].passed);
    }

    #[test]
    fn the_summary_uses_the_core_averages_and_sums_ects() {
        let courses = vec![
            course("A", "A", Some(5.0), vec![grade("3,5", Some("a"), 1, true)]),
            course("B", "B", Some(6.0), vec![grade("4,5", Some("b"), 1, true)]),
            course("C", "C", Some(2.0), vec![grade("2,0", Some("c"), 1, true)]),
            course("D", "D", None, vec![]),
        ];
        let view = build(&term(courses), Lang::Pl);
        // (3.5 + 4.5 + 2.0) / 3
        assert_eq!(view.summary.average.as_deref(), Some("3,33"));
        // (3.5*5 + 4.5*6 + 2.0*2) / 13 = 3.73
        assert_eq!(view.summary.weighted.as_deref(), Some("3,73"));
        assert_eq!(view.summary.ects_total, 13.0);
        assert_eq!(view.summary.ects_earned, 11.0);
        assert_eq!(view.graded, 3);
    }

    #[test]
    fn english_uses_a_decimal_point() {
        let courses = vec![course(
            "A",
            "A",
            Some(5.0),
            vec![grade("3,5", Some("a"), 1, true)],
        )];
        let view = build(&term(courses), Lang::En);
        assert_eq!(view.summary.average.as_deref(), Some("3.50"));
    }

    #[test]
    fn the_best_grade_is_marked_unless_all_are_equal() {
        let rows = |symbols: &[&str]| {
            let courses = symbols
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    course(
                        &format!("C{i}"),
                        "A",
                        Some(1.0),
                        vec![grade(s, Some(&format!("e{i}")), 1, true)],
                    )
                })
                .collect();
            build(&term(courses), Lang::Pl).rows
        };
        let mixed = rows(&["3,5", "5,0", "4,0"]);
        let tiles: Vec<_> = mixed
            .iter()
            .map(|r| r.grade.as_ref().unwrap().tile)
            .collect();
        assert_eq!(tiles, [TileKind::Normal, TileKind::Best, TileKind::Normal]);
        let equal = rows(&["4,0", "4,0"]);
        assert!(
            equal
                .iter()
                .all(|r| r.grade.as_ref().unwrap().tile == TileKind::Normal)
        );
        // A failing grade is never "best".
        let failing = rows(&["2,0", "2,0", "3,0", "4,0"]);
        assert_eq!(failing[3].grade.as_ref().unwrap().tile, TileKind::Best);
        assert_eq!(failing[2].grade.as_ref().unwrap().tile, TileKind::Normal);
        assert_eq!(failing[0].grade.as_ref().unwrap().tile, TileKind::Fail);
    }

    #[test]
    fn five_with_an_exclamation_mark_is_the_best_grade() {
        let courses = vec![
            course("A", "A", Some(1.0), vec![grade("5,0", Some("a"), 1, true)]),
            course("B", "B", Some(1.0), vec![grade("5!", Some("b"), 1, true)]),
        ];
        let rows = build(&term(courses), Lang::Pl).rows;
        assert_eq!(rows[0].grade.as_ref().unwrap().tile, TileKind::Normal);
        assert_eq!(rows[1].grade.as_ref().unwrap().tile, TileKind::Best);
    }

    #[test]
    fn forms_are_unique_and_tagged() {
        assert_eq!(
            form_tag("cw"),
            Some(FormTag {
                code: "ĆW".into(),
                kind: Kind::Other
            })
        );
        assert_eq!(form_tag("WYK").unwrap().kind, Kind::Lecture);
        assert_eq!(form_tag("lab").unwrap().kind, Kind::Lab);
        assert_eq!(form_tag("EGZ").unwrap().kind, Kind::Exam);
        assert_eq!(form_tag("  "), None);
        let unit = |id: &str| UnitGrades {
            unit_id: id.into(),
            classtype_id: Some(id.into()),
            classtype_name: LangDict::default(),
            grades: vec![],
        };
        let mut c = course("A", "A", None, vec![]);
        c.units = vec![unit("WYK"), unit("LAB"), unit("WYK")];
        let codes: Vec<_> = forms_of(&c).into_iter().map(|f| f.code).collect();
        assert_eq!(codes, ["WYK", "LAB"]);
    }

    #[test]
    fn the_detail_explains_a_retake() {
        let mut passing = grade("3,0", Some("e1"), 2, true);
        passing.description = dict("dostateczny");
        passing.date_acquisition = NaiveDate::from_ymd_opt(2026, 7, 7)
            .unwrap()
            .and_hms_opt(10, 0, 0);
        let grades = vec![grade("2,0", Some("e1"), 1, true), passing];
        let mut c = course("RI-2-FIZ", "Fizyka", Some(5.0), grades);
        c.units = vec![UnitGrades {
            unit_id: "1".into(),
            classtype_id: Some("LAB".into()),
            classtype_name: dict("Laboratorium"),
            grades: vec![{
                let mut g = grade("4,0", None, 1, false);
                g.description = dict("Zaliczenie na ocenę");
                g.comment = Some("dobra robota".into());
                g
            }],
        }];
        let d = detail(&c, "2025/26-L", Lang::Pl);
        assert_eq!(d.name, "Fizyka");
        assert_eq!(d.meta, "2025/26-L · 5 ECTS · RI-2-FIZ");
        assert_eq!(d.big, Some(("3,0".into(), TileKind::Normal)));
        assert_eq!(d.caption, "Ocena końcowa · dostateczny");
        assert_eq!(d.sub.as_deref(), Some("7 lip 2026"));
        assert_eq!(
            d.note.as_deref(),
            Some("I termin niezaliczony (2,0). Do średniej liczy się ocena końcowa 3,0.")
        );
        assert_eq!(d.groups.len(), 2);
        assert_eq!(d.groups[0].entries.len(), 2);
        assert_eq!(d.groups[0].entries[0].label, "I termin");
        assert_eq!(d.groups[0].entries[0].tile, TileKind::Fail);
        assert_eq!(d.groups[1].title, "Laboratorium");
        assert_eq!(d.groups[1].tag.as_ref().unwrap().code, "LAB");
        assert_eq!(d.groups[1].entries[0].label, "Zaliczenie na ocenę");
        assert_eq!(d.groups[1].entries[0].meta, "dobra robota");
    }

    #[test]
    fn the_detail_of_an_ungraded_course_says_so() {
        let c = course("X", "Pusty", None, vec![]);
        let d = detail(&c, "2025/26-L", Lang::En);
        assert_eq!(d.big, None);
        assert_eq!(d.caption, "No final grade yet");
        assert!(d.groups.is_empty());
        assert_eq!(d.meta, "2025/26-L · X");
    }

    #[test]
    fn a_grade_that_does_not_count_says_so() {
        let c = course("X", "A", Some(2.0), vec![grade("4,0", Some("a"), 1, false)]);
        let d = detail(&c, "T", Lang::En);
        // Not counting: shown through the fallback, flagged in the sub line.
        assert_eq!(d.sub.as_deref(), Some("Not counted in the average"));
    }

    #[test]
    fn names_fall_back_to_the_course_code() {
        let mut c = course("RI-1", "", None, vec![]);
        c.name = LangDict::default();
        let view = build(&term(vec![c]), Lang::Pl);
        assert_eq!(view.rows[0].name, "RI-1");
        assert_eq!(view.rows[0].ects_text, "\u{2014}");
    }

    #[test]
    fn session_labels() {
        assert_eq!(session_label(1, Lang::Pl), "I termin");
        assert_eq!(session_label(3, Lang::Pl), "III termin");
        assert_eq!(session_label(2, Lang::En), "Attempt 2");
        assert_eq!(session_label(0, Lang::Pl), "I termin");
    }
}
