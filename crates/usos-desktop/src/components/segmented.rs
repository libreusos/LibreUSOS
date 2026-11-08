use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::theme::{FONT_UI, ThemeExt};

type SelectHandler = Box<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// A single-choice segmented control (desktop: 26 px segments in a 3 px tray).
#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    options: Vec<SharedString>,
    selected: usize,
    on_select: Option<SelectHandler>,
}

impl Segmented {
    pub fn new(id: impl Into<ElementId>, options: Vec<SharedString>, selected: usize) -> Self {
        Self {
            id: id.into(),
            options,
            selected,
            on_select: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Segmented {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let handler = self.on_select.map(std::rc::Rc::new);
        let id = self.id;
        div()
            .id(id.clone())
            .flex()
            .flex_none()
            .gap(px(2.0))
            .p(px(3.0))
            .rounded(px(10.0))
            .bg(t.surface_container_high)
            .children(self.options.into_iter().enumerate().map(|(index, label)| {
                let on = index == self.selected;
                let handler = handler.clone();
                div()
                    .id((id.clone(), index.to_string()))
                    .flex()
                    .items_center()
                    .h(px(26.0))
                    .px(px(12.0))
                    .rounded(px(7.0))
                    .font_family(FONT_UI)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .tab_stop(true)
                    .focus_visible(|s| s.border_2().border_color(t.primary))
                    .text_color(if on {
                        t.on_surface
                    } else {
                        t.on_surface_variant
                    })
                    .when(on, |d| {
                        if t.dark {
                            d.bg(t.surface_container_highest)
                        } else {
                            d.bg(t.group_bg())
                                .border_1()
                                .border_color(t.outline_variant)
                        }
                    })
                    .when(!on, |d| d.hover(|s| s.bg(t.on_surface.opacity(0.08))))
                    .child(label)
                    .when_some(handler, |d, handler| {
                        d.on_click(move |_, window, cx| handler(index, window, cx))
                    })
            }))
    }
}
