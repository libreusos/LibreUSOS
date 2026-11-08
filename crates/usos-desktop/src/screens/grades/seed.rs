//! Adds invented grades and tests to the cache of a throw-away installation, for screenshots. Run
//! it after the timetable seed (it keeps the terms and the account that one made):
//!
//! `XDG_DATA_HOME=/some/dir cargo test -p usos-desktop -- --ignored seed_demo_grades`
//!
//! Nothing here is real data.

use chrono::{NaiveDate, NaiveDateTime};
use usos_core::crstests::{CourseTests, NodeKind, TestNode, TestRoot};
use usos_core::grades::{CourseGrades, Grade, TermGrades, UnitGrades};
use usos_core::{LangDict, Term};
use usos_store::{Store, repo};

fn names(pl: &str, en: &str) -> LangDict {
    LangDict {
        pl: Some(pl.into()),
        en: Some(en.into()),
    }
}

fn at(month: u32, day: u32) -> Option<NaiveDateTime> {
    NaiveDate::from_ymd_opt(2026, month, day).and_then(|d| d.and_hms_opt(12, 0, 0))
}

fn grade(
    session: u32,
    symbol: &str,
    passes: bool,
    counts: bool,
    when: Option<NaiveDateTime>,
) -> Grade {
    Grade {
        exam_id: Some("1".into()),
        session,
        symbol: Some(symbol.into()),
        passes: Some(passes),
        counts_into_average: counts,
        date_modified: when,
        ..Grade::default()
    }
}

/// A unit: its id, the class type (pl, en, code) and the grades.
type Unit<'a> = (&'a str, (&'a str, &'a str, &'a str), Vec<Grade>);

fn course(
    id: &str,
    term: &str,
    name: (&str, &str),
    ects: f64,
    grades: Vec<Grade>,
    units: Vec<Unit>,
) -> CourseGrades {
    CourseGrades {
        course_id: id.into(),
        name: names(name.0, name.1),
        term_id: term.into(),
        ects: Some(ects),
        grades,
        units: units
            .into_iter()
            .map(|(unit_id, kind, grades)| UnitGrades {
                unit_id: unit_id.into(),
                classtype_id: Some(kind.2.into()),
                classtype_name: names(kind.0, kind.1),
                grades,
            })
            .collect(),
    }
}

fn term(
    id: &str,
    name: (&str, &str),
    from: (i32, u32, u32),
    to: (i32, u32, u32),
    active: bool,
) -> Term {
    Term {
        id: id.into(),
        name: names(name.0, name.1),
        start_date: NaiveDate::from_ymd_opt(from.0, from.1, from.2).unwrap(),
        end_date: NaiveDate::from_ymd_opt(to.0, to.1, to.2).unwrap(),
        finish_date: None,
        order_key: None,
        is_active: Some(active),
    }
}

fn task(
    id: &str,
    name: &str,
    points: Option<f64>,
    max: f64,
    when: Option<NaiveDateTime>,
) -> TestNode {
    TestNode {
        id: id.into(),
        kind: NodeKind::Task,
        name: names(name, name),
        points,
        points_max: Some(max),
        results_visible: Some(true),
        last_changed: when,
        ..TestNode::default()
    }
}

fn folder(id: &str, name: (&str, &str), children: Vec<TestNode>) -> TestNode {
    let points: f64 = children.iter().filter_map(|c| c.points).sum();
    let max: f64 = children
        .iter()
        .filter(|c| !c.hidden)
        .filter_map(|c| c.points_max)
        .sum();
    TestNode {
        id: id.into(),
        kind: NodeKind::Folder,
        name: names(name.0, name.1),
        points: Some(points),
        points_max: Some(max),
        results_visible: Some(true),
        last_changed: children.iter().filter_map(|c| c.last_changed).max(),
        children,
        ..TestNode::default()
    }
}

fn root(
    id: &str,
    course_name: (&str, &str),
    test_name: (&str, &str),
    limited: bool,
    children: Vec<TestNode>,
) -> TestRoot {
    let mut node = folder(id, test_name, children);
    node.kind = NodeKind::Root;
    TestRoot {
        course_id: id.into(),
        course_name: names(course_name.0, course_name.1),
        term_id: "2026Z".into(),
        limited_to_groups: limited,
        root: node,
    }
}

#[test]
#[ignore = "writes demo data to the cache under XDG_DATA_HOME"]
fn seed_demo_grades() {
    // Never fall back to the real data directory (`dirs` ignores XDG_DATA_HOME off Linux).
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .expect("set XDG_DATA_HOME to a throw-away directory");
    let path = std::path::PathBuf::from(data_home)
        .join("libreusos")
        .join("cache.sqlite");
    std::fs::create_dir_all(path.parent().unwrap()).expect("data dir");
    let store = Store::open(path.to_str().expect("utf-8 path")).expect("open store");
    let account = repo::cached_identity(&store, usos_core::session::DEFAULT_BASE_URL)
        .expect("run the timetable seed first")
        .account;

    let mut terms = store
        .get_terms(&account)
        .expect("terms")
        .map(|t| t.terms)
        .unwrap_or_default();
    terms.retain(|t| t.id == "2026Z");
    terms.push(term(
        "2026L",
        ("Semestr letni 2025/26", "Summer semester 2025/26"),
        (2026, 2, 16),
        (2026, 9, 30),
        false,
    ));
    terms.push(term(
        "2025Z",
        ("Semestr zimowy 2025/26", "Winter semester 2025/26"),
        (2025, 10, 1),
        (2026, 2, 15),
        false,
    ));
    store.put_terms(&account, &terms).expect("terms");

    let now = TermGrades {
        term_id: "2026Z".into(),
        courses: vec![
            course(
                "MAT1",
                "2026Z",
                ("Analiza matematyczna", "Calculus"),
                6.0,
                vec![grade(1, "4,5", true, true, at(11, 4))],
                vec![
                    (
                        "3",
                        ("Wykład", "Lecture", "WYK"),
                        vec![grade(1, "4,5", true, true, at(11, 4))],
                    ),
                    (
                        "4",
                        ("Ćwiczenia", "Classes", "CW"),
                        vec![grade(1, "4,0", true, true, at(10, 30))],
                    ),
                ],
            ),
            course(
                "FIZ1",
                "2026Z",
                ("Fizyka", "Physics"),
                5.0,
                vec![grade(1, "3,5", true, true, at(10, 28))],
                vec![],
            ),
            course(
                "PRG1",
                "2026Z",
                ("Programowanie", "Programming"),
                7.0,
                vec![grade(1, "4,5", true, true, at(11, 5))],
                vec![
                    (
                        "1",
                        ("Wykład", "Lecture", "WYK"),
                        vec![grade(1, "5", true, true, at(11, 5))],
                    ),
                    (
                        "2",
                        ("Laboratorium", "Laboratory", "LAB"),
                        vec![grade(1, "4,0", true, true, at(11, 3))],
                    ),
                ],
            ),
            course(
                "SIE1",
                "2026Z",
                ("Sieci komputerowe", "Computer networks"),
                4.0,
                vec![],
                vec![],
            ),
            course(
                "DYS1",
                "2026Z",
                ("Matematyka dyskretna", "Discrete mathematics"),
                5.0,
                vec![grade(1, "ZAL", true, false, at(10, 20))],
                vec![],
            ),
            course(
                "SEM1",
                "2026Z",
                ("Seminarium dyplomowe", "Diploma seminar"),
                3.0,
                vec![],
                vec![],
            ),
        ],
        average: Some(4.25),
        weighted_average: Some(4.31),
    };
    store
        .put_term_grades(&account, "2026Z", &now)
        .expect("grades");

    let summer = TermGrades {
        term_id: "2026L".into(),
        courses: vec![
            course(
                "ALG0",
                "2026L",
                (
                    "Algorytmy i struktury danych",
                    "Algorithms and data structures",
                ),
                6.0,
                vec![
                    grade(1, "2,0", false, false, at(2, 20)),
                    grade(2, "3,5", true, true, at(3, 4)),
                ],
                vec![],
            ),
            course(
                "BAZ0",
                "2026L",
                ("Bazy danych", "Databases"),
                5.0,
                vec![grade(1, "5!", true, true, at(2, 11))],
                vec![],
            ),
            course(
                "ANG0",
                "2026L",
                ("Język angielski", "English"),
                2.0,
                vec![grade(1, "ZAL", true, false, at(1, 28))],
                vec![],
            ),
            course(
                "PHY0",
                "2026L",
                ("Fizyka II", "Physics II"),
                5.0,
                vec![grade(1, "4,0", true, true, at(2, 9))],
                vec![],
            ),
        ],
        average: Some(4.25),
        weighted_average: Some(4.19),
    };
    store
        .put_term_grades(&account, "2026L", &summer)
        .expect("grades");

    let tests = CourseTests {
        tests: vec![
            root(
                "PRG1",
                ("Programowanie", "Programming"),
                ("Laboratorium", "Laboratory"),
                false,
                vec![
                    task("p1", "zad. 1 · Listy i stosy", Some(8.0), 10.0, at(10, 13)),
                    task("p2", "zad. 2 · Drzewa BST", Some(9.0), 10.0, at(10, 20)),
                    task("p3", "zad. 3 · Kopce", Some(6.0), 10.0, at(10, 27)),
                    task(
                        "p4",
                        "zad. 4 · Grafy, BFS i DFS",
                        Some(8.0),
                        10.0,
                        at(11, 3),
                    ),
                    task("p5", "zad. 5–6", None, 20.0, None),
                ],
            ),
            root(
                "MAT1",
                ("Analiza matematyczna", "Calculus"),
                ("Kolokwium", "Midterm"),
                false,
                vec![task("k1", "Kolokwium 1", Some(23.5), 30.0, at(11, 5))],
            ),
            root(
                "DYS1",
                ("Matematyka dyskretna", "Discrete mathematics"),
                ("Ćwiczenia", "Classes"),
                false,
                vec![
                    task("d1", "Kolokwium 1", Some(9.5), 12.5, at(10, 29)),
                    task("d2", "Kolokwium 2", Some(8.0), 12.5, at(11, 2)),
                ],
            ),
            // The lecturer hid the whole test.
            {
                let mut hidden = root(
                    "SIE1",
                    ("Sieci komputerowe", "Computer networks"),
                    ("Laboratorium", "Laboratory"),
                    false,
                    vec![task("s1", "Projekt", None, 40.0, None)],
                );
                hidden.root.hidden = true;
                hidden.root.results_visible = Some(false);
                hidden.root.points = None;
                hidden
            },
            root(
                "ANG1",
                ("Język angielski B2", "English B2"),
                ("Lektorat", "Language class"),
                true,
                vec![task("a1", "Test", None, 50.0, None)],
            ),
        ],
        skipped: Vec::new(),
    };
    store
        .put_course_tests(&account, "2026Z", &tests)
        .expect("tests");
}
