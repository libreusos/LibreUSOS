use gpui::prelude::FluentBuilder as _;
use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use crate::theme::{FONT_MONO, ThemeExt};

/// Key caps ("Ctrl", "K", "Esc"): mono with a hairline border (`.kbd` in the mockups).
#[derive(IntoElement)]
pub struct Kbd {
    keys: Vec<SharedString>,
    muted: bool,
}

impl Kbd {
    /// A single cap or several caps in a row ("Ctrl", "K").
    pub fn new(keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            keys: keys.into_iter().map(Into::into).collect(),
            muted: false,
        }
    }

    /// Dim the caps (for a key that is not served yet).
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let muted = self.muted;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.0))
            .children(self.keys.into_iter().map(move |key| {
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(px(22.0))
                    .h(px(20.0))
                    .px(px(6.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(t.outline_variant)
                    .bg(t.group_bg())
                    .font_family(FONT_MONO)
                    .text_size(px(11.0))
                    .text_color(t.on_surface)
                    .when(muted, |d| d.text_color(t.on_surface_variant))
                    .whitespace_nowrap()
                    .child(key)
            }))
    }
}
