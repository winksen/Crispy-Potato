//! Shared display-text rules for curated and collected input.
pub(crate) fn forbidden(c: char, multiline: bool) -> bool {
    (c.is_control() && !(multiline && matches!(c, '\t' | '\n' | '\r')))
        || matches!(c, '\u{00ad}' | '\u{061c}' | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}' | '\u{feff}' | '\u{e0000}'..='\u{e007f}')
}

fn leaf(field: &str) -> &str {
    field
        .rsplit('.')
        .next()
        .unwrap_or(field)
        .split('[')
        .next()
        .unwrap_or(field)
}

pub(crate) fn multiline(field: &str) -> bool {
    matches!(
        leaf(field),
        "summary" | "detail" | "note" | "reason" | "what" | "description"
    )
}

/// Limits count Unicode scalar values, matching JSON Schema maxLength.
/// A default also bounds newly added strings until given a narrower policy.
pub(crate) fn max_length(field: &str) -> usize {
    if field.contains("so_survey") || field.contains("SurveyName") {
        return 60;
    }
    match leaf(field) {
        "tagline" | "name" | "aliases" | "label" => 100,
        "choose_when" => 140,
        "title" => 200,
        "summary" => 4000,
        "detail" | "note" | "reason" | "what" | "description" => 2000,
        "website" | "docs" | "repo" | "url" | "source" | "sources" => 2048,
        _ => 256,
    }
}

pub(crate) fn problem(field: &str, value: &str) -> Option<String> {
    if let Some(c) = value.chars().find(|c| forbidden(*c, multiline(field))) {
        return Some(format!(
            "contains forbidden character U+{:04X}",
            u32::from(c)
        ));
    }
    let length = value.chars().count();
    let max = max_length(field);
    (length > max).then(|| format!("is {length} characters; the limit is {max}"))
}
