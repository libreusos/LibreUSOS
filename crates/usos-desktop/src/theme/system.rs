//! What the desktop says about itself (SYSTEM.md section 4.1): accent colour, contrast and
//! reduced motion. Parsing is pure and tested; the readers at the bottom talk to the XDG portal,
//! gsettings, kdeglobals and Windows.

use material_colors::color::Rgb;

/// Settings read from the platform. `None` / `false` mean "the desktop says nothing".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SystemSettings {
    pub accent: Option<Rgb>,
    pub high_contrast: bool,
    pub reduced_motion: bool,
}

/// libadwaita accent names and their colours (GNOME 47+ `accent-color`).
pub fn gnome_accent(name: &str) -> Option<Rgb> {
    let hex = match name.trim().trim_matches('\'').trim_matches('"') {
        "blue" => (0x35, 0x84, 0xe4),
        "teal" => (0x21, 0x90, 0xa4),
        "green" => (0x3a, 0x94, 0x4a),
        "yellow" => (0xc8, 0x88, 0x00),
        "orange" => (0xed, 0x5b, 0x00),
        "red" => (0xe6, 0x2d, 0x42),
        "pink" => (0xd5, 0x61, 0x99),
        "purple" => (0x91, 0x41, 0xac),
        "slate" => (0x6f, 0x83, 0x96),
        _ => return None,
    };
    Some(Rgb::new(hex.0, hex.1, hex.2))
}

/// The portal's `(ddd)` sRGB value. A component outside 0..=1 (or not finite) means unset.
#[cfg(any(target_os = "linux", target_os = "freebsd", test))]
pub fn portal_color(r: f64, g: f64, b: f64) -> Option<Rgb> {
    let ok = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
    if !(ok(r) && ok(g) && ok(b)) {
        return None;
    }
    let byte = |v: f64| (v * 255.0).round() as u8;
    Some(Rgb::new(byte(r), byte(g), byte(b)))
}

fn parse_rgb_triplet(text: &str) -> Option<Rgb> {
    let parts: Option<Vec<u8>> = text.split(',').map(|p| p.trim().parse().ok()).collect();
    match parts?.as_slice() {
        // A fourth component (alpha) is tolerated, more is not a colour.
        &[r, g, b] | &[r, g, b, _] => Some(Rgb::new(r, g, b)),
        _ => None,
    }
}

/// `kdeglobals`: `[General] AccentColor=r,g,b`, else `[Colors:Selection] BackgroundNormal=r,g,b`.
pub fn parse_kdeglobals(text: &str) -> Option<Rgb> {
    let mut section = "";
    let mut accent = None;
    let mut selection = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with(';') || line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match (section, key.trim()) {
            ("General", "AccentColor") => accent = accent.or(parse_rgb_triplet(value)),
            ("Colors:Selection", "BackgroundNormal") => {
                selection = selection.or(parse_rgb_triplet(value));
            }
            _ => {}
        }
    }
    accent.or(selection)
}

/// Whether `XDG_CURRENT_DESKTOP` (a colon-separated list) names `desktop`.
pub fn desktop_is(current: &str, desktop: &str) -> bool {
    current.split(':').any(|d| {
        d.to_ascii_uppercase()
            .contains(&desktop.to_ascii_uppercase())
    })
}

/// The accent chain of SYSTEM.md section 4.1 (steps 2 to 5), first hit wins. The readers for
/// steps 3 to 5 are only called when their step is reached.
pub fn pick_accent(
    current_desktop: &str,
    portal: Option<Rgb>,
    gnome: impl FnOnce() -> Option<Rgb>,
    kde: impl FnOnce() -> Option<Rgb>,
    windows: impl FnOnce() -> Option<Rgb>,
) -> Option<Rgb> {
    if portal.is_some() {
        return portal;
    }
    if desktop_is(current_desktop, "GNOME")
        && let Some(c) = gnome()
    {
        return Some(c);
    }
    if desktop_is(current_desktop, "KDE")
        && let Some(c) = kde()
    {
        return Some(c);
    }
    windows()
}

/// `gsettings get <schema> <key>` output, trimmed. `None` when gsettings is missing or fails.
#[cfg(unix)]
fn gsettings(schema: &str, key: &str) -> Option<String> {
    let out = std::process::Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(not(unix))]
fn gsettings(_schema: &str, _key: &str) -> Option<String> {
    None
}

fn current_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
}

fn gnome_accent_from_gsettings() -> Option<Rgb> {
    gnome_accent(&gsettings("org.gnome.desktop.interface", "accent-color")?)
}

fn kde_accent_from_file() -> Option<Rgb> {
    let path = dirs::config_dir()?.join("kdeglobals");
    parse_kdeglobals(&std::fs::read_to_string(path).ok()?)
}

/// GNOME `enable-animations = false` (reduced motion without the portal key).
fn gnome_animations_off() -> bool {
    desktop_is(&current_desktop(), "GNOME")
        && gsettings("org.gnome.desktop.interface", "enable-animations").as_deref() == Some("false")
}

/// Windows accent colour (`UISettings::GetColorValue(UIColorType::Accent)`).
#[cfg(windows)]
fn windows_accent() -> Option<Rgb> {
    use windows::UI::ViewManagement::{UIColorType, UISettings};
    let c = UISettings::new()
        .ok()?
        .GetColorValue(UIColorType::Accent)
        .ok()?;
    Some(Rgb::new(c.R, c.G, c.B))
}

// shortcut: Windows high contrast and "animations off" are not read yet (SYSTEM.md 4.1 lists
// them); the defaults apply and the user can override both in Ustawienia > Wygląd.
#[cfg(not(windows))]
fn windows_accent() -> Option<Rgb> {
    None
}

/// Read everything the platform offers. Blocking (gsettings, file reads) except the portal,
/// which is async; call from a background task.
pub async fn read_system() -> SystemSettings {
    let desktop = current_desktop();
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    let (portal_accent, high_contrast, reduced_motion) = portal::read().await;
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    let (portal_accent, high_contrast, reduced_motion) = (None, false, false);

    SystemSettings {
        accent: pick_accent(
            &desktop,
            portal_accent,
            gnome_accent_from_gsettings,
            kde_accent_from_file,
            windows_accent,
        ),
        high_contrast,
        reduced_motion: reduced_motion || gnome_animations_off(),
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod portal {
    //! XDG settings portal through ashpd, on GPUI's executor (no tokio).

    use ashpd::desktop::settings::{Contrast, ReducedMotion, Settings};
    use futures_lite::{StreamExt, future};
    use material_colors::color::Rgb;

    /// `(accent, high contrast, reduced motion)`. Any missing key is "unset".
    pub async fn read() -> (Option<Rgb>, bool, bool) {
        let Ok(settings) = Settings::new().await else {
            return (None, false, false);
        };
        let accent = settings
            .accent_color()
            .await
            .ok()
            .and_then(|c| super::portal_color(c.red(), c.green(), c.blue()));
        let contrast = matches!(settings.contrast().await, Ok(Contrast::High));
        let motion = matches!(
            settings.reduced_motion().await,
            Ok(ReducedMotion::ReducedMotion)
        );
        (accent, contrast, motion)
    }

    /// Resolves when the portal reports any change of accent, contrast or reduced motion.
    /// Never resolves when there is no portal.
    pub async fn changed() {
        let Ok(settings) = Settings::new().await else {
            return std::future::pending().await;
        };
        let (Ok(mut accent), Ok(mut contrast), Ok(mut motion)) = (
            settings.receive_accent_color_changed().await,
            settings.receive_contrast_changed().await,
            settings.receive_reduced_motion_changed().await,
        ) else {
            return std::future::pending().await;
        };
        // A stream that ends (the portal went away) is not a change: it never resolves.
        let accent = async {
            if accent.next().await.is_none() {
                std::future::pending::<()>().await;
            }
        };
        let contrast = async {
            if contrast.next().await.is_none() {
                std::future::pending::<()>().await;
            }
        };
        let motion = async {
            if motion.next().await.is_none() {
                std::future::pending::<()>().await;
            }
        };
        future::or(accent, future::or(contrast, motion)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdeglobals_prefers_accent_color() {
        let text = "[Colors:Selection]\nBackgroundNormal=61,174,233\n\n[General]\nAccentColor=146,65,172\nColorScheme=BreezeLight\n";
        assert_eq!(parse_kdeglobals(text), Some(Rgb::new(146, 65, 172)));
    }

    #[test]
    fn kdeglobals_falls_back_to_selection_background() {
        let text = "[General]\nColorScheme=BreezeLight\n[Colors:Selection]\nBackgroundNormal=61,174,233\nForegroundNormal=255,255,255\n";
        assert_eq!(parse_kdeglobals(text), Some(Rgb::new(61, 174, 233)));
    }

    #[test]
    fn kdeglobals_rejects_garbage() {
        assert_eq!(parse_kdeglobals(""), None);
        assert_eq!(parse_kdeglobals("[General]\nAccentColor=1,2\n"), None);
        assert_eq!(parse_kdeglobals("[General]\nAccentColor=300,2,3\n"), None);
        assert_eq!(parse_kdeglobals("[General]\nAccentColor=1,2,3,4,5\n"), None);
        // Same key in another section is not the accent.
        assert_eq!(parse_kdeglobals("[Other]\nAccentColor=1,2,3\n"), None);
        // Alpha is tolerated.
        assert_eq!(
            parse_kdeglobals("[General]\nAccentColor=1,2,3,255\n"),
            Some(Rgb::new(1, 2, 3))
        );
    }

    #[test]
    fn gnome_names_map_to_libadwaita_colours() {
        let expected = [
            ("blue", 0x3584e4),
            ("teal", 0x2190a4),
            ("green", 0x3a944a),
            ("yellow", 0xc88800),
            ("orange", 0xed5b00),
            ("red", 0xe62d42),
            ("pink", 0xd56199),
            ("purple", 0x9141ac),
            ("slate", 0x6f8396),
        ];
        for (name, hex) in expected {
            let c = gnome_accent(name).unwrap();
            let got = (u32::from(c.red) << 16) | (u32::from(c.green) << 8) | u32::from(c.blue);
            assert_eq!(got, hex, "{name}");
        }
        // gsettings prints the value quoted.
        assert_eq!(gnome_accent("'green'\n"), gnome_accent("green"));
        assert_eq!(gnome_accent("'mauve'"), None);
    }

    #[test]
    fn portal_values_outside_0_1_are_unset() {
        assert_eq!(portal_color(0.0, 0.5, 1.0), Some(Rgb::new(0, 128, 255)));
        assert_eq!(portal_color(-0.1, 0.5, 0.5), None);
        assert_eq!(portal_color(0.5, 1.2, 0.5), None);
        assert_eq!(portal_color(0.5, 0.5, f64::NAN), None);
        assert_eq!(portal_color(f64::INFINITY, 0.5, 0.5), None);
    }

    #[test]
    fn accent_chain_order_and_gating() {
        let portal = Some(Rgb::new(1, 1, 1));
        let gnome = || Some(Rgb::new(2, 2, 2));
        let kde = || Some(Rgb::new(3, 3, 3));
        let win = || Some(Rgb::new(4, 4, 4));
        // The portal wins everywhere.
        assert_eq!(pick_accent("GNOME", portal, gnome, kde, win), portal);
        // GNOME without portal accent uses gsettings.
        assert_eq!(pick_accent("ubuntu:GNOME", None, gnome, kde, win), gnome());
        // Other desktops never read gsettings (it answers 'blue' by default).
        assert_eq!(
            pick_accent("sway", None, || panic!("gsettings read"), kde, win),
            win()
        );
        assert_eq!(
            pick_accent("KDE", None, || panic!("gsettings read"), kde, win),
            kde()
        );
        // A failing step falls through to the next.
        assert_eq!(pick_accent("KDE", None, gnome, || None, win), win());
        assert_eq!(pick_accent("sway", None, gnome, kde, || None), None);
    }

    #[test]
    fn desktop_names_are_matched_case_insensitively_in_the_list() {
        assert!(desktop_is("ubuntu:GNOME", "GNOME"));
        assert!(desktop_is("KDE", "kde"));
        assert!(!desktop_is("sway", "GNOME"));
        assert!(!desktop_is("", "KDE"));
    }
}
