//! Fills the cache of a throw-away installation with an invented timetable, for screenshots:
//!
//! `XDG_DATA_HOME=/some/dir cargo test -p usos-desktop -- --ignored seed_demo_cache`
//!
//! Pair it with `USOS_DEMO_NOW=2026-11-10T11:07` (Tuesday, a class running, a clash after it) or
//! `USOS_DEMO_NOW=2026-11-11T09:30` (a public holiday: no classes). Nothing here is real data.

use chrono::{Days, NaiveDate, NaiveDateTime};
use usos_core::{Activity, ActivityKind, Frequency, LangDict, Term, User};
use usos_store::{Store, repo};

fn names(pl: &str, en: &str) -> LangDict {
    LangDict {
        pl: Some(pl.into()),
        en: Some(en.into()),
    }
}

/// One weekly class: weekday (0 = Monday), start, end, type code and names, group, place.
struct Slot {
    day: u64,
    start: (u32, u32),
    end: (u32, u32),
    code: &'static str,
    type_name: (&'static str, &'static str),
    course: (&'static str, &'static str, &'static str),
    group: Option<u32>,
    room: &'static str,
    building: &'static str,
    frequency: Frequency,
}

const LECTURE: (&str, &str) = ("Wykład", "Lecture");
const LAB: (&str, &str) = ("Laboratorium", "Laboratory");
const CLASSES: (&str, &str) = ("Ćwiczenia", "Classes");
const SEMINAR: (&str, &str) = ("Seminarium", "Seminar");

const MATH: (&str, &str, &str) = ("MAT", "Analiza matematyczna", "Calculus");
const PHYSICS: (&str, &str, &str) = ("FIZ", "Fizyka", "Physics");
const CODING: (&str, &str, &str) = ("PRG", "Programowanie", "Programming");
const NETWORKS: (&str, &str, &str) = ("SIE", "Sieci komputerowe", "Computer networks");
const DISCRETE: (&str, &str, &str) = ("DYS", "Matematyka dyskretna", "Discrete mathematics");
const PROJECT: (&str, &str, &str) = ("SEM", "Seminarium dyplomowe", "Diploma seminar");

#[allow(clippy::too_many_arguments)]
fn slot(
    day: u64,
    start: (u32, u32),
    end: (u32, u32),
    code: &'static str,
    type_name: (&'static str, &'static str),
    course: (&'static str, &'static str, &'static str),
    group: Option<u32>,
    room: &'static str,
    frequency: Frequency,
) -> Slot {
    Slot {
        day,
        start,
        end,
        code,
        type_name,
        course,
        group,
        room,
        building: "AEI",
        frequency,
    }
}

fn week() -> Vec<Slot> {
    let every = Frequency::EveryWeek;
    vec![
        slot(
            0,
            (8, 15),
            (9, 45),
            "WYK",
            LECTURE,
            MATH,
            None,
            "Aula C",
            every.clone(),
        ),
        slot(
            0,
            (10, 0),
            (11, 30),
            "CW",
            CLASSES,
            MATH,
            Some(2),
            "216",
            every.clone(),
        ),
        slot(
            1,
            (8, 15),
            (9, 45),
            "WYK",
            LECTURE,
            PHYSICS,
            None,
            "Aula C",
            every.clone(),
        ),
        slot(
            1,
            (10, 15),
            (11, 45),
            "LAB",
            LAB,
            CODING,
            Some(2),
            "12",
            every.clone(),
        ),
        slot(
            1,
            (12, 0),
            (13, 30),
            "CW",
            CLASSES,
            NETWORKS,
            Some(2),
            "216",
            every.clone(),
        ),
        slot(
            1,
            (12, 45),
            (14, 15),
            "SEM",
            SEMINAR,
            PROJECT,
            None,
            "314",
            every.clone(),
        ),
        slot(
            1,
            (15, 0),
            (16, 30),
            "LAB",
            LAB,
            NETWORKS,
            Some(1),
            "108",
            Frequency::EveryFortnight,
        ),
        slot(
            2,
            (10, 0),
            (11, 30),
            "CW",
            CLASSES,
            DISCRETE,
            Some(2),
            "216",
            every.clone(),
        ),
        slot(
            2,
            (12, 0),
            (13, 30),
            "WYK",
            LECTURE,
            DISCRETE,
            None,
            "Aula C",
            every.clone(),
        ),
        slot(
            3,
            (9, 0),
            (10, 30),
            "WYK",
            LECTURE,
            CODING,
            None,
            "Aula C",
            every.clone(),
        ),
        slot(
            3,
            (12, 0),
            (13, 30),
            "CW",
            CLASSES,
            NETWORKS,
            Some(2),
            "216",
            every.clone(),
        ),
        slot(
            3,
            (12, 45),
            (14, 15),
            "SEM",
            SEMINAR,
            PROJECT,
            None,
            "314",
            every.clone(),
        ),
        slot(
            3,
            (14, 30),
            (16, 0),
            "LAB",
            LAB,
            CODING,
            Some(2),
            "12",
            every.clone(),
        ),
        slot(
            4,
            (10, 0),
            (11, 30),
            "WYK",
            LECTURE,
            PHYSICS,
            None,
            "Aula C",
            every.clone(),
        ),
        slot(
            4,
            (12, 0),
            (13, 30),
            "LAB",
            LAB,
            PHYSICS,
            Some(2),
            "7",
            every,
        ),
    ]
}

fn activity(day: NaiveDate, slot: &Slot) -> Activity {
    let at = |(h, m): (u32, u32)| -> NaiveDateTime {
        (day + Days::new(slot.day)).and_hms_opt(h, m, 0).unwrap()
    };
    Activity {
        kind: ActivityKind::Classgroup,
        start: at(slot.start),
        end: at(slot.end),
        name: names(slot.course.1, slot.course.2),
        course_id: Some(slot.course.0.into()),
        course_name: names(slot.course.1, slot.course.2),
        classtype_id: Some(slot.code.into()),
        classtype_name: names(slot.type_name.0, slot.type_name.1),
        group_number: slot.group,
        building_id: Some(slot.building.into()),
        building_name: LangDict::default(),
        room_id: None,
        room_number: Some(slot.room.into()),
        lecturer_ids: vec![],
        unit_id: None,
        frequency: Some(slot.frequency.clone()),
        url: None,
    }
}

#[test]
#[ignore = "writes to the user data directory; run it by hand with XDG_DATA_HOME set"]
fn seed_demo_cache() {
    // Never fall back to the real data directory (`dirs` ignores XDG_DATA_HOME off Linux).
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .expect("set XDG_DATA_HOME to a throw-away directory");
    let path = std::path::PathBuf::from(data_home)
        .join("libreusos")
        .join("cache.sqlite");
    std::fs::create_dir_all(path.parent().unwrap()).expect("data dir");
    let store = Store::open(path.to_str().expect("utf-8 path")).expect("open store");
    let base_url = usos_core::session::DEFAULT_BASE_URL;
    let user = User {
        id: "1".into(),
        first_name: "Jan".into(),
        last_name: "Demo".into(),
    };
    let identity = repo::remember_identity(&store, base_url, user).expect("identity");
    let account = identity.account;

    let term = Term {
        id: "2026Z".into(),
        name: names("Semestr zimowy 2026/27", "Winter semester 2026/27"),
        start_date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2027, 2, 14).unwrap(),
        finish_date: None,
        order_key: None,
        is_active: Some(true),
    };
    store.put_terms(&account, &[term]).expect("terms");

    // Five weeks from Monday 2026-11-02, with a public holiday on Wednesday 11 November.
    let first = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    let holiday = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
    let mut all = Vec::new();
    for week_index in 0..5u64 {
        let monday = first + Days::new(week_index * 7);
        for slot in week() {
            let activity = activity(monday, &slot);
            if activity.start.date() != holiday {
                all.push(activity);
            }
        }
    }
    let to = first + Days::new(5 * 7 - 1);
    store
        .replace_activities(&account, first, to, &all)
        .expect("activities");
}
