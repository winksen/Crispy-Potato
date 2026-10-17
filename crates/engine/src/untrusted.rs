//! Typed response parsing and a mandatory sanitization boundary before storage.
use crate::ingest::Batch;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.len() > crate::http::MAX_BODY {
        return Err("response too large");
    }
    // serde_json's recursion limit remains enabled; never deserialize arbitrary
    // source responses through an unbounded parser or include them in errors.
    serde_json::from_slice(bytes).map_err(|_| "invalid or incomplete source response")
}

pub fn text(value: &str) -> String {
    value.chars().filter(|c| !c.is_control() && !matches!(*c, '<' | '>' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')).take(256).collect()
}

fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn date(value: &str) -> bool {
    value.len() == 10 && value.parse::<model::PartialDate>().is_ok()
}

fn json(value: &mut Value, depth: usize, budget: &mut usize) -> Result<(), &'static str> {
    if depth > 16 || *budget == 0 {
        return Err("fact exceeds structural limits");
    }
    *budget -= 1;
    match value {
        Value::String(s) => *s = text(s),
        Value::Number(n) if n.as_f64().is_none_or(|v| !v.is_finite() || v < 0.0) => {
            return Err("invalid fact number");
        }
        Value::Array(values) => {
            for value in values {
                json(value, depth + 1, budget)?;
            }
        }
        Value::Object(values) => {
            for (name, value) in values {
                if !key(name) {
                    return Err("invalid fact key");
                }
                json(value, depth + 1, budget)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn clean(batch: &mut Batch) -> Result<(), &'static str> {
    if batch.observations.len() + batch.releases.len() + batch.facts.len() > 10_000 {
        return Err("too many records");
    }
    for o in &batch.observations {
        if !o.value.is_finite() {
            return Err("invalid observation number");
        }
        if !key(&o.metric)
            || !matches!(o.unit.as_str(), "count" | "pct")
            || !matches!(o.period.as_str(), "instant" | "week" | "month" | "year")
            || !date(&o.observed_at)
        {
            return Err("invalid observation fields");
        }
    }
    for release in &mut batch.releases {
        release.version = text(&release.version);
        if release.version.is_empty()
            || release.released_at.as_ref().is_some_and(|v| !date(v))
            || release.eol_at.as_ref().is_some_and(|v| !date(v))
        {
            return Err("invalid release fields");
        }
    }
    let mut budget = 10_000;
    for fact in &mut batch.facts {
        if !key(&fact.field) {
            return Err("invalid fact field");
        }
        json(&mut fact.value, 0, &mut budget)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostile_text_is_bounded_plain_text() {
        let value = format!("<script>\u{202e}\n{}", "é".repeat(100_000));
        let cleaned = text(&value);
        assert_eq!(cleaned.chars().count(), 256);
        assert!(!cleaned.contains(['<', '>', '\n', '\u{202e}']));
    }
    #[test]
    fn typed_parser_rejects_missing_fields_nan_and_deep_nesting() {
        assert!(decode::<crate::observations::Observation>(br#"{"value":1}"#).is_err());
        assert!(decode::<Value>(b"NaN").is_err());
        let nested = format!("{}0{}", "[".repeat(200), "]".repeat(200));
        assert!(decode::<Value>(nested.as_bytes()).is_err());
    }
    #[test]
    fn rejects_nonfinite_numbers_before_quarantine() {
        let mut batch = Batch::default();
        batch.observations.push(crate::observations::Observation {
            entity_id: "rust".into(),
            source_id: "test".into(),
            metric: "test.count".into(),
            value: 0.0,
            unit: "pct".into(),
            period: "instant".into(),
            observed_at: "2026-10-17".into(),
        });
        for bad in [f64::NAN, f64::INFINITY] {
            batch.observations[0].value = bad;
            assert!(clean(&mut batch).is_err());
        }
        batch.observations[0].value = 100.0;
        assert!(clean(&mut batch).is_ok());
    }
}
