//! Finds the `TODO` placeholders `cli new` writes, so a file still being
//! written is reported field by field instead of failing on its types.

use std::fmt;

use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// The text that marks a value still to be written.
pub const PLACEHOLDER: &str = "TODO";

/// The fields whose value holds [`PLACEHOLDER`], in the order they appear,
/// such as `strengths[0].title`. Empty if `source` is not valid YAML; the full
/// parse reports that.
pub fn find(source: &str, options: serde_saphyr::Options) -> Vec<String> {
    let mut found = Vec::new();
    let walk = Walk {
        field: String::new(),
        found: &mut found,
    };
    let walked = serde_saphyr::with_deserializer_from_str_with_options(source, options, |de| {
        walk.deserialize(de)
    });
    if walked.is_err() {
        return Vec::new();
    }
    found
}

/// Visits any YAML value and records the fields of strings holding a placeholder.
struct Walk<'a> {
    field: String,
    found: &'a mut Vec<String>,
}

impl<'de> DeserializeSeed<'de> for Walk<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Walk<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any YAML value")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<(), E> {
        if value.contains(PLACEHOLDER) {
            self.found.push(self.field);
        }
        Ok(())
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let Walk { field, found } = self;
        let mut index = 0;
        while seq
            .next_element_seed(Walk {
                field: format!("{field}[{index}]"),
                found: &mut *found,
            })?
            .is_some()
        {
            index += 1;
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let Walk { field, found } = self;
        while let Some(Key(key)) = map.next_key()? {
            let field = if field.is_empty() {
                key
            } else {
                format!("{field}.{key}")
            };
            map.next_value_seed(Walk {
                field,
                found: &mut *found,
            })?;
        }
        Ok(())
    }
}

/// A map key of any scalar type, as text: `2022` in a map of survey years.
struct Key(String);

impl<'de> de::Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;

        impl Visitor<'_> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map key")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Key, E> {
                Ok(Key(value.to_owned()))
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Key, E> {
                Ok(Key(value.to_string()))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Key, E> {
                Ok(Key(value.to_string()))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Key, E> {
                Ok(Key(value.to_string()))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Key, E> {
                Ok(Key(value.to_string()))
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
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
