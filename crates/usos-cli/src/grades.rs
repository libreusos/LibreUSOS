//! `usos grades`: grades per term, or the latest ones. Argument handling and formatting only;
//! fetching, parsing and the average are in `usos-core`.

use anyhow::{Result, bail};
use clap::Args;
use usos_core::Lang;
use usos_core::grades::{CourseGrades, Grade, LatestGrade, TermGrades};

use crate::cache::Data;

#[derive(Args)]
pub struct GradesArgs {
    /// Show this term (e.g. 2026/2027-Z) instead of the current one.
    #[arg(long, value_name = "ID", conflicts_with_all = ["all", "latest"])]
    term: Option<String>,
    /// Show every term you have course editions in.
    #[arg(long, conflicts_with = "latest")]
    all: bool,
    /// Show grades modified in the last DAYS days (default 7), newest first.
    #[arg(long, value_name = "DAYS", num_args = 0..=1, default_missing_value = "7",
        value_parser = clap::value_parser!(u32).range(1..))]
    latest: Option<u32>,
    /// Print the grades as JSON instead of text.
    #[arg(long)]
    json: bool,
}

pub async fn run(data: &Data, lang: Lang, args: GradesArgs) -> Result<()> {
    if let Some(days) = args.latest {
        // Not cached: the list is about what changed just now.
        let Some(client) = data.client() else {
            bail!("--latest needs a connection to USOS (it is not cached)");
        };
        let list = client
            .latest_grades(days)
            .await
            .map_err(|e| crate::explain(e, true))?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&list)?);
        } else {
            print!("{}", render_latest(&list, lang, days));
        }
        return Ok(());
    }
    let ids: Vec<String> = if let Some(id) = args.term {
        vec![id]
    } else if args.all {
        data.terms()
            .await?
            .terms
            .into_iter()
            .map(|t| t.id)
            .collect()
    } else {
        vec![data.current_term_id("use --term or --all").await?]
    };
    let mut terms: Vec<_> = data
        .term_grades(&ids)
        .await?
        .into_iter()
        .map(|r| r.grades)
        .collect();
    if args.all {
        terms.retain(|t| !t.courses.is_empty());
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&terms)?);
    } else {
        print!("{}", render_terms(&terms, lang));
    }
    Ok(())
}

fn text(pl: &'static str, en: &'static str, lang: Lang) -> &'static str {
    match lang {
        Lang::Pl => pl,
        Lang::En => en,
    }
}

/// The grade as students see it: the symbol, else the description.
fn shown(g: &Grade, lang: Lang) -> String {
    g.symbol
        .as_deref()
        .or_else(|| g.description.get(lang))
        .unwrap_or("-")
        .to_owned()
}

/// `s1 2  s2 4,5`
fn sessions(grades: &[Grade], lang: Lang) -> String {
    grades
        .iter()
        .map(|g| format!("s{} {}", g.session, shown(g, lang)))
        .collect::<Vec<_>>()
        .join("  ")
}

fn number(value: f64, lang: Lang) -> String {
    let s = format!("{value:.2}");
    match lang {
        Lang::Pl => s.replace('.', ","),
        Lang::En => s,
    }
}

fn render_course(c: &CourseGrades, lang: Lang, out: &mut String) {
    out.push_str(c.name.get(lang).unwrap_or(&c.course_id));
    out.push('\n');
    if !c.has_grades() {
        out.push_str(&format!(
            "  ({})\n",
            text("brak ocen", "no grades yet", lang)
        ));
        return;
    }
    if !c.grades.is_empty() {
        out.push_str(&format!(
            "  {:<6} {}\n",
            text("Końc.", "Final", lang),
            sessions(&c.grades, lang)
        ));
    }
    for u in c.units.iter().filter(|u| !u.grades.is_empty()) {
        let kind = u
            .classtype_id
            .as_deref()
            .or_else(|| u.classtype_name.get(lang))
            .unwrap_or(&u.unit_id);
        out.push_str(&format!("  {kind:<6} {}\n", sessions(&u.grades, lang)));
    }
}

fn render_terms(terms: &[TermGrades], lang: Lang) -> String {
    if terms.is_empty() {
        return format!("{}\n", text("Brak ocen.", "No grades.", lang));
    }
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{}\n", t.term_id));
        if t.courses.is_empty() {
            out.push_str(&format!("({})\n", text("brak ocen", "no grades", lang)));
            continue;
        }
        for c in &t.courses {
            render_course(c, lang, &mut out);
        }
        // Weighted when at least one course has a usable ECTS weight, else the unweighted one.
        let shown_average = match (t.weighted_average, t.average) {
            (Some(w), _) => Some((
                text("średnia ważona ECTS", "ECTS-weighted average", lang),
                w,
            )),
            (None, Some(a)) => Some((text("średnia (bez wag)", "unweighted average", lang), a)),
            (None, None) => None,
        };
        if let Some((label, value)) = shown_average {
            out.push_str(&format!("{label}: {}\n", number(value, lang)));
        }
    }
    out
}

fn render_latest(list: &[LatestGrade], lang: Lang, days: u32) -> String {
    if list.is_empty() {
        return match lang {
            Lang::Pl => format!("Brak ocen zmienionych w ciągu ostatnich {days} dni.\n"),
            Lang::En => format!("No grades modified in the last {days} days.\n"),
        };
    }
    let mut out = String::new();
    for l in list {
        let when = l
            .grade
            .date_modified
            .map_or_else(|| "-".to_owned(), |d| d.format("%Y-%m-%d").to_string());
        let course = l
            .course_name
            .get(lang)
            .or(l.course_id.as_deref())
            .unwrap_or("-");
        out.push_str(&format!(
            "{when}  {:<5} {course}  s{} {}\n",
            l.classtype_id
                .as_deref()
                .unwrap_or(text("Końc.", "Final", lang)),
            l.grade.session,
            shown(&l.grade, lang)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use usos_core::LangDict;
    use usos_core::grades::{UnitGrades, average, weighted_average};

    #[derive(Parser)]
    struct Wrap {
        #[command(flatten)]
        args: GradesArgs,
    }

    fn parse(flags: &[&str]) -> Result<Wrap, clap::Error> {
        Wrap::try_parse_from(std::iter::once("grades").chain(flags.iter().copied()))
    }

    fn grade(symbol: &str, session: u32) -> Grade {
        Grade {
            exam_id: Some("1".into()),
            session,
            symbol: Some(symbol.into()),
            counts_into_average: true,
            ..Grade::default()
        }
    }

    fn name(pl: &str, en: &str) -> LangDict {
        LangDict {
            pl: Some(pl.into()),
            en: Some(en.into()),
        }
    }

    fn term() -> TermGrades {
        let course = CourseGrades {
            course_id: "C1".into(),
            name: name("Matematyka", "Mathematics"),
            term_id: "T".into(),
            ects: Some(5.0),
            grades: vec![grade("2", 1), grade("3,5", 2)],
            units: vec![
                UnitGrades {
                    unit_id: "10".into(),
                    classtype_id: Some("WYK".into()),
                    classtype_name: LangDict::default(),
                    grades: vec![Grade {
                        symbol: Some("ZAL".into()),
                        session: 1,
                        ..Grade::default()
                    }],
                },
                UnitGrades {
                    unit_id: "11".into(),
                    classtype_id: Some("CW".into()),
                    classtype_name: LangDict::default(),
                    grades: vec![],
                },
            ],
        };
        let empty = CourseGrades {
            course_id: "C2".into(),
            name: LangDict::default(),
            term_id: "T".into(),
            ects: None,
            grades: vec![],
            units: vec![],
        };
        let courses = vec![course, empty];
        TermGrades {
            term_id: "T".into(),
            average: average(&courses),
            weighted_average: weighted_average(&courses),
            courses,
        }
    }

    #[test]
    fn flags() {
        assert!(parse(&[]).is_ok());
        assert_eq!(parse(&["--latest"]).unwrap().args.latest, Some(7));
        assert_eq!(parse(&["--latest", "30"]).unwrap().args.latest, Some(30));
        assert!(parse(&["--latest", "0"]).is_err());
        assert!(parse(&["--term", "X", "--all"]).is_err());
        assert!(parse(&["--latest", "--all"]).is_err());
        assert!(parse(&["--latest", "3", "--term", "X"]).is_err());
        assert!(parse(&["--all", "--json"]).is_ok());
    }

    #[test]
    fn renders_terms() {
        let out = render_terms(&[term()], Lang::En);
        assert_eq!(
            out,
            "T\nMathematics\n  Final  s1 2  s2 3,5\n  WYK    s1 ZAL\nC2\n  (no grades yet)\nECTS-weighted average: 3.50\n"
        );
        let mut unweighted = term();
        unweighted.weighted_average = None;
        assert!(
            render_terms(&[unweighted.clone()], Lang::En).ends_with("unweighted average: 3.50\n")
        );
        assert!(render_terms(&[unweighted], Lang::Pl).ends_with("średnia (bez wag): 3,50\n"));
        let pl = render_terms(&[term()], Lang::Pl);
        assert!(pl.contains("Matematyka\n  Końc."));
        assert!(pl.ends_with("średnia ważona ECTS: 3,50\n"));
        assert_eq!(render_terms(&[], Lang::En), "No grades.\n");
    }

    #[test]
    fn renders_latest() {
        let g = LatestGrade {
            course_id: Some("C1".into()),
            term_id: None,
            course_name: name("Matematyka", "Mathematics"),
            unit_id: None,
            classtype_id: Some("LAB".into()),
            grade: Grade {
                date_modified: "2026-12-01T08:00:00".parse().ok(),
                ..grade("4,5", 1)
            },
        };
        assert_eq!(
            render_latest(&[g], Lang::En, 30),
            "2026-12-01  LAB   Mathematics  s1 4,5\n"
        );
        assert_eq!(
            render_latest(&[], Lang::En, 30),
            "No grades modified in the last 30 days.\n"
        );
    }
}
