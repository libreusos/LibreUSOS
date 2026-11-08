//! The theming pipeline of SYSTEM.md section 4: in-app choice, then the system, then the brand.

use material_colors::color::Rgb;

use super::system::SystemSettings;
use super::tokens::{BRAND_SEED, parse_hex};
use crate::settings::{AccentChoice, Prefs, ThemeMode};

/// Where the accent seed came from; the settings page words it ("Pulpit nie podaje koloru").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentSource {
    /// The user picked a custom colour.
    Custom,
    /// The user picked the LibreUSOS colour.
    Brand,
    /// The desktop's accent.
    System,
    /// Following the system, but the desktop exposes no accent: the brand colour.
    NotDetected,
}

/// Everything the theme needs, decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub dark: bool,
    pub seed: Rgb,
    pub accent_source: AccentSource,
    /// Material contrast level: 1.0 when high contrast is on, else 0.0.
    pub contrast: f64,
    pub reduced_motion: bool,
}

/// `system_dark` is the window appearance GPUI reports (Light when the system has no preference).
pub fn resolve(prefs: &Prefs, system: &SystemSettings, system_dark: bool) -> Resolved {
    let dark = match prefs.theme_mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => system_dark,
    };
    let (seed, accent_source) = match prefs.accent {
        AccentChoice::Custom => match parse_hex(&prefs.custom_color) {
            Some(c) => (c, AccentSource::Custom),
            None => (BRAND_SEED, AccentSource::Brand),
        },
        AccentChoice::Brand => (BRAND_SEED, AccentSource::Brand),
        AccentChoice::System => match system.accent {
            Some(c) => (c, AccentSource::System),
            None => (BRAND_SEED, AccentSource::NotDetected),
        },
    };
    Resolved {
        dark,
        seed,
        accent_source,
        contrast: if prefs.high_contrast.resolve(system.high_contrast) {
            1.0
        } else {
            0.0
        },
        reduced_motion: prefs.reduced_motion.resolve(system.reduced_motion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Toggle;

    fn sys(accent: Option<Rgb>) -> SystemSettings {
        SystemSettings {
            accent,
            ..Default::default()
        }
    }

    #[test]
    fn appearance_precedence() {
        let mut p = Prefs::default();
        let s = sys(None);
        // No preference anywhere: light.
        assert!(!resolve(&p, &s, false).dark);
        // System dark is followed by default.
        assert!(resolve(&p, &s, true).dark);
        // The in-app override wins over the system.
        p.theme_mode = ThemeMode::Light;
        assert!(!resolve(&p, &s, true).dark);
        p.theme_mode = ThemeMode::Dark;
        assert!(resolve(&p, &s, false).dark);
    }

    #[test]
    fn accent_precedence() {
        let green = Rgb::new(0x3a, 0x94, 0x4a);
        let mut p = Prefs::default();
        let r = resolve(&p, &sys(Some(green)), false);
        assert_eq!((r.seed, r.accent_source), (green, AccentSource::System));
        let r = resolve(&p, &sys(None), false);
        assert_eq!(
            (r.seed, r.accent_source),
            (BRAND_SEED, AccentSource::NotDetected)
        );
        p.accent = AccentChoice::Brand;
        let r = resolve(&p, &sys(Some(green)), false);
        assert_eq!((r.seed, r.accent_source), (BRAND_SEED, AccentSource::Brand));
        p.accent = AccentChoice::Custom;
        p.custom_color = "#C4532F".into();
        let r = resolve(&p, &sys(Some(green)), false);
        assert_eq!(r.seed, Rgb::new(0xC4, 0x53, 0x2F));
        assert_eq!(r.accent_source, AccentSource::Custom);
        p.custom_color = "not a colour".into();
        assert_eq!(resolve(&p, &sys(None), false).seed, BRAND_SEED);
    }

    #[test]
    fn contrast_and_motion() {
        let mut p = Prefs::default();
        let mut s = sys(None);
        let r = resolve(&p, &s, false);
        assert_eq!((r.contrast, r.reduced_motion), (0.0, false));
        s.high_contrast = true;
        s.reduced_motion = true;
        let r = resolve(&p, &s, false);
        assert_eq!((r.contrast, r.reduced_motion), (1.0, true));
        p.high_contrast = Toggle::Off;
        p.reduced_motion = Toggle::Off;
        let r = resolve(&p, &s, false);
        assert_eq!((r.contrast, r.reduced_motion), (0.0, false));
    }
}
