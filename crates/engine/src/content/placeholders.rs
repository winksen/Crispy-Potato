//! Locate unfinished scaffold values without changing their diagnostics.
/// The text that marks a value still to be written.
pub const PLACEHOLDER: &str = "TODO";

pub fn find(source: &str, options: serde_saphyr::Options) -> Vec<String> {
    super::yaml_strings::strings(source, options)
        .into_iter()
        .filter_map(|(field, value)| value.contains(PLACEHOLDER).then_some(field))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(source: &str) -> Vec<String> {
        find(source, serde_saphyr::Options::default())
    }

    #[test]
    fn finds_placeholders_in_document_order() {
        let source = "\
name: TODO
tagline: Done.
so_survey:
  2022: TODO.js
  2023: Done
strengths:
  - title: Done
    detail: TODO
summary: >
  TODO
free: true
";
        assert_eq!(
            fields(source),
            ["name", "so_survey.2022", "strengths[0].detail", "summary"]
        );
    }

    #[test]
    fn comments_and_invalid_yaml_have_no_placeholders() {
        assert_eq!(
            fields("# TODO: none here\nname: Done\n"),
            Vec::<String>::new()
        );
        assert_eq!(fields("name: [TODO\n"), Vec::<String>::new());
    }
}
