use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::Kind;

/// Allowed values for one field, keyed by the value written in entity files.
pub type Terms = IndexMap<String, Term>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Term {
    pub label: String,
    pub description: String,
}

/// `content/vocab/traits.yaml`: the allowed values for each trait.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitVocab {
    pub typing: Terms,
    pub memory: Terms,
    pub execution: Terms,
    pub paradigms: Terms,
    pub concurrency: Terms,
    pub targets: Terms,
    pub architecture: Terms,
    pub rendering: Terms,
    pub platforms: Terms,
}

#[derive(Debug, Clone, Serialize)]
pub struct Vocab {
    /// `content/vocab/domains.yaml`. Also the allowed framework ecosystems.
    pub domains: Terms,
    pub traits: TraitVocab,
}

/// `content/planned.yaml`: entities in scope that don't have a file yet.
/// Other entities may reference them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Planned {
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    #[serde(default)]
    pub runtimes: Vec<String>,
}

impl Planned {
    pub fn ids(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::Language => &self.languages,
            Kind::Framework => &self.frameworks,
            Kind::Runtime => &self.runtimes,
        }
    }
}
