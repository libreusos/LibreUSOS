//! Sprawdziany: the tests of one term (points from the lecturer's entries in USOS) as a table
//! whose rows open into their tasks. Results the lecturer hid, and tests limited to class
//! groups, say so instead of showing numbers.

pub(crate) mod model;

use std::collections::HashSet;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use self::model::{ChildRow, Outcome, TestRow, TestsView};
use super::grades::term::{self, Slot};
use crate::assets::{self, icons};
use crate::components::{EmptyState, GradeTile, KindTag, ProgressBar, mono_label, tnum};
use crate::i18n::{I18nExt, Lang, Text, strings};
use crate::page::Page;
use crate::state::{AppState, SyncStatus};
use crate::theme::{ThemeExt, Tokens};
use crate::view_util;

const COL_TYPE: Text = Text::new("Typ", "Type");
const COL_TEST: Text = Text::new("Sprawdzian", "Test");
const COL_LAST: Text = Text::new("Ostatnio", "Updated");
const COL_RESULT: Text = Text::new("Wynik", "Result");
const COL_POINTS: Text = Text::new("Punkty", "Points");
const HIDDEN: Text = Text::new(
    "Wyniki ukryte przez prowadzącego",
    "Results hidden by the lecturer",
);
const GROUPS_ONLY: Text = Text::new(
    "Dostępne tylko dla wybranych grup",
    "Only available to some groups",
);
const UNGRADED: Text = Text::new("nieocenione", "not graded");
const FOOTNOTE: Text = Text::new(
    "Punkty to suma wpisów prowadzącego w USOS. LibreUSOS niczego nie przelicza na oceny.",
    "Points are the sum of the lecturer's entries in USOS. LibreUSOS does not turn them into grades.",
);
const SKIPPED: Text = Text::new(
    "Część sprawdzianów nie wczytała się; USOS odmówił ich pokazania.",
    "Some tests could not be loaded; USOS refused to show them.",
);
const EMPTY_TITLE: Text = Text::new("Brak sprawdzianów", "No tests");
const EMPTY_BODY: Text = Text::new(
    "Prowadzący nie dodali jeszcze żadnych sprawdzianów ani punktów w tym semestrze.",
    "No lecturer has added tests or points for this term yet.",
);
const LOADING_TITLE: Text = Text::new("Wczytywanie sprawdzianów…", "Loading tests…");
const UNAVAILABLE_TITLE: Text = Text::new(
    "Nie udało się pobrać sprawdzianów",
    "The tests could not be loaded",
);
const UNAVAILABLE_BODY: Text = Text::new(
    "Spróbuj ponownie, gdy wrócisz do sieci. Zapisanych danych dla tego semestru nie ma.",
    "Try again when you are back online. There is no saved data for this term.",
);
const NO_TERM_TITLE: Text = Text::new("Nie znaleziono semestru", "No term found");

const W_TAG: f32 = 56.0;
const W_LAST: f32 = 90.0;
const W_BAR: f32 = 170.0;
const W_POINTS: f32 = 120.0;
const W_CHEVRON: f32 = 32.0;

pub struct TestsScreen {
    term: Option<String>,
    menu_open: bool,
    /// Root ids of the opened tests.
    open: HashSet<String>,
}

impl TestsScreen {
    pub fn new() -> Self {
        Self {
            term: crate::demo::var("USOS_DEMO_TERM"),
            menu_open: false,
            open: crate::demo::var("USOS_DEMO_TEST").into_iter().collect(),
        }
    }
}

enum Body {
    Table(TestsView),
    Loading,
    Unavailable,
    NoTerm,
}

impl Render for TestsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        let lang = cx.lang();
        let today = view_util::today();
        let state = AppState::global(cx);

        let (term_view, body, missing) = {
            let app = state.read(cx);
            let term_view = term::resolve(&app.data, self.term.as_deref(), today);
            let mut missing = None;
            let syncing = app.sync == SyncStatus::Syncing;
            let body = match &term_view.shown {
                None if syncing => Body::Loading,
                None => Body::NoTerm,
                Some(shown) => {
                    if term::needs_load(&app.data, &app.term_cache, &shown.id) {
                        missing = Some(shown.id.clone());
                    }
                    match term::tests_slot(&app.data, &app.term_cache, &shown.id, syncing) {
                        Slot::Ready(result) => Body::Table(model::build(&result.tests, lang)),
                        Slot::Loading => Body::Loading,
                        Slot::Missing => Body::Loading,
                        Slot::Unavailable => Body::Unavailable,
                    }
                }
            };
            (term_view, body, missing)
        };
        if let Some(term_id) = missing {
            let state = state.clone();
            cx.defer(move |cx| state.update(cx, |state, cx| state.load_term(&term_id, cx)));
        }

        let toolbar = self.toolbar(&term_view, &body, today, lang, &t, cx);
        let content = match body {
            Body::Table(view) => self.table(view, lang, &t, cx),
            Body::Loading => {
                EmptyState::new(icons::PROGRESS, cx.t(&LOADING_TITLE)).into_any_element()
            }
            Body::Unavailable => EmptyState::new(icons::OFFLINE, cx.t(&UNAVAILABLE_TITLE))
                .body(cx.t(&UNAVAILABLE_BODY))
                .into_any_element(),
            Body::NoTerm => EmptyState::new(icons::TESTS, cx.t(&NO_TERM_TITLE)).into_any_element(),
        };

        Page::new(cx.t(&strings::NAV_TESTS)).child(
            div()
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .child(toolbar)
                .child(content)
                .when(self.menu_open, |d| {
                    d.child(term::scrim(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.menu_open = false;
                        cx.notify();
                    })))
                }),
        )
    }
}

impl TestsScreen {
    fn toolbar(
        &self,
        term_view: &term::TermView,
        body: &Body,
        today: chrono::NaiveDate,
        lang: Lang,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let shown_id = term_view.shown.as_ref().map(|t| t.id.as_str());
        let label = term_view
            .shown
            .as_ref()
            .map_or_else(String::new, |term| term::chip_label(term, today, lang));
        let items = term::menu_items(&term_view.options, shown_id, today, lang);
        let ids: Vec<String> = term_view.options.iter().map(|t| t.id.clone()).collect();
        let entity = cx.entity();
        let picker = term::picker(
            "tests-term",
            label,
            items,
            self.menu_open,
            cx.listener(|this, _: &ClickEvent, _, cx| {
                this.menu_open = !this.menu_open;
                cx.notify();
            }),
            move |index, _, cx: &mut App| {
                if let Some(id) = ids.get(index).cloned() {
                    entity.update(cx, |this, cx| {
                        this.term = Some(id);
                        this.menu_open = false;
                        cx.notify();
                    });
                }
            },
        );
        let caption = match body {
            Body::Table(view) => Some(model::points_caption(view.with_points, lang)),
            _ => None,
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(12.0))
            .child(picker)
            .children(caption.map(|text| {
                div()
                    .text_size(px(13.0))
                    .text_color(t.on_surface_variant)
                    .child(text)
            }))
            .into_any_element()
    }

    fn table(&self, view: TestsView, lang: Lang, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        if view.rows.is_empty() {
            return EmptyState::new(icons::TESTS, cx.t(&EMPTY_TITLE))
                .body(cx.t(&EMPTY_BODY))
                .into_any_element();
        }
        let mut rows: Vec<AnyElement> = Vec::new();
        for row in &view.rows {
            let open = self.open.contains(&row.id);
            rows.push(self.test_row(row, open, lang, t, cx));
            if open {
                rows.extend(row.children.iter().map(|c| child_row(c, lang, t, cx)));
            }
        }
        div()
            .id("tests-main")
            .flex_1()
            .min_h_0()
            .px(px(24.0))
            .pb(px(24.0))
            .flex()
            .flex_col()
            .child(header(t, cx))
            .children(rows)
            .when(view.skipped > 0, |d| d.child(note(cx.t(&SKIPPED), t)))
            .child(note(cx.t(&FOOTNOTE), t))
            .into_any_element()
    }

    fn test_row(
        &self,
        row: &TestRow,
        open: bool,
        lang: Lang,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let id = row.id.clone();
        let expandable = row.expandable();
        div()
            .id(SharedString::from(format!("test-{}", row.id)))
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(px(56.0))
            .px(px(12.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(t.outline_variant)
            .when(open, |d| d.bg(t.secondary_container))
            .when(expandable, |d| {
                d.cursor_pointer()
                    .when(!open, |d| {
                        d.hover(|s| s.bg(t.hover(t.surface, t.on_surface)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if !this.open.remove(&id) {
                            this.open.insert(id.clone());
                        }
                        cx.notify();
                    }))
            })
            .child(
                div().w(px(W_TAG)).flex_none().flex().children(
                    row.tag
                        .as_ref()
                        .map(|tag| KindTag::new(tag.code.clone(), tag.kind)),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(14.5)).child(row.course_name.clone()))
                    .when(!row.subtitle.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(t.on_surface_variant)
                                .child(row.subtitle.clone()),
                        )
                    }),
            )
            .child(last_cell(row.last_changed.clone(), &t))
            .child(result_cells(&row.outcome, lang, &t, cx))
            .child(
                div()
                    .w(px(W_CHEVRON))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .text_color(t.on_surface_variant)
                    .when(expandable, |d| {
                        d.child(assets::icon(
                            if open {
                                "expand_less"
                            } else {
                                icons::CHEVRON_RIGHT
                            },
                            20.0,
                            false,
                        ))
                    }),
            )
            .into_any_element()
    }
}

fn header(t: &Tokens, cx: &App) -> AnyElement {
    let label = |text: &Text| mono_label(cx.t(text), t.on_surface_variant);
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(32.0))
        .px(px(12.0))
        .border_b_1()
        .border_color(t.outline_variant)
        .child(div().w(px(W_TAG)).flex_none().child(label(&COL_TYPE)))
        .child(div().flex_1().child(label(&COL_TEST)))
        .child(div().w(px(W_LAST)).flex_none().child(label(&COL_LAST)))
        .child(div().w(px(W_BAR)).flex_none().child(label(&COL_RESULT)))
        .child(
            div()
                .w(px(W_POINTS))
                .flex_none()
                .flex()
                .justify_end()
                .child(label(&COL_POINTS)),
        )
        .child(div().w(px(W_CHEVRON)).flex_none())
        .into_any_element()
}

fn last_cell(text: Option<String>, t: &Tokens) -> AnyElement {
    div()
        .w(px(W_LAST))
        .flex_none()
        .text_size(px(13.5))
        .text_color(t.on_surface_variant)
        .child(text.unwrap_or_default())
        .into_any_element()
}

/// The bar and points columns of one row. A message (hidden, groups only) spans both.
fn result_cells(outcome: &Outcome, _lang: Lang, t: &Tokens, cx: &App) -> AnyElement {
    let both = |icon: &'static str, text: SharedString| {
        div()
            .w(px(W_BAR + W_POINTS + 12.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .text_size(px(13.0))
            .text_color(t.on_surface_variant)
            .child(assets::icon(icon, 18.0, false))
            .child(text)
            .into_any_element()
    };
    match outcome {
        Outcome::Hidden => both("visibility_off", cx.t(&HIDDEN)),
        Outcome::GroupsOnly => both("group", cx.t(&GROUPS_ONLY)),
        Outcome::Points { text, fill } => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(W_BAR))
                    .child(ProgressBar::new(*fill).width(W_BAR - 10.0)),
            )
            .child(points_text(text.clone(), px(W_POINTS)))
            .into_any_element(),
        Outcome::Grade { symbol, tile } => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(div().w(px(W_BAR)))
            .child(
                div()
                    .w(px(W_POINTS))
                    .flex()
                    .justify_end()
                    .child(GradeTile::new(symbol.clone(), *tile)),
            )
            .into_any_element(),
        Outcome::Ungraded => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.0))
            .text_size(px(13.0))
            .text_color(t.on_surface_variant)
            .child(div().w(px(W_BAR)).child(cx.t(&UNGRADED)))
            .child(div().w(px(W_POINTS)))
            .into_any_element(),
    }
}

fn points_text(text: String, width: gpui::Pixels) -> impl IntoElement {
    div()
        .w(width)
        .flex()
        .justify_end()
        .font_features(tnum())
        .text_size(px(15.0))
        .child(text)
}

fn child_row(row: &ChildRow, lang: Lang, t: &Tokens, cx: &App) -> AnyElement {
    let indent = 12.0 + 24.0 * row.depth as f32;
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .min_h(px(40.0))
        .px(px(12.0))
        .py(px(6.0))
        .border_b_1()
        .border_color(t.outline_variant)
        .child(div().w(px(W_TAG)).flex_none())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(px(indent - 12.0))
                .flex()
                .flex_col()
                .child(div().text_size(px(14.0)).child(row.label.clone()))
                .children(row.detail.clone().map(|detail| {
                    div()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(t.on_surface_variant)
                        .child(detail)
                })),
        )
        .child(last_cell(row.last_changed.clone(), t))
        .child(result_cells(&row.outcome, lang, t, cx))
        .child(div().w(px(W_CHEVRON)).flex_none())
        .into_any_element()
}

fn note(text: SharedString, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .pt(px(14.0))
        .text_size(px(12.5))
        .text_color(t.on_surface_variant)
        .child(assets::icon(icons::INFO, 16.0, false))
        .child(text)
        .into_any_element()
}
