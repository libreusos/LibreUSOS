//! The pure part of the Today and Timetable screens: core activities become view-model items,
//! and all the layout and time arithmetic (hour range, overlap lanes, gaps, now/next, week
//! numbers, names of days and months, counts) lives here as plain functions with no GPUI.
//!
//! Times are installation-local `NaiveDateTime`s, as USOS sends them. Arithmetic on them is wall
//! clock arithmetic and is never shifted by a daylight-saving change, which is what a student
//! reads off the timetable.

use chrono::{Datelike, Days, Duration, NaiveDate, NaiveDateTime, Timelike, Weekday};
use usos_core::{Activity, ActivityKind, Frequency, Lang as CoreLang, Term};

use crate::i18n::{Counted, Lang, NBSP, sentence_case};
use crate::theme::Kind;

/// Free time shorter than this is the break between periods, not an "Okienko".
pub const MIN_GAP_MINUTES: i64 = 30;
/// The grid always covers at least 08:00-16:00 (SYSTEM.md section 10, "Hour range").
pub const GRID_DEFAULT_START: u32 = 8;
pub const GRID_DEFAULT_END: u32 = 16;

pub const CLASSES: Counted = Counted::new(["zajęcie", "zajęcia", "zajęć"], ["class", "classes"]);

/// The identity of one class occurrence, stable across syncs (unlike an index into the list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassKey {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub course_id: Option<String>,
    pub classtype_id: Option<String>,
    pub group_number: Option<u32>,
}

/// One class occurrence as the screens draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassItem {
    pub key: ClassKey,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// The USOS course name (or the generic activity name), exactly as USOS sends it.
    pub title: String,
    /// Short kind code: WYK, LAB, ĆW, ...
    pub code: String,
    pub kind: Kind,
    /// The spelled-out class type ("Wykład"), for the detail pane.
    pub type_name: Option<String>,
    /// "s. 314" or "Aula C".
    pub room: Option<String>,
    pub building_id: Option<String>,
    pub building_name: Option<String>,
    pub group: Option<u32>,
    pub frequency: Option<Frequency>,
    pub url: Option<String>,
    /// Indices (in the list this item belongs to) of the items it overlaps.
    pub clashes: Vec<usize>,
}

impl ClassItem {
    /// "s. 216, AEI": the room and the building code.
    pub fn place(&self) -> Option<String> {
        match (&self.room, &self.building_id) {
            (Some(room), Some(building)) => Some(format!("{room}, {building}")),
            (Some(room), None) => Some(room.clone()),
            (None, Some(building)) => Some(building.clone()),
            (None, None) => None,
        }
    }

    pub fn is_past(&self, now: NaiveDateTime) -> bool {
        self.end <= now
    }

    pub fn is_current(&self, now: NaiveDateTime) -> bool {
        self.start <= now && now < self.end
    }

    pub fn minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }

    pub fn is_clash(&self) -> bool {
        !self.clashes.is_empty()
    }
}

fn core_lang(lang: Lang) -> CoreLang {
    match lang {
        Lang::Pl => CoreLang::Pl,
        Lang::En => CoreLang::En,
    }
}

/// The class kind code for the tag (SYSTEM.md section 3): known USOS codes are mapped, unknown
/// ones print the first three letters of the USOS name, uppercased.
pub fn kind_code(
    classtype_id: Option<&str>,
    classtype_name: Option<&str>,
    activity: &ActivityKind,
) -> (String, Kind) {
    if matches!(activity, ActivityKind::Exam) {
        return ("EGZ".to_owned(), Kind::Exam);
    }
    let id = classtype_id.map(|s| s.trim().to_uppercase());
    match id.as_deref() {
        Some("WYK") => return ("WYK".to_owned(), Kind::Lecture),
        Some("LAB") => return ("LAB".to_owned(), Kind::Lab),
        Some("EGZ" | "KOL") => return (id.unwrap_or_default(), Kind::Exam),
        Some("CW" | "ĆW") => return ("ĆW".to_owned(), Kind::Other),
        Some("PRO" | "PROJ") => return ("PRO".to_owned(), Kind::Other),
        Some("SEM") => return ("SEM".to_owned(), Kind::Other),
        Some("LEK") => return ("LEK".to_owned(), Kind::Other),
        Some("WF") => return ("WF".to_owned(), Kind::Other),
        _ => {}
    }
    let from_name: String = classtype_name
        .unwrap_or("")
        .trim()
        .chars()
        .filter(|c| c.is_alphabetic())
        .take(3)
        .collect::<String>()
        .to_uppercase();
    if !from_name.is_empty() {
        return (from_name, Kind::Other);
    }
    if let Some(id) = id.filter(|id| !id.is_empty()) {
        return (id.chars().take(3).collect(), Kind::Other);
    }
    ("ZAJ".to_owned(), Kind::Other)
}

/// "s. 314" for a numeric room, the room as is otherwise ("Aula C").
pub fn room_text(room_number: Option<&str>) -> Option<String> {
    let room = room_number?.trim();
    if room.is_empty() {
        return None;
    }
    if room.starts_with(|c: char| c.is_ascii_digit()) {
        Some(format!("s.{NBSP}{room}"))
    } else {
        Some(room.to_owned())
    }
}

type Span = (NaiveDateTime, NaiveDateTime);

/// Two classes overlap when each starts before the other ends. Back-to-back classes do not.
pub fn overlaps(a: Span, b: Span) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// The overlapping stretch of two classes, when there is one.
pub fn overlap_span(a: Span, b: Span) -> Option<Span> {
    overlaps(a, b).then(|| (a.0.max(b.0), a.1.min(b.1)))
}

/// Turn the core activities into items, sorted by start then end, with each item's clashes.
pub fn class_items(activities: &[Activity], lang: Lang) -> Vec<ClassItem> {
    let core = core_lang(lang);
    let mut items: Vec<ClassItem> = activities
        .iter()
        .map(|a| {
            let title = a
                .course_name
                .get(core)
                .or_else(|| a.name.get(core))
                .unwrap_or("")
                .to_owned();
            let type_name = a.classtype_name.get(core).map(str::to_owned);
            let (code, kind) = kind_code(a.classtype_id.as_deref(), type_name.as_deref(), &a.kind);
            ClassItem {
                key: ClassKey {
                    start: a.start,
                    end: a.end,
                    course_id: a.course_id.clone(),
                    classtype_id: a.classtype_id.clone(),
                    group_number: a.group_number,
                },
                start: a.start,
                end: a.end,
                title,
                code,
                kind,
                type_name,
                room: room_text(a.room_number.as_deref()),
                building_id: a.building_id.clone().filter(|s| !s.trim().is_empty()),
                building_name: a.building_name.get(core).map(str::to_owned),
                group: a.group_number,
                frequency: a.frequency.clone(),
                url: a.url.clone(),
                clashes: Vec::new(),
            }
        })
        .collect();
    items.sort_by(|a, b| {
        (a.start, a.end, &a.key.course_id).cmp(&(b.start, b.end, &b.key.course_id))
    });
    for i in 0..items.len() {
        let span = (items[i].start, items[i].end);
        let clashes: Vec<usize> = (0..items.len())
            .filter(|&j| j != i && overlaps(span, (items[j].start, items[j].end)))
            .collect();
        items[i].clashes = clashes;
    }
    items
}

/// Indices of the items that start on `date`, in list order.
pub fn day_indices(items: &[ClassItem], date: NaiveDate) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.start.date() == date)
        .map(|(i, _)| i)
        .collect()
}

/// The first day after `after` that has a class.
pub fn next_class_day(items: &[ClassItem], after: NaiveDate) -> Option<NaiveDate> {
    items
        .iter()
        .map(|item| item.start.date())
        .filter(|date| *date > after)
        .min()
}

/// The first clash of an item: the other item's index and the overlapping stretch.
pub fn first_clash(items: &[ClassItem], index: usize) -> Option<(usize, Span)> {
    let item = items.get(index)?;
    item.clashes.iter().find_map(|&other| {
        let o = items.get(other)?;
        overlap_span((item.start, item.end), (o.start, o.end)).map(|span| (other, span))
    })
}

// ---------------------------------------------------------------------------------------------
// Grid geometry
// ---------------------------------------------------------------------------------------------

/// The hours a week grid shows: from `start` to `end` (the last hour line), fitted to the data.
/// Starts at the earlier of 08:00 and the first start rounded down; ends at the later of 16:00
/// and the last end rounded up (a class ending past midnight counts as 24:00). That is never
/// narrower than 8 hours.
pub fn hour_range(items: &[&ClassItem]) -> (u32, u32) {
    let mut start = GRID_DEFAULT_START;
    let mut end = GRID_DEFAULT_END;
    for item in items {
        start = start.min(item.start.hour());
        let item_end = if item.end.date() > item.start.date() {
            24
        } else {
            (item.end.hour() * 60 + item.end.minute()).div_ceil(60)
        };
        end = end.max(item_end.min(24));
    }
    if end < start + 8 {
        end = (start + 8).min(24);
        start = start.min(end.saturating_sub(8));
    }
    (start, end)
}

/// Where a time falls on a grid that starts at `grid_start` hours, in minutes from the top,
/// clamped to the day (so a class past midnight ends at 24:00).
pub fn grid_minutes(day: NaiveDate, at: NaiveDateTime, grid_start: u32) -> i64 {
    let from_midnight = if at.date() > day {
        24 * 60
    } else if at.date() < day {
        0
    } else {
        i64::from(at.hour() * 60 + at.minute())
    };
    from_midnight - i64::from(grid_start) * 60
}

/// A class's lane among the classes it overlaps: it takes `1 / count` of the column width at
/// position `index`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane {
    pub index: usize,
    pub count: usize,
}

/// Side-by-side lanes for overlapping spans. Spans that overlap, directly or through a chain,
/// share a cluster; each cluster gets as many lanes as it needs and nothing else shrinks.
/// The result is in input order.
pub fn lanes(spans: &[Span]) -> Vec<Lane> {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by_key(|&i| (spans[i].0, spans[i].1, i));
    let mut out = vec![Lane { index: 0, count: 1 }; spans.len()];
    let mut cluster: Vec<usize> = Vec::new();
    let mut lane_ends: Vec<NaiveDateTime> = Vec::new();
    let mut cluster_end: Option<NaiveDateTime> = None;
    fn close(cluster: &mut Vec<usize>, lanes: usize, out: &mut [Lane]) {
        for &i in cluster.iter() {
            out[i].count = lanes.max(1);
        }
        cluster.clear();
    }
    for i in order {
        let (start, end) = spans[i];
        // An empty span still takes a minute, so it cannot hide under its neighbour.
        let end = end.max(start + Duration::minutes(1));
        if cluster_end.is_some_and(|c| start >= c) {
            close(&mut cluster, lane_ends.len(), &mut out);
            lane_ends.clear();
            cluster_end = None;
        }
        let lane = match lane_ends.iter().position(|&e| e <= start) {
            Some(lane) => {
                lane_ends[lane] = end;
                lane
            }
            None => {
                lane_ends.push(end);
                lane_ends.len() - 1
            }
        };
        out[i].index = lane;
        cluster.push(i);
        cluster_end = Some(cluster_end.map_or(end, |c| c.max(end)));
    }
    close(&mut cluster, lane_ends.len(), &mut out);
    out
}

// ---------------------------------------------------------------------------------------------
// Day timeline
// ---------------------------------------------------------------------------------------------

/// Free time between two classes of one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// The gap sits before this position in the list given to [`gaps`].
    pub before: usize,
    pub minutes: i64,
}

/// The free periods of a day. `day` is sorted by start. Overlapping classes extend the running
/// end, so no gap is reported inside a clash; a gap needs at least [`MIN_GAP_MINUTES`].
pub fn gaps(day: &[&ClassItem]) -> Vec<Gap> {
    let mut out = Vec::new();
    let mut busy_until: Option<NaiveDateTime> = None;
    for (position, item) in day.iter().enumerate() {
        if let Some(until) = busy_until {
            let minutes = (item.start - until).num_minutes();
            if minutes >= MIN_GAP_MINUTES {
                out.push(Gap {
                    before: position,
                    minutes,
                });
            }
        }
        busy_until = Some(busy_until.map_or(item.end, |u| u.max(item.end)));
    }
    out
}

/// The class a "now" card is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// A class is running (index into the list).
    Now(usize),
    /// Nothing is running; this class starts later today.
    Upcoming(usize),
}

impl Focus {
    pub fn index(self) -> usize {
        match self {
            Focus::Now(i) | Focus::Upcoming(i) => i,
        }
    }
}

/// What is happening now and what comes after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NowNext {
    pub focus: Option<Focus>,
    /// The first class after the focus that starts later today.
    pub next: Option<usize>,
}

/// `items` is the whole sorted list; `now` is local. A running class wins (the earliest started
/// when several run); otherwise the next class that starts later on the same day.
pub fn now_next(items: &[ClassItem], now: NaiveDateTime) -> NowNext {
    let today = now.date();
    let later_today = |after: NaiveDateTime| {
        items
            .iter()
            .position(|i| i.start > after && i.start.date() == today)
    };
    if let Some(index) = items.iter().position(|item| item.is_current(now)) {
        return NowNext {
            focus: Some(Focus::Now(index)),
            next: later_today(now),
        };
    }
    match later_today(now) {
        Some(index) => NowNext {
            focus: Some(Focus::Upcoming(index)),
            next: later_today(items[index].start),
        },
        None => NowNext {
            focus: None,
            next: None,
        },
    }
}

/// Whole minutes until `end`, rounded up so the last minute still says "1 min".
pub fn minutes_left(end: NaiveDateTime, now: NaiveDateTime) -> i64 {
    let seconds = (end - now).num_seconds().max(0);
    (seconds + 59) / 60
}

/// How far into a class `now` is, 0 to 1.
pub fn progress(start: NaiveDateTime, end: NaiveDateTime, now: NaiveDateTime) -> f32 {
    let total = (end - start).num_seconds();
    if total <= 0 {
        return if now >= end { 1.0 } else { 0.0 };
    }
    let done = (now - start).num_seconds();
    (done as f32 / total as f32).clamp(0.0, 1.0)
}

/// "38 min", "1 h 20 min", "2 h": a length of time with no-break spaces.
pub fn duration_text(minutes: i64) -> String {
    let minutes = minutes.max(0);
    let (h, m) = (minutes / 60, minutes % 60);
    match (h, m) {
        (0, m) => format!("{m}{NBSP}min"),
        (h, 0) => format!("{h}{NBSP}h"),
        (h, m) => format!("{h}{NBSP}h{NBSP}{m}{NBSP}min"),
    }
}

/// `10:15`.
pub fn time_text(at: NaiveDateTime) -> String {
    format!("{:02}:{:02}", at.hour(), at.minute())
}

/// `10:15–11:45` with an en dash.
pub fn range_text(start: NaiveDateTime, end: NaiveDateTime) -> String {
    format!("{}–{}", time_text(start), time_text(end))
}

/// How often a class meets, only from the USOS code ("Co tydzień"). `None` for codes that say
/// nothing useful to a student. Parity is named only where USOS says so.
pub fn frequency_text(frequency: &Frequency, lang: Lang) -> Option<&'static str> {
    let (pl, en) = match frequency {
        Frequency::EveryWeek => ("Co tydzień", "Every week"),
        Frequency::EveryFortnight => ("Co dwa tygodnie", "Every two weeks"),
        Frequency::FortnightOdd => ("Co dwa tygodnie (nieparzyste)", "Every two weeks (odd)"),
        Frequency::FortnightEven => ("Co dwa tygodnie (parzyste)", "Every two weeks (even)"),
        Frequency::EveryMonth => ("Co miesiąc", "Every month"),
        Frequency::EveryWorkingDay => ("W dni robocze", "On working days"),
        Frequency::EveryWeekend => ("W weekendy", "On weekends"),
        Frequency::EveryDay => ("Codziennie", "Every day"),
        Frequency::Once => ("Jednorazowo", "Once"),
        Frequency::Irregular | Frequency::Other(_) => return None,
    };
    Some(match lang {
        Lang::Pl => pl,
        Lang::En => en,
    })
}

// ---------------------------------------------------------------------------------------------
// Weeks
// ---------------------------------------------------------------------------------------------

/// The Monday of the week of `date`.
pub fn week_monday(date: NaiveDate) -> NaiveDate {
    date - Days::new(u64::from(date.weekday().num_days_from_monday()))
}

/// The number of the week in the term, counting the week of the term's first day as week 1.
/// `None` outside the term. USOS does not publish week parity, so none is derived.
pub fn term_week(date: NaiveDate, term: &Term) -> Option<u32> {
    let last = term.finish_date.unwrap_or(term.end_date);
    if date < term.start_date || date > last {
        return None;
    }
    let weeks = (week_monday(date) - week_monday(term.start_date)).num_days() / 7;
    u32::try_from(weeks + 1).ok()
}

/// The first and last Monday a week view can show: the weeks covered by the cached range.
pub fn week_bounds(from: NaiveDate, to: NaiveDate) -> (NaiveDate, NaiveDate) {
    (week_monday(from), week_monday(to))
}

/// The days a week grid has a column for: Monday to Friday, plus Saturday and Sunday only when
/// they have classes (Sunday brings Saturday along so the columns stay contiguous).
pub fn visible_days(monday: NaiveDate, items: &[ClassItem]) -> Vec<NaiveDate> {
    let has = |offset: u64| {
        let day = monday + Days::new(offset);
        items.iter().any(|item| item.start.date() == day)
    };
    let last = if has(6) {
        6
    } else if has(5) {
        5
    } else {
        4
    };
    (0..=last)
        .map(|offset| monday + Days::new(offset))
        .collect()
}

/// Which weekend days the grid hides (for the legend line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenWeekend {
    None,
    Sunday,
    Both,
}

pub fn hidden_weekend(days: &[NaiveDate]) -> HiddenWeekend {
    match days.len() {
        0..=5 => HiddenWeekend::Both,
        6 => HiddenWeekend::Sunday,
        _ => HiddenWeekend::None,
    }
}

/// Which two-hour slots (08-10, 10-12, ... 18-20) of Monday to Friday have a class, for the
/// week motif of the empty Today screen. A class marks every slot it overlaps.
pub fn motif_busy(monday: NaiveDate, items: &[ClassItem]) -> [[bool; 6]; 5] {
    let mut busy = [[false; 6]; 5];
    for item in items {
        let offset = (item.start.date() - monday).num_days();
        let Ok(day) = usize::try_from(offset) else {
            continue;
        };
        if day >= 5 {
            continue;
        }
        for (slot, flag) in busy[day].iter_mut().enumerate() {
            let from = item.start.date().and_hms_opt(8 + 2 * slot as u32, 0, 0);
            let to = item.start.date().and_hms_opt(10 + 2 * slot as u32, 0, 0);
            if let (Some(from), Some(to)) = (from, to)
                && overlaps((item.start, item.end), (from, to))
            {
                *flag = true;
            }
        }
    }
    busy
}

// ---------------------------------------------------------------------------------------------
// Names and dates
// ---------------------------------------------------------------------------------------------

const WEEKDAYS_PL: [&str; 7] = [
    "poniedziałek",
    "wtorek",
    "środa",
    "czwartek",
    "piątek",
    "sobota",
    "niedziela",
];
const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const WEEKDAYS_SHORT_PL: [&str; 7] = ["Pn", "Wt", "Śr", "Cz", "Pt", "So", "Nd"];
const WEEKDAYS_SHORT_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS_GEN_PL: [&str; 12] = [
    "stycznia",
    "lutego",
    "marca",
    "kwietnia",
    "maja",
    "czerwca",
    "lipca",
    "sierpnia",
    "września",
    "października",
    "listopada",
    "grudnia",
];
const MONTHS_NOM_PL: [&str; 12] = [
    "styczeń",
    "luty",
    "marzec",
    "kwiecień",
    "maj",
    "czerwiec",
    "lipiec",
    "sierpień",
    "wrzesień",
    "październik",
    "listopad",
    "grudzień",
];
const MONTHS_SHORT_PL: [&str; 12] = [
    "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
];
const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_SHORT_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn wd(day: Weekday) -> usize {
    day.num_days_from_monday() as usize
}

fn mo(date: NaiveDate) -> usize {
    date.month0() as usize
}

/// "wtorek" / "Tuesday" (lower case in Polish, as the language spells it).
pub fn weekday_name(date: NaiveDate, lang: Lang) -> &'static str {
    match lang {
        Lang::Pl => WEEKDAYS_PL[wd(date.weekday())],
        Lang::En => WEEKDAYS_EN[wd(date.weekday())],
    }
}

/// "Wt" / "Tue".
pub fn weekday_short(date: NaiveDate, lang: Lang) -> &'static str {
    match lang {
        Lang::Pl => WEEKDAYS_SHORT_PL[wd(date.weekday())],
        Lang::En => WEEKDAYS_SHORT_EN[wd(date.weekday())],
    }
}

/// "10 listopada" / "10 November".
pub fn day_month(date: NaiveDate, lang: Lang) -> String {
    match lang {
        Lang::Pl => format!("{} {}", date.day(), MONTHS_GEN_PL[mo(date)]),
        Lang::En => format!("{} {}", date.day(), MONTHS_EN[mo(date)]),
    }
}

/// "10 lis" / "10 Nov".
pub fn day_month_short(date: NaiveDate, lang: Lang) -> String {
    match lang {
        Lang::Pl => format!("{} {}", date.day(), MONTHS_SHORT_PL[mo(date)]),
        Lang::En => format!("{} {}", date.day(), MONTHS_SHORT_EN[mo(date)]),
    }
}

/// "Wtorek, 10 listopada" / "Tuesday, 10 November".
pub fn date_long(date: NaiveDate, lang: Lang) -> String {
    sentence_case(&format!(
        "{}, {}",
        weekday_name(date, lang),
        day_month(date, lang)
    ))
}

/// "listopad 2026" / "November 2026" (the eyebrow of a week).
pub fn month_year(date: NaiveDate, lang: Lang) -> String {
    match lang {
        Lang::Pl => format!("{} {}", MONTHS_NOM_PL[mo(date)], date.year()),
        Lang::En => format!("{} {}", MONTHS_EN[mo(date)], date.year()),
    }
}

/// The title of a week: "9–15 listopada", "26 października – 1 listopada".
pub fn week_title(monday: NaiveDate, lang: Lang) -> String {
    let sunday = monday + Days::new(6);
    if monday.month() == sunday.month() {
        let month = match lang {
            Lang::Pl => MONTHS_GEN_PL[mo(sunday)],
            Lang::En => MONTHS_EN[mo(sunday)],
        };
        format!("{}–{} {month}", monday.day(), sunday.day())
    } else {
        format!("{} – {}", day_month(monday, lang), day_month(sunday, lang))
    }
}

/// "dziś" for today, "jutro, środa" for tomorrow, else the weekday and short date.
pub fn relative_day(date: NaiveDate, today: NaiveDate, lang: Lang) -> String {
    let delta = (date - today).num_days();
    let name = weekday_name(date, lang);
    match (delta, lang) {
        (0, Lang::Pl) => "dziś".to_owned(),
        (0, Lang::En) => "today".to_owned(),
        (1, Lang::Pl) => format!("jutro, {name}"),
        (1, Lang::En) => format!("tomorrow, {name}"),
        _ => format!("{name} {}", day_month_short(date, lang)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usos_core::LangDict;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn at(y: i32, m: u32, day: u32, h: u32, min: u32) -> NaiveDateTime {
        d(y, m, day).and_hms_opt(h, min, 0).unwrap()
    }

    fn item(start: NaiveDateTime, end: NaiveDateTime) -> ClassItem {
        ClassItem {
            key: ClassKey {
                start,
                end,
                course_id: None,
                classtype_id: None,
                group_number: None,
            },
            start,
            end,
            title: "Test".into(),
            code: "WYK".into(),
            kind: Kind::Lecture,
            type_name: None,
            room: None,
            building_id: None,
            building_name: None,
            group: None,
            frequency: None,
            url: None,
            clashes: Vec::new(),
        }
    }

    fn on(day: u32, from: (u32, u32), to: (u32, u32)) -> ClassItem {
        item(
            at(2026, 11, day, from.0, from.1),
            at(2026, 11, day, to.0, to.1),
        )
    }

    fn refs(items: &[ClassItem]) -> Vec<&ClassItem> {
        items.iter().collect()
    }

    fn activity(start: NaiveDateTime, end: NaiveDateTime, course: &str) -> Activity {
        Activity {
            kind: ActivityKind::Classgroup,
            start,
            end,
            name: LangDict {
                pl: Some("Nazwa ogólna".into()),
                en: None,
            },
            course_id: Some(course.into()),
            course_name: LangDict {
                pl: Some(format!("Kurs {course}")),
                en: Some(format!("Course {course}")),
            },
            classtype_id: Some("CW".into()),
            classtype_name: LangDict::default(),
            group_number: Some(2),
            building_id: Some("AEI".into()),
            building_name: LangDict::default(),
            room_id: None,
            room_number: Some("216".into()),
            lecturer_ids: vec![],
            unit_id: None,
            frequency: Some(Frequency::EveryWeek),
            url: None,
        }
    }

    #[test]
    fn kind_codes_follow_the_table() {
        let class = ActivityKind::Classgroup;
        assert_eq!(
            kind_code(Some("WYK"), None, &class),
            ("WYK".into(), Kind::Lecture)
        );
        assert_eq!(
            kind_code(Some("LAB"), None, &class),
            ("LAB".into(), Kind::Lab)
        );
        assert_eq!(
            kind_code(Some("CW"), None, &class),
            ("ĆW".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("cw"), None, &class),
            ("ĆW".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("PRO"), None, &class),
            ("PRO".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("SEM"), None, &class),
            ("SEM".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("LEK"), None, &class),
            ("LEK".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("WF"), None, &class),
            ("WF".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("WYK"), None, &ActivityKind::Exam),
            ("EGZ".into(), Kind::Exam)
        );
        assert_eq!(
            kind_code(Some("KOL"), None, &class),
            ("KOL".into(), Kind::Exam)
        );
    }

    #[test]
    fn every_class_type_gets_a_style() {
        let class = ActivityKind::Classgroup;
        // Known fills, then ids USOS may return that the table does not list: the neutral kind.
        for (id, kind) in [
            ("WYK", Kind::Lecture),
            ("LAB", Kind::Lab),
            ("EGZ", Kind::Exam),
            ("KOL", Kind::Exam),
            ("CW", Kind::Other),
            ("PRO", Kind::Other),
            ("SEM", Kind::Other),
            ("LEK", Kind::Other),
            ("WF", Kind::Other),
            ("FOR", Kind::Other),
            ("KON", Kind::Other),
            ("PRA", Kind::Other),
            ("XYZ", Kind::Other),
        ] {
            let (code, got) = kind_code(Some(id), None, &class);
            assert_eq!(got, kind, "{id}");
            assert!(!code.is_empty(), "{id}");
        }
    }

    #[test]
    fn unknown_kinds_print_three_letters_of_the_name() {
        let class = ActivityKind::Classgroup;
        assert_eq!(
            kind_code(Some("XYZ"), Some("Konwersatorium"), &class),
            ("KON".into(), Kind::Other)
        );
        assert_eq!(
            kind_code(Some("ZZ"), None, &class),
            ("ZZ".into(), Kind::Other)
        );
        assert_eq!(kind_code(None, None, &class), ("ZAJ".into(), Kind::Other));
        assert_eq!(
            kind_code(None, Some("  "), &class),
            ("ZAJ".into(), Kind::Other)
        );
    }

    #[test]
    fn rooms_get_s_dot_only_when_numeric() {
        assert_eq!(room_text(Some("314")).as_deref(), Some("s.\u{a0}314"));
        assert_eq!(room_text(Some("Aula C")).as_deref(), Some("Aula C"));
        assert_eq!(room_text(Some("  ")), None);
        assert_eq!(room_text(None), None);
    }

    #[test]
    fn back_to_back_classes_do_not_overlap() {
        let a = (at(2026, 11, 10, 10, 0), at(2026, 11, 10, 11, 45));
        let b = (at(2026, 11, 10, 11, 45), at(2026, 11, 10, 13, 30));
        let c = (at(2026, 11, 10, 11, 0), at(2026, 11, 10, 12, 0));
        assert!(!overlaps(a, b));
        assert!(overlaps(a, c));
        assert!(overlaps(c, b));
        assert_eq!(
            overlap_span(c, b),
            Some((at(2026, 11, 10, 11, 45), at(2026, 11, 10, 12, 0)))
        );
        assert_eq!(overlap_span(a, b), None);
    }

    #[test]
    fn items_record_their_clashes() {
        let items = class_items(
            &[
                activity(at(2026, 11, 12, 12, 45), at(2026, 11, 12, 14, 15), "B"),
                activity(at(2026, 11, 12, 12, 0), at(2026, 11, 12, 13, 30), "A"),
                activity(at(2026, 11, 12, 14, 30), at(2026, 11, 12, 16, 0), "C"),
            ],
            Lang::Pl,
        );
        assert_eq!(items[0].title, "Kurs A");
        assert_eq!(items[0].clashes, vec![1]);
        assert_eq!(items[1].clashes, vec![0]);
        assert!(items[2].clashes.is_empty());
        assert!(items[0].is_clash() && !items[2].is_clash());
        assert_eq!(
            first_clash(&items, 0),
            Some((1, (at(2026, 11, 12, 12, 45), at(2026, 11, 12, 13, 30))))
        );
        assert_eq!(first_clash(&items, 2), None);
        assert_eq!(first_clash(&items, 9), None);
    }

    #[test]
    fn items_are_sorted_and_named_from_the_course() {
        let items = class_items(
            &[
                activity(at(2026, 11, 10, 12, 0), at(2026, 11, 10, 13, 30), "B"),
                activity(at(2026, 11, 10, 8, 0), at(2026, 11, 10, 9, 30), "A"),
            ],
            Lang::En,
        );
        assert_eq!(items[0].title, "Course A");
        assert_eq!(items[1].title, "Course B");
        assert_eq!(items[0].code, "ĆW");
        assert_eq!(items[0].place().as_deref(), Some("s.\u{a0}216, AEI"));
        // The generic name is the fallback when the course has no name.
        let mut bare = activity(at(2026, 11, 10, 8, 0), at(2026, 11, 10, 9, 30), "A");
        bare.course_name = LangDict::default();
        let items = class_items(&[bare], Lang::En);
        assert_eq!(items[0].title, "Nazwa ogólna");
    }

    #[test]
    fn hour_range_defaults_to_eight_to_sixteen() {
        assert_eq!(hour_range(&[]), (8, 16));
        let items = [on(9, (9, 0), (10, 30))];
        assert_eq!(hour_range(&refs(&items)), (8, 16));
    }

    #[test]
    fn hour_range_grows_to_fit() {
        // A 07:30 class starts the grid at 07:00; a 19:15 end ends it at 20:00.
        let items = [on(9, (7, 30), (9, 0)), on(10, (17, 30), (19, 15))];
        assert_eq!(hour_range(&refs(&items)), (7, 20));
        // An end on the hour stays on it.
        let items = [on(9, (14, 0), (18, 0))];
        assert_eq!(hour_range(&refs(&items)), (8, 18));
    }

    #[test]
    fn hour_range_clamps_at_midnight() {
        let late = item(at(2026, 11, 10, 22, 0), at(2026, 11, 11, 1, 30));
        assert_eq!(hour_range(&[&late]), (8, 24));
        let to_midnight = on(10, (21, 0), (23, 59));
        assert_eq!(hour_range(&[&to_midnight]), (8, 24));
        let early = on(10, (0, 15), (1, 0));
        assert_eq!(hour_range(&[&early]), (0, 16));
    }

    #[test]
    fn hour_range_is_never_under_eight_hours() {
        for h in 0..24 {
            let it = on(10, (h, 0), (h, 30));
            let (s, e) = hour_range(&[&it]);
            assert!(e - s >= 8, "{h}: {s}..{e}");
            assert!(e <= 24);
            assert!(s <= h);
        }
    }

    #[test]
    fn grid_minutes_clamp_to_the_day() {
        let day = d(2026, 11, 10);
        assert_eq!(grid_minutes(day, at(2026, 11, 10, 10, 15), 8), 135);
        assert_eq!(
            grid_minutes(day, at(2026, 11, 11, 1, 30), 8),
            24 * 60 - 8 * 60
        );
        assert_eq!(grid_minutes(day, at(2026, 11, 9, 23, 0), 8), -8 * 60);
    }

    #[test]
    fn lanes_for_a_clean_day_are_single() {
        let spans = [
            (at(2026, 11, 10, 8, 0), at(2026, 11, 10, 9, 30)),
            (at(2026, 11, 10, 9, 30), at(2026, 11, 10, 11, 0)),
        ];
        assert_eq!(
            lanes(&spans),
            vec![Lane { index: 0, count: 1 }, Lane { index: 0, count: 1 }]
        );
        assert!(lanes(&[]).is_empty());
    }

    #[test]
    fn two_overlapping_classes_sit_side_by_side() {
        let spans = [
            (at(2026, 11, 12, 12, 45), at(2026, 11, 12, 14, 15)),
            (at(2026, 11, 12, 12, 0), at(2026, 11, 12, 13, 30)),
            (at(2026, 11, 12, 14, 30), at(2026, 11, 12, 16, 0)),
        ];
        let l = lanes(&spans);
        assert_eq!(l[1], Lane { index: 0, count: 2 });
        assert_eq!(l[0], Lane { index: 1, count: 2 });
        assert_eq!(l[2], Lane { index: 0, count: 1 });
    }

    #[test]
    fn three_overlapping_classes_get_three_lanes() {
        let spans = [
            (at(2026, 11, 10, 10, 0), at(2026, 11, 10, 12, 0)),
            (at(2026, 11, 10, 10, 30), at(2026, 11, 10, 11, 30)),
            (at(2026, 11, 10, 11, 0), at(2026, 11, 10, 13, 0)),
        ];
        let l = lanes(&spans);
        assert_eq!(l.iter().map(|l| l.count).collect::<Vec<_>>(), vec![3, 3, 3]);
        let mut used: Vec<usize> = l.iter().map(|l| l.index).collect();
        used.sort_unstable();
        assert_eq!(used, vec![0, 1, 2]);
    }

    #[test]
    fn a_chain_shares_one_cluster_and_a_free_lane_is_reused() {
        // A overlaps B, B overlaps C, A does not overlap C: two lanes, C reuses A's.
        let spans = [
            (at(2026, 11, 10, 10, 0), at(2026, 11, 10, 11, 0)),
            (at(2026, 11, 10, 10, 30), at(2026, 11, 10, 12, 0)),
            (at(2026, 11, 10, 11, 0), at(2026, 11, 10, 12, 30)),
        ];
        let l = lanes(&spans);
        assert_eq!(l[0], Lane { index: 0, count: 2 });
        assert_eq!(l[1], Lane { index: 1, count: 2 });
        assert_eq!(l[2], Lane { index: 0, count: 2 });
    }

    #[test]
    fn identical_and_empty_spans_do_not_hide_each_other() {
        let s = (at(2026, 11, 10, 10, 0), at(2026, 11, 10, 11, 0));
        let empty = (at(2026, 11, 10, 10, 0), at(2026, 11, 10, 10, 0));
        let l = lanes(&[s, s, empty]);
        assert_eq!(l.iter().map(|l| l.count).collect::<Vec<_>>(), vec![3, 3, 3]);
    }

    #[test]
    fn gaps_start_at_thirty_minutes() {
        let items = [
            on(10, (8, 30), (10, 0)),
            on(10, (10, 15), (11, 45)), // 15 min break: no gap
            on(10, (12, 0), (13, 30)),  // 15 min break
            on(10, (14, 15), (15, 45)), // 45 min gap
            on(10, (16, 0), (17, 30)),  // 15 min
        ];
        let g = gaps(&refs(&items));
        assert_eq!(
            g,
            vec![Gap {
                before: 3,
                minutes: 45
            }]
        );
        assert!(gaps(&[]).is_empty());
        assert!(gaps(&refs(&items[..1])).is_empty());
    }

    #[test]
    fn a_clash_does_not_create_a_false_gap() {
        // The second class ends before the first, the third starts right after the first ends.
        let items = [
            on(10, (8, 0), (12, 0)),
            on(10, (9, 0), (10, 0)),
            on(10, (12, 0), (13, 0)),
            on(10, (14, 0), (15, 0)),
        ];
        assert_eq!(
            gaps(&refs(&items)),
            vec![Gap {
                before: 3,
                minutes: 60
            }]
        );
    }

    #[test]
    fn exactly_thirty_minutes_is_a_gap_and_twenty_nine_is_not() {
        let items = [on(10, (8, 0), (9, 0)), on(10, (9, 30), (10, 0))];
        assert_eq!(gaps(&refs(&items)).len(), 1);
        let items = [on(10, (8, 0), (9, 0)), on(10, (9, 29), (10, 0))];
        assert!(gaps(&refs(&items)).is_empty());
    }

    #[test]
    fn now_and_next_during_a_class() {
        let items = [
            on(10, (8, 30), (10, 0)),
            on(10, (10, 15), (11, 45)),
            on(10, (12, 0), (13, 30)),
            on(11, (8, 0), (9, 0)),
        ];
        let r = now_next(&items, at(2026, 11, 10, 11, 7));
        assert_eq!(r.focus, Some(Focus::Now(1)));
        assert_eq!(r.next, Some(2));
        assert_eq!(r.focus.map(Focus::index), Some(1));
    }

    #[test]
    fn before_the_first_class_the_card_is_upcoming() {
        let items = [on(10, (8, 30), (10, 0)), on(10, (12, 0), (13, 0))];
        let r = now_next(&items, at(2026, 11, 10, 7, 55));
        assert_eq!(r.focus, Some(Focus::Upcoming(0)));
        assert_eq!(r.next, Some(1));
        // Between classes: the next one is the focus and nothing follows when it is the last.
        let r = now_next(&items, at(2026, 11, 10, 10, 30));
        assert_eq!(r.focus, Some(Focus::Upcoming(1)));
        assert_eq!(r.next, None);
    }

    #[test]
    fn after_the_last_class_there_is_no_card() {
        let items = [on(10, (8, 30), (10, 0)), on(11, (8, 0), (9, 0))];
        let r = now_next(&items, at(2026, 11, 10, 18, 0));
        assert_eq!(
            r,
            NowNext {
                focus: None,
                next: None
            }
        );
        assert_eq!(now_next(&[], at(2026, 11, 10, 9, 0)).focus, None);
    }

    #[test]
    fn a_class_ending_now_is_over_and_one_starting_now_is_running() {
        let items = [on(10, (8, 0), (9, 0)), on(10, (9, 0), (10, 0))];
        let r = now_next(&items, at(2026, 11, 10, 9, 0));
        assert_eq!(r.focus, Some(Focus::Now(1)));
        assert!(items[0].is_past(at(2026, 11, 10, 9, 0)));
    }

    #[test]
    fn with_a_clash_the_earliest_running_class_is_the_focus() {
        let items = [on(10, (12, 0), (13, 30)), on(10, (12, 45), (14, 15))];
        let r = now_next(&items, at(2026, 11, 10, 13, 0));
        assert_eq!(r.focus, Some(Focus::Now(0)));
        assert_eq!(r.next, None);
    }

    #[test]
    fn a_class_running_across_midnight_is_current_the_next_morning() {
        let items = [item(at(2026, 11, 10, 22, 0), at(2026, 11, 11, 1, 30))];
        let r = now_next(&items, at(2026, 11, 11, 0, 30));
        assert_eq!(r.focus, Some(Focus::Now(0)));
        assert_eq!(minutes_left(items[0].end, at(2026, 11, 11, 0, 30)), 60);
    }

    #[test]
    fn minutes_left_rounds_up() {
        let end = at(2026, 11, 10, 11, 45);
        assert_eq!(minutes_left(end, at(2026, 11, 10, 11, 7)), 38);
        let half = end - Duration::seconds(37 * 60 + 30);
        assert_eq!(minutes_left(end, half), 38);
        assert_eq!(minutes_left(end, end - Duration::seconds(1)), 1);
        assert_eq!(minutes_left(end, end), 0);
        assert_eq!(minutes_left(end, end + Duration::minutes(5)), 0);
    }

    #[test]
    fn progress_is_clamped() {
        let (s, e) = (at(2026, 11, 10, 10, 15), at(2026, 11, 10, 11, 45));
        assert_eq!(progress(s, e, at(2026, 11, 10, 9, 0)), 0.0);
        assert_eq!(progress(s, e, at(2026, 11, 10, 12, 0)), 1.0);
        let half = progress(s, e, at(2026, 11, 10, 11, 0));
        assert!((half - 0.5).abs() < 1e-6);
        assert_eq!(progress(s, s, s), 1.0);
        assert_eq!(progress(s, s, s - Duration::minutes(1)), 0.0);
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration_text(0), "0\u{a0}min");
        assert_eq!(duration_text(45), "45\u{a0}min");
        assert_eq!(duration_text(60), "1\u{a0}h");
        assert_eq!(duration_text(80), "1\u{a0}h\u{a0}20\u{a0}min");
        assert_eq!(duration_text(-5), "0\u{a0}min");
    }

    #[test]
    fn times_use_an_en_dash() {
        let (s, e) = (at(2026, 11, 10, 8, 5), at(2026, 11, 10, 9, 35));
        assert_eq!(time_text(s), "08:05");
        assert_eq!(range_text(s, e), "08:05–09:35");
    }

    #[test]
    fn dst_changes_do_not_move_wall_clock_arithmetic() {
        // 25 Oct 2026: clocks go back from 03:00 to 02:00 in Poland. A timetable is written in
        // wall clock time, so 02:30-04:00 is 90 minutes and the gap to 05:00 is 60.
        let a = item(at(2026, 10, 25, 2, 30), at(2026, 10, 25, 4, 0));
        assert_eq!(a.minutes(), 90);
        let b = item(at(2026, 10, 25, 5, 0), at(2026, 10, 25, 6, 0));
        assert_eq!(
            gaps(&[&a, &b]),
            vec![Gap {
                before: 1,
                minutes: 60
            }]
        );
        // 29 Mar 2026: clocks go forward from 02:00 to 03:00; the same rule holds.
        let c = item(at(2026, 3, 29, 1, 30), at(2026, 3, 29, 3, 30));
        assert_eq!(c.minutes(), 120);
        assert_eq!(minutes_left(c.end, at(2026, 3, 29, 3, 0)), 30);
        assert_eq!(hour_range(&[&c]).0, 1);
    }

    #[test]
    fn term_weeks_count_from_the_week_of_the_first_day() {
        let term = Term {
            id: "2026/27-Z".into(),
            name: LangDict::default(),
            start_date: d(2026, 10, 1), // a Thursday
            end_date: d(2027, 2, 28),
            finish_date: Some(d(2027, 3, 5)),
            order_key: None,
            is_active: None,
        };
        assert_eq!(term_week(d(2026, 10, 1), &term), Some(1));
        assert_eq!(term_week(d(2026, 10, 4), &term), Some(1));
        assert_eq!(term_week(d(2026, 10, 5), &term), Some(2));
        assert_eq!(term_week(d(2026, 11, 10), &term), Some(7));
        assert_eq!(term_week(d(2026, 9, 30), &term), None);
        assert_eq!(term_week(d(2027, 3, 5), &term), Some(23));
        assert_eq!(term_week(d(2027, 3, 6), &term), None);
        let no_finish = Term {
            finish_date: None,
            ..term
        };
        assert_eq!(term_week(d(2027, 3, 5), &no_finish), None);
    }

    #[test]
    fn week_bounds_cover_the_cached_range() {
        let bounds = week_bounds(d(2026, 11, 9), d(2026, 12, 6));
        assert_eq!(bounds, (d(2026, 11, 9), d(2026, 11, 30)));
        assert_eq!(week_monday(d(2026, 11, 15)), d(2026, 11, 9));
        assert_eq!(week_monday(d(2026, 11, 9)), d(2026, 11, 9));
    }

    #[test]
    fn weekends_are_shown_only_when_they_have_classes() {
        let monday = d(2026, 11, 9);
        let days = visible_days(monday, &[on(10, (9, 0), (10, 0))]);
        assert_eq!(days.len(), 5);
        assert_eq!(hidden_weekend(&days), HiddenWeekend::Both);
        let days = visible_days(monday, &[on(14, (9, 0), (10, 0))]);
        assert_eq!(days.len(), 6);
        assert_eq!(hidden_weekend(&days), HiddenWeekend::Sunday);
        let days = visible_days(monday, &[on(15, (9, 0), (10, 0))]);
        assert_eq!(days.len(), 7);
        assert_eq!(hidden_weekend(&days), HiddenWeekend::None);
        assert_eq!(visible_days(monday, &[]).len(), 5);
    }

    #[test]
    fn day_helpers_find_classes() {
        let items = [
            on(10, (8, 0), (9, 0)),
            on(10, (10, 0), (11, 0)),
            on(12, (8, 0), (9, 0)),
        ];
        assert_eq!(day_indices(&items, d(2026, 11, 10)), vec![0, 1]);
        assert_eq!(day_indices(&items, d(2026, 11, 11)), Vec::<usize>::new());
        assert_eq!(
            next_class_day(&items, d(2026, 11, 10)),
            Some(d(2026, 11, 12))
        );
        assert_eq!(next_class_day(&items, d(2026, 11, 12)), None);
        assert_eq!(next_class_day(&[], d(2026, 11, 12)), None);
    }

    #[test]
    fn the_motif_marks_the_slots_a_class_overlaps() {
        let monday = d(2026, 11, 9);
        let items = [
            on(9, (8, 30), (10, 0)),   // Monday slot 0
            on(10, (9, 0), (11, 0)),   // Tuesday slots 0 and 1
            on(11, (19, 0), (20, 30)), // Wednesday slot 5
            on(14, (9, 0), (10, 0)),   // Saturday: not in the motif
            on(2, (9, 0), (10, 0)),    // the week before: ignored
        ];
        let busy = motif_busy(monday, &items);
        assert_eq!(busy[0], [true, false, false, false, false, false]);
        assert_eq!(busy[1], [true, true, false, false, false, false]);
        assert_eq!(busy[2], [false, false, false, false, false, true]);
        assert_eq!(busy[3], [false; 6]);
        assert_eq!(motif_busy(monday, &[]), [[false; 6]; 5]);
    }

    #[test]
    fn counts_use_the_polish_forms() {
        assert_eq!(CLASSES.format(1, Lang::Pl), "1\u{a0}zajęcie");
        assert_eq!(CLASSES.format(3, Lang::Pl), "3\u{a0}zajęcia");
        assert_eq!(CLASSES.format(5, Lang::Pl), "5\u{a0}zajęć");
        assert_eq!(CLASSES.format(12, Lang::Pl), "12\u{a0}zajęć");
        assert_eq!(CLASSES.format(22, Lang::Pl), "22\u{a0}zajęcia");
        assert_eq!(CLASSES.format(0, Lang::Pl), "0\u{a0}zajęć");
        assert_eq!(CLASSES.format(1, Lang::En), "1\u{a0}class");
        assert_eq!(CLASSES.format(5, Lang::En), "5\u{a0}classes");
    }

    #[test]
    fn frequency_names_come_only_from_the_usos_code() {
        assert_eq!(
            frequency_text(&Frequency::EveryWeek, Lang::Pl),
            Some("Co tydzień")
        );
        assert_eq!(
            frequency_text(&Frequency::FortnightOdd, Lang::En),
            Some("Every two weeks (odd)")
        );
        assert_eq!(frequency_text(&Frequency::Irregular, Lang::Pl), None);
        assert_eq!(
            frequency_text(&Frequency::Other("x".into()), Lang::Pl),
            None
        );
    }

    #[test]
    fn dates_are_spelled_in_both_languages() {
        let tue = d(2026, 11, 10);
        assert_eq!(weekday_name(tue, Lang::Pl), "wtorek");
        assert_eq!(weekday_name(tue, Lang::En), "Tuesday");
        assert_eq!(weekday_short(tue, Lang::Pl), "Wt");
        assert_eq!(weekday_short(d(2026, 11, 11), Lang::Pl), "Śr");
        assert_eq!(day_month(tue, Lang::Pl), "10 listopada");
        assert_eq!(day_month(tue, Lang::En), "10 November");
        assert_eq!(day_month_short(tue, Lang::Pl), "10 lis");
        assert_eq!(
            date_long(d(2026, 11, 12), Lang::Pl),
            "Czwartek, 12 listopada"
        );
        assert_eq!(date_long(tue, Lang::En), "Tuesday, 10 November");
        assert_eq!(month_year(tue, Lang::Pl), "listopad 2026");
        assert_eq!(month_year(tue, Lang::En), "November 2026");
    }

    #[test]
    fn week_titles_handle_month_boundaries() {
        assert_eq!(week_title(d(2026, 11, 9), Lang::Pl), "9–15 listopada");
        assert_eq!(week_title(d(2026, 11, 9), Lang::En), "9–15 November");
        assert_eq!(
            week_title(d(2026, 10, 26), Lang::Pl),
            "26 października – 1 listopada"
        );
        assert_eq!(
            week_title(d(2026, 12, 28), Lang::En),
            "28 December – 3 January"
        );
    }

    #[test]
    fn relative_days() {
        let today = d(2026, 11, 10);
        assert_eq!(relative_day(today, today, Lang::Pl), "dziś");
        assert_eq!(
            relative_day(d(2026, 11, 11), today, Lang::Pl),
            "jutro, środa"
        );
        assert_eq!(
            relative_day(d(2026, 11, 12), today, Lang::Pl),
            "czwartek 12 lis"
        );
        assert_eq!(
            relative_day(d(2026, 11, 11), today, Lang::En),
            "tomorrow, Wednesday"
        );
    }
}
