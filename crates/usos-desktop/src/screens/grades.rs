//! Oceny: the grades of one term as a table with a summary card and, for the selected course,
//! the detail pane. Grade symbols are shown exactly as USOS gives them; the view-model
//! (`model`) does the sums and `term` the term picker Sprawdziany shares.

pub(crate) mod model;
#[cfg(test)]
mod seed;
pub(crate) mod term;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use self::model::{CourseRow, Detail, DetailGroup, Entry, GradesView, Summary};
use self::term::Slot;
use crate::assets::icons;
use crate::components::{
    Btn, BtnStyle, EmptyState, GradeTile, KindTag, TileKind, mono_label, tnum,
};
use crate::i18n::{Counted, I18nExt, Lang, Text, nbsp, number, strings};
use crate::page::Page;
use crate::state::{AppState, SyncStatus};
use crate::theme::{FONT_UI, ThemeExt, Tokens};
use crate::view_util;

const COL_COURSE: Text = Text::new("Przedmiot", "Course");
const COL_FORMS: Text = Text::new("Formy", "Forms");
const COL_ECTS: Text = Text::new("ECTS", "ECTS");
const COL_GRADE: Text = Text::new("Ocena", "Grade");
const STAT_AVERAGE: Text = Text::new("Średnia", "Average");
const STAT_WEIGHTED: Text = Text::new("Ważona ECTS", "ECTS-weighted");
const STAT_ECTS: Text = Text::new("ECTS", "ECTS");
const COMPUTATION_NOTE: Text = Text::new(
    "Z ocen końcowych; ZAL nie wlicza się. Ważona: suma ocena × ECTS przez sumę ECTS.",
    "From final grades; pass marks such as ZAL are left out. Weighted: the sum of grade × ECTS over the sum of ECTS.",
);
const COURSES: Counted = Counted::new(
    ["przedmiot", "przedmioty", "przedmiotów"],
    ["course", "courses"],
);
const EMPTY_TITLE: Text = Text::new("Brak ocen w tym semestrze", "No grades in this term");
const EMPTY_BODY: Text = Text::new(
    "USOS nie podał jeszcze żadnych przedmiotów ani ocen dla tego semestru.",
    "USOS has not listed any courses or grades for this term yet.",
);
const LOADING_TITLE: Text = Text::new("Wczytywanie ocen…", "Loading grades…");
const UNAVAILABLE_TITLE: Text = Text::new(
    "Nie udało się pobrać ocen",
    "The grades could not be loaded",
);
const UNAVAILABLE_BODY: Text = Text::new(
    "Spróbuj ponownie, gdy wrócisz do sieci. Zapisanych danych dla tego semestru nie ma.",
    "Try again when you are back online. There is no saved data for this term.",
);
const NO_TERM_TITLE: Text = Text::new("Nie znaleziono semestru", "No term found");
const NO_TERM_BODY: Text = Text::new(
    "Odśwież dane, aby pobrać listę semestrów z USOS.",
    "Refresh to fetch the list of terms from USOS.",
);
const CLOSE_DETAIL: Text = Text::new("Zamknij szczegóły", "Close details");

const DETAIL_WIDTH: f32 = 440.0;
const FORMS_WIDTH: f32 = 112.0;
const ECTS_WIDTH: f32 = 40.0;
const GRADE_WIDTH: f32 = 140.0;

pub struct GradesScreen {
    /// The term the user picked; `None` follows the current term.
    term: Option<String>,
    menu_open: bool,
    /// The course whose detail pane is open.
    selected: Option<String>,
}

impl GradesScreen {
    pub fn new() -> Self {
        Self {
            term: crate::demo::var("USOS_DEMO_TERM"),
            menu_open: false,
            selected: crate::demo::var("USOS_DEMO_COURSE"),
        }
    }
}

/// What the body shows for the chosen term (built once per frame, never stored).
#[allow(clippy::large_enum_variant)]
enum Body {
    Table {
        view: GradesView,
        detail: Option<Detail>,
    },
    Loading,
    Unavailable,
    NoTerm,
}

impl Render for GradesScreen {
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
                    match term::grades_slot(&app.data, &app.term_cache, &shown.id, syncing) {
                        Slot::Ready(result) => {
                            let detail = self.selected.as_deref().and_then(|id| {
                                let course =
                                    result.grades.courses.iter().find(|c| c.course_id == id)?;
                                Some(model::detail(course, &shown.id, lang))
                            });
                            Body::Table {
                                view: model::build(&result.grades, lang),
                                detail,
                            }
                        }
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

        let shown_id = term_view.shown.as_ref().map(|t| t.id.clone());
        let toolbar = self.toolbar(&term_view, &body, shown_id.as_deref(), today, lang, &t, cx);
        let content = match body {
            Body::Table { view, detail } => self.table_body(view, detail, lang, &t, cx),
            Body::Loading => {
                EmptyState::new(icons::PROGRESS, cx.t(&LOADING_TITLE)).into_any_element()
            }
            Body::Unavailable => EmptyState::new(icons::OFFLINE, cx.t(&UNAVAILABLE_TITLE))
                .body(cx.t(&UNAVAILABLE_BODY))
                .into_any_element(),
            Body::NoTerm => EmptyState::new(icons::GRADES, cx.t(&NO_TERM_TITLE))
                .body(cx.t(&NO_TERM_BODY))
                .into_any_element(),
        };

        Page::new(cx.t(&strings::NAV_GRADES)).child(
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

impl GradesScreen {
    #[allow(clippy::too_many_arguments)]
    fn toolbar(
        &self,
        term_view: &term::TermView,
        body: &Body,
        shown_id: Option<&str>,
        today: chrono::NaiveDate,
        lang: Lang,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = term_view
            .shown
            .as_ref()
            .map_or_else(String::new, |term| term::chip_label(term, today, lang));
        let items = term::menu_items(&term_view.options, shown_id, today, lang);
        let ids: Vec<String> = term_view.options.iter().map(|t| t.id.clone()).collect();
        let entity = cx.entity();
        let picker = term::picker(
            "grades-term",
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
                        if this.term.as_deref() != Some(id.as_str()) {
                            this.selected = None;
                        }
                        this.term = Some(id);
                        this.menu_open = false;
                        cx.notify();
                    });
                }
            },
        );
        let caption = match body {
            Body::Table { view, .. } => Some(COURSES.format(view.rows.len() as u64, lang)),
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

    fn table_body(
        &self,
        view: GradesView,
        detail: Option<Detail>,
        lang: Lang,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let header = table_header(t, cx);
        let rows: Vec<AnyElement> = view
            .rows
            .iter()
            .map(|row| self.course_row(row, t, cx))
            .collect();
        let main = div()
            .id("grades-main")
            .flex_1()
            .min_w_0()
            .px(px(24.0))
            .pb(px(24.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(summary_card(&view.summary, lang, t, cx))
            .child(if view.rows.is_empty() {
                EmptyState::new(icons::GRADES, cx.t(&EMPTY_TITLE))
                    .body(cx.t(&EMPTY_BODY))
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .child(header)
                    .children(rows)
                    .into_any_element()
            });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(main)
            .children(detail.map(|d| self.detail_pane(d, t, cx)))
            .into_any_element()
    }

    fn course_row(&self, row: &CourseRow, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.selected.as_deref() == Some(row.course_id.as_str());
        let course_id = row.course_id.clone();
        let t = *t;
        div()
            .id(SharedString::from(format!("grade-row-{}", row.course_id)))
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_h(px(48.0))
            .px(px(12.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(t.outline_variant)
            .cursor_pointer()
            .when(selected, |d| d.bg(t.secondary_container))
            .when(!selected, |d| {
                d.hover(|s| s.bg(t.hover(t.surface, t.on_surface)))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.selected = if this.selected.as_deref() == Some(course_id.as_str()) {
                    None
                } else {
                    Some(course_id.clone())
                };
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(14.5))
                    .line_height(px(20.0))
                    .child(row.name.clone()),
            )
            .child(
                div()
                    .w(px(FORMS_WIDTH))
                    .flex_none()
                    .flex()
                    .gap(px(4.0))
                    .children(
                        row.forms
                            .iter()
                            .map(|tag| KindTag::new(tag.code.clone(), tag.kind)),
                    ),
            )
            .child(
                div()
                    .w(px(ECTS_WIDTH))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .font_features(tnum())
                    .text_size(px(14.0))
                    .child(row.ects_text.clone()),
            )
            .child(grade_cell(row, &t))
            .into_any_element()
    }

    fn detail_pane(&self, detail: Detail, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let t = *t;
        let close = Btn::new("grade-detail-close", BtnStyle::Ghost)
            .icon(icons::CLOSE)
            .tooltip(cx.t(&CLOSE_DETAIL))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.selected = None;
                cx.notify();
            }));
        div()
            .id("grade-detail")
            .flex_none()
            .w(px(DETAIL_WIDTH))
            .h_full()
            .overflow_y_scroll()
            .border_l_1()
            .border_color(t.outline_variant)
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(mono_label(detail.meta.clone(), t.on_surface_variant))
                            .child(
                                div()
                                    .text_size(px(24.0))
                                    .line_height(px(30.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(detail.name.clone()),
                            ),
                    )
                    .child(close),
            )
            .child(detail_headline(&detail, &t))
            .children(detail.note.clone().map(|note| detail_note(note, &t)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(t.outline_variant)
                    .overflow_hidden()
                    .children(
                        detail
                            .groups
                            .iter()
                            .enumerate()
                            .map(|(index, group)| detail_group(group, index > 0, &t)),
                    ),
            )
            .into_any_element()
    }
}

fn table_header(t: &Tokens, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(32.0))
        .px(px(12.0))
        .border_b_1()
        .border_color(t.outline_variant)
        .child(
            div()
                .flex_1()
                .child(mono_label(cx.t(&COL_COURSE), t.on_surface_variant)),
        )
        .child(
            div()
                .w(px(FORMS_WIDTH))
                .flex_none()
                .child(mono_label(cx.t(&COL_FORMS), t.on_surface_variant)),
        )
        .child(
            div()
                .w(px(ECTS_WIDTH))
                .flex_none()
                .flex()
                .justify_end()
                .child(mono_label(cx.t(&COL_ECTS), t.on_surface_variant)),
        )
        .child(
            div()
                .w(px(GRADE_WIDTH))
                .flex_none()
                .flex()
                .justify_end()
                .child(mono_label(cx.t(&COL_GRADE), t.on_surface_variant)),
        )
        .into_any_element()
}

/// The grade column: the earlier session struck through ("I termin 2,0"), then the tile.
fn grade_cell(row: &CourseRow, t: &Tokens) -> AnyElement {
    let cell = div()
        .w(px(GRADE_WIDTH))
        .flex_none()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(8.0));
    match &row.grade {
        Some(grade) => cell
            .children(grade.earlier.as_ref().map(|(label, symbol)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(12.0))
                    .text_color(t.on_surface_variant)
                    .child(label.clone())
                    .child(
                        div()
                            .line_through()
                            .font_features(tnum())
                            .child(symbol.clone()),
                    )
            }))
            .child(GradeTile::new(grade.symbol.clone(), grade.tile))
            .into_any_element(),
        None => cell.child(GradeTile::pending()).into_any_element(),
    }
}

fn stat(label: SharedString, value: String, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(mono_label(label, t.on_surface_variant))
        .child(
            div()
                .font_family(FONT_UI)
                .font_features(tnum())
                .text_size(px(24.0))
                .line_height(px(30.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(value),
        )
        .into_any_element()
}

fn summary_card(summary: &Summary, lang: Lang, t: &Tokens, cx: &App) -> AnyElement {
    let dash = "\u{2014}".to_owned();
    let ects = format!(
        "{} / {}",
        number(summary.ects_earned, 1, lang),
        number(summary.ects_total, 1, lang)
    );
    div()
        .flex()
        .items_center()
        .gap(px(28.0))
        .px(px(16.0))
        .py(px(14.0))
        .rounded(px(16.0))
        .border_1()
        .border_color(t.outline_variant)
        .child(stat(
            cx.t(&STAT_AVERAGE),
            summary.average.clone().unwrap_or_else(|| dash.clone()),
            t,
        ))
        .child(stat(
            cx.t(&STAT_WEIGHTED),
            summary.weighted.clone().unwrap_or(dash),
            t,
        ))
        .child(stat(cx.t(&STAT_ECTS), ects, t))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(t.on_surface_variant)
                .child(nbsp(cx.t(&COMPUTATION_NOTE).as_ref())),
        )
        .into_any_element()
}

fn detail_headline(detail: &Detail, t: &Tokens) -> AnyElement {
    let big = detail.big.as_ref().map(|(symbol, tile)| {
        let color = match tile {
            TileKind::Fail => t.error,
            _ => t.on_surface,
        };
        div()
            .font_family(FONT_UI)
            .font_features(tnum())
            .text_size(px(48.0))
            .line_height(px(52.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(color)
            .child(symbol.clone())
    });
    div()
        .flex()
        .items_center()
        .gap(px(14.0))
        .children(big)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(detail.caption.clone()),
                )
                .children(detail.sub.clone().map(|sub| {
                    div()
                        .text_size(px(12.5))
                        .text_color(t.on_surface_variant)
                        .child(sub)
                })),
        )
        .into_any_element()
}

fn detail_note(note: String, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .px(px(14.0))
        .py(px(12.0))
        .rounded(px(12.0))
        .bg(t.surface_container_high)
        .text_size(px(13.5))
        .line_height(px(19.0))
        .child(
            div()
                .flex_none()
                .pt(px(1.0))
                .text_color(t.on_surface_variant)
                .child(crate::assets::icon(icons::INFO, 18.0, false)),
        )
        .child(div().flex_1().min_w_0().child(note))
        .into_any_element()
}

fn detail_group(group: &DetailGroup, divider: bool, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(14.0))
        .when(divider, |d| d.border_t_1().border_color(t.outline_variant))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .children(
                    group
                        .tag
                        .as_ref()
                        .map(|tag| KindTag::new(tag.code.clone(), tag.kind)),
                )
                .child(
                    div()
                        .text_size(px(14.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(group.title.clone()),
                ),
        )
        .children(group.entries.iter().map(|entry| detail_entry(entry, t)))
        .into_any_element()
}

fn detail_entry(entry: &Entry, t: &Tokens) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(14.0)).child(entry.label.clone()))
                .when(!entry.meta.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(t.on_surface_variant)
                            .child(entry.meta.clone()),
                    )
                }),
        )
        .child(GradeTile::new(entry.symbol.clone(), entry.tile))
        .into_any_element()
}
