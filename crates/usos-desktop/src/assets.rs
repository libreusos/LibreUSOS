//! Bundled assets: Geist fonts, Material Symbols icons and the logo, served to GPUI through one
//! [`AssetSource`]. gpui-component's own Lucide icons (checkmarks, chevrons) come from
//! `gpui-kit-assets`, chained behind ours.

use std::borrow::Cow;

use anyhow::Result;
use gpui::{
    App, AssetSource, Hsla, IntoElement, RenderOnce, SharedString, Styled, Window, px, svg,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "fonts/*.ttf"]
#[include = "icons/*.svg"]
#[include = "logo/*.svg"]
struct Embedded;

/// Our assets first, then gpui-component's.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = Embedded::get(path) {
            return Ok(Some(file.data));
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut found: Vec<SharedString> = Embedded::iter()
            .filter(|name| name.starts_with(path))
            .map(|name| SharedString::from(name.to_string()))
            .collect();
        found.extend(gpui_kit_assets::Assets.list(path)?);
        Ok(found)
    }
}

/// Register the bundled fonts with the text system. Call once before the first window.
pub fn install_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = Embedded::iter()
        .filter(|name| name.starts_with("fonts/"))
        .filter_map(|name| Embedded::get(&name).map(|file| file.data))
        .collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("could not register the bundled fonts: {e}");
    }
}

/// Material Symbols names (SYSTEM.md section 8). Only the glyphs copied into `assets/icons/`.
pub mod icons {
    pub const TODAY: &str = "today";
    pub const WEEK: &str = "calendar_view_week";
    pub const GRADES: &str = "school";
    pub const TESTS: &str = "fact_check";
    pub const DEADLINES: &str = "event_upcoming";
    pub const REGISTRATIONS: &str = "how_to_reg";
    pub const SURVEYS: &str = "ballot";
    pub const FIND_TIMETABLE: &str = "travel_explore";
    pub const PAYMENTS: &str = "payments";
    pub const NEWS: &str = "newspaper";
    pub const MESSAGES: &str = "mail";
    pub const SETTINGS: &str = "settings";
    pub const SEARCH: &str = "search";
    pub const REFRESH: &str = "refresh";
    pub const SYNC: &str = "sync";
    pub const SYNC_PROBLEM: &str = "sync_problem";
    pub const OFFLINE: &str = "cloud_off";
    pub const SESSION: &str = "lock_clock";
    pub const CLOSE: &str = "close";
    pub const CHECK: &str = "check";
    pub const OPEN_EXTERNAL: &str = "open_in_new";
    pub const COPY: &str = "content_copy";
    pub const LINK: &str = "link";
    pub const LOGOUT: &str = "logout";
    pub const SWITCH: &str = "swap_vert";
    pub const ACCOUNT: &str = "account_circle";
    pub const ERROR: &str = "error";
    pub const WARNING: &str = "warning";
    pub const INFO: &str = "info";
    pub const CHEVRON_RIGHT: &str = "chevron_right";
    pub const CHEVRON_LEFT: &str = "chevron_left";
    pub const EXPAND_MORE: &str = "expand_more";
    pub const ARROW_BACK: &str = "arrow_back";
    pub const ARROW_FORWARD: &str = "arrow_forward";
    pub const PROGRESS: &str = "progress_activity";
    pub const LIGHT_MODE: &str = "light_mode";
    pub const DARK_MODE: &str = "dark_mode";
    pub const PALETTE: &str = "palette";
    pub const TRANSLATE: &str = "translate";
    pub const NOTIFICATIONS: &str = "notifications";
    pub const KEYBOARD: &str = "keyboard";
    pub const CONTRAST: &str = "contrast";
}

/// A single-colour SVG mask. GPUI paints an `svg()` with the element's own text colour and does
/// not inherit it, so this reads the colour cascaded from the parent at render time.
#[derive(IntoElement)]
pub struct Glyph {
    path: SharedString,
    width: f32,
    height: f32,
    color: Option<Hsla>,
}

impl Glyph {
    /// Override the inherited colour.
    pub fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl RenderOnce for Glyph {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(window.text_style().color);
        svg()
            .path(self.path)
            .w(px(self.width))
            .h(px(self.height))
            .flex_none()
            .text_color(color)
    }
}

/// A Material Symbols glyph at 20 px (24 px above 20), outline or filled. It takes the text colour
/// of its parent, or `.text_color(..)`.
pub fn icon(name: &str, size: f32, filled: bool) -> Glyph {
    let grid = if size > 20.0 { 24 } else { 20 };
    let fill = if filled { "_fill1" } else { "" };
    Glyph {
        path: format!("icons/{name}{fill}_{grid}px.svg").into(),
        width: size,
        height: size,
        color: None,
    }
}

/// The LibreUSOS mark, a single-colour mask: tint with the parent's text colour.
pub fn logo(size: f32) -> Glyph {
    // The mark's view box is 58 by 46.
    Glyph {
        path: "logo/mark-mono.svg".into(),
        width: size * 58.0 / 46.0,
        height: size,
        color: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_files_are_present() {
        for font in [
            "fonts/Geist-400.ttf",
            "fonts/Geist-500.ttf",
            "fonts/Geist-600.ttf",
            "fonts/GeistMono-400.ttf",
            "fonts/GeistMono-500.ttf",
        ] {
            assert!(Embedded::get(font).is_some(), "{font}");
        }
        assert!(Embedded::get("logo/mark-mono.svg").is_some());
    }

    #[test]
    fn every_named_icon_has_all_four_variants() {
        // Keep this list in step with `icons`: a missing file would render as nothing.
        let names = [
            icons::TODAY,
            icons::WEEK,
            icons::GRADES,
            icons::TESTS,
            icons::DEADLINES,
            icons::REGISTRATIONS,
            icons::SURVEYS,
            icons::FIND_TIMETABLE,
            icons::PAYMENTS,
            icons::NEWS,
            icons::MESSAGES,
            icons::SETTINGS,
            icons::SEARCH,
            icons::REFRESH,
            icons::SYNC,
            icons::SYNC_PROBLEM,
            icons::OFFLINE,
            icons::SESSION,
            icons::CLOSE,
            icons::CHECK,
            icons::OPEN_EXTERNAL,
            icons::COPY,
            icons::LINK,
            icons::LOGOUT,
            icons::SWITCH,
            icons::ACCOUNT,
            icons::ERROR,
            icons::WARNING,
            icons::INFO,
            icons::CHEVRON_RIGHT,
            icons::CHEVRON_LEFT,
            icons::EXPAND_MORE,
            icons::ARROW_BACK,
            icons::ARROW_FORWARD,
            icons::PROGRESS,
            icons::LIGHT_MODE,
            icons::DARK_MODE,
            icons::PALETTE,
            icons::TRANSLATE,
            icons::NOTIFICATIONS,
            icons::KEYBOARD,
            icons::CONTRAST,
        ];
        for name in names {
            for variant in ["", "_fill1"] {
                for grid in [20, 24] {
                    let path = format!("icons/{name}{variant}_{grid}px.svg");
                    assert!(Embedded::get(&path).is_some(), "{path}");
                }
            }
        }
    }
}
