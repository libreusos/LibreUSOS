//! The class detail: a 340 px side pane next to the grid, or a full-width page when narrow.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClipboardItem, IntoElement, ParentElement, SharedString, Styled, Window, div,
    px,
};

use super::model::{self, ClassItem};
use super::timeline::{CLASH, clash_badge, group_text};
use crate::assets::{self, icons};
use crate::components::{Btn, BtnStyle, KindTag};
use crate::i18n::{I18nExt, Text, strings};
use crate::nav::{Nav, Route};
use crate::theme::{FONT_UI, ThemeExt};

pub const WIDTH: f32 = 340.0;

const CLASHES_WITH: Text = Text::new("Nakłada się na", "Overlaps with");
const ROOM: Text = Text::new("Sala", "Room");
const LECTURER: Text = Text::new("Prowadzący", "Lecturer");
const GROUP: Text = Text::new("Grupa", "Group");
const FREQUENCY: Text = Text::new("Częstotliwość", "Frequency");
const GRADES: Text = Text::new("Oceny", "Grades");
const WEB: Text = Text::new("USOSweb", "USOSweb");
const COPY: Text = Text::new("Kopiuj", "Copy");
const CLOSE: Text = Text::new("Zamknij", "Close");
const BACK: Text = Text::new("Plan", "Timetable");

pub type CloseHandler = Rc<dyn Fn(&mut Window, &mut App)>;

fn row(
    icon: &'static str,
    label: SharedString,
    value: SharedString,
    muted: bool,
    cx: &App,
) -> impl IntoElement {
    let t = *cx.tokens();
    div()
        .flex()
        .items_start()
        .gap(px(12.0))
        .text_color(t.on_surface_variant)
        .child(div().mt(px(1.0)).child(assets::icon(icon, 20.0, false)))
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_size(px(11.5)).child(label))
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(if muted {
                            t.on_surface_variant
                        } else {
                            t.on_surface
                        })
                        .child(value),
                ),
        )
}

/// The text copied by the "Kopiuj" button: title, time and place, nothing else.
pub fn copy_text(item: &ClassItem, cx: &App) -> String {
    let mut lines = vec![
        item.title.clone(),
        format!(
            "{}, {}",
            model::date_long(item.start.date(), cx.lang()),
            model::range_text(item.start, item.end)
        ),
    ];
    lines.extend(item.place());
    lines.join("\n")
}

/// `items` is the list `index` points into (for the clash partners). `narrow` turns the close
/// button into a "back to the timetable" one.
pub fn detail(
    items: &[ClassItem],
    index: usize,
    narrow: bool,
    on_close: CloseHandler,
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    let lang = cx.lang();
    let item = &items[index];

    let close = if narrow {
        Btn::new("detail-close", BtnStyle::Ghost)
            .icon(icons::ARROW_BACK)
            .label(cx.t(&BACK))
    } else {
        Btn::new("detail-close", BtnStyle::Ghost)
            .icon(icons::CLOSE)
            .tooltip(cx.t(&CLOSE))
    }
    .on_click(move |_, window, cx| on_close(window, cx));

    let mut tags = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .child(KindTag::new(item.code.clone(), item.kind));
    if let Some(name) = &item.type_name {
        tags = tags.child(
            div()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(name.clone()),
        );
    }
    if item.is_clash() {
        tags = tags.child(clash_badge(cx));
    }

    let when = format!(
        "{} · {} · {}",
        model::date_long(item.start.date(), lang),
        model::range_text(item.start, item.end),
        model::duration_text(item.minutes())
    );

    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .font_family(FONT_UI)
        .text_color(t.on_surface)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(tags)
                .child(close),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(20.0))
                        .line_height(px(26.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(item.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(13.5))
                        .text_color(t.on_surface_variant)
                        .child(when),
                ),
        );

    if item.is_clash() {
        let partners = item.clashes.iter().filter_map(|&other| {
            let o = items.get(other)?;
            let (from, to) = model::overlap_span((item.start, item.end), (o.start, o.end))?;
            Some(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(o.title.clone()),
                    )
                    .child(format!(
                        "{} {} · {}",
                        o.code,
                        model::range_text(o.start, o.end),
                        model::duration_text((to - from).num_minutes())
                    )),
            )
        });
        column = column.child(
            div()
                .flex()
                .items_start()
                .gap(px(10.0))
                .rounded(px(12.0))
                .p(px(12.0))
                .bg(t.error_container)
                .text_color(t.on_error_container)
                .text_size(px(13.0))
                .child(
                    div()
                        .mt(px(1.0))
                        .child(assets::icon(icons::WARNING, 20.0, true)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!(
                            "{}: {}",
                            cx.t(&CLASH),
                            cx.t(&CLASHES_WITH).to_lowercase()
                        )))
                        .children(partners),
                ),
        );
    }

    let mut rows = div().flex().flex_col().gap(px(14.0));
    if let Some(place) = item.place() {
        rows = rows.child(row("location_on", cx.t(&ROOM), place.into(), false, cx));
    }
    rows = rows.child(row(
        "person",
        cx.t(&LECTURER),
        cx.t(&strings::SOON),
        true,
        cx,
    ));
    if let Some(group) = group_text(item, cx) {
        rows = rows.child(row("meeting_room", cx.t(&GROUP), group.into(), false, cx));
    }
    if let Some(text) = item
        .frequency
        .as_ref()
        .and_then(|f| model::frequency_text(f, lang))
    {
        rows = rows.child(row(
            "calendar_month",
            cx.t(&FREQUENCY),
            text.into(),
            false,
            cx,
        ));
    }
    column = column.child(rows);

    let copy = copy_text(item, cx);
    let mut actions = div().flex().flex_wrap().gap(px(8.0)).child(
        Btn::new("detail-grades", BtnStyle::Outlined)
            .icon(icons::GRADES)
            .label(cx.t(&GRADES))
            .on_click(|_, _, cx| Nav::go(Route::Grades, cx)),
    );
    if let Some(url) = item
        .url
        .clone()
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
    {
        actions = actions.child(
            Btn::new("detail-web", BtnStyle::Outlined)
                .icon(icons::OPEN_EXTERNAL)
                .label(cx.t(&WEB))
                .on_click(move |_, _, cx| cx.open_url(&url)),
        );
    }
    actions = actions.child(
        Btn::new("detail-copy", BtnStyle::Outlined)
            .icon(icons::COPY)
            .label(cx.t(&COPY))
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
            }),
    );
    column.child(actions).into_any_element()
}
