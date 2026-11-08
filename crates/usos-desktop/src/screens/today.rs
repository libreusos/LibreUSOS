//! Today: the running class with a progress bar, the day as a timeline, and what comes next.
//! Terminy and Nowe wyniki wait for core support and are disabled "wkrótce" cards.

mod cards;

use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    div, px,
};

use super::week::env::{self, PlanView};
use super::week::model::{self, ClassItem, Focus, NowNext};
use super::week::timeline::{self, clash_badge, place_and_group};
use crate::assets::icons;
use crate::components::{Btn, BtnStyle, KindTag, MOTIF_DAYS, ProgressBar, SoonCard, WeekMotif};
use crate::i18n::{I18nExt, Text, strings};
use crate::nav::{Nav, Route};
use crate::page::Page;
use crate::state::AppState;
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

const NARROW_BELOW: f32 = 960.0;
const SIDE_WIDTH: f32 = 380.0;

const WEEK: Text = Text::new("Tydzień", "Week");
const WHOLE_WEEK: Text = Text::new("Cały tydzień", "Whole week");
const REFRESH: Text = Text::new("Odśwież", "Refresh");
const TERM_WEEK: Text = Text::new("tydzień", "week");
const DAY_PLAN: Text = Text::new("Plan dnia", "Day plan");
/// The now card sits above the day in the narrow layout and beside it in the wide one.
const RUNNING_ABOVE: Text = Text::new("w toku · szczegóły wyżej", "in progress · details above");
const RUNNING_BESIDE: Text = Text::new("w toku · szczegóły obok", "in progress · details beside");
const NOW: Text = Text::new("Teraz", "Now");
const UP_NEXT: Text = Text::new("Następne", "Up next");
const NEXT: Text = Text::new("Następne", "Next");
const LEFT_PREFIX: Text = Text::new("do końca ", "");
const LEFT_SUFFIX: Text = Text::new("", " left");
const STARTS_IN: Text = Text::new("za ", "in ");
const NO_CLASSES: Text = Text::new("Dziś bez zajęć", "No classes today");
const NO_CLASSES_HINT: Text = Text::new(
    "Na dziś nie ma nic w planie.",
    "Nothing is scheduled for today.",
);
const UPCOMING: Text = Text::new("Najbliższe zajęcia", "Next classes");
const MORE_PREFIX: Text = Text::new("Jeszcze ", "Plus ");
const CLASH_AT: Text = Text::new(", w tym kolizja o ", ", including a clash at ");
const NO_DATA: Text = Text::new(
    "Plan nie został jeszcze pobrany. Odśwież, gdy będziesz online.",
    "The timetable has not been fetched yet. Refresh when you are online.",
);
const DEADLINES_HINT: Text = Text::new(
    "Terminy zaliczeń i rejestracji pojawią się tutaj.",
    "Deadlines and registrations will appear here.",
);
const RESULTS: Text = Text::new("Nowe wyniki", "New results");
const RESULTS_HINT: Text = Text::new(
    "Ostatnio opublikowane oceny i punkty pojawią się tutaj.",
    "Recently published grades and points will appear here.",
);
const MOTIF_CLASSES: Text = Text::new("zajęcia", "classes");
const MOTIF_FREE: Text = Text::new("wolne", "free");
const MOTIF_TODAY: Text = Text::new("dziś", "today");

pub struct TodayScreen {
    started: bool,
}

impl TodayScreen {
    pub fn new() -> Self {
        Self { started: false }
    }
}

impl Render for TodayScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx);
        if !self.started {
            self.started = true;
            env::spawn_minute_ticker(cx);
            env::observe_state(&state, cx);
            if env::demo("USOS_DEMO_ROUTE").as_deref() == Some("week") {
                cx.spawn(async move |_, cx| {
                    cx.update(|cx| Nav::go(Route::Week, cx));
                })
                .detach();
            }
        }
        let lang = cx.lang();
        let now = env::now();
        let today = now.date();
        let view = PlanView::read(&state, today, lang, cx);
        let narrow = window.viewport_size().width < px(NARROW_BELOW);

        let mut eyebrow = format!(
            "{} {}",
            model::weekday_name(today, lang),
            model::day_month(today, lang)
        )
        .to_uppercase();
        if let Some(week) = view.term_week(today) {
            eyebrow = format!("{eyebrow} · {} {week}", cx.t(&TERM_WEEK).to_uppercase());
        }
        let refresh_state = state.clone();
        let page = Page::new(cx.t(&strings::NAV_TODAY))
            .eyebrow(eyebrow)
            .action(
                Btn::new("today-week", BtnStyle::Outlined)
                    .icon(icons::WEEK)
                    .label(cx.t(&WEEK))
                    .on_click(|_, _, cx| Nav::go(Route::Week, cx)),
            )
            .action(
                Btn::new("today-sync", BtnStyle::Ghost)
                    .icon(icons::SYNC)
                    .tooltip(cx.t(&REFRESH))
                    .on_click(move |_, _, cx| refresh_state.update(cx, |s, cx| s.sync(cx))),
            );

        if !view.loaded {
            return page.child(message(cx.t(&NO_DATA), cx)).into_any_element();
        }

        let day = model::day_indices(&view.items, today);
        let focus = model::now_next(&view.items, now);
        let main = if day.is_empty() {
            cards::empty_day(&view, today, cx)
        } else {
            cards::running_day(&view, &day, now, narrow, cx)
        };
        let now_card = (focus.focus.is_some() && !day.is_empty())
            .then(|| cards::now_card(&view.items, focus, now, cx));
        let soon = div()
            .flex()
            .flex_col()
            .gap(px(24.0))
            .child(SoonCard::new(
                "soon-deadlines",
                icons::DEADLINES,
                cx.t(&strings::NAV_DEADLINES),
                cx.t(&DEADLINES_HINT),
            ))
            .child(SoonCard::new(
                "soon-results",
                icons::GRADES,
                cx.t(&RESULTS),
                cx.t(&RESULTS_HINT),
            ));

        let pad = if narrow { 16.0 } else { 32.0 };
        let body = if narrow {
            // The now card first, then the day, then what is still to come.
            div()
                .flex()
                .flex_col()
                .gap(px(24.0))
                .p(px(pad))
                .children(now_card)
                .child(main)
                .child(soon)
        } else {
            let side = div()
                .flex()
                .flex_col()
                .gap(px(24.0))
                .children(now_card)
                .child(soon);
            div()
                .flex()
                .items_start()
                .gap(px(40.0))
                .px(px(pad))
                .py(px(24.0))
                .child(div().flex_1().min_w_0().child(main))
                .child(div().w(px(SIDE_WIDTH)).flex_none().child(side))
        };
        page.child(body).into_any_element()
    }
}

fn message(text: SharedString, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    div()
        .py(px(48.0))
        .flex()
        .justify_center()
        .text_size(px(14.0))
        .text_color(t.on_surface_variant)
        .child(text)
        .into_any_element()
}
