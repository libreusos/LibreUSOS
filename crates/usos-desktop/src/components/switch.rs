use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window,
    div, px,
};

use crate::theme::ThemeExt;

/// An M3 switch (52 by 32).
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    on: bool,
    disabled: bool,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>, on: bool) -> Self {
        Self {
            id: id.into(),
            on,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let (track, thumb, border) = if self.on {
            (t.primary, t.on_primary, t.primary)
        } else {
            (t.surface_container_highest, t.outline, t.outline)
        };
        let size = if self.on { 24.0 } else { 16.0 };
        let mut switch = div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .w(px(52.0))
            .h(px(32.0))
            .px(px(if self.on { 2.0 } else { 6.0 }))
            .rounded(px(16.0))
            .border_2()
            .border_color(border)
            .bg(track)
            .when(self.on, |d| d.justify_end())
            .child(div().size(px(size)).rounded_full().bg(thumb));
        if self.disabled {
            switch = switch.opacity(0.38);
        } else {
            switch = switch
                .cursor_pointer()
                .tab_stop(true)
                .focus_visible(|s| s.border_color(t.on_surface));
        }
        switch
    }
}
