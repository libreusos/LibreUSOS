//! The timetable: a week grid with a class detail pane, a day timeline below 960 px.

mod detail;
pub(super) mod env;
mod grid;
pub(super) mod model;
#[cfg(test)]
mod seed;
pub(super) mod timeline;

use std::rc::Rc;

use chrono::{Days, NaiveDate};
use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};

use self::env::PlanView;
use self::model::{ClassKey, HiddenWeekend};
use crate::assets::icons;
use crate::components::{Btn, BtnStyle, KindTag, Segmented};
use crate::i18n::{I18nExt, Text};
use crate::page::Page;
use crate::state::AppState;
use crate::theme::{Kind, ThemeExt};

const NARROW_BELOW: f32 = 960.0;

const DAY: Text = Text::new("Dzień", "Day");
const WEEK: Text = Text::new("Tydzień", "Week");
const TODAY: Text = Text::new("Dziś", "Today");
const PREVIOUS: Text = Text::new("Poprzedni", "Previous");
const NEXT: Text = Text::new("Następny", "Next");
const TERM_WEEK: Text = Text::new("tydzień", "week");
const NO_DATA: Text = Text::new(
    "Plan nie został jeszcze pobrany. Odśwież, gdy będziesz online.",
    "The timetable has not been fetched yet. Refresh when you are online.",
);
const NO_CLASSES_WEEK: Text = Text::new("Brak zajęć w tym tygodniu", "No classes this week");
const NO_CLASSES_DAY: Text = Text::new("Brak zajęć tego dnia", "No classes this day");
const LEGEND_LECTURE: Text = Text::new("wykład", "lecture");
const LEGEND_LAB: Text = Text::new("laboratorium", "laboratory");
const LEGEND_OTHER: Text = Text::new(
    "ćwiczenia, projekt, seminarium i inne",
    "classes, project, seminar and others",
);
const LEGEND_EXAM: Text = Text::new("egzamin, kolokwium", "exam, test");
const LEGEND_PAST: Text = Text::new("minione", "past");
const WEEKEND_BOTH: Text = Text::new(
    "Sob. i niedz. bez zajęć: ukryte",
    "Sat. and Sun. without classes: hidden",
);
const WEEKEND_SUNDAY: Text = Text::new("Niedz. bez zajęć: ukryta", "Sun. without classes: hidden");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Day,
    Week,
}

pub struct WeekScreen {
    mode: Mode,
    /// The day being looked at; `None` follows today.
    anchor: Option<NaiveDate>,
    selected: Option<ClassKey>,
    /// Debug builds: the start time ("12:45") of a class to open on the first render.
    demo_select: Option<String>,
    started: bool,
}

impl WeekScreen {
    pub fn new() -> Self {
        Self {
            mode: if env::demo("USOS_DEMO_MODE").as_deref() == Some("day") {
                Mode::Day
            } else {
                Mode::Week
            },
            anchor: None,
            selected: None,
            demo_select: env::demo("USOS_DEMO_SELECT"),
            started: false,
        }
    }

    fn legend(&self, hidden: HiddenWeekend, cx: &gpui::App) -> AnyElement {
        let t = *cx.tokens();
        let entry = |tag: KindTag, text: SharedString| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(tag)
                .child(text)
        };
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(16.0))
            .gap_y(px(6.0))
            .text_size(px(12.5))
            .text_color(t.on_surface_variant)
            .child(entry(
                KindTag::new("WYK", Kind::Lecture),
                cx.t(&LEGEND_LECTURE),
            ))
            .child(entry(KindTag::new("LAB", Kind::Lab), cx.t(&LEGEND_LAB)))
            .child(entry(KindTag::new("ĆW", Kind::Other), cx.t(&LEGEND_OTHER)))
            .child(entry(KindTag::new("EGZ", Kind::Exam), cx.t(&LEGEND_EXAM)))
            .child(entry(
                KindTag::new("---", Kind::Other).past(true),
                cx.t(&LEGEND_PAST),
            ));
        match hidden {
            HiddenWeekend::Both => row = row.child(cx.t(&WEEKEND_BOTH)),
            HiddenWeekend::Sunday => row = row.child(cx.t(&WEEKEND_SUNDAY)),
            HiddenWeekend::None => {}
        }
        row.into_any_element()
    }
}

impl Render for WeekScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::global(cx);
        if !self.started {
            self.started = true;
            env::spawn_minute_ticker(cx);
            env::observe_state(&state, cx);
        }
        let t = *cx.tokens();
        let lang = cx.lang();
        let now = env::now();
        let today = now.date();
        let view = PlanView::read(&state, today, lang, cx);
        let narrow = window.viewport_size().width < px(NARROW_BELOW);
        let mode = if narrow { Mode::Day } else { self.mode };

        let bounds = model::week_bounds(view.bounds.0, view.bounds.1);
        let day = self
            .anchor
            .unwrap_or(today)
            .clamp(view.bounds.0, view.bounds.1.max(view.bounds.0));
        let monday = model::week_monday(day);
        let days = model::visible_days(monday, &view.items);
        let this = cx.entity();
        if let Some(time) = self.demo_select.take() {
            self.selected = view
                .items
                .iter()
                .find(|item| item.start.date() == day && model::time_text(item.start) == time)
                .map(|item| item.key.clone());
        }

        // Header: title, eyebrow, controls.
        let title = match mode {
            Mode::Week => model::week_title(monday, lang),
            Mode::Day => model::date_long(day, lang),
        };
        let mut eyebrow = model::month_year(monday, lang).to_uppercase();
        if let Some(week) = view.term_week(day) {
            eyebrow = format!("{eyebrow} · {} {week}", cx.t(&TERM_WEEK).to_uppercase());
        }
        let (can_prev, can_next) = match mode {
            Mode::Week => (monday > bounds.0, monday < bounds.1),
            Mode::Day => (day > view.bounds.0, day < view.bounds.1),
        };
        let step = move |forward: bool| {
            let this = this.clone();
            let bounds = view.bounds;
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                this.update(cx, |screen, cx| {
                    let from = screen
                        .anchor
                        .unwrap_or(today)
                        .clamp(bounds.0, bounds.1.max(bounds.0));
                    let days = if screen.mode == Mode::Week && !narrow {
                        7
                    } else {
                        1
                    };
                    let moved = if forward {
                        from + Days::new(days)
                    } else {
                        from - Days::new(days)
                    };
                    screen.anchor = Some(moved.clamp(bounds.0, bounds.1.max(bounds.0)));
                    screen.selected = None;
                    cx.notify();
                });
            }
        };
        let mut page = Page::new(title).eyebrow(eyebrow);
        if !narrow {
            let this = cx.entity();
            page = page.action(
                Segmented::new(
                    "week-mode",
                    vec![cx.t(&DAY), cx.t(&WEEK)],
                    usize::from(mode == Mode::Week),
                )
                .on_select(move |index, _, cx| {
                    this.update(cx, |screen, cx| {
                        screen.mode = if index == 0 { Mode::Day } else { Mode::Week };
                        cx.notify();
                    });
                }),
            );
        }
        let this = cx.entity();
        page = page
            .action(
                Btn::new("week-today", BtnStyle::Outlined)
                    .label(cx.t(&TODAY))
                    .on_click(move |_, _, cx| {
                        this.update(cx, |screen, cx| {
                            screen.anchor = None;
                            screen.selected = None;
                            cx.notify();
                        });
                    }),
            )
            .action(
                Btn::new("week-prev", BtnStyle::Ghost)
                    .icon(icons::CHEVRON_LEFT)
                    .tooltip(cx.t(&PREVIOUS))
                    .disabled(!can_prev)
                    .on_click(step(false)),
            )
            .action(
                Btn::new("week-next", BtnStyle::Ghost)
                    .icon(icons::CHEVRON_RIGHT)
                    .tooltip(cx.t(&NEXT))
                    .disabled(!can_next)
                    .on_click(step(true)),
            );

        // The selected class, if it is still in the data.
        let selected_index = self
            .selected
            .as_ref()
            .and_then(|key| view.items.iter().position(|item| &item.key == key));
        let this = cx.entity();
        let on_select: timeline::SelectHandler = Rc::new(move |key, _, cx| {
            this.update(cx, |screen, cx| {
                screen.selected = if screen.selected.as_ref() == Some(key) {
                    None
                } else {
                    Some(key.clone())
                };
                cx.notify();
            });
        });
        let this = cx.entity();
        let on_close: detail::CloseHandler = Rc::new(move |_, cx| {
            this.update(cx, |screen, cx| {
                screen.selected = None;
                cx.notify();
            });
        });

        // Narrow: an open class takes the whole page.
        if let (true, Some(index)) = (narrow, selected_index) {
            return page
                .child(div().w_full().p(px(16.0)).child(detail::detail(
                    &view.items,
                    index,
                    true,
                    on_close,
                    cx,
                )))
                .into_any_element();
        }

        let week_has_classes = days
            .iter()
            .any(|d| view.items.iter().any(|item| item.start.date() == *d));
        let main: AnyElement = if !view.loaded {
            message(cx.t(&NO_DATA), cx)
        } else if mode == Mode::Week {
            if week_has_classes {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(grid::week_grid(
                        grid::GridSpec {
                            items: &view.items,
                            days: &days,
                            today,
                            now,
                            selected: self.selected.as_ref(),
                            on_select,
                        },
                        cx,
                    ))
                    .child(self.legend(model::hidden_weekend(&days), cx))
                    .into_any_element()
            } else {
                message(cx.t(&NO_CLASSES_WEEK), cx)
            }
        } else {
            let indices = model::day_indices(&view.items, day);
            let mut column = div().flex().flex_col().gap(px(12.0));
            if narrow {
                column = column.child(day_strip(&days, day, lang, cx.entity(), cx));
            }
            column
                .child(if indices.is_empty() {
                    message(cx.t(&NO_CLASSES_DAY), cx)
                } else {
                    timeline::day_timeline(
                        timeline::Timeline {
                            id: "week-day",
                            items: &view.items,
                            day: &indices,
                            now,
                            running_line: None,
                            selected: self.selected.as_ref(),
                            on_select: Some(on_select),
                        },
                        cx,
                    )
                })
                .into_any_element()
        };

        let mut body = div()
            .flex()
            .items_start()
            .gap(px(24.0))
            .w_full()
            .px(px(if narrow { 16.0 } else { 32.0 }))
            .py(px(24.0))
            .child(div().flex_1().min_w_0().child(main));
        if let Some(index) = selected_index {
            body = body.child(
                div()
                    .w(px(detail::WIDTH))
                    .flex_none()
                    .pl(px(24.0))
                    .border_l_1()
                    .border_color(t.outline_variant)
                    .child(detail::detail(&view.items, index, false, on_close, cx)),
            );
        }
        page.child(body).into_any_element()
    }
}

fn message(text: SharedString, cx: &gpui::App) -> AnyElement {
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

/// The narrow layout's day picker: one chip per visible day of the week.
fn day_strip(
    days: &[NaiveDate],
    selected: NaiveDate,
    lang: crate::i18n::Lang,
    this: gpui::Entity<WeekScreen>,
    _cx: &gpui::App,
) -> AnyElement {
    use chrono::Datelike;
    let labels: Vec<SharedString> = days
        .iter()
        .map(|d| format!("{} {}", model::weekday_short(*d, lang), d.day()).into())
        .collect();
    // A hidden weekend day has no chip: highlight nothing rather than Monday.
    let index = days
        .iter()
        .position(|d| *d == selected)
        .unwrap_or(usize::MAX);
    let days = days.to_vec();
    Segmented::new("week-days", labels, index)
        .on_select(move |i, _, cx| {
            if let Some(day) = days.get(i).copied() {
                this.update(cx, |screen, cx| {
                    screen.anchor = Some(day);
                    screen.selected = None;
                    cx.notify();
                });
            }
        })
        .into_any_element()
}
