//! The frame every screen draws: a 52 px header (eyebrow, title, actions, window close), the
//! sync banner, then the body. Screens build one `Page` in `render` and put their content in it.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::assets::icons;
use crate::chrome::{self, Side};
use crate::components::{Banner, BannerKind};
use crate::i18n::{I18nExt, strings};
use crate::state::{AppState, SyncStatus};
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

#[derive(IntoElement)]
pub struct Page {
    eyebrow: Option<SharedString>,
    title: SharedString,
    actions: Vec<AnyElement>,
    body: Vec<AnyElement>,
    banners: bool,
}

impl Page {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            eyebrow: None,
            title: title.into(),
            actions: Vec::new(),
            body: Vec::new(),
            banners: true,
        }
    }

    /// The small mono line above the title ("WTOREK · TYDZ. 11 NIEPARZYSTY").
    pub fn eyebrow(mut self, eyebrow: impl Into<SharedString>) -> Self {
        self.eyebrow = Some(eyebrow.into());
        self
    }

    /// A button at the right end of the header.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.body.push(child.into_any_element());
        self
    }

    /// Hide the sync banner (for a screen that explains the state itself).
    pub fn without_banners(mut self) -> Self {
        self.banners = false;
        self
    }
}

impl RenderOnce for Page {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *cx.tokens();
        let close_left = chrome::close_side(cx) == Side::Left;
        // Only the title block is a drag handle: a double click on an action button must not also
        // maximise the window.
        let title_block = chrome::drag_area(
            div()
                .id("page-title")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .justify_center()
                .when_some(self.eyebrow, |d, eyebrow| {
                    d.child(
                        div()
                            .font_family(FONT_MONO)
                            .text_size(px(11.0))
                            .line_height(px(14.0))
                            .text_color(t.on_surface_variant)
                            .child(eyebrow.to_uppercase()),
                    )
                })
                .child(
                    div()
                        .font_family(FONT_UI)
                        .text_size(px(22.0))
                        .line_height(px(28.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(t.on_surface)
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .child(self.title),
                ),
            window,
        );
        let header = div()
            .id("page-header")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.0))
            .h(px(52.0))
            .pl(px(24.0))
            .pr(px(10.0))
            .border_b_1()
            .border_color(t.outline_variant)
            .when(close_left, |d| d.children(chrome::close_button(window, cx)))
            .child(title_block)
            .children(self.actions)
            .when(!close_left, |d| {
                d.children(chrome::close_button(window, cx))
            });
        let banner = if self.banners { sync_banner(cx) } else { None };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.surface)
            .text_color(t.on_surface)
            .font_family(FONT_UI)
            .child(header)
            .when_some(banner, |d, banner| {
                d.child(div().flex_none().px(px(24.0)).pt(px(14.0)).child(banner))
            })
            .child(
                div()
                    .id("page-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(self.body),
            )
    }
}

/// The strip under the header that says why the data may be stale (SYSTEM.md section 11).
/// Offline is not a banner: the sidebar footer says it.
fn sync_banner(cx: &App) -> Option<AnyElement> {
    let state = AppState::global(cx);
    let sync = state.read(cx).sync.clone();
    match sync {
        SyncStatus::Expired => Some(
            Banner::new(
                BannerKind::Error,
                icons::SESSION,
                cx.t(&strings::SESSION_EXPIRED_BANNER),
            )
            .action(cx.t(&strings::SIGN_IN_AGAIN), move |_, _, cx| {
                state.update(cx, |state, cx| {
                    if let Some(url) = state.base_url().map(str::to_owned) {
                        state.begin_login(url, cx);
                    }
                });
            })
            .into_any_element(),
        ),
        SyncStatus::Error { details } => Some(
            Banner::new(
                BannerKind::Error,
                icons::SYNC_PROBLEM,
                cx.t(&strings::SYNC_ERROR_BANNER),
            )
            .detail(details)
            .action(cx.t(&strings::TRY_AGAIN), move |_, _, cx| {
                state.update(cx, |state, cx| state.sync(cx));
            })
            .into_any_element(),
        ),
        _ => None,
    }
}
