//! Small pure helpers the Oceny, Sprawdziany, Ustawienia and palette view-models share: the
//! language bridge to `usos-core`, USOS names, short dates and search folding.

use chrono::{Datelike, NaiveDate};
use usos_core::LangDict;

use crate::i18n::Lang;

/// Today's date. Debug builds honour `LIBREUSOS_TODAY=YYYY-MM-DD`, so the demo screenshots
/// show the same day the mock-ups do.
pub fn today() -> NaiveDate {
    #[cfg(debug_assertions)]
    if let Some(day) = std::env::var("LIBREUSOS_TODAY")
        .ok()
        .and_then(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok())
    {
        return day;
    }
    chrono::Local::now().date_naive()
}

/// The `usos-core` language for a UI language.
pub fn core_lang(lang: Lang) -> usos_core::Lang {
    match lang {
        Lang::Pl => usos_core::Lang::Pl,
        Lang::En => usos_core::Lang::En,
    }
}

/// A USOS name in `lang` (falling back to the other language); empty when USOS gave none. Names
/// from USOS are shown exactly as given.
pub fn pick(dict: &LangDict, lang: Lang) -> String {
    dict.get(core_lang(lang))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// "sty", "lut", ... / "Jan", "Feb", ...
pub fn month_abbr(month: u32, lang: Lang) -> &'static str {
    const PL: [&str; 12] = [
        "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
    ];
    const EN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let index = (month.clamp(1, 12) - 1) as usize;
    match lang {
        Lang::Pl => PL[index],
        Lang::En => EN[index],
    }
}

/// "16 cze 2026" / "16 Jun 2026".
pub fn short_date(date: NaiveDate, lang: Lang) -> String {
    format!(
        "{} {} {}",
        date.day(),
        month_abbr(date.month(), lang),
        date.year()
    )
}

/// "5 lis" / "5 Nov" (the year is left out for the "last changed" columns).
pub fn day_month(date: NaiveDate, lang: Lang) -> String {
    format!("{} {}", date.day(), month_abbr(date.month(), lang))
}

/// Roman numerals for small positive numbers ("I termin"); larger ones stay Arabic.
pub fn roman(n: u32) -> String {
    const NUMERALS: [&str; 10] = ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"];
    match n {
        1..=9 => NUMERALS[n as usize].to_owned(),
        _ => n.to_string(),
    }
}

/// Lowercase with Polish (and common Latin) diacritics removed, so that "lacznie" finds "Łącznie".
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'ą' | 'ä' | 'á' | 'à' | 'â' => 'a',
            'ć' | 'č' | 'ç' => 'c',
            'ę' | 'é' | 'è' | 'ê' | 'ë' => 'e',
            'ł' => 'l',
            'ń' | 'ñ' => 'n',
            'ó' | 'ö' | 'ô' | 'ò' => 'o',
            'ś' | 'š' => 's',
            'ź' | 'ż' | 'ž' => 'z',
            'ü' | 'ú' | 'ù' | 'û' => 'u',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn dates_use_polish_and_english_months() {
        assert_eq!(short_date(day(2026, 6, 16), Lang::Pl), "16 cze 2026");
        assert_eq!(short_date(day(2026, 6, 16), Lang::En), "16 Jun 2026");
        assert_eq!(day_month(day(2026, 10, 13), Lang::Pl), "13 paź");
        assert_eq!(day_month(day(2026, 11, 5), Lang::Pl), "5 lis");
        assert_eq!(day_month(day(2026, 11, 5), Lang::En), "5 Nov");
    }

    #[test]
    fn every_month_has_a_name() {
        for month in 1..=12 {
            assert!(!month_abbr(month, Lang::Pl).is_empty());
            assert!(!month_abbr(month, Lang::En).is_empty());
        }
        // Out-of-range input never panics.
        assert_eq!(month_abbr(0, Lang::En), "Jan");
        assert_eq!(month_abbr(13, Lang::En), "Dec");
    }

    #[test]
    fn roman_numerals_for_exam_sessions() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(2), "II");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(10), "10");
        assert_eq!(roman(0), "0");
    }

    #[test]
    fn folding_removes_polish_diacritics() {
        assert_eq!(fold("Łącznie Źdźbło ŻÓŁĆ"), "lacznie zdzblo zolc");
        assert_eq!(fold("Sprawdziany"), "sprawdziany");
        assert_eq!(fold("Płatności"), "platnosci");
        assert_eq!(fold(""), "");
    }

    #[test]
    fn names_fall_back_to_the_other_language() {
        let only_pl = LangDict {
            pl: Some("  Fizyka ".into()),
            en: None,
        };
        assert_eq!(pick(&only_pl, Lang::En), "Fizyka");
        assert_eq!(pick(&LangDict::default(), Lang::Pl), "");
    }
}
