//! LibreUSOS desktop: a GPUI app on top of `usos-core` and `usos-store`.
//! See README.md for the module map and how to add a screen.

// The foundation offers more API (icons, components, strings, formatters) than the shell
// uses; screens added later consume it.
// `usos_core::Error` is large; boxing it is a change for the core, not for this crate.
#![allow(clippy::result_large_err)]
// No console window behind the app on Windows (debug builds keep it for eprintln!).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod chrome;
mod components;
mod demo;
mod i18n;
mod nav;
mod page;
mod palette;
mod screens;
mod settings;
mod shell;
mod shortcuts;
mod state;
mod theme;
mod view_util;

use gpui::{
    AppContext as _, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowDecorations,
    WindowOptions, px, size,
};
use gpui_component::Root;

use crate::assets::AppAssets;
use crate::i18n::strings;
use crate::nav::{
    Back, Forward, GoGrades, GoTests, GoToday, GoWeek, Nav, OpenPalette, OpenSettings, Quit,
    Refresh,
};
use crate::settings::Prefs;
use crate::shell::Shell;
use crate::state::AppState;

/// The id compositors group and match the window by (the .desktop file name without suffix).
const APP_ID: &str = "pl.libreusos.LibreUSOS";

fn main() {
    gpui_platform::application()
        .with_assets(AppAssets)
        .run(|cx| {
            gpui_component::init(cx);
            Prefs::init(cx);
            i18n::init(cx);
            let system_dark = matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            );
            theme::init(cx, system_dark);
            assets::install_fonts(cx);
            theme::watch_system(cx);
            Nav::init(cx);
            AppState::init(cx);

            cx.bind_keys([
                KeyBinding::new("ctrl-1", GoToday, None),
                KeyBinding::new("ctrl-2", GoWeek, None),
                KeyBinding::new("ctrl-3", GoGrades, None),
                KeyBinding::new("ctrl-4", GoTests, None),
                KeyBinding::new("ctrl-k", OpenPalette, None),
                KeyBinding::new("ctrl-,", OpenSettings, None),
                KeyBinding::new("ctrl-r", Refresh, None),
                KeyBinding::new("f5", Refresh, None),
                KeyBinding::new("alt-left", Back, None),
                KeyBinding::new("alt-right", Forward, None),
                KeyBinding::new("ctrl-q", Quit, None),
            ]);
            shortcuts::bind(cx);
            // Escape closes the topmost overlay or goes back, but never inside a text field
            // that wants it, so it is bound to the shell's context only.
            cx.bind_keys([KeyBinding::new("escape", Back, Some("Shell"))]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1180.0), px(760.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(strings::APP_NAME.into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(640.0), px(480.0))),
                window_decorations: Some(WindowDecorations::Server),
                app_id: Some(APP_ID.to_owned()),
                ..Default::default()
            };
            cx.open_window(options, |window, cx| {
                let shell = cx.new(|cx| Shell::new(window, cx));
                shell.update(cx, |shell, cx| shell.focus(window, cx));
                demo::go(window, cx);
                cx.new(|cx| Root::new(shell, window, cx))
            })
            .expect("the window must open");
            cx.activate(true);
        });
}
