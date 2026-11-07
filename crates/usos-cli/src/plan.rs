//! `usos plan`: the timetable. Range selection and formatting only; fetching is in `usos-core`.

use anyhow::Result;
use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, Weekday};
use clap::Args;
use usos_core::timetable::{group_by_day, week_of};
use usos_core::{Activity, Error, Lang};

use crate::cache::Data;

#[derive(Args)]
pub struct PlanArgs {
    /// Show the week containing this date (YYYY-MM-DD); with --days, N days starting at it.
    #[arg(long, value_name = "YYYY-MM-DD", conflicts_with_all = ["today", "tomorrow", "next"])]
    date: Option<NaiveDate>,
    /// Show N days (1-92) from --date, or from today when --date is not given.
    // The 92-day cap is a deliberate UI limit, well below the core's MAX_RANGE_DAYS (400).
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u16).range(1..=92),
        conflicts_with_all = ["today", "tomorrow", "next"])]
    days: Option<u16>,
    /// Show only today.
    #[arg(long, conflicts_with_all = ["tomorrow", "next"])]
    today: bool,
    /// Show only tomorrow.
    #[arg(long, conflicts_with = "next")]
    tomorrow: bool,
    /// Show next week (Monday to Sunday).
    #[arg(long)]
    next: bool,
    /// Print the activities as JSON instead of text.
    #[arg(long)]
    json: bool,
}

/// The inclusive date range the flags ask for, given today's date.
fn resolve_range(args: &PlanArgs, today: NaiveDate) -> Result<(NaiveDate, NaiveDate), Error> {
    let add = |d: NaiveDate, n: u64| {
        d.checked_add_days(Days::new(n))
            .ok_or_else(|| Error::InvalidArgument(format!("date {d} is out of range")))
    };
    let one_day = |d: NaiveDate| Ok((d, d));
    if args.today {
        one_day(today)
    } else if args.tomorrow {
        one_day(add(today, 1)?)
    } else if args.next {
        week_range(add(today, 7)?)
    } else if let Some(n) = args.days {
        let start = args.date.unwrap_or(today);
        Ok((start, add(start, u64::from(n) - 1)?))
    } else {
        week_range(args.date.unwrap_or(today))
    }
}

pub async fn run(data: &Data, lang: Lang, args: PlanArgs) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let (from, to) = resolve_range(&args, today)?;
    let activities = data.plan(from, to).await?.activities;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json_view(&activities)?)?
        );
        return Ok(());
    }
    print!("{}", render(&activities, lang, from, to));
    Ok(())
}

/// Activities as JSON objects, each with an added `clash` flag.
fn json_view(activities: &[Activity]) -> Result<Vec<serde_json::Value>> {
    let mut out = Vec::with_capacity(activities.len());
    for day in group_by_day(activities) {
        for (a, clash) in day.activities.iter().zip(day.clashes) {
            let mut v = serde_json::to_value(a)?;
            v["clash"] = clash.into();
            out.push(v);
        }
    }
    Ok(out)
}

fn week_range(day: NaiveDate) -> Result<(NaiveDate, NaiveDate), Error> {
    let week = week_of(day)?;
    Ok((week.from, week.to))
}

fn weekday_abbr(day: Weekday, lang: Lang) -> &'static str {
    match (lang, day) {
        (Lang::Pl, Weekday::Mon) => "Pon",
        (Lang::Pl, Weekday::Tue) => "Wt",
        (Lang::Pl, Weekday::Wed) => "Śr",
        (Lang::Pl, Weekday::Thu) => "Czw",
        (Lang::Pl, Weekday::Fri) => "Pt",
        (Lang::Pl, Weekday::Sat) => "Sob",
        (Lang::Pl, Weekday::Sun) => "Nie",
        (Lang::En, Weekday::Mon) => "Mon",
        (Lang::En, Weekday::Tue) => "Tue",
        (Lang::En, Weekday::Wed) => "Wed",
        (Lang::En, Weekday::Thu) => "Thu",
        (Lang::En, Weekday::Fri) => "Fri",
        (Lang::En, Weekday::Sat) => "Sat",
        (Lang::En, Weekday::Sun) => "Sun",
    }
}

fn hhmm(t: NaiveDateTime) -> String {
    t.format("%H:%M").to_string()
}

/// `08:30-10:00  CW    Course name  room, building`, prefixed with `!` when it clashes with another activity
fn line(a: &Activity, lang: Lang, clash: bool) -> String {
    let class = a
        .classtype_id
        .as_deref()
        .or_else(|| a.classtype_name.get(lang))
        .unwrap_or_else(|| a.kind.as_str());
    let course = a
        .course_name
        .get(lang)
        .or_else(|| a.name.get(lang))
        .unwrap_or("-");
    let place: Vec<&str> = [a.room_number.as_deref(), a.building_name.get(lang)]
        .into_iter()
        .flatten()
        .collect();
    format!(
        "{}{}-{}  {:<5} {}  {}",
        if clash { "!" } else { "" },
        hhmm(a.start),
        hhmm(a.end),
        class,
        course,
        place.join(", ")
    )
    .trim_end()
    .to_owned()
}

fn render(activities: &[Activity], lang: Lang, from: NaiveDate, to: NaiveDate) -> String {
    if activities.is_empty() {
        return format!("No classes from {from} to {to}.\n");
    }
    let mut out = String::new();
    for (i, plan) in group_by_day(activities).into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let day = plan.date;
        out.push_str(&format!("{} {day}\n", weekday_abbr(day.weekday(), lang)));
        for (a, clash) in plan.activities.iter().zip(plan.clashes) {
            out.push_str(&line(a, lang, clash));
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use usos_core::timetable::parse_activities;

    #[derive(Parser)]
    struct Wrap {
        #[command(flatten)]
        args: PlanArgs,
    }

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn range(flags: &[&str]) -> (NaiveDate, NaiveDate) {
        let w = Wrap::try_parse_from(std::iter::once("plan").chain(flags.iter().copied())).unwrap();
        resolve_range(&w.args, d("2026-11-04")).unwrap()
    }

    #[test]
    fn ranges() {
        let (mon, sun) = (d("2026-11-02"), d("2026-11-08"));
        assert_eq!(range(&[]), (mon, sun));
        assert_eq!(range(&["--today"]), (d("2026-11-04"), d("2026-11-04")));
        assert_eq!(range(&["--tomorrow"]), (d("2026-11-05"), d("2026-11-05")));
        assert_eq!(range(&["--next"]), (d("2026-11-09"), d("2026-11-15")));
        assert_eq!(
            range(&["--date", "2026-11-18"]),
            (d("2026-11-16"), d("2026-11-22"))
        );
        assert_eq!(range(&["--days", "3"]), (d("2026-11-04"), d("2026-11-06")));
        assert_eq!(
            range(&["--date", "2026-12-30", "--days", "5"]),
            (d("2026-12-30"), d("2027-01-03"))
        );
    }

    #[test]
    fn extreme_dates_are_errors() {
        let w = Wrap::try_parse_from(["plan", "--tomorrow"]).unwrap();
        assert!(resolve_range(&w.args, NaiveDate::MAX).is_err());
        let w = Wrap::try_parse_from(["plan", "--days", "5"]).unwrap();
        assert!(resolve_range(&w.args, NaiveDate::MAX).is_err());
    }

    #[test]
    fn marks_clashes() {
        let acts = parse_activities(serde_json::json!([
            {"type":"classgroup","start_time":"2026-11-05 08:00:00","end_time":"2026-11-05 10:00:00",
             "course_name":{"en":"A"},"classtype_id":"CW"},
            {"type":"classgroup","start_time":"2026-11-05 09:00:00","end_time":"2026-11-05 11:00:00",
             "course_name":{"en":"B"},"classtype_id":"CW"},
            {"type":"classgroup","start_time":"2026-11-05 11:00:00","end_time":"2026-11-05 12:00:00",
             "course_name":{"en":"C"},"classtype_id":"CW"}
        ]))
        .unwrap();
        let out = render(&acts, Lang::En, d("2026-11-05"), d("2026-11-05"));
        assert_eq!(
            out,
            "Thu 2026-11-05\n!08:00-10:00  CW    A\n!09:00-11:00  CW    B\n11:00-12:00  CW    C\n"
        );
        let json = json_view(&acts).unwrap();
        assert_eq!(json[0]["clash"], true);
        assert_eq!(json[2]["clash"], false);
    }

    #[test]
    fn conflicting_flags_rejected() {
        for flags in [
            &["--today", "--next"][..],
            &["--tomorrow", "--today"],
            &["--next", "--days", "3"],
            &["--today", "--date", "2026-11-04"],
            &["--days", "0"],
        ] {
            assert!(
                Wrap::try_parse_from(std::iter::once("plan").chain(flags.iter().copied())).is_err()
            );
        }
    }

    #[test]
    fn renders_grouped_days() {
        let acts = parse_activities(serde_json::json!([
            {"type":"classgroup","start_time":"2026-11-04 08:30:00","end_time":"2026-11-04 10:00:00",
             "course_name":{"en":"Mathematics","pl":"Matematyka"},"classtype_id":"CW",
             "building_name":{"en":"","pl":"Wydział AEI"},"room_number":"101"},
            {"type":"exam","start_time":"2026-11-06 12:00:00","end_time":"2026-11-06 13:30:00",
             "name":{"en":"Physics exam","pl":"Fizyka egzamin"}}
        ]))
        .unwrap();
        let out = render(&acts, Lang::En, d("2026-11-02"), d("2026-11-08"));
        assert_eq!(
            out,
            "Wed 2026-11-04\n08:30-10:00  CW    Mathematics  101, Wydział AEI\n\n\
             Fri 2026-11-06\n12:00-13:30  exam  Physics exam\n"
        );
        let pl = render(&acts, Lang::Pl, d("2026-11-02"), d("2026-11-08"));
        assert!(pl.starts_with("Śr 2026-11-04\n08:30-10:00  CW    Matematyka"));
        assert_eq!(
            render(&[], Lang::En, d("2026-11-02"), d("2026-11-08")),
            "No classes from 2026-11-02 to 2026-11-08.\n"
        );
    }
}
