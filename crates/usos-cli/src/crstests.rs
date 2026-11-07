//! `usos tests`: course tests ("sprawdziany") per term as a tree. Argument handling and
//! formatting only; fetching, parsing and the totals are in `usos-core`.

use anyhow::Result;
use clap::Args;
use usos_core::Lang;
use usos_core::crstests::{NodeKind, TestNode, TestRoot};

use crate::cache::Data;

#[derive(Args)]
pub struct TestsArgs {
    /// Show this term (e.g. 2026/2027-Z) instead of the current one.
    #[arg(long, value_name = "ID")]
    term: Option<String>,
    /// Print the tests as JSON instead of text.
    #[arg(long)]
    json: bool,
}

pub async fn run(data: &Data, lang: Lang, args: TestsArgs) -> Result<()> {
    let term = match args.term {
        Some(id) => id,
        None => data.current_term_id("use --term").await?,
    };
    let result = data.course_tests(&term).await?.tests;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print!("{}", render_tests(&result.tests, &term, lang));
    }
    if !result.skipped.is_empty() {
        eprintln!(
            "{}",
            match lang {
                Lang::Pl => format!(
                    "Nie udało się wczytać sprawdzianów: {}.",
                    result.skipped.len()
                ),
                Lang::En => format!("{} tests could not be loaded.", result.skipped.len()),
            }
        );
    }
    Ok(())
}

fn text(pl: &'static str, en: &'static str, lang: Lang) -> &'static str {
    match lang {
        Lang::Pl => pl,
        Lang::En => en,
    }
}

/// Up to two decimals, without trailing zeros; a decimal comma in Polish.
fn number(value: f64, lang: Lang) -> String {
    let s = format!("{value:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    match lang {
        Lang::Pl => s.replace('.', ","),
        Lang::En => s.to_owned(),
    }
}

/// `7 / 10`, `- / 10`, `7` (no maximum), or empty when the node has neither.
fn points(n: &TestNode, lang: Lang) -> String {
    match (n.points, n.points_max) {
        (Some(p), Some(m)) => format!("{} / {}", number(p, lang), number(m, lang)),
        (None, Some(m)) => format!("- / {}", number(m, lang)),
        (Some(p), None) => number(p, lang),
        (None, None) => String::new(),
    }
}

/// The value column: points for tasks and folders, the grade for grade nodes.
fn value(n: &TestNode, lang: Lang) -> String {
    if n.hidden {
        return format!("({})", text("wyniki ukryte", "results hidden", lang));
    }
    if n.kind == NodeKind::Grade {
        return n
            .grade
            .as_ref()
            .and_then(|g| g.symbol.as_deref().or_else(|| g.description.get(lang)))
            .unwrap_or("-")
            .to_owned();
    }
    points(n, lang)
}

struct Row {
    label: String,
    value: String,
    note: String,
}

fn collect(n: &TestNode, depth: usize, lang: Lang, rows: &mut Vec<Row>) {
    for c in &n.children {
        let mut note = c
            .last_changed
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        if let Some(comment) = &c.comment {
            if !note.is_empty() {
                note.push_str("  ");
            }
            let comment = comment.replace(['\r', '\n'], " ");
            note.push_str(&format!("\"{comment}\""));
        }
        rows.push(Row {
            label: format!(
                "{}{}",
                "  ".repeat(depth),
                c.name.get(lang).unwrap_or(&c.id)
            ),
            value: value(c, lang),
            note,
        });
        collect(c, depth + 1, lang, rows);
    }
}

fn render_test(t: &TestRoot, lang: Lang, out: &mut String) {
    let title = t.course_name.get(lang).unwrap_or(&t.course_id);
    let total = points(&t.root, lang);
    out.push_str(title);
    if !total.is_empty() {
        out.push_str(&format!("  [{total}]"));
    }
    if t.limited_to_groups {
        out.push_str(&format!(
            "  ({})",
            text("tylko wybrane grupy", "selected groups only", lang)
        ));
    }
    out.push('\n');
    let mut rows = Vec::new();
    collect(&t.root, 1, lang, &mut rows);
    if rows.is_empty() {
        out.push_str(&format!("  ({})\n", text("pusty", "empty", lang)));
        return;
    }
    let width = rows
        .iter()
        .map(|r| r.label.chars().count())
        .max()
        .unwrap_or(0);
    let value_width = rows
        .iter()
        .map(|r| r.value.chars().count())
        .max()
        .unwrap_or(0);
    for r in rows {
        let line = format!("{:<width$}  {:<value_width$}  {}", r.label, r.value, r.note);
        out.push_str(line.trim_end());
        out.push('\n');
    }
}

fn render_tests(tests: &[TestRoot], term: &str, lang: Lang) -> String {
    if tests.is_empty() {
        return match lang {
            Lang::Pl => format!("Brak sprawdzianów w semestrze {term}.\n"),
            Lang::En => format!("No tests in term {term}.\n"),
        };
    }
    let mut out = String::new();
    for (i, t) in tests.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_test(t, lang, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use usos_core::LangDict;
    use usos_core::crstests::NodeGrade;

    #[derive(Parser)]
    struct Wrap {
        #[command(flatten)]
        args: TestsArgs,
    }

    fn name(pl: &str, en: &str) -> LangDict {
        LangDict {
            pl: Some(pl.into()),
            en: Some(en.into()),
        }
    }

    fn node(id: &str, kind: NodeKind, n: LangDict, children: Vec<TestNode>) -> TestNode {
        TestNode {
            id: id.into(),
            kind,
            name: n,
            children,
            ..TestNode::default()
        }
    }

    fn test() -> TestRoot {
        let mut t1 = node("11", NodeKind::Task, name("Zadanie 1", "Task 1"), vec![]);
        t1.points = Some(7.5);
        t1.points_max = Some(10.0);
        t1.comment = Some("ok".into());
        t1.last_changed = "2026-11-02T12:00:00".parse().ok();
        let mut t2 = node("12", NodeKind::Task, name("Zadanie 2", ""), vec![]);
        t2.points_max = Some(10.0);
        let mut f = node(
            "10",
            NodeKind::Folder,
            name("Kolokwium", "Quiz"),
            vec![t1, t2],
        );
        f.points = Some(7.5);
        f.points_max = Some(20.0);
        let mut g = node("13", NodeKind::Grade, name("Ocena", "Grade"), vec![]);
        g.grade = Some(NodeGrade {
            symbol: Some("4,5".into()),
            ..NodeGrade::default()
        });
        let g2 = node("14", NodeKind::Grade, name("Zaliczenie", "Credit"), vec![]);
        let mut root = node(
            "9",
            NodeKind::Root,
            name("Analiza", "Analysis"),
            vec![f, g, g2],
        );
        root.points = Some(7.5);
        root.points_max = Some(20.0);
        TestRoot {
            course_id: "C1".into(),
            course_name: name("Analiza", "Analysis"),
            term_id: "T".into(),
            limited_to_groups: true,
            root,
        }
    }

    #[test]
    fn flags() {
        let p =
            |f: &[&str]| Wrap::try_parse_from(std::iter::once("tests").chain(f.iter().copied()));
        assert!(p(&[]).is_ok());
        assert_eq!(
            p(&["--term", "X", "--json"]).unwrap().args.term.as_deref(),
            Some("X")
        );
        assert!(p(&["--all"]).is_err());
    }

    #[test]
    fn numbers() {
        assert_eq!(number(7.0, Lang::En), "7");
        assert_eq!(number(7.5, Lang::Pl), "7,5");
        assert_eq!(number(0.1 + 0.2, Lang::En), "0.3");
        assert_eq!(number(10.0, Lang::Pl), "10");
    }

    #[test]
    fn renders_a_tree() {
        let out = render_tests(&[test()], "T", Lang::En);
        assert_eq!(
            out,
            "Analysis  [7.5 / 20]  (selected groups only)\n\
             \x20 Quiz         7.5 / 20\n\
             \x20   Task 1     7.5 / 10  2026-11-02  \"ok\"\n\
             \x20   Zadanie 2  - / 10\n\
             \x20 Grade        4,5\n\
             \x20 Credit       -\n"
        );
        let pl = render_tests(&[test()], "T", Lang::Pl);
        assert!(pl.starts_with("Analiza  [7,5 / 20]  (tylko wybrane grupy)\n  Kolokwium"));
    }

    #[test]
    fn renders_hidden_and_multiline_comments() {
        let mut t = test();
        let f = &mut t.root.children[0];
        f.children[0].comment = Some("a\nb".into());
        f.children[1].hidden = true;
        let out = render_tests(&[t], "T", Lang::En);
        assert!(out.contains("\"a b\""));
        assert!(out.contains("(results hidden)"));
    }

    #[test]
    fn renders_empty() {
        assert_eq!(render_tests(&[], "T", Lang::En), "No tests in term T.\n");
        assert_eq!(
            render_tests(&[], "T", Lang::Pl),
            "Brak sprawdzianów w semestrze T.\n"
        );
        let mut t = test();
        t.limited_to_groups = false;
        t.root = node("9", NodeKind::Root, LangDict::default(), vec![]);
        assert_eq!(render_tests(&[t], "T", Lang::En), "Analysis\n  (empty)\n");
    }
}
