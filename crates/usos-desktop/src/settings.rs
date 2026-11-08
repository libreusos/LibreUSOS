//! Desktop-only preferences, kept in a small TOML file under the platform config directory
//! (`<config dir>/libreusos/desktop.toml`, next to the core's `config.toml`).

use std::path::{Path, PathBuf};

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

/// In-app light/dark choice (Wygląd > Motyw).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Follow the system (default).
    #[default]
    System,
    Light,
    Dark,
}

/// In-app accent choice (Wygląd > Kolor akcentu).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentChoice {
    /// Take the desktop's accent (default).
    #[default]
    System,
    /// The LibreUSOS brand colour.
    Brand,
    /// `Prefs::custom_color`.
    Custom,
}

/// UI language choice. `System` follows the locale.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageChoice {
    #[default]
    System,
    Pl,
    En,
}

/// A setting that follows the system unless forced on or off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Toggle {
    #[default]
    System,
    On,
    Off,
}

impl Toggle {
    /// The effective value given the system's.
    pub fn resolve(self, system: bool) -> bool {
        match self {
            Toggle::System => system,
            Toggle::On => true,
            Toggle::Off => false,
        }
    }
}

/// Every desktop-only preference. Missing keys take their defaults, so an older or hand-edited
/// file never fails to load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub theme_mode: ThemeMode,
    pub accent: AccentChoice,
    /// `#RRGGBB`, used when `accent` is `custom`.
    pub custom_color: String,
    pub language: LanguageChoice,
    /// Wygląd > Dostępność > Ogranicz ruch.
    pub reduced_motion: Toggle,
    /// Wygląd > Dostępność > Wysoki kontrast.
    pub high_contrast: Toggle,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            accent: AccentChoice::System,
            custom_color: "#3F5AA6".to_owned(),
            language: LanguageChoice::System,
            reduced_motion: Toggle::System,
            high_contrast: Toggle::System,
        }
    }
}

impl Prefs {
    /// `<config dir>/libreusos/desktop.toml`.
    pub fn default_path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("libreusos").join("desktop.toml"))
    }

    /// Read `path`. A missing, unreadable or malformed file gives the defaults (the file is left
    /// alone until the next [`Prefs::save`]).
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write `path` atomically (temporary file, then rename), creating the directory.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }
}

/// The preferences live in a global so every view reads the same copy.
struct PrefsStore {
    prefs: Prefs,
    path: Option<PathBuf>,
}

impl Global for PrefsStore {}

impl Prefs {
    /// Load the file at the default path into the global. Call once at start-up.
    pub fn init(cx: &mut App) {
        let path = Self::default_path();
        let prefs = path.as_deref().map(Self::load).unwrap_or_default();
        cx.set_global(PrefsStore { prefs, path });
    }

    /// The current preferences.
    pub fn global(cx: &App) -> &Prefs {
        &cx.global::<PrefsStore>().prefs
    }

    /// Change the preferences: save them, then re-run the theme pipeline and the language
    /// choice so open windows repaint. A failed write is logged, not fatal.
    pub fn update(cx: &mut App, change: impl FnOnce(&mut Prefs)) {
        let store = cx.global_mut::<PrefsStore>();
        let before = store.prefs.clone();
        change(&mut store.prefs);
        if store.prefs == before {
            return;
        }
        if let Some(path) = &store.path
            && let Err(e) = store.prefs.save(path)
        {
            eprintln!("could not save preferences to {}: {e}", path.display());
        }
        crate::i18n::refresh(cx);
        crate::theme::refresh(cx);
        cx.refresh_windows();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("desktop.toml");
        let prefs = Prefs {
            theme_mode: ThemeMode::Dark,
            accent: AccentChoice::Custom,
            custom_color: "#557F3E".into(),
            language: LanguageChoice::En,
            reduced_motion: Toggle::On,
            high_contrast: Toggle::Off,
        };
        prefs.save(&path).unwrap();
        assert_eq!(Prefs::load(&path), prefs);
        // Saving again replaces the file.
        let mut changed = prefs.clone();
        changed.theme_mode = ThemeMode::Light;
        changed.save(&path).unwrap();
        assert_eq!(Prefs::load(&path), changed);
    }

    #[test]
    fn missing_and_broken_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Prefs::load(&dir.path().join("none.toml")), Prefs::default());
        let bad = dir.path().join("bad.toml");
        std::fs::write(&bad, "theme_mode = [").unwrap();
        assert_eq!(Prefs::load(&bad), Prefs::default());
        let wrong_value = dir.path().join("wrong.toml");
        std::fs::write(&wrong_value, "theme_mode = \"purple\"\n").unwrap();
        assert_eq!(Prefs::load(&wrong_value), Prefs::default());
    }

    #[test]
    fn partial_files_keep_defaults_for_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.toml");
        std::fs::write(&path, "language = \"pl\"\nsome_future_key = 3\n").unwrap();
        let prefs = Prefs::load(&path);
        assert_eq!(prefs.language, LanguageChoice::Pl);
        assert_eq!(prefs.theme_mode, ThemeMode::System);
    }

    #[test]
    fn toggle_resolution() {
        assert!(Toggle::System.resolve(true));
        assert!(!Toggle::System.resolve(false));
        assert!(Toggle::On.resolve(false));
        assert!(!Toggle::Off.resolve(true));
    }
}
