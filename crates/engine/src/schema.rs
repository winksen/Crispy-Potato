//! JSON Schemas for entity files, so editors can complete fields and values
//! and flag mistakes while a file is written.
//!
//! The schemas are generated from the `model` types, with the vocabulary values
//! from `content/vocab/` as enums, and committed under `schemas/`. Each entity
//! file points at its schema in its first line.

use model::{Entity, FrameworkTraits, Kind, LanguageTraits, RuntimeTraits, Terms, Vocab};
use schemars::generate::SchemaSettings;
use serde_json::{Map, Value};

/// The file name of the schema for entity files of `kind`: `language.schema.json`.
pub fn file_name(kind: Kind) -> String {
    format!("{kind}.schema.json")
}

/// The first line of an entity file of `kind`, which points editors at its schema.
pub fn modeline(kind: Kind) -> String {
    format!(
        "# yaml-language-server: $schema=../../schemas/{}",
        file_name(kind)
    )
}

/// The schema for entity files of `kind`, as pretty-printed JSON.
///
/// Fails only if the model types no longer have a field the vocabularies
/// apply to, which a test catches.
pub fn entity_schema(kind: Kind, vocab: &Vocab) -> Result<String, String> {
    let generator = SchemaSettings::draft07().into_generator();
    let schema = match kind {
        Kind::Language => generator.into_root_schema_for::<Entity<LanguageTraits>>(),
        Kind::Framework => generator.into_root_schema_for::<Entity<FrameworkTraits>>(),
        Kind::Runtime => generator.into_root_schema_for::<Entity<RuntimeTraits>>(),
    };
    let mut schema = schema.to_value();
    if let Some(root) = schema.as_object_mut() {
        root.insert(
            "title".into(),
            format!("A {kind} in content/{}/", kind.dir()).into(),
        );
    }
    // Files in content/languages/ are languages, and so on.
    if let Some(kind_property) = schema.pointer_mut("/properties/kind") {
        *kind_property = serde_json::json!({ "const": kind.to_string() });
    }

    let traits = &vocab.traits;
    let mut enums: Vec<(&str, &Terms)> =
        vec![("/definitions/DomainFit/properties/domain", &vocab.domains)];
    match kind {
        Kind::Language => enums.extend([
            (
                "/definitions/LanguageTraits/properties/typing/items",
                &traits.typing,
            ),
            (
                "/definitions/LanguageTraits/properties/memory/items",
                &traits.memory,
            ),
            (
                "/definitions/LanguageTraits/properties/execution/items",
                &traits.execution,
            ),
            (
                "/definitions/LanguageTraits/properties/paradigms/items",
                &traits.paradigms,
            ),
            (
                "/definitions/LanguageTraits/properties/concurrency/items",
                &traits.concurrency,
            ),
            (
                "/definitions/LanguageTraits/properties/targets/items",
                &traits.targets,
            ),
        ]),
        Kind::Framework => enums.extend([
            (
                "/definitions/FrameworkTraits/properties/ecosystem",
                &vocab.domains,
            ),
            (
                "/definitions/FrameworkTraits/properties/architecture/items",
                &traits.architecture,
            ),
            (
                "/definitions/FrameworkTraits/properties/rendering/items",
                &traits.rendering,
            ),
            (
                "/definitions/FrameworkTraits/properties/platforms/items",
                &traits.platforms,
            ),
        ]),
        Kind::Runtime => enums.extend([(
            "/definitions/RuntimeTraits/properties/targets/items",
            &traits.targets,
        )]),
    }
    for (pointer, terms) in enums {
        let Some(target) = schema.pointer_mut(pointer).and_then(Value::as_object_mut) else {
            return Err(format!("the {kind} schema has no {pointer}"));
        };
        set_enum(target, terms);
    }

    serde_json::to_string_pretty(&schema)
        .map(|json| json + "\n")
        .map_err(|err| err.to_string())
}

/// Restricts a string schema to the vocabulary's values, each described by its
/// label and description.
fn set_enum(target: &mut Map<String, Value>, terms: &Terms) {
    let values = terms.keys().cloned().map(Value::from).collect();
    target.insert("enum".into(), Value::Array(values));
    let descriptions = terms
        .values()
        .map(|term| format!("{}: {}", term.label, term.description).into())
        .collect();
    // Shown by editors next to each completion.
    target.insert("enumDescriptions".into(), Value::Array(descriptions));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::content;

    #[test]
    fn every_kind_gets_a_schema_with_vocabulary_enums() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let vocab = content::load_vocab(&root).unwrap();
        for kind in Kind::ALL {
            let json = entity_schema(kind, &vocab).unwrap();
            let schema: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(schema["additionalProperties"], Value::Bool(false));
            let domains = &schema["definitions"]["DomainFit"]["properties"]["domain"]["enum"];
            assert!(
                domains
                    .as_array()
                    .is_some_and(|d| d.contains(&"web-backend".into()))
            );
        }
        let json = entity_schema(Kind::Language, &vocab).unwrap();
        let schema: Value = serde_json::from_str(&json).unwrap();
        let memory = &schema["definitions"]["LanguageTraits"]["properties"]["memory"]["items"];
        assert_eq!(
            memory["enum"],
            serde_json::json!(["gc", "ownership", "manual", "arc"])
        );
    }
}
