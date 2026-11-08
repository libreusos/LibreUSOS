//! The day timeline shared by Today and the Day mode of the timetable: time column, rail with
//! nodes, the class text, "Okienko" gaps and the clash marking (`.tl` in the mockups).

use std::rc::Rc;

use chrono::NaiveDateTime;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::model::{self, ClassItem, ClassKey};
use crate::assets::{self, icons};
use crate::components::{Badge, BadgeKind, KindTag};
use crate::i18n::{I18nExt, NBSP, Text};
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

pub const CLASH: Text = Text::new("Kolizja", "Clash");
pub const NOW_BADGE: Text = Text::new("Teraz", "Now");
const FREE_PERIOD: Text = Text::new("Okienko", "Free period");
const OVERLAP_BY: Text = Text::new("nakłada się o", "overlaps by");
const GROUP: Text = Text::new("gr.", "grp.");

pub type SelectHandler = Rc<dyn Fn(&ClassKey, &mut Window, &mut App)>;

/// The KOLIZJA badge: the error container, a warning glyph and the word, so the colour is never
/// the only signal.
pub fn clash_badge(cx: &App) -> impl IntoElement {
    let t = *cx.tokens();
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(3.0))
        .h(px(16.0))
        .pl(px(4.0))
        .pr(px(6.0))
        .rounded(px(4.0))
        .bg(t.error_container)
        .text_color(t.on_error_container)
        .font_family(FONT_UI)
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .child(assets::icon(icons::WARNING, 12.0, false))
        .child(cx.t(&CLASH).to_uppercase())
}

/// "gr. 2".
pub fn group_text(item: &ClassItem, cx: &App) -> Option<String> {
    item.group.map(|g| format!("{}{NBSP}{g}", cx.t(&GROUP)))
}

/// "s. 216, AEI · gr. 2".
pub fn place_and_group(item: &ClassItem, cx: &App) -> Option<String> {
    let parts: Vec<String> = [item.place(), group_text(item, cx)]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// "Kolizja 12:45–13:30 · nakłada się o 45 min" for the first clash of an item.
pub fn clash_summary(items: &[ClassItem], index: usize, cx: &App) -> Option<String> {
    let (_, (from, to)) = model::first_clash(items, index)?;
    let minutes = (to - from).num_minutes();
    Some(format!(
        "{} {} · {} {}",
        cx.t(&CLASH),
        model::range_text(from, to),
        cx.t(&OVERLAP_BY),
        model::duration_text(minutes)
    ))
}

/// What the timeline needs besides the data.
pub struct Timeline<'a> {
    pub id: &'static str,
    pub items: &'a [ClassItem],
    /// Indices into `items` of the day's classes, in order.
    pub day: &'a [usize],
    pub now: NaiveDateTime,
    /// When set, the running class collapses to this line (Today shows it in the card above).
    pub running_line: Option<SharedString>,
    pub selected: Option<&'a ClassKey>,
    pub on_select: Option<SelectHandler>,
}

fn joined(items: &[ClassItem], a: usize, b: usize) -> bool {
    items.get(a).is_some_and(|item| item.clashes.contains(&b))
}

pub fn day_timeline(spec: Timeline, cx: &App) -> AnyElement {
    let day_items: Vec<&ClassItem> = spec.day.iter().map(|&i| &spec.items[i]).collect();
    let gaps = model::gaps(&day_items);
    let count = spec.day.len();
    let mut rows: Vec<AnyElement> = Vec::new();
    for (position, &index) in spec.day.iter().enumerate() {
        if let Some(gap) = gaps.iter().find(|g| g.before == position) {
            rows.push(gap_row(gap.minutes, cx));
        }
        let joined_prev = position > 0 && joined(spec.items, index, spec.day[position - 1]);
        let joined_next = position + 1 < count && joined(spec.items, index, spec.day[position + 1]);
        rows.push(slot(
            &spec,
            index,
            position,
            Joins {
                first: position == 0,
                last: position + 1 == count,
                prev: joined_prev,
                next: joined_next,
            },
            cx,
        ));
    }
    div().flex().flex_col().children(rows).into_any_element()
}

fn gap_row(minutes: i64, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    let label = format!("{} · {}", cx.t(&FREE_PERIOD), model::duration_text(minutes));
    div()
        .flex()
        .h(px(28.0))
        .px(px(8.0))
        .child(div().w(px(52.0)).flex_none())
        .child(
            div().w(px(16.0)).flex_none().relative().child(
                div()
                    .absolute()
                    .left(px(7.0))
                    .top_0()
                    .bottom_0()
                    .w(px(1.0))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .children((0..6).map(|_| {
                        div()
                            .flex_none()
                            .w(px(1.0))
                            .h(px(3.0))
                            .bg(t.outline_variant)
                    })),
            ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(px(8.0))
                .flex()
                .items_center()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(label),
        )
        .into_any_element()
}

struct Joins {
    first: bool,
    last: bool,
    prev: bool,
    next: bool,
}

fn slot(spec: &Timeline, index: usize, position: usize, joins: Joins, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    let item = &spec.items[index];
    let past = item.is_past(spec.now);
    let current = item.is_current(spec.now);
    let collapsed = current && spec.running_line.is_some();
    let text = if past {
        t.on_surface_variant
    } else {
        t.on_surface
    };
    let selected = spec.selected == Some(&item.key);
    let clash = item.is_clash();

    // Time column.
    let time = div()
        .w(px(52.0))
        .flex_none()
        .flex()
        .flex_col()
        .items_end()
        .py(px(8.0))
        .font_family(FONT_MONO)
        .text_size(px(13.0))
        .line_height(px(18.0))
        .child(div().text_color(text).child(model::time_text(item.start)))
        .child(
            div()
                .text_color(t.on_surface_variant)
                .child(model::time_text(item.end)),
        );

    // Rail: the line through the day and this class's node.
    let line_top = if joins.first { 14.0 } else { 0.0 };
    let line = div()
        .absolute()
        .left(px(7.0))
        .w(px(1.0))
        .top(px(line_top))
        .bg(t.outline_variant);
    let line = if joins.last {
        line.h(px(14.0 - line_top))
    } else {
        line.bottom_0()
    };
    let node = if current {
        div()
            .absolute()
            .left(px(2.5))
            .top(px(11.0))
            .size(px(11.0))
            .rounded(px(3.0))
            .bg(t.primary)
    } else if past {
        div()
            .absolute()
            .left(px(3.5))
            .top(px(12.0))
            .size(px(9.0))
            .rounded_full()
            .bg(t.outline_variant)
    } else {
        div()
            .absolute()
            .left(px(3.5))
            .top(px(12.0))
            .size(px(9.0))
            .rounded_full()
            .border_1()
            .border_color(t.outline)
            .bg(t.surface)
    };
    let rail = div()
        .w(px(16.0))
        .flex_none()
        .relative()
        .child(line)
        .child(node);

    // Content.
    let mut content = div()
        .flex_1()
        .min_w_0()
        .relative()
        .py(px(8.0))
        .pl(px(8.0))
        .flex()
        .flex_col()
        .gap(px(2.0));
    if clash {
        let bar = if past { t.outline } else { t.error };
        content = content.pl(px(16.0)).child(
            div()
                .absolute()
                .left(px(6.0))
                .w(px(3.0))
                .rounded(px(2.0))
                .bg(bar)
                .top(px(if joins.prev { 0.0 } else { 8.0 }))
                .bottom(px(if joins.next { 0.0 } else { 8.0 })),
        );
    }
    let title = div()
        .text_size(px(14.5))
        .line_height(px(20.0))
        .text_color(text)
        .child(item.title.clone());
    if collapsed {
        content = content.child(title.truncate()).child(
            div()
                .text_size(px(12.5))
                .text_color(t.primary)
                .child(spec.running_line.clone().unwrap_or_default()),
        );
    } else {
        let mut meta = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .mt(px(1.0))
            .text_size(px(12.5))
            .text_color(t.on_surface_variant)
            .child(KindTag::new(item.code.clone(), item.kind).past(past));
        if current {
            meta = meta.child(Badge::new(cx.t(&NOW_BADGE), BadgeKind::Now));
        }
        if clash {
            meta = meta.child(clash_badge(cx));
        }
        if let Some(line) = place_and_group(item, cx) {
            meta = meta.child(div().min_w_0().truncate().child(line));
        }
        content = content.child(title).child(meta);
        if let Some(frequency) = item
            .frequency
            .as_ref()
            .filter(|f| {
                !matches!(
                    f,
                    usos_core::Frequency::EveryWeek | usos_core::Frequency::Once
                )
            })
            .and_then(|f| model::frequency_text(f, cx.lang()))
        {
            content = content.child(
                div()
                    .text_size(px(12.0))
                    .text_color(t.on_surface_variant)
                    .child(frequency),
            );
        }
        // The overlap line sits on the first class of a clashing group only.
        if clash
            && item.clashes.iter().all(|&other| other > index)
            && let Some(summary) = clash_summary(spec.items, index, cx)
        {
            content = content.child(
                div()
                    .mt(px(4.0))
                    .self_start()
                    .rounded(px(6.0))
                    .px(px(8.0))
                    .py(px(3.0))
                    .bg(t.error_container)
                    .text_color(t.on_error_container)
                    .text_size(px(12.0))
                    .child(summary),
            );
        }
    }

    let key = item.key.clone();
    let on_select = spec.on_select.clone();
    div()
        .id((spec.id, position))
        .flex()
        .px(px(8.0))
        .rounded(px(10.0))
        .font_family(FONT_UI)
        .when(selected, |d| d.bg(t.primary.opacity(0.10)))
        .when(on_select.is_some(), |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(t.on_surface.opacity(crate::theme::tokens::HOVER_LAYER)))
        })
        .when_some(on_select, move |d, handler| {
            d.on_click(move |_, window, cx| handler(&key, window, cx))
        })
        .child(time)
        .child(rail)
        .child(content)
        .into_any_element()
}
