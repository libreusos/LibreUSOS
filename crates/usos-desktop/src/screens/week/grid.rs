//! The week grid: hour gutter, one column per day, class blocks placed by time, the now line.

use chrono::{NaiveDate, NaiveDateTime};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, relative,
};

use super::model::{self, ClassItem, ClassKey};
use super::timeline::SelectHandler;
use crate::assets::{self, icons};
use crate::i18n::I18nExt;
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

/// Pixels per hour.
pub const HOUR_HEIGHT: f32 = 68.0;
const GUTTER: f32 = 56.0;
const HEADER: f32 = 44.0;

pub struct GridSpec<'a> {
    pub items: &'a [ClassItem],
    pub days: &'a [NaiveDate],
    pub today: NaiveDate,
    pub now: NaiveDateTime,
    pub selected: Option<&'a ClassKey>,
    pub on_select: SelectHandler,
}

fn px_of(minutes: i64) -> f32 {
    minutes as f32 * HOUR_HEIGHT / 60.0
}

pub fn week_grid(spec: GridSpec, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    let lang = cx.lang();
    let visible: Vec<&ClassItem> = spec
        .items
        .iter()
        .filter(|item| spec.days.contains(&item.start.date()))
        .collect();
    let (first_hour, last_hour) = model::hour_range(&visible);
    let height = (last_hour - first_hour) as f32 * HOUR_HEIGHT;

    let headers = spec.days.iter().map(|&day| {
        let is_today = day == spec.today;
        let clashes = spec
            .items
            .iter()
            .any(|item| item.start.date() == day && item.is_clash());
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .border_l_1()
            .border_color(t.outline_variant)
            .text_size(px(13.0))
            .text_color(if is_today { t.primary } else { t.on_surface })
            .font_weight(if is_today {
                gpui::FontWeight::SEMIBOLD
            } else {
                gpui::FontWeight::MEDIUM
            })
            .child(format!(
                "{} {}",
                model::weekday_short(day, lang),
                model::day_month_short(day, lang)
            ))
            .when(clashes, |d| {
                d.child(assets::icon(icons::WARNING, 14.0, true).text_color(t.error))
            })
    });
    let header = div()
        .flex()
        .flex_none()
        .h(px(HEADER))
        .border_b_1()
        .border_color(t.outline_variant)
        .child(div().w(px(GUTTER)).flex_none())
        .children(headers);

    let gutter = div()
        .w(px(GUTTER))
        .flex_none()
        .relative()
        .font_family(FONT_MONO)
        .text_size(px(11.0))
        .text_color(t.on_surface_variant)
        .children((first_hour..last_hour).map(|hour| {
            div()
                .absolute()
                .right(px(8.0))
                .top(px((hour - first_hour) as f32 * HOUR_HEIGHT + 2.0))
                .child(format!("{hour:02}:00"))
        }));

    let columns = spec.days.iter().enumerate().map(|(column, &day)| {
        let day_items: Vec<usize> = (0..spec.items.len())
            .filter(|&i| spec.items[i].start.date() == day)
            .collect();
        let spans: Vec<_> = day_items
            .iter()
            .map(|&i| (spec.items[i].start, spec.items[i].end))
            .collect();
        let lanes = model::lanes(&spans);
        let blocks = day_items
            .iter()
            .zip(lanes)
            .enumerate()
            .map(|(n, (&i, lane))| {
                let item = &spec.items[i];
                block(&spec, item, day, first_hour, lane, (column, n), cx)
            });
        let lines = (0..(last_hour - first_hour)).map(|h| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(h as f32 * HOUR_HEIGHT))
                .h(px(1.0))
                .bg(t.outline_variant.opacity(0.6))
        });
        let now_line = (day == spec.today).then(|| {
            let minutes = model::grid_minutes(day, spec.now, first_hour);
            (minutes >= 0 && minutes <= i64::from(last_hour - first_hour) * 60).then(|| {
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(px_of(minutes) - 1.0))
                    .h(px(2.0))
                    .bg(t.primary)
                    .child(
                        div()
                            .absolute()
                            .left(px(-4.0))
                            .top(px(-3.0))
                            .size(px(8.0))
                            .rounded_full()
                            .bg(t.primary),
                    )
            })
        });
        div()
            .flex_1()
            .min_w_0()
            .relative()
            .border_l_1()
            .border_color(t.outline_variant)
            .when(day == spec.today, |d| d.bg(t.primary.opacity(0.04)))
            .children(lines)
            .children(blocks)
            .children(now_line.flatten())
    });

    let body = div()
        .flex()
        .h(px(height))
        .flex_none()
        .child(gutter)
        .children(columns);

    // The now chip sits in the gutter, over the hour labels.
    let chip = spec.days.contains(&spec.today).then(|| {
        let minutes = model::grid_minutes(spec.today, spec.now, first_hour);
        (minutes >= 0 && minutes <= i64::from(last_hour - first_hour) * 60).then(|| {
            div()
                .absolute()
                .left(px(4.0))
                .top(px(HEADER + px_of(minutes) - 8.0))
                .px(px(5.0))
                .h(px(16.0))
                .flex()
                .items_center()
                .rounded(px(4.0))
                .bg(t.primary)
                .text_color(t.on_primary)
                .font_family(FONT_MONO)
                .text_size(px(11.0))
                .child(model::time_text(spec.now))
        })
    });

    div()
        .relative()
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .border_1()
        .border_color(t.outline_variant)
        .overflow_hidden()
        .bg(t.surface)
        .font_family(FONT_UI)
        .child(header)
        .child(body)
        .children(chip.flatten())
        .into_any_element()
}

fn block(
    spec: &GridSpec,
    item: &ClassItem,
    day: NaiveDate,
    first_hour: u32,
    lane: model::Lane,
    id: (usize, usize),
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    let top = px_of(model::grid_minutes(day, item.start, first_hour));
    let bottom = px_of(model::grid_minutes(day, item.end, first_hour));
    let height = (bottom - top).max(20.0);
    let past = item.is_past(spec.now);
    let colors = if past {
        t.past_block(item.kind)
    } else {
        t.kind(item.kind)
    };
    let selected = spec.selected == Some(&item.key);
    let key = item.key.clone();
    let handler = spec.on_select.clone();

    let mut inner = div()
        .id(("block", id.0 * 100 + id.1))
        .size_full()
        .overflow_hidden()
        .rounded(px(8.0))
        .px(px(if lane.count > 1 { 5.0 } else { 6.0 }))
        .py(px(6.0))
        .flex()
        .flex_col()
        .gap(px(1.0))
        .cursor_pointer()
        .on_click(move |_, window, cx| handler(&key, window, cx));
    inner = inner
        .bg(colors.fill)
        .text_color(colors.text)
        .when_some(colors.border, |d, c| d.border_1().border_color(c));
    if selected {
        inner = inner.border_2().border_color(t.on_surface);
    }
    let code_row = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .font_family(FONT_MONO)
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(item.code.to_uppercase())
        .when(item.is_clash(), |d| {
            d.child(assets::icon(icons::WARNING, 11.0, true))
        });
    let inner = inner.child(code_row).child(
        div()
            .text_size(px(if lane.count > 1 { 11.5 } else { 12.0 }))
            .line_height(px(15.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .line_clamp(3)
            .child(item.title.clone()),
    );
    let inner = match (&item.room, height >= 64.0) {
        (Some(room), true) => inner.child(div().text_size(px(11.5)).child(room.clone())),
        _ => inner,
    };

    div()
        .absolute()
        .top(px(top))
        .h(px(height))
        .left(relative(lane.index as f32 / lane.count as f32))
        .w(relative(1.0 / lane.count as f32))
        .p(px(2.0))
        .child(inner)
        .into_any_element()
}
