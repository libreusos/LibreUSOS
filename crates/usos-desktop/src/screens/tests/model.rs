//! The view-model of Sprawdziany: pure functions from a term's course tests to the rows the
//! table draws. Points are the sum of the lecturer's entries as USOS reports them; nothing here
//! converts them into grades.

use usos_core::crstests::{CourseTests, NodeKind, TestNode, TestRoot};

use crate::components::TileKind;
use crate::i18n::{Counted, Lang, fraction, number};
use crate::screens::grades::model::{FormTag, form_tag};
use crate::view_util::{day_month, fold, pick};

/// What the result column of a row shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// "23,5 / 30" with the bar's fill (0 when the maximum is unknown or zero).
    Points { text: String, fill: f32 },
    /// A grade node's grade ("4,0", "ZAL").
    Grade { symbol: String, tile: TileKind },
    /// The lecturer hid the results ("Wyniki ukryte przez prowadzącego").
    Hidden,
    /// The test is only for some class groups ("Dostępne tylko dla wybranych grup").
    GroupsOnly,
    /// Nothing was entered yet ("nieocenione").
    Ungraded,
}

impl Outcome {
    /// Whether the row shows points the student can read.
    pub fn has_points(&self) -> bool {
        matches!(self, Outcome::Points { .. })
    }
}

/// A node below a test, flattened with its depth.
#[derive(Debug, Clone, PartialEq)]
pub struct ChildRow {
    pub id: String,
    pub depth: usize,
    pub label: String,
    /// The lecturer's description of the node, when it has one.
    pub detail: Option<String>,
    pub last_changed: Option<String>,
    pub outcome: Outcome,
}

/// One test of the term.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRow {
    pub id: String,
    pub course_name: String,
    /// "Laboratorium · suma punktów" or "Zadania laboratoryjne · 4 z 6 ocenione".
    pub subtitle: String,
    pub tag: Option<FormTag>,
    pub last_changed: Option<String>,
    pub outcome: Outcome,
    pub children: Vec<ChildRow>,
}

impl TestRow {
    /// A row opens when it has nodes to show and its results are readable.
    pub fn expandable(&self) -> bool {
        !self.children.is_empty() && !matches!(self.outcome, Outcome::Hidden | Outcome::GroupsOnly)
    }
}

/// Everything the table draws.
#[derive(Debug, Clone, PartialEq)]
pub struct TestsView {
    pub rows: Vec<TestRow>,
    /// Rows showing points.
    pub with_points: usize,
    /// Tests USOS refused to show (the root ids), as a count.
    pub skipped: usize,
}

const WITH_POINTS: Counted = Counted::new(
    ["sprawdzian", "sprawdziany", "sprawdzianów"],
    ["test", "tests"],
);

/// "3\u{a0}sprawdziany z punktami" / "3 tests with points".
pub fn points_caption(count: usize, lang: Lang) -> String {
    let n = count as u64;
    match lang {
        Lang::Pl => format!("{} z punktami", WITH_POINTS.format(n, lang)),
        Lang::En => format!("{} with points", WITH_POINTS.format(n, lang)),
    }
}

/// The class form hinted by a test's name: "Laboratorium" is a LAB, "Ćwiczenia" a ĆW, ... A name
/// that says nothing gets no tag (a guess would mislead).
pub fn tag_for(name: &str) -> Option<FormTag> {
    let name = fold(name);
    let id = if name.contains("laborator") {
        "LAB"
    } else if name.contains("cwicz") {
        "CW"
    } else if name.contains("lektorat") {
        "LEK"
    } else if name.contains("wyklad") || name.contains("lecture") {
        "WYK"
    } else if name.contains("egzamin") || name.contains("exam") {
        "EGZ"
    } else if name.contains("kolokwium") || name.contains("quiz") {
        "KOL"
    } else {
        return None;
    };
    form_tag(id)
}

fn fill(points: f64, max: Option<f64>) -> f32 {
    match max {
        Some(max) if max > 0.0 => (points / max).clamp(0.0, 1.0) as f32,
        _ => 0.0,
    }
}

fn outcome(node: &TestNode, lang: Lang) -> Outcome {
    if node.hidden || node.results_visible == Some(false) {
        return Outcome::Hidden;
    }
    if node.kind == NodeKind::Grade || (node.points.is_none() && node.grade.is_some()) {
        return match node
            .grade
            .as_ref()
            .and_then(|g| g.symbol.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(symbol) => Outcome::Grade {
                symbol: symbol.to_owned(),
                tile: if node.grade.as_ref().and_then(|g| g.passes) == Some(false) {
                    TileKind::Fail
                } else {
                    TileKind::Normal
                },
            },
            None => Outcome::Ungraded,
        };
    }
    match node.points {
        Some(points) => Outcome::Points {
            text: match node.points_max {
                Some(max) => fraction(points, max, lang),
                None => number(points, 2, lang),
            },
            fill: fill(points, node.points_max),
        },
        None => Outcome::Ungraded,
    }
}

fn changed(node: &TestNode, lang: Lang) -> Option<String> {
    node.last_changed.map(|at| day_month(at.date(), lang))
}

/// The nodes below `node`, depth first, each with its depth (the root's children are depth 1).
fn flatten(node: &TestNode, depth: usize, lang: Lang, out: &mut Vec<ChildRow>) {
    for child in &node.children {
        let detail = pick(&child.description, lang);
        out.push(ChildRow {
            id: child.id.clone(),
            depth,
            label: pick(&child.name, lang),
            detail: (!detail.is_empty()).then_some(detail),
            last_changed: changed(child, lang),
            outcome: outcome(child, lang),
        });
        if !child.hidden {
            flatten(child, depth + 1, lang, out);
        }
    }
}

/// The readable leaves below `node` (tasks and grade nodes) as (graded, total).
fn leaf_counts(node: &TestNode) -> (usize, usize) {
    node.children
        .iter()
        .filter(|c| !c.hidden && c.results_visible != Some(false))
        .map(|c| {
            if c.children.is_empty() {
                let graded = c.points.is_some() || c.grade.is_some();
                (usize::from(graded), 1)
            } else {
                leaf_counts(c)
            }
        })
        .fold((0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1))
}

const SUM_OF_POINTS: [&str; 2] = ["suma punktów", "sum of points"];

fn subtitle(root: &TestNode, lang: Lang) -> String {
    let name = pick(&root.name, lang);
    let (graded, total) = leaf_counts(root);
    let extra = if total > 1 && graded < total {
        Some(match lang {
            Lang::Pl => format!("{graded} z {total} ocenione"),
            Lang::En => format!("{graded} of {total} graded"),
        })
    } else if total > 1 {
        Some(SUM_OF_POINTS[usize::from(lang == Lang::En)].to_owned())
    } else {
        None
    };
    match (name.is_empty(), extra) {
        (true, None) => String::new(),
        (true, Some(extra)) => extra,
        (false, None) => name,
        (false, Some(extra)) => format!("{name} \u{b7} {extra}"),
    }
}

fn row(test: &TestRoot, lang: Lang) -> TestRow {
    let root = &test.root;
    let outcome = if test.limited_to_groups {
        Outcome::GroupsOnly
    } else {
        outcome(root, lang)
    };
    let readable = !matches!(outcome, Outcome::Hidden | Outcome::GroupsOnly);
    let mut children = Vec::new();
    if readable {
        flatten(root, 1, lang, &mut children);
    }
    let course_name = {
        let name = pick(&test.course_name, lang);
        if name.is_empty() {
            test.course_id.clone()
        } else {
            name
        }
    };
    TestRow {
        id: root.id.clone(),
        course_name,
        subtitle: subtitle(root, lang),
        tag: tag_for(&pick(&root.name, lang)),
        last_changed: if readable { changed(root, lang) } else { None },
        outcome,
        children,
    }
}

/// Rows with readable results come first (the most recently changed on top), then the tests whose
/// results are hidden or limited to groups, each keeping USOS's order.
pub fn build(tests: &CourseTests, lang: Lang) -> TestsView {
    let mut rows: Vec<(Option<chrono::NaiveDateTime>, TestRow)> = tests
        .tests
        .iter()
        .map(|t| (t.root.last_changed, row(t, lang)))
        .collect();
    let readable = |r: &TestRow| !matches!(r.outcome, Outcome::Hidden | Outcome::GroupsOnly);
    rows.sort_by(|(a_at, a), (b_at, b)| {
        readable(b)
            .cmp(&readable(a))
            .then_with(|| match (readable(a), readable(b)) {
                (true, true) => b_at.cmp(a_at),
                _ => std::cmp::Ordering::Equal,
            })
    });
    let rows: Vec<TestRow> = rows.into_iter().map(|(_, r)| r).collect();
    let with_points = rows.iter().filter(|r| r.outcome.has_points()).count();
    TestsView {
        rows,
        with_points,
        skipped: tests.skipped.len(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use usos_core::LangDict;
    use usos_core::crstests::NodeGrade;

    use super::*;

    fn dict(pl: &str) -> LangDict {
        LangDict {
            pl: Some(pl.into()),
            en: None,
        }
    }

    fn at(day: u32) -> Option<NaiveDateTime> {
        NaiveDate::from_ymd_opt(2026, 11, day).and_then(|d| d.and_hms_opt(12, 0, 0))
    }

    fn task(id: &str, name: &str, points: Option<f64>, max: f64, day: u32) -> TestNode {
        TestNode {
            id: id.into(),
            kind: NodeKind::Task,
            name: dict(name),
            points,
            points_max: Some(max),
            last_changed: if points.is_some() { at(day) } else { None },
            ..TestNode::default()
        }
    }

    fn root_of(id: &str, course: &str, name: &str, children: Vec<TestNode>) -> TestRoot {
        let points = usos_core::crstests::sum_points(&children);
        let max = usos_core::crstests::sum_max(&children);
        let last = children.iter().filter_map(|c| c.last_changed).max();
        TestRoot {
            course_id: format!("C-{id}"),
            course_name: dict(course),
            term_id: "2026/27-Z".into(),
            limited_to_groups: false,
            root: TestNode {
                id: id.into(),
                kind: NodeKind::Root,
                name: dict(name),
                points,
                points_max: max,
                last_changed: last,
                children,
                ..TestNode::default()
            },
        }
    }

    fn view(roots: Vec<TestRoot>) -> TestsView {
        build(
            &CourseTests {
                tests: roots,
                skipped: Vec::new(),
            },
            Lang::Pl,
        )
    }

    #[test]
    fn points_read_as_a_fraction_with_a_fill() {
        let test = root_of(
            "r1",
            "Bazy danych",
            "Laboratorium",
            vec![
                task("a", "zad. 1", Some(11.5), 15.0, 5),
                task("b", "zad. 2", Some(12.0), 15.0, 3),
            ],
        );
        let v = view(vec![test]);
        let row = &v.rows[0];
        assert_eq!(
            row.outcome,
            Outcome::Points {
                text: "23,5 / 30".into(),
                fill: (23.5_f64 / 30.0) as f32
            }
        );
        assert_eq!(row.subtitle, "Laboratorium \u{b7} suma punktów");
        assert_eq!(row.last_changed.as_deref(), Some("5 lis"));
        assert_eq!(v.with_points, 1);
    }

    #[test]
    fn partial_points_count_the_graded_tasks() {
        let mut tasks: Vec<TestNode> = (1..=4)
            .map(|i| task(&format!("t{i}"), &format!("zad. {i}"), Some(8.0), 10.0, 3))
            .collect();
        tasks.push(task("t5", "zad. 5", None, 10.0, 0));
        tasks.push(task("t6", "zad. 6", None, 10.0, 0));
        let v = view(vec![root_of(
            "r2",
            "Algorytmy",
            "Zadania laboratoryjne",
            tasks,
        )]);
        let row = &v.rows[0];
        assert_eq!(row.subtitle, "Zadania laboratoryjne \u{b7} 4 z 6 ocenione");
        assert!(row.outcome.has_points());
        assert_eq!(row.children.len(), 6);
        assert_eq!(row.children[4].outcome, Outcome::Ungraded);
        assert_eq!(row.children[0].depth, 1);
        assert!(row.expandable());
    }

    #[test]
    fn english_subtitle_and_caption() {
        let tasks = vec![
            task("a", "Task 1", Some(1.0), 2.0, 2),
            task("b", "Task 2", None, 2.0, 0),
        ];
        let v = build(
            &CourseTests {
                tests: vec![root_of("r", "Algorithms", "Lab tasks", tasks)],
                skipped: Vec::new(),
            },
            Lang::En,
        );
        assert_eq!(v.rows[0].subtitle, "Lab tasks \u{b7} 1 of 2 graded");
        assert_eq!(points_caption(1, Lang::En), "1\u{a0}test with points");
        assert_eq!(points_caption(3, Lang::Pl), "3\u{a0}sprawdziany z punktami");
        assert_eq!(
            points_caption(5, Lang::Pl),
            "5\u{a0}sprawdzianów z punktami"
        );
    }

    #[test]
    fn hidden_results_show_no_points_and_no_children() {
        let mut test = root_of(
            "h",
            "Programowanie obiektowe",
            "Laboratorium",
            vec![task("a", "zad. 1", Some(5.0), 10.0, 4)],
        );
        test.root.hidden = true;
        test.root.results_visible = Some(false);
        let v = view(vec![test]);
        let row = &v.rows[0];
        assert_eq!(row.outcome, Outcome::Hidden);
        assert!(row.children.is_empty());
        assert!(!row.expandable());
        assert_eq!(row.last_changed, None);
        assert_eq!(v.with_points, 0);
    }

    #[test]
    fn group_limited_tests_say_so() {
        let mut test = root_of("g", "Język angielski B2", "Lektorat", Vec::new());
        test.limited_to_groups = true;
        let v = view(vec![test]);
        assert_eq!(v.rows[0].outcome, Outcome::GroupsOnly);
        assert_eq!(v.rows[0].tag.as_ref().map(|t| t.code.as_str()), Some("LEK"));
        assert!(!v.rows[0].expandable());
    }

    #[test]
    fn a_hidden_child_stays_in_the_list_but_not_in_the_sum() {
        let mut hidden = task("h", "zad. 3", Some(7.0), 10.0, 4);
        hidden.hidden = true;
        hidden.results_visible = Some(false);
        let test = root_of(
            "r",
            "Fizyka",
            "Laboratorium",
            vec![task("a", "zad. 1", Some(6.0), 10.0, 2), hidden],
        );
        let v = view(vec![test]);
        let row = &v.rows[0];
        assert_eq!(
            row.outcome,
            Outcome::Points {
                text: "6 / 10".into(),
                fill: 0.6
            }
        );
        assert_eq!(row.children[1].outcome, Outcome::Hidden);
        // Only the readable leaf counts towards "graded of total".
        assert_eq!(row.subtitle, "Laboratorium");
    }

    #[test]
    fn nested_folders_are_indented() {
        let inner = TestNode {
            id: "f".into(),
            kind: NodeKind::Folder,
            name: dict("Kolokwia"),
            points: Some(15.0),
            points_max: Some(20.0),
            children: vec![task("k1", "Kolokwium 1", Some(15.0), 20.0, 6)],
            last_changed: at(6),
            ..TestNode::default()
        };
        let v = view(vec![root_of("r", "Analiza", "Ćwiczenia", vec![inner])]);
        let depths: Vec<usize> = v.rows[0].children.iter().map(|c| c.depth).collect();
        assert_eq!(depths, vec![1, 2]);
        assert_eq!(v.rows[0].children[0].label, "Kolokwia");
        assert_eq!(v.rows[0].tag.as_ref().map(|t| t.code.as_str()), Some("ĆW"));
    }

    #[test]
    fn grade_nodes_show_the_grade_as_given() {
        let grade = TestNode {
            id: "g".into(),
            kind: NodeKind::Grade,
            name: dict("Ocena"),
            grade: Some(NodeGrade {
                symbol: Some("4,5".into()),
                passes: Some(true),
                ..NodeGrade::default()
            }),
            ..TestNode::default()
        };
        let failing = TestNode {
            grade: Some(NodeGrade {
                symbol: Some("2,0".into()),
                passes: Some(false),
                ..NodeGrade::default()
            }),
            ..grade.clone()
        };
        let v = view(vec![root_of(
            "r",
            "Analiza",
            "Egzamin",
            vec![grade, failing],
        )]);
        let kids = &v.rows[0].children;
        assert_eq!(
            kids[0].outcome,
            Outcome::Grade {
                symbol: "4,5".into(),
                tile: TileKind::Normal
            }
        );
        assert_eq!(
            kids[1].outcome,
            Outcome::Grade {
                symbol: "2,0".into(),
                tile: TileKind::Fail
            }
        );
    }

    #[test]
    fn readable_tests_come_first_newest_change_on_top() {
        let old = root_of(
            "old",
            "A",
            "Kolokwium",
            vec![task("a", "zad", Some(1.0), 2.0, 2)],
        );
        let new = root_of(
            "new",
            "B",
            "Kolokwium",
            vec![task("b", "zad", Some(1.0), 2.0, 9)],
        );
        let mut hidden = root_of("hid", "C", "Laboratorium", Vec::new());
        hidden.root.hidden = true;
        let mut groups = root_of("grp", "D", "Lektorat", Vec::new());
        groups.limited_to_groups = true;
        let v = view(vec![hidden, old, groups, new]);
        let ids: Vec<&str> = v.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["new", "old", "hid", "grp"]);
    }

    #[test]
    fn a_test_without_entries_is_ungraded() {
        let v = view(vec![root_of("e", "Fizyka", "Kolokwium", Vec::new())]);
        assert_eq!(v.rows[0].outcome, Outcome::Ungraded);
        assert!(!v.rows[0].expandable());
        assert_eq!(v.with_points, 0);
    }

    #[test]
    fn tags_come_from_the_test_name() {
        let code = |name: &str| tag_for(name).map(|t| t.code);
        assert_eq!(code("Laboratorium").as_deref(), Some("LAB"));
        assert_eq!(code("Zadania laboratoryjne").as_deref(), Some("LAB"));
        assert_eq!(code("Ćwiczenia").as_deref(), Some("ĆW"));
        assert_eq!(code("Lektorat").as_deref(), Some("LEK"));
        assert_eq!(code("Wykład").as_deref(), Some("WYK"));
        assert_eq!(code("Egzamin"), Some("EGZ".to_owned()));
        assert_eq!(code("Punkty z projektu"), None);
    }

    #[test]
    fn a_missing_maximum_still_shows_the_points() {
        let mut node = task("a", "zad", Some(7.5), 0.0, 3);
        node.points_max = None;
        let test = TestRoot {
            course_id: "C".into(),
            course_name: dict("Fizyka"),
            term_id: "T".into(),
            limited_to_groups: false,
            root: TestNode {
                id: "r".into(),
                kind: NodeKind::Root,
                points: Some(7.5),
                points_max: None,
                children: vec![node],
                ..TestNode::default()
            },
        };
        let v = view(vec![test]);
        assert_eq!(
            v.rows[0].outcome,
            Outcome::Points {
                text: "7,5".into(),
                fill: 0.0
            }
        );
    }

    #[test]
    fn skipped_tests_are_counted() {
        let v = build(
            &CourseTests {
                tests: Vec::new(),
                skipped: vec!["x".into(), "y".into()],
            },
            Lang::Pl,
        );
        assert_eq!(v.skipped, 2);
        assert!(v.rows.is_empty());
    }
}
