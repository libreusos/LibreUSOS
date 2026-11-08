use gpui::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px, relative};

use crate::theme::ThemeExt;

/// A determinate bar: primary fill on a quiet track. By default a 6 px bar that fills its row
/// (the now card); with `width` a fixed 4 px points bar (SYSTEM.md section 10, "Points").
#[derive(IntoElement)]
pub struct ProgressBar {
    fraction: f32,
    width: Option<f32>,
}

impl ProgressBar {
    /// `fraction` is clamped to 0..=1; NaN counts as 0.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            },
            width: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let (bar, height) = match self.width {
            Some(w) => (div().flex_none().w(px(w)), 4.0),
            None => (div().flex_1().min_w(px(40.0)), 6.0),
        };
        bar.h(px(height))
            .rounded_full()
            .overflow_hidden()
            .bg(t.surface_container_highest)
            .child(
                div()
                    .h_full()
                    .w(relative(self.fraction))
                    .rounded_full()
                    .bg(t.primary),
            )
    }
}
