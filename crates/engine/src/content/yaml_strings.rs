use std::fmt;

use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// Visit decoded YAML strings before typed URL/date parsing can normalize them.
pub(super) fn strings(source: &str, options: serde_saphyr::Options) -> Vec<(String, String)> {
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

/// Visits strings without allocating a complete YAML value tree.
struct Walk<'a> {
    field: String,
    found: &'a mut Vec<(String, String)>,
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
        self.found.push((self.field, value.to_owned()));
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
            if crate::text_policy::problem("<map key>", &key).is_some() {
                found.push(("<map key>".into(), key.clone()));
            }
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
