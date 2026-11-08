use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::tooltip::Tooltip;

use super::ClickHandler;
use crate::assets;
use crate::theme::{FONT_UI, ThemeExt};

/// The desktop button styles (SYSTEM.md section 10; `dbtn` in the mockups).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtnStyle {
    /// The one primary action of a view.
    Filled,
    /// Secondary actions.
    Outlined,
    /// Quiet actions (Anuluj, header icons).
    Ghost,
    /// Destructive confirmations.
    Danger,
}

/// A 32 px button: label, icon or both.
#[derive(IntoElement)]
pub struct Btn {
    id: ElementId,
    style: BtnStyle,
    label: Option<SharedString>,
    icon: Option<&'static str>,
    tooltip: Option<SharedString>,
    disabled: bool,
    wide: bool,
    on_click: Option<ClickHandler>,
}

impl Btn {
    pub fn new(id: impl Into<ElementId>, style: BtnStyle) -> Self {
        Self {
            id: id.into(),
            style,
            label: None,
            icon: None,
            tooltip: None,
            disabled: false,
            wide: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// A Material Symbols name from `assets::icons`.
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Fill the row and grow to 40 px (the main action of a card).
    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Btn {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let (bg, fg, border) = match self.style {
            BtnStyle::Filled => (t.primary, t.on_primary, t.primary),
            BtnStyle::Outlined => (t.group_bg(), t.on_surface, t.outline_variant),
            BtnStyle::Ghost => (
                gpui::transparent_black(),
                t.on_surface_variant,
                gpui::transparent_black(),
            ),
            BtnStyle::Danger => (t.group_bg(), t.error, t.outline_variant),
        };
        let (hover_bg, press_bg) = if self.style == BtnStyle::Ghost {
            (
                t.on_surface.opacity(crate::theme::tokens::HOVER_LAYER),
                t.on_surface.opacity(crate::theme::tokens::PRESSED_LAYER),
            )
        } else {
            (t.hover(bg, fg), t.pressed(bg, fg))
        };
        let icon_only = self.label.is_none();
        let mut button: gpui::Stateful<Div> = div()
            .id(self.id)
            .h(px(if self.wide { 40.0 } else { 32.0 }))
            .flex()
            .flex_none()
            .when(self.wide, |d| d.w_full())
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .font_family(FONT_UI)
            .text_size(px(13.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when_else(icon_only, |d| d.w(px(32.0)), |d| d.px(px(12.0)))
            .focus_visible(|s| s.border_color(t.primary).border_2());
        if self.disabled {
            button = button
                .bg(t.on_surface.opacity(0.12))
                .text_color(t.on_surface.opacity(0.38))
                .border_color(gpui::transparent_black());
        } else {
            button = button
                .cursor_pointer()
                .tab_stop(true)
                .hover(move |s| s.bg(hover_bg))
                .active(move |s| s.bg(press_bg));
            if let Some(handler) = self.on_click {
                button = button.on_click(move |event, window, cx| handler(event, window, cx));
            }
        }
        if let Some(text) = self.tooltip {
            button = button.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx));
        }
        button
            .children(self.icon.map(|name| assets::icon(name, 18.0, false)))
            .children(self.label)
    }
}

use gpui::prelude::FluentBuilder as _;
