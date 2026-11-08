//! Fuzzy matching for the palette: case and Polish diacritics are ignored, so "lacznie" finds
//! "Łącznie", and the letters of the query only have to appear in order ("spz" finds
//! "Sprawdziany").

use crate::view_util::fold;

/// How well `query` matches `text`; `None` when it does not. Higher is better. An empty query
/// matches everything equally.
pub fn score(query: &str, text: &str) -> Option<i32> {
    let query: Vec<char> = fold(query).chars().filter(|c| !c.is_whitespace()).collect();
    if query.is_empty() {
        return Some(0);
    }
    let text: Vec<char> = fold(text).chars().collect();

    // A run of the whole query beats scattered letters; the earlier and the closer to a word
    // start, the better.
    let run = text
        .windows(query.len())
        .position(|window| window == query.as_slice());
    if let Some(at) = run {
        let word_start = at == 0 || !text[at - 1].is_alphanumeric();
        return Some(1000 - at as i32 + if word_start { 200 } else { 0 });
    }

    // Scattered letters: every gap costs, a letter that starts a word is worth more.
    let mut score = 500;
    let mut from = 0;
    let mut previous: Option<usize> = None;
    for letter in query {
        let at = from + text[from..].iter().position(|&c| c == letter)?;
        if at == 0 || !text[at - 1].is_alphanumeric() {
            score += 20;
        }
        if let Some(previous) = previous {
            score -= (at - previous - 1) as i32;
        }
        previous = Some(at);
        from = at + 1;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_matches_everything() {
        assert_eq!(score("", "Oceny"), Some(0));
        assert_eq!(score("  ", "Oceny"), Some(0));
    }

    #[test]
    fn case_and_diacritics_do_not_matter() {
        assert!(score("lacznie", "Łącznie").is_some());
        assert!(score("ŁĄCZ", "łącznie").is_some());
        assert!(score("zolc", "Żółć").is_some());
        assert!(score("dzis", "Dziś").is_some());
    }

    #[test]
    fn a_missing_letter_or_a_wrong_order_does_not_match() {
        assert_eq!(score("x", "Oceny"), None);
        assert_eq!(score("ycn", "Oceny"), None);
        assert_eq!(score("oo", "Oceny"), None);
    }

    #[test]
    fn letters_may_be_scattered() {
        assert!(score("spz", "Sprawdziany").is_some());
        assert!(score("zmk", "Zmień uczelnię lub konto").is_some());
    }

    #[test]
    fn a_whole_run_beats_scattered_letters() {
        assert!(score("spr", "Sprawdziany") > score("spy", "Sprawdziany"));
    }

    #[test]
    fn the_start_of_the_text_beats_the_middle() {
        assert!(score("oce", "Oceny") > score("cen", "Oceny"));
    }

    #[test]
    fn a_word_start_beats_the_middle_of_a_word() {
        assert!(score("kon", "Zmień konto") > score("kon", "Pokonaj"));
    }

    #[test]
    fn spaces_in_the_query_are_ignored() {
        assert_eq!(score("sync now", "Syncnow"), score("syncnow", "Syncnow"));
    }
}
