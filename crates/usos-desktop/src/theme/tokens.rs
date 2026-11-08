//! Material 3 colour roles generated from one seed (SYSTEM.md section 3), plus the class-kind
//! colours and the state-layer helper. Pure: no GPUI global is touched here.

use gpui::{Hsla, Rgba};
use material_colors::color::{Lab, Rgb};
use material_colors::dynamic_color::{SpecVersion, Variant};
use material_colors::scheme::Scheme;
use material_colors::theme::ThemeBuilder;

/// The LibreUSOS brand seed ("ink blue"), used when the desktop provides no accent.
pub const BRAND_SEED: Rgb = Rgb::new(0x3F, 0x5A, 0xA6);

/// State layer opacity on hover (M3).
pub const HOVER_LAYER: f32 = 0.08;
/// State layer opacity when pressed or focused (M3).
pub const PRESSED_LAYER: f32 = 0.12;
/// Below this CIE76 distance the LAB colours get an outline (SYSTEM.md section 3).
pub const KIND_MIN_DELTA_E: f64 = 15.0;

/// Material 3 colour roles, generated from one seed. `Hsla` because that is what GPUI paints with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub primary: Hsla,
    pub on_primary: Hsla,
    pub primary_container: Hsla,
    pub on_primary_container: Hsla,
    pub secondary_container: Hsla,
    pub on_secondary_container: Hsla,
    pub tertiary_container: Hsla,
    pub on_tertiary_container: Hsla,
    pub error: Hsla,
    pub on_error: Hsla,
    pub error_container: Hsla,
    pub on_error_container: Hsla,
    pub surface: Hsla,
    pub on_surface: Hsla,
    pub on_surface_variant: Hsla,
    pub surface_container_lowest: Hsla,
    pub surface_container_low: Hsla,
    pub surface_container: Hsla,
    pub surface_container_high: Hsla,
    pub surface_container_highest: Hsla,
    pub outline: Hsla,
    pub outline_variant: Hsla,
    pub inverse_surface: Hsla,
    pub inverse_on_surface: Hsla,
    pub scrim: Hsla,
    pub dark: bool,
}

/// Convert a Material colour to GPUI's `Hsla`.
pub fn hsla(c: Rgb) -> Hsla {
    Rgba {
        r: f32::from(c.red) / 255.0,
        g: f32::from(c.green) / 255.0,
        b: f32::from(c.blue) / 255.0,
        a: 1.0,
    }
    .into()
}

/// `#RRGGBB` of a colour (alpha ignored). Used by tests.
#[cfg(test)]
pub fn to_hex(c: Hsla) -> String {
    let c = Rgba::from(c);
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", byte(c.r), byte(c.g), byte(c.b))
}

/// Parse `#RRGGBB` (the `#` is optional).
pub fn parse_hex(text: &str) -> Option<Rgb> {
    let t = text.trim().trim_start_matches('#');
    if t.len() != 6 || !t.is_ascii() {
        return None;
    }
    let v = u32::from_str_radix(t, 16).ok()?;
    Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Blend `on` over `base` at `alpha` (an M3 state layer). The result is opaque.
pub fn state_layer(base: Hsla, on: Hsla, alpha: f32) -> Hsla {
    let (b, o) = (Rgba::from(base), Rgba::from(on));
    let mix = |x: f32, y: f32| x + (y - x) * alpha;
    Rgba {
        r: mix(b.r, o.r),
        g: mix(b.g, o.g),
        b: mix(b.b, o.b),
        a: 1.0,
    }
    .into()
}

impl Tokens {
    /// material-colors: `ThemeBuilder::with_source(seed)` TonalSpot, spec 2021 (explicit, so a
    /// default change in the crate cannot drift the tones away from Android), at `contrast`
    /// (0.0 normal, 1.0 high).
    pub fn from_seed(seed: Rgb, dark: bool, contrast: f64) -> Self {
        let theme = ThemeBuilder::with_source(seed)
            .variant(Variant::TonalSpot)
            .spec_version(SpecVersion::Spec2021)
            .contrast_level(contrast)
            .build();
        let scheme = if dark {
            theme.schemes.dark
        } else {
            theme.schemes.light
        };
        Self::from_scheme(&scheme, dark)
    }

    /// Field-by-field copy of a Material scheme.
    pub fn from_scheme(s: &Scheme, dark: bool) -> Self {
        Self {
            primary: hsla(s.primary),
            on_primary: hsla(s.on_primary),
            primary_container: hsla(s.primary_container),
            on_primary_container: hsla(s.on_primary_container),
            secondary_container: hsla(s.secondary_container),
            on_secondary_container: hsla(s.on_secondary_container),
            tertiary_container: hsla(s.tertiary_container),
            on_tertiary_container: hsla(s.on_tertiary_container),
            error: hsla(s.error),
            on_error: hsla(s.on_error),
            error_container: hsla(s.error_container),
            on_error_container: hsla(s.on_error_container),
            surface: hsla(s.surface),
            on_surface: hsla(s.on_surface),
            on_surface_variant: hsla(s.on_surface_variant),
            surface_container_lowest: hsla(s.surface_container_lowest),
            surface_container_low: hsla(s.surface_container_low),
            surface_container: hsla(s.surface_container),
            surface_container_high: hsla(s.surface_container_high),
            surface_container_highest: hsla(s.surface_container_highest),
            outline: hsla(s.outline),
            outline_variant: hsla(s.outline_variant),
            inverse_surface: hsla(s.inverse_surface),
            inverse_on_surface: hsla(s.inverse_on_surface),
            scrim: hsla(s.scrim),
            dark,
        }
    }

    /// Fill of grouped lists, the detail pane and inner panels.
    pub fn group_bg(&self) -> Hsla {
        if self.dark {
            self.surface_container_low
        } else {
            self.surface_container_lowest
        }
    }

    /// Hover colour of a filled role: its "on" colour over it at 8 %.
    pub fn hover(&self, base: Hsla, on: Hsla) -> Hsla {
        state_layer(base, on, HOVER_LAYER)
    }

    /// Pressed colour of a filled role: its "on" colour over it at 12 %.
    pub fn pressed(&self, base: Hsla, on: Hsla) -> Hsla {
        state_layer(base, on, PRESSED_LAYER)
    }
}

/// The class kinds that get their own colour (SYSTEM.md section 3, "Class-kind mapping").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Wykład.
    Lecture,
    /// Laboratorium.
    Lab,
    /// Ćwiczenia, projekt, seminarium, lektorat, WF and anything else.
    Other,
    /// Egzamin, kolokwium.
    Exam,
}

/// Fill, text and optional border of a kind tag or block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KindColors {
    pub fill: Hsla,
    pub text: Hsla,
    /// 1 px `outline_variant` for the neutral kind; 1.5 px `on_tertiary_container` for LAB when
    /// the contrast fallback applies.
    pub border: Option<Hsla>,
}

fn lab_of(c: Hsla) -> Lab {
    let c = Rgba::from(c);
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb::new(byte(c.r), byte(c.g), byte(c.b)).into()
}

/// CIE76 colour distance between two colours.
pub fn delta_e(a: Hsla, b: Hsla) -> f64 {
    let (a, b) = (lab_of(a), lab_of(b));
    ((a.l - b.l).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt()
}

impl Tokens {
    /// Whether LAB tags and blocks need the outline: their fill is closer than
    /// [`KIND_MIN_DELTA_E`] to the lecture fill or to the neutral fill.
    pub fn lab_needs_outline(&self) -> bool {
        delta_e(self.tertiary_container, self.primary_container) < KIND_MIN_DELTA_E
            || delta_e(self.tertiary_container, self.surface_container_highest) < KIND_MIN_DELTA_E
    }

    pub fn kind(&self, kind: Kind) -> KindColors {
        match kind {
            Kind::Lecture => KindColors {
                fill: self.primary_container,
                text: self.on_primary_container,
                border: None,
            },
            Kind::Lab => KindColors {
                fill: self.tertiary_container,
                text: self.on_tertiary_container,
                border: self
                    .lab_needs_outline()
                    .then_some(self.on_tertiary_container),
            },
            Kind::Other => KindColors {
                fill: self.surface_container_highest,
                text: self.on_surface,
                border: Some(self.outline_variant),
            },
            Kind::Exam => KindColors {
                fill: self.primary,
                text: self.on_primary,
                border: None,
            },
        }
    }
}

/// How much of the kind fill a past block keeps over `surface_container`.
const PAST_TINT: f32 = 0.4;
/// The exam fill is the saturated `primary`, so a past exam keeps less of it.
const PAST_EXAM_TINT: f32 = 0.22;

impl Tokens {
    /// A grid block of a class that is over: an opaque, muted version of the kind fill, so the
    /// hour lines never run through the text and the kind stays readable, with muted text and a
    /// hairline. The neutral kind stays plain `surface_container`.
    pub fn past_block(&self, kind: Kind) -> KindColors {
        let fill = match kind {
            Kind::Other => self.surface_container,
            Kind::Exam => state_layer(self.surface_container, self.primary, PAST_EXAM_TINT),
            _ => state_layer(self.surface_container, self.kind(kind).fill, PAST_TINT),
        };
        KindColors {
            fill,
            text: self.on_surface_variant,
            border: Some(self.outline_variant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(c: Hsla) -> String {
        to_hex(c)
    }

    #[test]
    fn same_seed_gives_same_tokens() {
        let a = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let b = Tokens::from_seed(BRAND_SEED, false, 0.0);
        assert_eq!(a, b);
        assert_ne!(a, Tokens::from_seed(BRAND_SEED, true, 0.0));
    }

    /// Values from `design/tokens/schemes.json` (brand, light).
    #[test]
    fn brand_light_matches_design_tokens() {
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        assert!(!t.dark);
        assert_eq!(hex(t.primary), "#4B5C92");
        assert_eq!(hex(t.on_primary), "#FFFFFF");
        assert_eq!(hex(t.primary_container), "#DBE1FF");
        assert_eq!(hex(t.on_primary_container), "#324478");
        assert_eq!(hex(t.secondary_container), "#DDE1F9");
        assert_eq!(hex(t.tertiary_container), "#FFD6F8");
        assert_eq!(hex(t.on_tertiary_container), "#5A3D58");
        assert_eq!(hex(t.error), "#BA1A1A");
        assert_eq!(hex(t.error_container), "#FFDAD6");
        assert_eq!(hex(t.surface), "#FAF8FF");
        assert_eq!(hex(t.on_surface), "#1A1B21");
        assert_eq!(hex(t.surface_container_lowest), "#FFFFFF");
        assert_eq!(hex(t.surface_container_low), "#F4F3FA");
        assert_eq!(hex(t.surface_container), "#EEEDF4");
        assert_eq!(hex(t.surface_container_high), "#E8E7EF");
        assert_eq!(hex(t.surface_container_highest), "#E3E2E9");
        assert_eq!(hex(t.outline), "#757680");
        assert_eq!(hex(t.outline_variant), "#C5C6D0");
        assert_eq!(hex(t.inverse_surface), "#2F3036");
        assert_eq!(hex(t.scrim), "#000000");
    }

    #[test]
    fn brand_dark_matches_design_tokens() {
        let t = Tokens::from_seed(BRAND_SEED, true, 0.0);
        assert!(t.dark);
        assert_eq!(hex(t.primary), "#B4C5FF");
        assert_eq!(hex(t.on_primary), "#1A2D60");
        assert_eq!(hex(t.primary_container), "#324478");
        assert_eq!(hex(t.surface), "#121318");
        assert_eq!(hex(t.on_surface), "#E3E2E9");
        assert_eq!(hex(t.surface_container_low), "#1A1B21");
        assert_eq!(hex(t.surface_container_highest), "#34343A");
        assert_eq!(hex(t.outline), "#8F909A");
        assert_eq!(hex(t.outline_variant), "#45464F");
        assert_eq!(hex(t.error), "#FFB4AB");
        assert_eq!(hex(t.group_bg()), "#1A1B21");
    }

    #[test]
    fn moss_light_matches_design_tokens() {
        let t = Tokens::from_seed(Rgb::new(0x55, 0x7F, 0x3E), false, 0.0);
        assert_eq!(hex(t.primary), "#456732");
        assert_eq!(hex(t.primary_container), "#C5EFAB");
        assert_eq!(hex(t.tertiary_container), "#BBEBEC");
        assert_eq!(hex(t.surface), "#F8FAF0");
        assert_eq!(hex(t.group_bg()), "#FFFFFF");
    }

    #[test]
    fn high_contrast_changes_the_scheme() {
        let normal = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let high = Tokens::from_seed(BRAND_SEED, false, 1.0);
        assert_ne!(normal.primary, high.primary);
    }

    /// `lightKinds` / `darkKinds` in `design/tokens/schemes.json`: only grey light is outlined.
    #[test]
    fn lab_outline_rule_matches_generator() {
        let grey = Rgb::new(0x80, 0x80, 0x80);
        assert!(Tokens::from_seed(grey, false, 0.0).lab_needs_outline());
        assert!(!Tokens::from_seed(grey, true, 0.0).lab_needs_outline());
        assert!(!Tokens::from_seed(BRAND_SEED, false, 0.0).lab_needs_outline());
        assert!(!Tokens::from_seed(BRAND_SEED, true, 0.0).lab_needs_outline());
        let moss = Rgb::new(0x55, 0x7F, 0x3E);
        assert!(!Tokens::from_seed(moss, false, 0.0).lab_needs_outline());
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        let d = delta_e(t.tertiary_container, t.primary_container);
        assert!((d - 16.5).abs() < 0.1, "got {d}");
    }

    #[test]
    fn kind_colours_follow_the_table() {
        let t = Tokens::from_seed(BRAND_SEED, false, 0.0);
        assert_eq!(t.kind(Kind::Lecture).fill, t.primary_container);
        assert_eq!(t.kind(Kind::Lab).fill, t.tertiary_container);
        assert_eq!(t.kind(Kind::Lab).border, None);
        assert_eq!(t.kind(Kind::Other).border, Some(t.outline_variant));
        assert_eq!(t.kind(Kind::Exam).text, t.on_primary);
    }

    /// WCAG contrast ratio of two colours.
    fn contrast(a: Hsla, b: Hsla) -> f64 {
        fn lum(c: Hsla) -> f64 {
            let c = Rgba::from(c);
            let lin = |v: f32| {
                let v = f64::from(v);
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
        }
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn block_text_is_readable_in_every_scheme_and_kind() {
        let kinds = [Kind::Lecture, Kind::Lab, Kind::Other, Kind::Exam];
        for dark in [false, true] {
            for contrast_level in [0.0, 1.0] {
                let t = Tokens::from_seed(BRAND_SEED, dark, contrast_level);
                for kind in kinds {
                    let live = t.kind(kind);
                    assert!(
                        contrast(live.text, live.fill) >= 4.5,
                        "{kind:?} dark={dark}"
                    );
                    let past = t.past_block(kind);
                    assert!(
                        contrast(past.text, past.fill) >= 4.5,
                        "past {kind:?} dark={dark}"
                    );
                    // Opaque, and distinct from the grid background.
                    assert_eq!(past.fill.a, 1.0);
                    assert_ne!(past.fill, t.surface);
                    assert_eq!(past.border, Some(t.outline_variant));
                }
            }
        }
    }

    #[test]
    fn state_layer_blends_in_srgb() {
        let white = hsla(Rgb::new(255, 255, 255));
        let black = hsla(Rgb::new(0, 0, 0));
        assert_eq!(hex(state_layer(white, black, 0.08)), "#EBEBEB");
        assert_eq!(hex(state_layer(white, black, 0.0)), "#FFFFFF");
        assert_eq!(hex(state_layer(white, black, 1.0)), "#000000");
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(parse_hex("#3F5AA6"), Some(BRAND_SEED));
        assert_eq!(parse_hex("3f5aa6"), Some(BRAND_SEED));
        assert_eq!(parse_hex("#3F5AA"), None);
        assert_eq!(parse_hex("#GGGGGG"), None);
        assert_eq!(to_hex(hsla(BRAND_SEED)), "#3F5AA6");
    }
}
