use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use crate::assets;
use crate::theme::{FONT_UI, ThemeExt};

/// A centred icon, title and one line of explanation for a table without rows, a term still
/// loading or data that could not be had (SYSTEM.md section 12, empty states).
#[derive(IntoElement)]
pub struct EmptyState {
    icon: &'static str,
    title: SharedString,
    body: Option<SharedString>,
}

impl EmptyState {
    pub fn new(icon: &'static str, title: impl Into<SharedString>) -> Self {
        Self {
            icon,
            title: title.into(),
            body: None,
        }
    }

    pub fn body(mut self, body: impl Into<SharedString>) -> Self {
        self.body = Some(body.into());
        self
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .py(px(64.0))
            .px(px(24.0))
            .font_family(FONT_UI)
            .text_center()
            .child(assets::icon(self.icon, 36.0, false).text_color(t.on_surface_variant))
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(t.on_surface)
                    .child(self.title),
            )
            .children(self.body.map(|body| {
                div()
                    .max_w(px(420.0))
                    .text_size(px(13.5))
                    .text_color(t.on_surface_variant)
                    .child(body)
            }))
    }
}
