use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use super::typography::tnum;
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

/// How a grade tile is coloured (SYSTEM.md section 10, "Grade tile").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    /// The default tile.
    Normal,
    /// The best grade of the term.
    Best,
    /// A failing grade.
    Fail,
    /// No grade yet: a dashed outline with a dash.
    Pending,
}

/// A fixed-width grade tile. The text is what USOS gave ("4,5", "ZAL"), never reformatted.
#[derive(IntoElement)]
pub struct GradeTile {
    text: SharedString,
    kind: TileKind,
    width: f32,
}

impl GradeTile {
    pub fn new(text: impl Into<SharedString>, kind: TileKind) -> Self {
        Self {
            text: text.into(),
            kind,
            width: 48.0,
        }
    }

    /// A tile without a grade yet.
    pub fn pending() -> Self {
        Self::new("\u{2014}", TileKind::Pending)
    }
}

/// Whether `text` is a numeric grade (set in the UI font) rather than a word such as "ZAL".
fn is_numeric(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.' || c == '!')
}

impl RenderOnce for GradeTile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let (bg, fg) = match self.kind {
            TileKind::Normal => (t.surface_container_high, t.on_surface),
            TileKind::Best => (t.primary_container, t.on_primary_container),
            TileKind::Fail => (t.error_container, t.on_error_container),
            TileKind::Pending => (gpui::transparent_black(), t.on_surface_variant),
        };
        let numeric = is_numeric(&self.text);
        let tile = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(self.width))
            .h(px(30.0))
            .rounded(px(10.0))
            .bg(bg)
            .text_color(fg)
            .whitespace_nowrap();
        let tile = if self.kind == TileKind::Pending {
            tile.border_1()
                .border_dashed()
                .border_color(t.outline_variant)
        } else {
            tile
        };
        if numeric {
            tile.font_family(FONT_UI)
                .font_features(tnum())
                .text_size(px(17.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(self.text)
        } else {
            tile.font_family(FONT_MONO)
                .text_size(px(12.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(self.text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_numeric;

    #[test]
    fn numeric_grades_use_the_ui_font() {
        for text in ["2,0", "3,5", "5!", "5.0"] {
            assert!(is_numeric(text), "{text}");
        }
        assert!(!is_numeric("ZAL"));
        assert!(!is_numeric("NZAL"));
        assert!(!is_numeric(""));
    }
}
