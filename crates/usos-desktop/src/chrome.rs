//! Window chrome: server-side or client-side decorations (SYSTEM.md section 11).
//!
//! The app asks the compositor for server-side decorations. When it gets them (sway, KDE, Windows)
//! the app draws no window buttons. When it does not (GNOME), the app draws one close button, on
//! the side the desktop's button layout puts it (default: right), plus the drag and resize areas
//! a borderless window needs.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Decorations, InteractiveElement, IntoElement, MouseButton, Pixels, Point,
    ResizeEdge, Styled, Window, WindowDecorations, div, px,
};

use crate::assets::icons;
use crate::components::{Btn, BtnStyle};
use crate::i18n::{I18nExt, strings};
use crate::theme::ThemeExt;

/// Which side of the title bar the close button sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Ask for server-side decorations; the compositor's answer is read with [`is_client`].
pub fn request_server_decorations(window: &Window) {
    window.request_decorations(WindowDecorations::Server);
}

/// Whether the app has to draw its own window buttons and edges.
pub fn is_client(window: &Window) -> bool {
    matches!(window.window_decorations(), Decorations::Client { .. })
}

/// The side of the close button, from the desktop's GNOME-style button layout. Without a layout
/// (or without a close button in it) the close button goes right.
pub fn close_side(cx: &App) -> Side {
    use gpui::WindowButton::Close;
    match cx.button_layout() {
        Some(layout) if layout.left.contains(&Some(Close)) => Side::Left,
        _ => Side::Right,
    }
}

/// The close button, or nothing when the compositor draws the window frame.
pub fn close_button(window: &Window, cx: &App) -> Option<AnyElement> {
    is_client(window).then(|| {
        Btn::new("window-close", BtnStyle::Ghost)
            .icon(icons::CLOSE)
            .tooltip(cx.t(&strings::WINDOW_CLOSE))
            .on_click(|_, window, _| window.remove_window())
            .into_any_element()
    })
}

/// How far (in pixels) the pointer must travel with the button down before the window moves.
const DRAG_THRESHOLD: f32 = 4.0;

/// Turn an element into a drag handle for client-side decorations (double click maximises).
///
/// The move starts once the pointer has travelled a few pixels after a press, not on the press
/// itself: controls inside the header do not stop the press from bubbling up here, and a move
/// started on the press would take the pointer grab from them (on Wayland their click would never
/// arrive).
pub fn drag_area<E>(element: E, window: &Window) -> E
where
    E: InteractiveElement,
{
    if !is_client(window) {
        return element;
    }
    // Where the left button went down; the move starts only past a small threshold, so a click
    // with a little jitter still reaches the control under the pointer.
    let pressed: Rc<Cell<Option<Point<Pixels>>>> = Rc::new(Cell::new(None));
    element
        .on_mouse_down(MouseButton::Left, {
            let pressed = pressed.clone();
            move |event, window, _| {
                if event.click_count == 2 {
                    window.zoom_window();
                } else {
                    pressed.set(Some(event.position));
                }
            }
        })
        .on_mouse_up(MouseButton::Left, {
            let pressed = pressed.clone();
            move |_, _, _| pressed.set(None)
        })
        .on_mouse_up_out(MouseButton::Left, {
            let pressed = pressed.clone();
            move |_, _, _| pressed.set(None)
        })
        .on_mouse_move(move |event, window, _| {
            let Some(origin) = pressed.get() else { return };
            if event.pressed_button != Some(MouseButton::Left) {
                pressed.set(None);
                return;
            }
            let delta = event.position - origin;
            if delta.x.abs() > px(DRAG_THRESHOLD) || delta.y.abs() > px(DRAG_THRESHOLD) {
                pressed.set(None);
                window.start_window_move();
            }
        })
}

/// Invisible resize strips along the window edges, only for client-side decorations.
pub fn resize_handles(window: &Window) -> Vec<AnyElement> {
    if !is_client(window) {
        return Vec::new();
    }
    const EDGE: f32 = 5.0;
    const CORNER: f32 = 10.0;
    let strip = |id: &'static str, edge: ResizeEdge| {
        div()
            .id(id)
            .absolute()
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                window.start_window_resize(edge);
            })
    };
    vec![
        strip("rs-top", ResizeEdge::Top)
            .top_0()
            .left(px(CORNER))
            .right(px(CORNER))
            .h(px(EDGE))
            .cursor_ns_resize()
            .into_any_element(),
        strip("rs-bottom", ResizeEdge::Bottom)
            .bottom_0()
            .left(px(CORNER))
            .right(px(CORNER))
            .h(px(EDGE))
            .cursor_ns_resize()
            .into_any_element(),
        strip("rs-left", ResizeEdge::Left)
            .left_0()
            .top(px(CORNER))
            .bottom(px(CORNER))
            .w(px(EDGE))
            .cursor_ew_resize()
            .into_any_element(),
        strip("rs-right", ResizeEdge::Right)
            .right_0()
            .top(px(CORNER))
            .bottom(px(CORNER))
            .w(px(EDGE))
            .cursor_ew_resize()
            .into_any_element(),
        strip("rs-tl", ResizeEdge::TopLeft)
            .top_0()
            .left_0()
            .size(px(CORNER))
            .cursor_nwse_resize()
            .into_any_element(),
        strip("rs-br", ResizeEdge::BottomRight)
            .bottom_0()
            .right_0()
            .size(px(CORNER))
            .cursor_nwse_resize()
            .into_any_element(),
        strip("rs-tr", ResizeEdge::TopRight)
            .top_0()
            .right_0()
            .size(px(CORNER))
            .cursor_nesw_resize()
            .into_any_element(),
        strip("rs-bl", ResizeEdge::BottomLeft)
            .bottom_0()
            .left_0()
            .size(px(CORNER))
            .cursor_nesw_resize()
            .into_any_element(),
    ]
}

/// A 1 px frame for borderless windows, so the window edge stays visible.
pub fn framed(root: gpui::Div, window: &Window, cx: &App) -> gpui::Div {
    let color = cx.tokens().outline_variant;
    root.when(is_client(window), |d| d.border_1().border_color(color))
}
