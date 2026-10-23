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
    value
        .chars()
        .filter(|c| !crate::text_policy::forbidden(*c, false) && !matches!(*c, '<' | '>'))
        .take(256)
        .collect()
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

pub(crate) fn observation_date(value: &str) -> bool {
    let today = model::current_date();
    observation_date_at(value, today.next_day().unwrap_or(today))
}

fn observation_date_at(value: &str, tomorrow: model::PartialDate) -> bool {
    date(value)
        && value >= "2008-01-01"
        && value
            .parse::<model::PartialDate>()
            .is_ok_and(|date| date <= tomorrow)
}

fn release_date(value: &str, latest_year: u16) -> bool {
    date(value)
        && value
            .parse::<model::PartialDate>()
            .is_ok_and(|date| (1940..=latest_year).contains(&date.year))
}

fn stored_text(value: &str) -> Result<String, &'static str> {
    if value.chars().count() > 256 {
        return Err("source string exceeds 256 characters");
    }
    Ok(text(value))
}

fn json(value: &mut Value, depth: usize, budget: &mut usize) -> Result<(), &'static str> {
    if depth > 16 || *budget == 0 {
        return Err("fact exceeds structural limits");
    }
    *budget -= 1;
    match value {
        Value::String(s) => *s = stored_text(s)?,
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
        release.version = stored_text(&release.version)?;
        if release.version.is_empty()
            || release.released_at.as_ref().is_some_and(|v| !release_date(v, model::current_year().saturating_add(1)))
            // Announced end-of-support dates legitimately extend beyond next year.
            || release.eol_at.as_ref().is_some_and(|v| !release_date(v, 9999))
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
    fn observation_dates_use_exact_utc_day_bounds() {
        let tomorrow = "2027-01-01".parse().unwrap();
        for date in ["2008-01-01", "2026-12-31", "2027-01-01"] {
            assert!(observation_date_at(date, tomorrow));
        }
        for date in ["2007-12-31", "2027-01-02", "2024-02-30", "2026"] {
            assert!(!observation_date_at(date, tomorrow));
        }
    }

    #[test]
    fn release_dates_and_storage_strings_are_refused_without_truncation() {
        let mut batch = Batch {
            releases: vec![crate::ingest::Release {
                version: "é".repeat(256),
                released_at: Some("1940-01-01".into()),
                eol_at: Some("2050-12-31".into()),
                lts: false,
            }],
            ..Batch::default()
        };
        assert!(clean(&mut batch).is_ok());
        for date in [
            "1939-12-31".into(),
            format!("{}-01-01", model::current_year() + 2),
        ] {
            batch.releases[0].released_at = Some(date);
            assert!(clean(&mut batch).is_err());
        }
        batch.releases[0].released_at = Some(format!("{}-12-31", model::current_year() + 1));
        assert!(clean(&mut batch).is_ok());
        let long = format!("{}a", "é".repeat(256));
        batch.releases[0].version = long.clone();
        assert!(clean(&mut batch).is_err());
        assert_eq!(batch.releases[0].version, long);
        batch.releases.clear();
        batch.facts.push(crate::ingest::Fact {
            field: "developer".into(),
            value: serde_json::json!({"names": ["é".repeat(256)]}),
        });
        assert!(clean(&mut batch).is_ok());
        batch.facts[0].value = serde_json::json!({"names": [long]});
        assert!(clean(&mut batch).is_err());
        assert_eq!(batch.facts[0].value["names"][0], long);
    }
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
