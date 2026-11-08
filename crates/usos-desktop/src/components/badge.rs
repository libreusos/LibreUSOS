use gpui::prelude::FluentBuilder as _;
use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use crate::theme::{FONT_MONO, FONT_UI, Kind, ThemeExt};

/// Status badges that follow the kind tag in a class's meta line (`.badge` in the mockups).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeKind {
    /// TERAZ: outlined in the primary colour.
    Now,
}

#[derive(IntoElement)]
pub struct Badge {
    text: SharedString,
    kind: BadgeKind,
}

impl Badge {
    pub fn new(text: impl Into<SharedString>, kind: BadgeKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let base = div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(16.0))
            .px(px(6.0))
            .rounded(px(4.0))
            .font_family(FONT_UI)
            .text_size(px(10.5))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .whitespace_nowrap();
        let base = match self.kind {
            BadgeKind::Now => base
                .text_color(t.primary)
                .border_1()
                .border_color(t.primary),
        };
        base.child(self.text.to_uppercase())
    }
}

/// The class kind code (WYK, LAB, ĆW...): mono, uppercase, filled from the kind table.
#[derive(IntoElement)]
pub struct KindTag {
    code: SharedString,
    kind: Kind,
    past: bool,
}

impl KindTag {
    pub fn new(code: impl Into<SharedString>, kind: Kind) -> Self {
        Self {
            code: code.into(),
            kind,
            past: false,
        }
    }

    /// Past classes use one treatment everywhere: transparent, hairline, muted text.
    pub fn past(mut self, past: bool) -> Self {
        self.past = past;
        self
    }
}

impl RenderOnce for KindTag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let colors = t.kind(self.kind);
        let tag = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .min_w(px(30.0))
            .h(px(16.0))
            .px(px(5.0))
            .rounded(px(4.0))
            .font_family(FONT_MONO)
            .text_size(px(10.5))
            .font_weight(gpui::FontWeight::MEDIUM)
            .whitespace_nowrap();
        if self.past {
            tag.text_color(t.on_surface_variant)
                .border_1()
                .border_color(t.outline_variant)
        } else {
            tag.bg(colors.fill)
                .text_color(colors.text)
                .when_some(colors.border, |d, border| d.border_1().border_color(border))
        }
        .child(self.code.to_uppercase())
    }
}
