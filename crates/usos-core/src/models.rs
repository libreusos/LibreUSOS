//! Shared data types.

use serde::{Deserialize, Serialize};

/// UI / data language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Pl,
    En,
}

/// USOS `LangDict`: `{"pl": "...", "en": "..."}`; either side may be missing or empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangDict {
    #[serde(default)]
    pub pl: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
}

impl LangDict {
    /// The text in `lang`, falling back to the other language when it is missing or empty.
    pub fn get(&self, lang: Lang) -> Option<&str> {
        let (primary, fallback) = match lang {
            Lang::Pl => (&self.pl, &self.en),
            Lang::En => (&self.en, &self.pl),
        };
        [primary, fallback]
            .into_iter()
            .filter_map(|v| v.as_deref())
            .find(|s| !s.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_other_language() {
        let d: LangDict = serde_json::from_str(r#"{"pl":"Matematyka","en":""}"#).unwrap();
        assert_eq!(d.get(Lang::Pl), Some("Matematyka"));
        assert_eq!(d.get(Lang::En), Some("Matematyka"));
        let e: LangDict = serde_json::from_str(r#"{"pl":null,"en":"Maths"}"#).unwrap();
        assert_eq!(e.get(Lang::Pl), Some("Maths"));
        assert_eq!(LangDict::default().get(Lang::En), None);
    }
}
