use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

/// Time slots per day in the motif (08-10, 10-12, ... 18-20).
pub const MOTIF_SLOTS: usize = 6;
/// Days in the motif (Monday to Friday).
pub const MOTIF_DAYS: usize = 5;

/// The "week at a glance" picture of the empty Today screen (`.motif` in the mockups): Monday to
/// Friday, six slots per day. A filled square is a class, an outlined one is free time, and
/// today's column is outlined in the primary colour with its label.
#[derive(IntoElement)]
pub struct WeekMotif {
    busy: [[bool; MOTIF_SLOTS]; MOTIF_DAYS],
    today: Option<usize>,
    day_labels: [SharedString; MOTIF_DAYS],
    legend: [SharedString; 3],
}

impl WeekMotif {
    /// `busy[day][slot]`; `today` is a column 0..5 when today is a weekday.
    pub fn new(
        busy: [[bool; MOTIF_SLOTS]; MOTIF_DAYS],
        today: Option<usize>,
        day_labels: [SharedString; MOTIF_DAYS],
        legend: [SharedString; 3],
    ) -> Self {
        Self {
            busy,
            today,
            day_labels,
            legend,
        }
    }
}

impl RenderOnce for WeekMotif {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let square = |filled: bool, today: bool| {
            let base = div().size(px(16.0)).flex_none().rounded(px(3.0));
            match (filled, today) {
                (true, true) => base
                    .bg(t.surface_container_highest)
                    .border_2()
                    .border_color(t.primary),
                (true, false) => base.bg(t.surface_container_highest),
                (false, true) => base.border_2().border_color(t.primary),
                (false, false) => base.border_1().border_color(t.outline_variant),
            }
        };
        let columns = self.busy.iter().enumerate().map(|(day, slots)| {
            let is_today = self.today == Some(day);
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .children(slots.iter().map(|&filled| square(filled, is_today)))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_size(px(10.0))
                        .text_color(if is_today {
                            t.primary
                        } else {
                            t.on_surface_variant
                        })
                        .child(self.day_labels[day].to_uppercase()),
                )
        });
        let legend_marks = [
            div()
                .size(px(10.0))
                .rounded(px(2.0))
                .bg(t.surface_container_highest),
            div()
                .size(px(10.0))
                .rounded(px(2.0))
                .border_1()
                .border_color(t.outline_variant),
            div()
                .size(px(10.0))
                .rounded(px(2.0))
                .border_2()
                .border_color(t.primary),
        ];
        div()
            .flex()
            .items_start()
            .gap(px(24.0))
            .child(div().flex().gap(px(8.0)).children(columns))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .pt(px(2.0))
                    .font_family(FONT_UI)
                    .text_size(px(12.5))
                    .text_color(t.on_surface_variant)
                    .children(
                        self.legend
                            .into_iter()
                            .zip(legend_marks)
                            .map(|(text, mark)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(mark)
                                    .child(text)
                            }),
                    ),
            )
    }
}
