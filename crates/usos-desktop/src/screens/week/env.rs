//! What the Today and Week screens read from the world: the clock, the minute ticker, and the
//! cached plan as view-model items.

use std::time::Duration;

use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};
use gpui::{Context, Entity};
use usos_core::Term;

use super::model::{self, ClassItem};
use crate::i18n::Lang;
use crate::state::{AppState, data};

/// The installation-local time. Debug builds honour `USOS_DEMO_NOW=2026-11-10T11:07` so the
/// screens can be checked against the design's fixed moment; release builds never read it.
pub fn now() -> NaiveDateTime {
    #[cfg(debug_assertions)]
    if let Some(pinned) = std::env::var("USOS_DEMO_NOW")
        .ok()
        .and_then(|v| NaiveDateTime::parse_from_str(v.trim(), "%Y-%m-%dT%H:%M").ok())
    {
        return pinned;
    }
    Local::now().naive_local()
}

/// Debug builds only: a `USOS_DEMO_*` variable that steers the screens for screenshots (the
/// headless compositor has no pointer). Release builds always get `None`.
pub fn demo(name: &str) -> Option<String> {
    #[cfg(debug_assertions)]
    {
        std::env::var(name).ok()
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = name;
        None
    }
}

/// Repaint `cx`'s view at the start of every minute (the "do końca N min" text and the now line
/// move once a minute). Call once, from the first render.
pub fn spawn_minute_ticker<T: 'static>(cx: &mut Context<T>) {
    cx.spawn(async move |this, cx| {
        loop {
            let seconds = u64::from(Local::now().second());
            // A little past the minute, so the new minute has surely begun.
            let wait = Duration::from_millis((60 - seconds) * 1000 + 250);
            cx.background_executor().timer(wait).await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                return;
            }
        }
    })
    .detach();
}

/// Repaint `cx`'s view whenever the app state changes. Call once, from the first render.
pub fn observe_state<T: 'static>(state: &Entity<AppState>, cx: &mut Context<T>) {
    cx.observe(state, |_, _, cx| cx.notify()).detach();
}

/// The cached plan, ready to draw.
pub struct PlanView {
    /// `false` until a plan was fetched or read from the cache once.
    pub loaded: bool,
    pub items: Vec<ClassItem>,
    pub term: Option<Term>,
    /// The first and last day the cache covers.
    pub bounds: (NaiveDate, NaiveDate),
}

impl PlanView {
    pub fn read(state: &Entity<AppState>, today: NaiveDate, lang: Lang, cx: &gpui::App) -> Self {
        let state = state.read(cx);
        let plan = state.data.plan.as_ref();
        let items = plan
            .map(|plan| model::class_items(&plan.activities, lang))
            .unwrap_or_default();
        let fallback = data::plan_range(today).unwrap_or((today, today));
        let bounds = match (state.data.plan_from, state.data.plan_to) {
            (Some(from), Some(to)) => (from, to),
            _ => fallback,
        };
        PlanView {
            loaded: plan.is_some(),
            items,
            term: state.data.term.clone(),
            bounds,
        }
    }

    pub fn term_week(&self, date: NaiveDate) -> Option<u32> {
        self.term
            .as_ref()
            .and_then(|term| model::term_week(date, term))
    }
}
