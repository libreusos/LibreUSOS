//! The one place where our `Tokens` become gpui-component's `ThemeColor` (SYSTEM.md section 3,
//! "Mapping onto ThemeColor"). Hover and active states are M3 state layers: the role's "on"
//! colour over the role at 8 % and 12 %.
//!
//! Fields the design table does not list keep the value of `ThemeColor::light()` / `dark()`,
//! except the ones set at the end of `theme_color` (the per-variant button colours, tab
//! foregrounds, list/table extras and the status colours), which follow the nearest listed role.

use gpui::Hsla;
use gpui_component::theme::ThemeColor;

use super::tokens::{HOVER_LAYER, PRESSED_LAYER, Tokens, state_layer};

fn hover(base: Hsla, on: Hsla) -> Hsla {
    state_layer(base, on, HOVER_LAYER)
}

fn active(base: Hsla, on: Hsla) -> Hsla {
    state_layer(base, on, PRESSED_LAYER)
}

/// Build the gpui-component colour set for `t`.
pub fn theme_color(t: &Tokens) -> ThemeColor {
    let mut c = if t.dark {
        *ThemeColor::dark()
    } else {
        *ThemeColor::light()
    };
    let group = t.group_bg();

    c.background = t.surface;
    c.foreground = t.on_surface;
    c.border = t.outline_variant;
    c.window_border = t.outline_variant;
    c.title_bar_border = t.outline_variant;
    c.sidebar_border = t.outline_variant;
    c.input = t.outline;
    c.ring = t.primary;

    c.primary = t.primary;
    c.primary_foreground = t.on_primary;
    c.primary_hover = hover(t.primary, t.on_primary);
    c.primary_active = active(t.primary, t.on_primary);

    c.secondary = t.secondary_container;
    c.secondary_foreground = t.on_secondary_container;
    c.secondary_hover = hover(t.secondary_container, t.on_secondary_container);
    c.secondary_active = active(t.secondary_container, t.on_secondary_container);

    c.muted = t.surface_container_high;
    c.muted_foreground = t.on_surface_variant;
    // gpui-component uses "accent" for hover backgrounds in lists and menus.
    c.accent = t.surface_container_high;
    c.accent_foreground = t.on_surface;

    c.title_bar = t.surface;
    c.sidebar = t.surface_container_low;
    c.sidebar_foreground = t.on_surface;
    c.sidebar_accent = t.secondary_container;
    c.sidebar_accent_foreground = t.on_secondary_container;
    c.sidebar_primary = t.primary;
    c.sidebar_primary_foreground = t.on_primary;

    c.popover = t.surface_container_high;
    c.popover_foreground = t.on_surface;

    c.list = group;
    c.table = group;
    c.list_hover = t.surface_container;
    c.table_hover = t.surface_container;
    c.list_active = t.secondary_container;
    c.table_active = t.secondary_container;
    c.table_head = group;
    c.table_row_border = t.outline_variant;

    c.tab = t.surface_container_high;
    c.tab_bar = t.surface_container_high;
    c.tab_active = group;

    c.selection = t.primary.opacity(0.3);
    c.link = t.primary;

    c.danger = t.error;
    c.danger_foreground = t.on_error;
    c.danger_hover = hover(t.error, t.on_error);
    c.danger_active = active(t.error, t.on_error);

    c.skeleton = t.surface_container_high;
    c.switch = t.surface_container_highest;
    c.progress_bar = t.primary;
    c.scrollbar_thumb = t.on_surface.opacity(0.3);
    c.overlay = t.scrim.opacity(0.32);

    // Present only because the struct requires them; our code never uses them.
    c.success = t.primary;
    c.info = t.secondary_container;
    c.warning = t.tertiary_container;

    // Not in the design table: keep the widgets that read these consistent with the roles above.
    c.button_primary = c.primary;
    c.button_primary_foreground = c.primary_foreground;
    c.button_primary_hover = c.primary_hover;
    c.button_primary_active = c.primary_active;
    c.button_secondary = c.secondary;
    c.button_secondary_foreground = c.secondary_foreground;
    c.button_secondary_hover = c.secondary_hover;
    c.button_secondary_active = c.secondary_active;
    c.button_danger = c.danger;
    c.button_danger_foreground = c.danger_foreground;
    c.button_danger_hover = c.danger_hover;
    c.button_danger_active = c.danger_active;
    c.link_hover = c.link;
    c.link_active = c.link;
    c.list_head = group;
    c.list_even = group;
    c.table_even = group;
    c.tab_bar_segmented = t.surface_container_high;
    c.tab_foreground = t.on_surface_variant;
    c.tab_active_foreground = t.on_surface;
    c.table_head_foreground = t.on_surface_variant;
    c.caret = t.primary;
    c.switch_thumb = t.outline;
    c.slider_bar = t.primary;
    c.slider_thumb = t.primary;
    c
}

impl From<&Tokens> for ThemeColor {
    fn from(t: &Tokens) -> Self {
        theme_color(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::{BRAND_SEED, to_hex};

    #[test]
    fn maps_the_documented_roles() {
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let c = theme_color(&t);
        assert_eq!(c.background, t.surface);
        assert_eq!(c.foreground, t.on_surface);
        assert_eq!(c.border, t.outline_variant);
        assert_eq!(c.sidebar_border, t.outline_variant);
        assert_eq!(c.input, t.outline);
        assert_eq!(c.ring, t.primary);
        assert_eq!(c.primary, t.primary);
        assert_eq!(c.secondary, t.secondary_container);
        assert_eq!(c.accent, t.surface_container_high);
        assert_eq!(c.sidebar, t.surface_container_low);
        assert_eq!(c.sidebar_accent, t.secondary_container);
        assert_eq!(c.popover, t.surface_container_high);
        assert_eq!(c.list, t.surface_container_lowest);
        assert_eq!(c.list_hover, t.surface_container);
        assert_eq!(c.tab_active, t.surface_container_lowest);
        assert_eq!(c.danger, t.error);
        assert_eq!(c.skeleton, t.surface_container_high);
    }

    #[test]
    fn dark_groups_use_the_low_container() {
        let t = Tokens::from_seed(BRAND_SEED, true, 0.0);
        let c = theme_color(&t);
        assert_eq!(c.list, t.surface_container_low);
        assert_eq!(c.table_head, t.surface_container_low);
        assert_eq!(c.tab_active, t.surface_container_low);
    }

    #[test]
    fn state_layers_are_8_and_12_percent() {
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let c = theme_color(&t);
        assert_eq!(c.primary_hover, state_layer(t.primary, t.on_primary, 0.08));
        assert_eq!(c.primary_active, state_layer(t.primary, t.on_primary, 0.12));
        // On white, a black layer at 8 % is #EBEBEB and at 12 % is #E0E0E0.
        let white_on_black = state_layer(
            gpui::hsla(0.0, 0.0, 1.0, 1.0),
            gpui::hsla(0.0, 0.0, 0.0, 1.0),
            0.12,
        );
        assert_eq!(to_hex(white_on_black), "#E0E0E0");
        assert_ne!(c.primary_hover, c.primary_active);
    }

    #[test]
    fn selection_and_overlay_are_translucent() {
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let c = theme_color(&t);
        assert!((c.selection.a - 0.3).abs() < 1e-6);
        assert!((c.overlay.a - 0.32).abs() < 1e-6);
        assert!((c.scrollbar_thumb.a - 0.3).abs() < 1e-6);
    }
}
