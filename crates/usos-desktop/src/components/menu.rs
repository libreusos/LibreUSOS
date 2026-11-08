//! A menu chip (the term picker) and its drop-down list.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::ClickHandler;
use crate::assets::{self, icons};
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

/// An outlined 32 px chip with a drop-down arrow.
#[derive(IntoElement)]
pub struct MenuChip {
    id: ElementId,
    label: SharedString,
    icon: Option<&'static str>,
    open: bool,
    on_click: Option<ClickHandler>,
}

impl MenuChip {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            open: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
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

impl RenderOnce for MenuChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .h(px(32.0))
            .pl(px(12.0))
            .pr(px(8.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(t.outline_variant)
            .bg(if self.open {
                t.secondary_container
            } else {
                t.group_bg()
            })
            .text_color(t.on_surface)
            .font_family(FONT_UI)
            .text_size(px(13.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .whitespace_nowrap()
            .cursor_pointer()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(t.primary).border_2())
            .hover(move |s| s.bg(t.hover(t.group_bg(), t.on_surface)))
            .children(self.icon.map(|name| assets::icon(name, 18.0, false)))
            .child(self.label)
            .child(assets::icon(icons::EXPAND_MORE, 18.0, false).text_color(t.on_surface_variant))
            .when_some(self.on_click, |d, handler| {
                d.on_click(move |event, window, cx| handler(event, window, cx))
            })
    }
}

/// One row of a [`MenuList`].
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: SharedString,
    /// A quiet mono tag after the label ("trwa", "zakończony").
    pub detail: Option<SharedString>,
    pub selected: bool,
}

type PickHandler = Box<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// The list under a [`MenuChip`]. The parent places it (absolute, deferred) and closes it.
#[derive(IntoElement)]
pub struct MenuList {
    id: ElementId,
    items: Vec<MenuItem>,
    on_pick: Option<PickHandler>,
}

impl MenuList {
    pub fn new(id: impl Into<ElementId>, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.into(),
            items,
            on_pick: None,
        }
    }

    pub fn on_pick(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for MenuList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let handler = self.on_pick.map(std::rc::Rc::new);
        let id = self.id;
        div()
            .id(id.clone())
            .flex()
            .flex_col()
            .min_w(px(260.0))
            .max_h(px(320.0))
            .overflow_y_scroll()
            .p(px(4.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(t.outline_variant)
            .bg(t.surface_container_high)
            .shadow_lg()
            .occlude()
            .font_family(FONT_UI)
            .children(self.items.into_iter().enumerate().map(|(index, item)| {
                let handler = handler.clone();
                div()
                    .id((id.clone(), index.to_string()))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .h(px(36.0))
                    .flex_none()
                    .px(px(10.0))
                    .rounded(px(7.0))
                    .text_size(px(13.0))
                    .text_color(if item.selected {
                        t.on_secondary_container
                    } else {
                        t.on_surface
                    })
                    .cursor_pointer()
                    .when(item.selected, |d| d.bg(t.secondary_container))
                    .when(!item.selected, |d| {
                        d.hover(|s| s.bg(t.surface_container_highest))
                    })
                    .child(
                        div().w(px(18.0)).flex_none().children(
                            item.selected
                                .then(|| assets::icon(icons::CHECK, 18.0, false)),
                        ),
                    )
                    .child(div().flex_1().whitespace_nowrap().child(item.label))
                    .children(item.detail.map(|detail| {
                        div()
                            .font_family(FONT_MONO)
                            .text_size(px(11.0))
                            .text_color(t.on_surface_variant)
                            .child(detail)
                    }))
                    .when_some(handler, |d, handler| {
                        d.on_click(move |_, window, cx| handler(index, window, cx))
                    })
            }))
    }
}
