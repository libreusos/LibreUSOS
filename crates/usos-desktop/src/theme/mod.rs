//! Theme: Material 3 tokens from a seed, mapped onto gpui-component, driven by the pipeline of
//! SYSTEM.md section 4.
//!
//! - [`tokens`]: `Tokens::from_seed`, kind colours, state layers.
//! - [`mapping`]: `Tokens` to gpui-component's `ThemeColor`.
//! - [`system`]: accent, contrast and motion from the desktop (portal, gsettings, kdeglobals,
//!   Windows), plus the GNOME button layout.
//! - [`pipeline`]: in-app choice, then system, then brand.
//!
//! Views read colours with `cx.tokens()` ([`ThemeExt`]).

pub mod mapping;
pub mod pipeline;
pub mod system;
pub mod tokens;

use gpui::{App, Global, px};
use gpui_component::{Theme, ThemeMode as ComponentMode};

pub use pipeline::{Resolved, resolve};
pub use system::SystemSettings;
pub use tokens::{Kind, Tokens};

use crate::settings::Prefs;

/// UI font families (registered by `assets::install`).
pub const FONT_UI: &str = "Geist";
pub const FONT_MONO: &str = "Geist Mono";

/// The inputs the pipeline last saw, and its result.
struct State {
    system: SystemSettings,
    system_dark: bool,
    resolved: Resolved,
    tokens: Tokens,
}

impl Global for State {}

/// Set up the globals and apply the first theme, before the first frame. The system settings are
/// read afterwards by [`watch_system`]; until then the desktop is assumed to say nothing.
pub fn init(cx: &mut App, system_dark: bool) {
    let system = SystemSettings::default();
    let resolved = resolve(Prefs::global(cx), &system, system_dark);
    let tokens = Tokens::from_seed(resolved.seed, resolved.dark, resolved.contrast);
    cx.set_global(State {
        system,
        system_dark,
        resolved,
        tokens,
    });
    apply(cx);
}

/// Push the current state into gpui-component and repaint every window.
fn apply(cx: &mut App) {
    let (tokens, dark) = {
        let s = cx.global::<State>();
        (s.tokens, s.resolved.dark)
    };
    Theme::change(
        if dark {
            ComponentMode::Dark
        } else {
            ComponentMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |theme| {
        theme.colors = mapping::theme_color(&tokens);
        theme.font_family = FONT_UI.into();
        theme.mono_font_family = FONT_MONO.into();
        theme.font_size = px(14.0);
        theme.mono_font_size = px(12.5);
        theme.radius = px(8.0);
        theme.radius_lg = px(12.0);
        theme.shadow = false;
    });
}

/// Recompute from the current prefs and system state (after either changed).
pub fn refresh(cx: &mut App) {
    let (system, system_dark) = {
        let s = cx.global::<State>();
        (s.system, s.system_dark)
    };
    let resolved = resolve(Prefs::global(cx), &system, system_dark);
    let tokens = Tokens::from_seed(resolved.seed, resolved.dark, resolved.contrast);
    let changed = {
        let s = cx.global::<State>();
        s.resolved != resolved
    };
    {
        let s = cx.global_mut::<State>();
        s.resolved = resolved;
        s.tokens = tokens;
    }
    if changed {
        apply(cx);
    }
}

/// The accent the desktop reports, if it reports one (Wygląd shows it next to "Z systemu").
pub fn system_accent(cx: &App) -> Option<material_colors::color::Rgb> {
    cx.global::<State>().system.accent
}

/// The window appearance GPUI reports changed.
pub fn set_system_dark(dark: bool, cx: &mut App) {
    cx.global_mut::<State>().system_dark = dark;
    refresh(cx);
}

/// New platform settings arrived.
pub fn set_system_settings(settings: SystemSettings, cx: &mut App) {
    cx.global_mut::<State>().system = settings;
    refresh(cx);
}

/// Read the platform settings in the background now and again whenever the portal reports a
/// change. gsettings and kdeglobals changes are picked up the next time something else triggers
/// a read (portal signal, appearance change).
pub fn watch_system(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            let settings = cx.background_executor().spawn(system::read_system()).await;
            cx.update(|cx| set_system_settings(settings, cx));
            wait_for_change(cx).await;
        }
    })
    .detach();
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
async fn wait_for_change(cx: &mut gpui::AsyncApp) {
    cx.background_executor()
        .spawn(system::portal::changed())
        .await;
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
async fn wait_for_change(_cx: &mut gpui::AsyncApp) {
    std::future::pending::<()>().await;
}

/// Theme accessors on `App` (and anything that derefs to it, such as `Context<T>`).
pub trait ThemeExt {
    /// The Material tokens of the active theme.
    fn tokens(&self) -> &Tokens;
    /// What the pipeline decided (dark, seed, accent source, contrast, motion).
    fn resolved_theme(&self) -> &Resolved;
}

impl ThemeExt for App {
    fn tokens(&self) -> &Tokens {
        &self.global::<State>().tokens
    }
    fn resolved_theme(&self) -> &Resolved {
        &self.global::<State>().resolved
    }
}
