//! The pieces of the Today screen: the day, the empty day, and the now card.

use chrono::{Datelike, Days, NaiveDate, NaiveDateTime};

use super::*;
use crate::assets;

fn heading(text: SharedString, trailing: Option<String>, cx: &App) -> impl IntoElement {
    let t = *cx.tokens();
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .font_family(FONT_UI)
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.on_surface)
                .child(text),
        )
        .children(trailing.map(|text| {
            div()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(text)
        }))
}

/// The day's timeline under "Plan dnia", then the next class day as a compact list.
pub(super) fn running_day(
    view: &PlanView,
    day: &[usize],
    now: NaiveDateTime,
    narrow: bool,
    cx: &App,
) -> AnyElement {
    let running = if narrow {
        &RUNNING_ABOVE
    } else {
        &RUNNING_BESIDE
    };
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(heading(
            cx.t(&DAY_PLAN),
            Some(cx.count(&model::CLASSES, day.len() as u64)),
            cx,
        ))
        .child(timeline::day_timeline(
            timeline::Timeline {
                id: "today-day",
                items: &view.items,
                day,
                now,
                running_line: Some(cx.t(running)),
                selected: None,
                on_select: None,
            },
            cx,
        ));
    if let Some(next_day) = model::next_class_day(&view.items, now.date()) {
        column = column.child(next_day_list(
            &view.items,
            next_day,
            now.date(),
            usize::MAX,
            cx,
        ));
    }
    column.into_any_element()
}

/// The classes of `date` as compact rows, at most `limit` plus a summary row for the rest.
fn next_day_list(
    items: &[ClassItem],
    date: NaiveDate,
    today: NaiveDate,
    limit: usize,
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    let indices = model::day_indices(items, date);
    let shown = indices.len().min(limit);
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .mt(px(12.0))
        .child(heading(
            model::relative_day(date, today, cx.lang()).into(),
            None,
            cx,
        ))
        .children(indices[..shown].iter().map(|&i| compact_row(items, i, cx)));
    if indices.len() > shown {
        let rest = &indices[shown..];
        let mut text = format!(
            "{}{}",
            cx.t(&MORE_PREFIX),
            cx.count(&model::CLASSES, rest.len() as u64)
        );
        if let Some(&clash) = rest.iter().find(|&&i| items[i].is_clash()) {
            text = format!(
                "{text}{}{}",
                cx.t(&CLASH_AT),
                model::time_text(items[clash].start)
            );
        }
        column = column.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(13.0))
                .text_color(t.on_surface_variant)
                .child(text)
                .child(assets::icon(icons::CHEVRON_RIGHT, 20.0, false)),
        );
    }
    column.into_any_element()
}

fn compact_row(items: &[ClassItem], index: usize, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    let item = &items[index];
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(8.0))
        .py(px(6.0))
        .font_family(FONT_UI)
        .text_size(px(13.5))
        .text_color(t.on_surface)
        .child(
            div()
                .w(px(92.0))
                .flex_none()
                .font_family(FONT_MONO)
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(model::range_text(item.start, item.end)),
        )
        .child(KindTag::new(item.code.clone(), item.kind))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(item.title.clone()),
        )
        .children(item.is_clash().then(|| clash_badge(cx)))
        .children(item.place().map(|place| {
            div()
                .w(px(96.0))
                .flex_none()
                .flex()
                .justify_end()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(place)
        }))
        .into_any_element()
}

/// No classes today: the week at a glance, what comes next, and a way to the whole week.
pub(super) fn empty_day(view: &PlanView, today: NaiveDate, cx: &App) -> AnyElement {
    let t = *cx.tokens();
    let lang = cx.lang();
    let monday = model::week_monday(today);
    let labels: [SharedString; MOTIF_DAYS] =
        std::array::from_fn(|i| model::weekday_short(monday + Days::new(i as u64), lang).into());
    let column_of_today = today.weekday().num_days_from_monday() as usize;
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .font_family(FONT_UI)
        .child(WeekMotif::new(
            model::motif_busy(monday, &view.items),
            (column_of_today < MOTIF_DAYS).then_some(column_of_today),
            labels,
            [cx.t(&MOTIF_CLASSES), cx.t(&MOTIF_FREE), cx.t(&MOTIF_TODAY)],
        ))
        .child(
            div()
                .mt(px(16.0))
                .text_size(px(24.0))
                .line_height(px(32.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.on_surface)
                .child(cx.t(&NO_CLASSES)),
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(t.on_surface_variant)
                .child(cx.t(&NO_CLASSES_HINT)),
        );
    if let Some(next_day) = model::next_class_day(&view.items, today) {
        column = column
            .child(div().mt(px(12.0)).child(heading(cx.t(&UPCOMING), None, cx)))
            .child(next_day_list(&view.items, next_day, today, 2, cx));
    }
    column
        .child(
            div().mt(px(8.0)).child(
                Btn::new("today-whole-week", BtnStyle::Outlined)
                    .icon(icons::WEEK)
                    .label(cx.t(&WHOLE_WEEK))
                    .on_click(|_, _, cx| Nav::go(Route::Week, cx)),
            ),
        )
        .into_any_element()
}

/// The class running now (or the next one today), and the one after it.
pub(super) fn now_card(
    items: &[ClassItem],
    focus: NowNext,
    now: NaiveDateTime,
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    let Some(kind) = focus.focus else {
        return div().into_any_element();
    };
    let item = &items[kind.index()];
    let (label, status): (SharedString, AnyElement) = if matches!(kind, Focus::Now(_)) {
        let minutes = model::minutes_left(item.end, now);
        (
            cx.t(&NOW),
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(ProgressBar::new(model::progress(item.start, item.end, now)))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.5))
                        .text_color(t.on_surface_variant)
                        .child(format!(
                            "{}{}{}",
                            cx.t(&LEFT_PREFIX),
                            model::duration_text(minutes),
                            cx.t(&LEFT_SUFFIX)
                        )),
                )
                .into_any_element(),
        )
    } else {
        let minutes = (item.start - now).num_minutes().max(1);
        (
            cx.t(&UP_NEXT),
            div()
                .text_size(px(13.0))
                .text_color(t.primary)
                .child(format!(
                    "{}{}",
                    cx.t(&STARTS_IN),
                    model::duration_text(minutes)
                ))
                .into_any_element(),
        )
    };

    let inner = div()
        .relative()
        .overflow_hidden()
        .rounded(px(16.0))
        .border_1()
        .border_color(t.outline_variant)
        .bg(t.group_bg())
        .py(px(14.0))
        .pl(px(20.0))
        .pr(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(3.0))
                .bg(t.primary),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().size(px(8.0)).rounded(px(2.0)).bg(t.primary))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_size(px(11.0))
                        .text_color(t.primary)
                        .child(label.to_uppercase()),
                )
                .child(KindTag::new(item.code.clone(), item.kind))
                .child(div().flex_1())
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_size(px(12.0))
                        .text_color(t.on_surface_variant)
                        .child(model::range_text(item.start, item.end)),
                ),
        )
        .child(
            div()
                .text_size(px(18.0))
                .line_height(px(24.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.on_surface)
                .child(item.title.clone()),
        )
        .children(place_and_group(item, cx).map(|line| {
            div()
                .text_size(px(13.0))
                .text_color(t.on_surface_variant)
                .child(line)
        }))
        .child(status);

    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .rounded(px(20.0))
        .p(px(4.0))
        .bg(t.surface_container)
        .font_family(FONT_UI)
        .child(inner);
    if let Some(next) = focus.next.map(|i| &items[i]) {
        let place = [next.room.clone(), Some(model::time_text(next.start))]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .text_size(px(13.0))
                .text_color(t.on_surface_variant)
                .child(assets::icon(icons::ARROW_FORWARD, 16.0, false))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(4.0))
                        .child(format!("{}:", cx.t(&NEXT)))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(t.on_surface)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(next.title.clone()),
                        ),
                )
                .child(div().flex_none().child(place)),
        );
    }
    card.into_any_element()
}
