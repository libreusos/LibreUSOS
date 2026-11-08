use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::tooltip::Tooltip;

use crate::assets;
use crate::i18n::{I18nExt, strings};
use crate::theme::{FONT_UI, ThemeExt};

/// A section whose data the core does not provide yet: the heading, a disabled card that says
/// what will appear here, and the "wkrótce" status. It never shows made-up rows.
#[derive(IntoElement)]
pub struct SoonCard {
    id: ElementId,
    title: SharedString,
    hint: SharedString,
    icon: &'static str,
}

impl SoonCard {
    pub fn new(
        id: impl Into<ElementId>,
        icon: &'static str,
        title: impl Into<SharedString>,
        hint: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            hint: hint.into(),
            icon,
        }
    }
}

impl RenderOnce for SoonCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let soon = cx.t(&strings::SOON);
        let tooltip = cx.t(&strings::SOON_TOOLTIP);
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .child(
                        div()
                            .font_family(FONT_UI)
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(t.on_surface_variant)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(t.on_surface_variant)
                            .child(soon),
                    ),
            )
            .child(
                div()
                    .id(self.id)
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(t.outline_variant)
                    .px(px(14.0))
                    .py(px(12.0))
                    .text_color(t.on_surface_variant)
                    .opacity(0.75)
                    .child(assets::icon(self.icon, 20.0, false))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.0))
                            .line_height(px(18.0))
                            .child(self.hint),
                    )
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)),
            )
    }
}
