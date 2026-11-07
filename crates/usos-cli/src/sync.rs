//! `usos sync`: refresh the offline cache. Printing only; the work is `usos_store::repo::sync`.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args;
use usos_core::Session;
use usos_store::repo::{self, SyncReport};

use crate::cache::open_store;
use crate::explain_repo;

#[derive(Args)]
pub struct SyncArgs {
    /// Weeks of the timetable to fetch, starting with the current one.
    #[arg(long, value_name = "N", default_value_t = 3,
        value_parser = clap::value_parser!(u32).range(1..=26))]
    weeks: u32,
}

pub async fn run(ctx: &Session, cache: Option<PathBuf>, args: SyncArgs) -> Result<()> {
    let store = open_store(cache)?;
    let client = ctx.client()?;
    if !client.is_logged_in() {
        bail!(
            "not logged in to {}; run `usos login` first",
            ctx.base_url()
        );
    }
    let today = chrono::Local::now().date_naive();
    let report = repo::sync(&client, &store, today, args.weeks)
        .await
        .map_err(|e| explain_repo(e, "the data"))?;
    print!("{}", render(&report));
    Ok(())
}

fn state(changed: bool) -> &'static str {
    if changed { "changed" } else { "unchanged" }
}

fn render(r: &SyncReport) -> String {
    let mut out = format!(
        "user: {}\nterms: {} ({})\nplan {}..{}: {} activities ({})\n",
        state(r.user_changed),
        r.terms,
        state(r.terms_changed),
        r.plan_from,
        r.plan_to,
        r.activities,
        state(r.plan_changed)
    );
    match &r.term_id {
        Some(term) => {
            out.push_str(&format!(
                "grades {term}: {} courses ({})\ntests {term}: {} tests ({})\n",
                r.courses,
                state(r.grades_changed),
                r.tests,
                state(r.tests_changed)
            ));
            if r.tests_skipped > 0 {
                out.push_str(&format!("{} tests could not be loaded\n", r.tests_skipped));
            }
        }
        None => out.push_str("no current term: grades and tests skipped\n"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_counts_only() {
        let r = SyncReport {
            account: "a".into(),
            user_changed: false,
            terms: 3,
            terms_changed: true,
            plan_from: "2026-11-02".parse().unwrap(),
            plan_to: "2026-11-22".parse().unwrap(),
            activities: 12,
            plan_changed: true,
            term_id: Some("T1".into()),
            courses: 7,
            grades_changed: false,
            tests: 0,
            tests_skipped: 1,
            tests_changed: false,
        };
        assert_eq!(
            render(&r),
            "user: unchanged\nterms: 3 (changed)\nplan 2026-11-02..2026-11-22: 12 activities (changed)\n\
             grades T1: 7 courses (unchanged)\ntests T1: 0 tests (unchanged)\n1 tests could not be loaded\n"
        );
    }
}
