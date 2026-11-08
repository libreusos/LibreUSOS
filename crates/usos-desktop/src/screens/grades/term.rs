//! The term picker Oceny and Sprawdziany share: which terms can be picked, where a term's data
//! comes from (the snapshot for the current term, the on-demand cache for the rest) and the
//! chip with its drop-down.

use std::collections::HashMap;

use chrono::NaiveDate;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, deferred, div, px,
};
use usos_core::Term;
use usos_store::repo::{CourseTestsResult, TermGradesResult};

use crate::assets::icons;
use crate::components::{MenuChip, MenuItem, MenuList};
use crate::i18n::Lang;
use crate::state::{Snapshot, TermData, TermLoad};

/// How many past terms the picker offers at most.
const MAX_TERMS: usize = 12;

/// Where a term is relative to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermStatus {
    Past,
    Current,
    Upcoming,
}

/// A term runs from its start to its finish date (the end of the exam session) when it has one.
pub fn status(term: &Term, today: NaiveDate) -> TermStatus {
    let end = term.finish_date.unwrap_or(term.end_date).max(term.end_date);
    if today < term.start_date {
        TermStatus::Upcoming
    } else if today > end {
        TermStatus::Past
    } else {
        TermStatus::Current
    }
}

/// "trwa" / "zakończony" / "przyszły".
pub fn status_word(status: TermStatus, lang: Lang) -> &'static str {
    match (status, lang) {
        (TermStatus::Current, Lang::Pl) => "trwa",
        (TermStatus::Past, Lang::Pl) => "zakończony",
        (TermStatus::Upcoming, Lang::Pl) => "przyszły",
        (TermStatus::Current, Lang::En) => "current",
        (TermStatus::Past, Lang::En) => "ended",
        (TermStatus::Upcoming, Lang::En) => "upcoming",
    }
}

/// The terms the picker lists: those that have started, newest first, at most [`MAX_TERMS`].
/// `current` (the snapshot's term) is always included, even when it has not started yet.
pub fn pickable(terms: &[Term], current: Option<&Term>, today: NaiveDate) -> Vec<Term> {
    let mut list: Vec<Term> = terms
        .iter()
        .filter(|t| t.start_date <= today || current.is_some_and(|c| c.id == t.id))
        .cloned()
        .collect();
    list.sort_by(|a, b| b.start_date.cmp(&a.start_date).then(a.id.cmp(&b.id)));
    list.dedup_by(|a, b| a.id == b.id);
    list.truncate(MAX_TERMS);
    if let Some(current) = current
        && !list.iter().any(|t| t.id == current.id)
    {
        list.insert(0, current.clone());
    }
    list
}

/// The term to show: the user's pick when it is still on offer, else the current one.
pub fn effective<'a>(
    options: &'a [Term],
    current: Option<&Term>,
    choice: Option<&str>,
) -> Option<&'a Term> {
    choice
        .and_then(|id| options.iter().find(|t| t.id == id))
        .or_else(|| current.and_then(|c| options.iter().find(|t| t.id == c.id)))
        .or_else(|| options.first())
}

/// The picker's options and the term shown, for a snapshot and the user's pick.
#[derive(Debug, Clone, PartialEq)]
pub struct TermView {
    pub options: Vec<Term>,
    pub shown: Option<Term>,
}

pub fn resolve(data: &Snapshot, choice: Option<&str>, today: NaiveDate) -> TermView {
    let all: &[Term] = data.terms.as_ref().map_or(&[], |r| r.terms.as_slice());
    let options = pickable(all, data.term.as_ref(), today);
    let shown = effective(&options, data.term.as_ref(), choice).cloned();
    TermView { options, shown }
}

/// What is known about one part (grades or tests) of a term.
#[derive(Debug, PartialEq)]
pub enum Slot<'a, T> {
    Ready(&'a T),
    /// The fetch is running.
    Loading,
    /// Nothing is known and nothing is on its way: ask for the term (`AppState::load_term`).
    Missing,
    /// The fetch ended with nothing.
    Unavailable,
}

fn is_current(data: &Snapshot, term_id: &str) -> bool {
    data.term.as_ref().is_some_and(|t| t.id == term_id)
}

fn slot<'a, T>(
    data: &'a Snapshot,
    cache: &'a HashMap<String, TermData>,
    term_id: &str,
    syncing: bool,
    from_snapshot: impl Fn(&'a Snapshot) -> Option<&'a T>,
    from_cache: impl Fn(&'a TermData) -> Option<&'a T>,
) -> Slot<'a, T> {
    if is_current(data, term_id) {
        return match from_snapshot(data) {
            Some(value) => Slot::Ready(value),
            // The first sync has not brought the term's data yet; nothing failed so far.
            None if syncing => Slot::Loading,
            None => Slot::Unavailable,
        };
    }
    match cache.get(term_id) {
        None => Slot::Missing,
        Some(entry) if entry.load == TermLoad::Loading => Slot::Loading,
        Some(entry) => from_cache(entry).map_or(Slot::Unavailable, Slot::Ready),
    }
}

/// Whether `term_id` (not the current term) has to be fetched: never asked for, or stale.
pub fn needs_load(data: &Snapshot, cache: &HashMap<String, TermData>, term_id: &str) -> bool {
    !is_current(data, term_id) && cache.get(term_id).is_none_or(|entry| entry.stale)
}

/// The grades of `term_id`.
pub fn grades_slot<'a>(
    data: &'a Snapshot,
    cache: &'a HashMap<String, TermData>,
    term_id: &str,
    syncing: bool,
) -> Slot<'a, TermGradesResult> {
    slot(
        data,
        cache,
        term_id,
        syncing,
        |d| d.grades.as_ref(),
        |c| c.grades.as_ref(),
    )
}

/// The tests of `term_id`.
pub fn tests_slot<'a>(
    data: &'a Snapshot,
    cache: &'a HashMap<String, TermData>,
    term_id: &str,
    syncing: bool,
) -> Slot<'a, CourseTestsResult> {
    slot(
        data,
        cache,
        term_id,
        syncing,
        |d| d.tests.as_ref(),
        |c| c.tests.as_ref(),
    )
}

/// The picker's rows: the term id with its status word, the shown one selected.
pub fn menu_items(
    options: &[Term],
    selected: Option<&str>,
    today: NaiveDate,
    lang: Lang,
) -> Vec<MenuItem> {
    options
        .iter()
        .map(|term| MenuItem {
            label: term.id.clone().into(),
            detail: Some(status_word(status(term, today), lang).into()),
            selected: selected == Some(term.id.as_str()),
        })
        .collect()
}

/// The chip text: "2025/26-L · trwa".
pub fn chip_label(term: &Term, today: NaiveDate, lang: Lang) -> String {
    format!(
        "{} \u{b7} {}",
        term.id,
        status_word(status(term, today), lang)
    )
}

/// The chip with its drop-down. `on_toggle` opens or closes it; `on_pick` gets the row index.
/// The caller also adds [`scrim`] to its root so that a click elsewhere closes the list.
pub fn picker(
    id: &'static str,
    label: impl Into<SharedString>,
    items: Vec<MenuItem>,
    open: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .relative()
        .flex_none()
        .child(
            MenuChip::new(id, label)
                .icon(icons::GRADES)
                .open(open)
                .on_click(on_toggle),
        )
        .when(open, |d| {
            d.child(
                deferred(
                    div()
                        .absolute()
                        .top(px(36.0))
                        .left_0()
                        .child(MenuList::new("term-menu", items).on_pick(on_pick)),
                )
                .with_priority(2),
            )
        })
        .into_any_element()
}

/// A transparent layer under the open list that closes it on click.
pub fn scrim(on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> AnyElement {
    deferred(
        div()
            .id("term-scrim")
            .absolute()
            .inset_0()
            .on_click(on_close),
    )
    .with_priority(1)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use usos_core::LangDict;

    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn term(id: &str, start: NaiveDate, end: NaiveDate, finish: Option<NaiveDate>) -> Term {
        Term {
            id: id.into(),
            name: LangDict::default(),
            start_date: start,
            end_date: end,
            finish_date: finish,
            order_key: None,
            is_active: None,
        }
    }

    fn terms() -> Vec<Term> {
        vec![
            term(
                "2024/25-Z",
                d(2024, 10, 1),
                d(2025, 2, 16),
                Some(d(2025, 2, 28)),
            ),
            term(
                "2025/26-L",
                d(2026, 2, 17),
                d(2026, 6, 30),
                Some(d(2026, 9, 30)),
            ),
            term(
                "2025/26-Z",
                d(2025, 10, 1),
                d(2026, 2, 15),
                Some(d(2026, 2, 28)),
            ),
            term(
                "2026/27-Z",
                d(2026, 10, 1),
                d(2027, 2, 14),
                Some(d(2027, 2, 28)),
            ),
            term("2026/27-L", d(2027, 2, 15), d(2027, 6, 30), None),
        ]
    }

    #[test]
    fn status_follows_the_dates() {
        let t = &terms()[3];
        assert_eq!(status(t, d(2026, 9, 30)), TermStatus::Upcoming);
        assert_eq!(status(t, d(2026, 10, 1)), TermStatus::Current);
        assert_eq!(status(t, d(2027, 2, 28)), TermStatus::Current);
        assert_eq!(status(t, d(2027, 3, 1)), TermStatus::Past);
        // Without a finish date the end date closes the term.
        let no_finish = &terms()[4];
        assert_eq!(status(no_finish, d(2027, 6, 30)), TermStatus::Current);
        assert_eq!(status(no_finish, d(2027, 7, 1)), TermStatus::Past);
    }

    #[test]
    fn status_words() {
        assert_eq!(status_word(TermStatus::Current, Lang::Pl), "trwa");
        assert_eq!(status_word(TermStatus::Past, Lang::Pl), "zakończony");
        assert_eq!(status_word(TermStatus::Current, Lang::En), "current");
    }

    #[test]
    fn the_picker_lists_started_terms_newest_first() {
        let all = terms();
        let today = d(2026, 11, 8);
        let current = all[3].clone();
        let ids: Vec<_> = pickable(&all, Some(&current), today)
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["2026/27-Z", "2025/26-L", "2025/26-Z", "2024/25-Z"]);
    }

    #[test]
    fn the_current_term_is_always_offered() {
        let all = terms();
        // The current term has not started (a break before the semester).
        let current = all[3].clone();
        let ids: Vec<_> = pickable(&all, Some(&current), d(2026, 9, 20))
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids[0], "2026/27-Z");
        assert!(!ids.contains(&"2026/27-L".to_owned()));
        // With no current term only started ones are listed.
        assert_eq!(pickable(&all, None, d(2026, 9, 20)).len(), 3);
    }

    #[test]
    fn the_list_is_capped() {
        let many: Vec<Term> = (0..30)
            .map(|i| {
                term(
                    &format!("T{i:02}"),
                    d(2000 + i, 10, 1),
                    d(2000 + i, 12, 1),
                    None,
                )
            })
            .collect();
        assert_eq!(pickable(&many, None, d(2040, 1, 1)).len(), MAX_TERMS);
    }

    #[test]
    fn the_pick_wins_unless_it_vanished() {
        let all = pickable(&terms(), Some(&terms()[3]), d(2026, 11, 8));
        let current = terms()[3].clone();
        assert_eq!(
            effective(&all, Some(&current), Some("2025/26-L")).map(|t| t.id.as_str()),
            Some("2025/26-L")
        );
        assert_eq!(
            effective(&all, Some(&current), Some("1999/00-Z")).map(|t| t.id.as_str()),
            Some("2026/27-Z")
        );
        assert_eq!(
            effective(&all, None, None).map(|t| t.id.as_str()),
            Some("2026/27-Z")
        );
        assert_eq!(effective(&[], None, None), None);
    }

    #[test]
    fn data_comes_from_the_snapshot_for_the_current_term_and_the_cache_for_others() {
        let current = terms()[3].clone();
        let data = Snapshot {
            term: Some(current),
            ..Snapshot::default()
        };
        let mut cache: HashMap<String, TermData> = HashMap::new();
        // Current term without grades: nothing is on its way.
        assert_eq!(
            grades_slot(&data, &cache, "2026/27-Z", false),
            Slot::Unavailable
        );
        // Unless a sync is still running.
        assert_eq!(grades_slot(&data, &cache, "2026/27-Z", true), Slot::Loading);
        // Another term never asked for.
        assert_eq!(
            grades_slot(&data, &cache, "2025/26-L", false),
            Slot::Missing
        );
        assert!(needs_load(&data, &cache, "2025/26-L"));
        assert!(!needs_load(&data, &cache, "2026/27-Z"));
        cache.insert("2025/26-L".into(), TermData::loading());
        assert!(!needs_load(&data, &cache, "2025/26-L"));
        cache.get_mut("2025/26-L").unwrap().stale = true;
        assert!(needs_load(&data, &cache, "2025/26-L"));
        cache.get_mut("2025/26-L").unwrap().stale = false;
        assert_eq!(
            grades_slot(&data, &cache, "2025/26-L", false),
            Slot::Loading
        );
        cache.insert(
            "2025/26-L".into(),
            TermData {
                load: TermLoad::Failed,
                ..TermData::loading()
            },
        );
        assert_eq!(
            tests_slot(&data, &cache, "2025/26-L", false),
            Slot::Unavailable
        );
    }

    #[test]
    fn menu_rows_mark_the_selection() {
        let all = pickable(&terms(), Some(&terms()[3]), d(2026, 11, 8));
        let items = menu_items(&all, Some("2025/26-L"), d(2026, 11, 8), Lang::Pl);
        assert_eq!(items.len(), 4);
        assert!(items[1].selected);
        assert!(!items[0].selected);
        assert_eq!(items[0].detail.as_deref(), Some("trwa"));
        assert_eq!(items[1].detail.as_deref(), Some("zakończony"));
        assert_eq!(
            chip_label(&all[0], d(2026, 11, 8), Lang::Pl),
            "2026/27-Z \u{b7} trwa"
        );
    }
}
