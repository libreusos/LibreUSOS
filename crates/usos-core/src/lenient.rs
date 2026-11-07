//! Lenient serde helpers shared by the response parsers: a field of the wrong type becomes
//! `None` / the default instead of failing the whole object.

use chrono::{NaiveDate, NaiveDateTime};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A value of the wrong type becomes `None` instead of failing the whole object.
pub(crate) fn lenient<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(Value::deserialize(d)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok()))
}

pub(crate) fn lenient_or_default<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    Ok(lenient(d)?.unwrap_or_default())
}

fn scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// A string or number as `Some(String)`; `null`, `""` and anything else as `None`.
pub(crate) fn opt_string<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Ok(Value::deserialize(d)
        .ok()
        .as_ref()
        .and_then(scalar_to_string))
}

pub(crate) fn string_list<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(d) {
        Ok(Value::Array(items)) => items.iter().filter_map(scalar_to_string).collect(),
        _ => Vec::new(),
    })
}

/// A datetime (`2026-01-20 10:15:00`) or a bare date; anything else is `None`.
pub(crate) fn opt_datetime<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<NaiveDateTime>, D::Error> {
    Ok(opt_string(d)?.and_then(|s| {
        let s = s.trim();
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .or_else(|| {
                s.parse::<NaiveDate>()
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            })
    }))
}

/// A finite number, or a numeric string (`"7,5"`); anything else is `None`.
pub(crate) fn opt_f64<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<f64>, D::Error> {
    Ok(match Value::deserialize(d) {
        Ok(Value::Number(n)) => n.as_f64(),
        Ok(Value::String(s)) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Deserialize)]
    struct F {
        #[serde(default, deserialize_with = "opt_f64")]
        v: Option<f64>,
    }

    fn parse(v: Value) -> Option<f64> {
        serde_json::from_value::<F>(json!({ "v": v })).unwrap().v
    }

    #[test]
    fn opt_f64_is_lenient() {
        assert_eq!(parse(json!("7,5")), Some(7.5));
        assert_eq!(parse(json!(" 3 ")), Some(3.0));
        assert_eq!(parse(json!(2.5)), Some(2.5));
        assert_eq!(parse(json!("abc")), None);
        assert_eq!(parse(json!("NaN")), None);
        assert_eq!(parse(json!("inf")), None);
        assert_eq!(parse(json!(null)), None);
        assert_eq!(parse(json!([1])), None);
    }
}
