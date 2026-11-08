//! Small text helpers shared by the tables and the settings pages.

use std::sync::Arc;

use gpui::{FontFeatures, Hsla, IntoElement, ParentElement, SharedString, Styled, div, px};

use crate::theme::FONT_MONO;

/// Tabular figures (`tnum`), so grades, points and times line up in columns (SYSTEM.md section 2).
pub fn tnum() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// A column header or section eyebrow: mono 10.5 px, uppercase, muted.
pub fn mono_label(text: impl Into<SharedString>, color: Hsla) -> impl IntoElement {
    let text: SharedString = text.into();
    div()
        .font_family(FONT_MONO)
        .text_size(px(10.5))
        .line_height(px(14.0))
        .text_color(color)
        .whitespace_nowrap()
        .child(text.to_uppercase())
}
