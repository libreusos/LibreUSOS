//! Polish and English UI text. Every string is a [`Text`] holding both languages, so a string
//! cannot exist in only one. Shared strings live in [`strings`]; a screen keeps its own `Text`
//! constants next to its view and reads them with `cx.t(&TEXT)`.
//!
//! The language comes from the system locale unless the user forces one (Ustawienia > Język).

pub mod strings;
pub mod text;

use gpui::{App, Global, SharedString};

use crate::settings::{LanguageChoice, Prefs};

pub use text::{Counted, NBSP, fixed, fraction, nbsp, number, sentence_case};

/// The two UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Pl,
    En,
}

impl Lang {
    /// The language of a locale name such as `pl_PL.UTF-8`, `en-GB` or `C`. `None` for the
    /// neutral `C`/`POSIX` locales, which say nothing about the user.
    pub fn from_locale(locale: &str) -> Option<Lang> {
        let name = locale.trim();
        if name.is_empty() || matches!(name, "C" | "POSIX") || name.starts_with("C.") {
            return None;
        }
        let code = name.split(['_', '-', '.', '@']).next().unwrap_or(name);
        Some(if code.eq_ignore_ascii_case("pl") {
            Lang::Pl
        } else {
            Lang::En
        })
    }

    /// The first usable locale among `vars` (looked up in order), else Polish: the app targets
    /// Polish universities, so an unreadable locale is not a reason to switch language.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Lang {
        ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
            .iter()
            .filter_map(|var| get(var))
            // LANGUAGE is a colon-separated priority list.
            .flat_map(|value| value.split(':').map(str::to_owned).collect::<Vec<_>>())
            .find_map(|value| Lang::from_locale(&value))
            .unwrap_or_else(system_fallback)
    }
}

#[cfg(windows)]
fn system_fallback() -> Lang {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    // SAFETY: the buffer outlives the call and its length is passed along.
    let len = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if len > 1 {
        let name = String::from_utf16_lossy(&buffer[..len as usize - 1]);
        if let Some(lang) = Lang::from_locale(&name) {
            return lang;
        }
    }
    Lang::Pl
}

#[cfg(not(windows))]
fn system_fallback() -> Lang {
    Lang::Pl
}

/// The language the system asks for.
pub fn system_lang() -> Lang {
    Lang::from_env(|name| std::env::var(name).ok())
}

/// One UI string in both languages.
#[derive(Debug, Clone, Copy)]
pub struct Text {
    pub pl: &'static str,
    pub en: &'static str,
}

impl Text {
    pub const fn new(pl: &'static str, en: &'static str) -> Self {
        Self { pl, en }
    }

    /// The text ready to show: Polish gets no-break spaces after one-letter words.
    pub fn render(&self, lang: Lang) -> SharedString {
        match lang {
            Lang::Pl => nbsp(self.pl).into(),
            Lang::En => self.en.into(),
        }
    }
}

struct Locale(Lang);

impl Global for Locale {}

/// Set the language from the saved preference. Call once after `Prefs::init`.
pub fn init(cx: &mut App) {
    let lang = choose(Prefs::global(cx).language);
    cx.set_global(Locale(lang));
}

/// Re-read the preference (after it changed).
pub fn refresh(cx: &mut App) {
    let lang = choose(Prefs::global(cx).language);
    cx.set_global(Locale(lang));
}

fn choose(choice: LanguageChoice) -> Lang {
    match choice {
        LanguageChoice::System => system_lang(),
        LanguageChoice::Pl => Lang::Pl,
        LanguageChoice::En => Lang::En,
    }
}

/// Translation helpers on `App` (and `Context<T>` through deref).
pub trait I18nExt {
    fn lang(&self) -> Lang;
    /// A string ready to show.
    fn t(&self, text: &Text) -> SharedString {
        text.render(self.lang())
    }
    /// A counted noun with its number: "5 ankiet".
    fn count(&self, noun: &Counted, n: u64) -> String {
        noun.format(n, self.lang())
    }
}

impl I18nExt for App {
    fn lang(&self) -> Lang {
        self.global::<Locale>().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_names() {
        assert_eq!(Lang::from_locale("pl_PL.UTF-8"), Some(Lang::Pl));
        assert_eq!(Lang::from_locale("pl"), Some(Lang::Pl));
        assert_eq!(Lang::from_locale("PL-pl"), Some(Lang::Pl));
        assert_eq!(Lang::from_locale("en_GB.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_locale("de_DE"), Some(Lang::En));
        assert_eq!(Lang::from_locale("C"), None);
        assert_eq!(Lang::from_locale("C.UTF-8"), None);
        assert_eq!(Lang::from_locale("POSIX"), None);
        assert_eq!(Lang::from_locale(""), None);
    }

    #[test]
    fn environment_precedence() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        // LC_ALL beats LANG.
        assert_eq!(
            Lang::from_env(env(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "pl_PL.UTF-8")])),
            Lang::En
        );
        // A neutral LC_ALL falls through to LANG.
        assert_eq!(
            Lang::from_env(env(&[("LC_ALL", "C"), ("LANG", "pl_PL.UTF-8")])),
            Lang::Pl
        );
        // LANGUAGE is a priority list.
        assert_eq!(
            Lang::from_env(env(&[("LANGUAGE", "en:pl"), ("LANG", "pl_PL")])),
            Lang::En
        );
        // Nothing set: Polish.
        assert_eq!(Lang::from_env(env(&[])), Lang::Pl);
    }

    #[test]
    fn text_picks_the_language() {
        const T: Text = Text::new("Zaloguj się w aplikacji", "Sign in");
        assert_eq!(T.render(Lang::En).as_ref(), "Sign in");
        assert_eq!(T.render(Lang::Pl).as_ref(), "Zaloguj się w\u{a0}aplikacji");
    }

    #[test]
    fn language_choice_overrides_the_system() {
        assert_eq!(choose(LanguageChoice::Pl), Lang::Pl);
        assert_eq!(choose(LanguageChoice::En), Lang::En);
    }
}
