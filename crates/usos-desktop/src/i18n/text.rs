//! Text helpers shared by every screen (SYSTEM.md section 14, NOTES A8 and B19): non-breaking
//! spaces, Polish plural forms, one fraction format and one capitalisation rule.

use super::Lang;

/// U+00A0 NO-BREAK SPACE.
pub const NBSP: char = '\u{a0}';

/// Replace the space after a one-letter word ("w", "i", "z", "o", "u", "a") and after "s."
/// with a no-break space, so a line never ends on a dangling letter. Other whitespace is kept.
pub fn nbsp(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for ch in text.chars() {
        if ch == ' ' {
            let letter = {
                let mut chars = word.chars();
                matches!((chars.next(), chars.next()), (Some(c), None) if c.is_alphabetic())
            };
            out.push(if letter || word == "s." { NBSP } else { ' ' });
            word.clear();
        } else if ch.is_whitespace() {
            out.push(ch);
            word.clear();
        } else {
            out.push(ch);
            word.push(ch);
        }
    }
    out
}

/// Which Polish plural form a count takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralForm {
    /// 1 ankieta
    One,
    /// 2-4, 22-24 (not 12-14): 3 ankiety
    Few,
    /// everything else: 5 ankiet, 0 ankiet, 12 ankiet
    Many,
}

/// The CLDR plural category of `n` in Polish (integers only).
pub fn plural_pl(n: u64) -> PluralForm {
    if n == 1 {
        PluralForm::One
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        PluralForm::Few
    } else {
        PluralForm::Many
    }
}

/// A counted noun in both languages: Polish `one / few / many`, English `one / other`.
#[derive(Debug, Clone, Copy)]
pub struct Counted {
    pub pl: [&'static str; 3],
    pub en: [&'static str; 2],
}

impl Counted {
    pub const fn new(pl: [&'static str; 3], en: [&'static str; 2]) -> Self {
        Self { pl, en }
    }

    /// The noun alone, without the number.
    pub fn noun(&self, n: u64, lang: Lang) -> &'static str {
        match lang {
            Lang::Pl => match plural_pl(n) {
                PluralForm::One => self.pl[0],
                PluralForm::Few => self.pl[1],
                PluralForm::Many => self.pl[2],
            },
            Lang::En => self.en[usize::from(n != 1)],
        }
    }

    /// "5 ankiet" / "1 survey".
    pub fn format(&self, n: u64, lang: Lang) -> String {
        format!("{n}{NBSP}{}", self.noun(n, lang))
    }
}

/// A number with at most `max_decimals` decimals, trailing zeros dropped, decimal comma in
/// Polish and point in English: `4.5 -> "4,5"`, `30.0 -> "30"`.
pub fn number(value: f64, max_decimals: usize, lang: Lang) -> String {
    let mut text = format!("{value:.max_decimals$}");
    if text.contains('.') {
        text.truncate(text.trim_end_matches('0').trim_end_matches('.').len());
    }
    if text == "-0" {
        text = "0".to_owned();
    }
    match lang {
        Lang::Pl => text.replace('.', ","),
        Lang::En => text,
    }
}

/// A value with a fixed number of decimals ("4,27"), for averages.
pub fn fixed(value: f64, decimals: usize, lang: Lang) -> String {
    let text = format!("{value:.decimals$}");
    match lang {
        Lang::Pl => text.replace('.', ","),
        Lang::En => text,
    }
}

/// The one fraction format: "23,5 / 30" (points over maximum).
pub fn fraction(value: f64, max: f64, lang: Lang) -> String {
    format!("{} / {}", number(value, 2, lang), number(max, 2, lang))
}

/// The one capitalisation rule: sentence case. Only the first letter is raised; names that come
/// from USOS are never passed through here.
pub fn sentence_case(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nbsp_after_one_letter_words() {
        assert_eq!(nbsp("Dane w pamięci"), "Dane w\u{a0}pamięci");
        assert_eq!(nbsp("a i o u z"), "a\u{a0}i\u{a0}o\u{a0}u\u{a0}z");
        assert_eq!(nbsp("Sala s. 314"), "Sala s.\u{a0}314");
        // Longer words and punctuation are left alone.
        assert_eq!(nbsp("na dole, po co"), "na dole, po co");
        assert_eq!(nbsp("W:"), "W:");
    }

    #[test]
    fn polish_plural_forms() {
        use PluralForm::*;
        let cases = [
            (0, Many),
            (1, One),
            (2, Few),
            (4, Few),
            (5, Many),
            (11, Many),
            (12, Many),
            (14, Many),
            (21, Many),
            (22, Few),
            (24, Few),
            (25, Many),
            (112, Many),
            (122, Few),
        ];
        for (n, form) in cases {
            assert_eq!(plural_pl(n), form, "n = {n}");
        }
    }

    #[test]
    fn counted_nouns() {
        const SURVEYS: Counted =
            Counted::new(["ankieta", "ankiety", "ankiet"], ["survey", "surveys"]);
        assert_eq!(SURVEYS.format(1, Lang::Pl), "1\u{a0}ankieta");
        assert_eq!(SURVEYS.format(3, Lang::Pl), "3\u{a0}ankiety");
        assert_eq!(SURVEYS.format(5, Lang::Pl), "5\u{a0}ankiet");
        assert_eq!(SURVEYS.format(1, Lang::En), "1\u{a0}survey");
        assert_eq!(SURVEYS.format(0, Lang::En), "0\u{a0}surveys");
    }

    #[test]
    fn numbers_and_fractions() {
        assert_eq!(number(4.5, 2, Lang::Pl), "4,5");
        assert_eq!(number(4.5, 2, Lang::En), "4.5");
        assert_eq!(number(30.0, 2, Lang::Pl), "30");
        assert_eq!(number(-0.0, 1, Lang::Pl), "0");
        assert_eq!(fixed(4.27, 2, Lang::Pl), "4,27");
        assert_eq!(fixed(4.0, 2, Lang::En), "4.00");
        assert_eq!(fraction(23.5, 30.0, Lang::Pl), "23,5 / 30");
        assert_eq!(fraction(18.5, 25.0, Lang::En), "18.5 / 25");
    }

    #[test]
    fn sentence_case_raises_only_the_first_letter() {
        assert_eq!(sentence_case("klucz wbudowany"), "Klucz wbudowany");
        assert_eq!(sentence_case("łączymy USOS"), "Łączymy USOS");
        assert_eq!(sentence_case(""), "");
    }
}
