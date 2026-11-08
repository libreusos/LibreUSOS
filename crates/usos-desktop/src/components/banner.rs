use gpui::prelude::FluentBuilder as _;
use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use super::{Btn, BtnStyle, ClickHandler};
use crate::assets;
use crate::theme::{FONT_UI, ThemeExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    /// Quiet information (offline, syncing).
    Info,
    /// Something the user has to act on (expired session, sync error).
    Error,
}

/// A full-width strip under the title bar with one optional action (`.banner` in the mockups).
#[derive(IntoElement)]
pub struct Banner {
    kind: BannerKind,
    icon: &'static str,
    text: SharedString,
    detail: Option<SharedString>,
    action: Option<(SharedString, ClickHandler)>,
}

impl Banner {
    pub fn new(kind: BannerKind, icon: &'static str, text: impl Into<SharedString>) -> Self {
        Self {
            kind,
            icon,
            text: text.into(),
            detail: None,
            action: None,
        }
    }

    /// A second, quieter line (an error's technical text).
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some((label.into(), Box::new(handler)));
        self
    }
}

impl RenderOnce for Banner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let (bg, fg) = match self.kind {
            BannerKind::Info => (t.surface_container_high, t.on_surface),
            BannerKind::Error => (t.error_container, t.on_error_container),
        };
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(10.0))
            .py(px(8.0))
            .pl(px(14.0))
            .pr(px(8.0))
            .bg(bg)
            .text_color(fg)
            .font_family(FONT_UI)
            .text_size(px(13.0))
            .child(assets::icon(self.icon, 20.0, false))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.text)
                    .when_some(self.detail, |d, detail| {
                        d.child(div().text_size(px(11.5)).opacity(0.8).child(detail))
                    }),
            )
            .when_some(self.action, |d, (label, handler)| {
                d.child(
                    Btn::new("banner-action", BtnStyle::Ghost)
                        .label(label)
                        .on_click(move |e, w, c| handler(e, w, c)),
                )
            })
    }
}
