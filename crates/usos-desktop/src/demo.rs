//! Debug builds only: `USOS_DEMO_*` variables that steer the app for screenshots, because a
//! headless compositor has no input devices. Release builds ignore all of them.
//!
//! - `USOS_DEMO_GO`: `grades`, `tests`, `settings`, `appearance`, `notifications`, `shortcuts` or `palette`; where
//!   to go once the app is up.
//! - `USOS_DEMO_TERM`, `USOS_DEMO_COURSE`: the term and the course Oceny opens on.
//! - `USOS_DEMO_TERM`, `USOS_DEMO_TEST`: the term and the test (course id) Sprawdziany opens.

use std::time::Duration;

use gpui::{Action, AnyWindowHandle, App, Window};

use crate::nav::{GoGrades, GoTests, Nav, OpenPalette, Route};
use crate::screens::settings::{self, Section};

/// The value of a `USOS_DEMO_*` variable; always `None` in release builds.
pub fn var(name: &str) -> Option<String> {
    if cfg!(debug_assertions) {
        std::env::var(name).ok().filter(|v| !v.is_empty())
    } else {
        None
    }
}

fn dispatch(window: AnyWindowHandle, action: Box<dyn Action>, cx: &mut App) {
    let _ = window.update(cx, |_, window, cx| window.dispatch_action(action, cx));
}

/// Go where `USOS_DEMO_GO` says, a moment after start so the sign-in has finished.
pub fn go(window: &Window, cx: &mut App) {
    let Some(target) = var("USOS_DEMO_GO") else {
        return;
    };
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_secs(3)).await;
        cx.update(|cx| match target.as_str() {
            "settings" => Nav::go(Route::Settings, cx),
            "appearance" => settings::open_section(Section::Appearance, cx),
            "notifications" => settings::open_section(Section::Notifications, cx),
            "shortcuts" => settings::open_section(Section::Shortcuts, cx),
            "grades" => dispatch(handle, GoGrades.boxed_clone(), cx),
            "tests" => dispatch(handle, GoTests.boxed_clone(), cx),
            "palette" => dispatch(handle, OpenPalette.boxed_clone(), cx),
            _ => {}
        });
    })
    .detach();
}
