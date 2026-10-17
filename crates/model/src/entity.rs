use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::PartialDate;

/// What an entity is on the site. Each kind has its own folder under `content/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Language,
    Framework,
    Runtime,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Language, Kind::Framework, Kind::Runtime];

    /// The folder under `content/` that holds entities of this kind.
    pub fn dir(self) -> &'static str {
        match self {
            Kind::Language => "languages",
            Kind::Framework => "frameworks",
            Kind::Runtime => "runtimes",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Language => "language",
            Kind::Framework => "framework",
            Kind::Runtime => "runtime",
        })
    }
}

/// A curated entity, as written in a `content/` YAML file.
///
/// `T` holds the kind-specific traits. Files are parsed as `Entity<LanguageTraits>`,
/// `Entity<FrameworkTraits>` and so on, then converted to `Entity<Traits>`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entity<T = Traits> {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub tagline: String,
    pub summary: String,
    pub links: Links,
    pub ids: ExternalIds,
    pub traits: T,
    pub domains: Vec<DomainFit>,
    pub strengths: Vec<Point>,
    pub weaknesses: Vec<Point>,
    /// 2–4 situations that each complete the sentence "Choose <name> when …",
    /// shown side by side on compare pages.
    pub choose_when: Vec<String>,
    #[serde(default)]
    pub relations: Relations,
    #[serde(default)]
    pub timeline: Vec<Milestone>,
    pub learning: Learning,
    #[serde(default)]
    pub notable_users: Vec<NotableUser>,
    /// Paths relative to `content/samples/`.
    #[serde(default)]
    pub samples: Vec<String>,
    #[serde(default)]
    pub overrides: Overrides,
}

impl<T> Entity<T> {
    pub fn map_traits<U>(self, f: impl FnOnce(T) -> U) -> Entity<U> {
        Entity {
            id: self.id,
            kind: self.kind,
            name: self.name,
            aliases: self.aliases,
            tagline: self.tagline,
            summary: self.summary,
            links: self.links,
            ids: self.ids,
            traits: f(self.traits),
            domains: self.domains,
            strengths: self.strengths,
            weaknesses: self.weaknesses,
            choose_when: self.choose_when,
            relations: self.relations,
            timeline: self.timeline,
            learning: self.learning,
            notable_users: self.notable_users,
            samples: self.samples,
            overrides: self.overrides,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Traits {
    Language(LanguageTraits),
    Framework(FrameworkTraits),
    Runtime(RuntimeTraits),
}

/// Every value comes from `content/vocab/traits.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LanguageTraits {
    pub typing: Vec<String>,
    pub memory: Vec<String>,
    pub execution: Vec<String>,
    pub paradigms: Vec<String>,
    pub concurrency: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameworkTraits {
    /// A value from `content/vocab/domains.yaml`.
    pub ecosystem: String,
    /// Ids of language entities.
    pub languages: Vec<String>,
    #[serde(default)]
    pub architecture: Vec<String>,
    #[serde(default)]
    pub rendering: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTraits {
    /// Ids of language entities.
    pub languages: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Links {
    pub website: Url,
    pub docs: Option<Url>,
    pub repo: Option<Url>,
}

/// How the engine finds this entity in each external source.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalIds {
    pub wikidata: Option<String>,
    /// `owner/repo`.
    pub github: Option<String>,
    pub npm: Option<String>,
    pub pypi: Option<String>,
    pub crates: Option<String>,
    pub nuget: Option<String>,
    pub packagist: Option<String>,
    pub rubygems: Option<String>,
    #[serde(rename = "pub")]
    pub pub_dev: Option<String>,
    pub hex: Option<String>,
    pub maven: Option<String>,
    pub stackoverflow_tag: Option<String>,
    /// The name used in the Stack Overflow Developer Survey.
    pub so_survey: Option<SurveyName>,
    /// The endoflife.date product slug.
    pub endoflife: Option<String>,
}

impl ExternalIds {
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none() && self.so_survey.is_none()
    }

    /// Each id written as a single string that is set, with its field name as
    /// written in content files. `so_survey`, which may hold a name per year,
    /// is left out.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        let ExternalIds {
            wikidata,
            github,
            npm,
            pypi,
            crates,
            nuget,
            packagist,
            rubygems,
            pub_dev,
            hex,
            maven,
            stackoverflow_tag,
            so_survey: _,
            endoflife,
        } = self;
        [
            ("wikidata", wikidata),
            ("github", github),
            ("npm", npm),
            ("pypi", pypi),
            ("crates", crates),
            ("nuget", nuget),
            ("packagist", packagist),
            ("rubygems", rubygems),
            ("pub", pub_dev),
            ("hex", hex),
            ("maven", maven),
            ("stackoverflow_tag", stackoverflow_tag),
            ("endoflife", endoflife),
        ]
        .into_iter()
        .filter_map(|(field, id)| Some((field, id.as_deref()?)))
    }
}

/// The name a technology has in the Stack Overflow Developer Survey, which
/// sometimes renames technologies between years.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SurveyName {
    /// The same name in every survey year.
    One(String),
    /// Each name with the first survey year that used it, such as
    /// `{2022: React.js, 2023: React}`. A name applies until the next year listed.
    ByYear(BTreeMap<u16, String>),
}

impl SurveyName {
    /// The name in the survey of `year`, or `None` for a year before the first
    /// one listed.
    pub fn for_year(&self, year: u16) -> Option<&str> {
        match self {
            SurveyName::One(name) => Some(name),
            SurveyName::ByYear(names) => names
                .range(..=year)
                .next_back()
                .map(|(_, name)| name.as_str()),
        }
    }
}

/// How well an entity suits one kind of project.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DomainFit {
    /// A value from `content/vocab/domains.yaml`.
    pub domain: String,
    pub fit: Fit,
    /// Required when `fit` is `possible` or `poor`.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    Excellent,
    Good,
    Possible,
    Poor,
}

/// A strength or a weakness.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub sources: Vec<Url>,
}

/// Links to other entities, by id. Each relation is written on one side only;
/// the engine derives the reverse direction.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Relations {
    /// Symmetric. Must have the same kind.
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// This entity is built on the target, e.g. `nextjs` → `react`.
    #[serde(default)]
    pub built_on: Vec<String>,
    /// This entity runs on the target runtime, e.g. `express` → `nodejs`.
    #[serde(default)]
    pub runs_on: Vec<String>,
    /// Symmetric, e.g. `react` ↔ `typescript`.
    #[serde(default)]
    pub often_with: Vec<String>,
    /// This entity was replaced by the target, e.g. `angularjs` → `angular`.
    #[serde(default)]
    pub succeeded_by: Vec<String>,
}

impl Relations {
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &[String])> {
        let Relations {
            alternatives,
            built_on,
            runs_on,
            often_with,
            succeeded_by,
        } = self;
        [
            ("alternatives", alternatives.as_slice()),
            ("built_on", built_on.as_slice()),
            ("runs_on", runs_on.as_slice()),
            ("often_with", often_with.as_slice()),
            ("succeeded_by", succeeded_by.as_slice()),
        ]
        .into_iter()
    }
}

/// A milestone that is not a release. Releases are collected automatically.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    pub date: PartialDate,
    pub title: String,
    pub url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Learning {
    pub curve: LearningCurve,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LearningCurve {
    Gentle,
    Moderate,
    Steep,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub title: String,
    pub url: Url,
    pub kind: ResourceKind,
    pub free: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    Book,
    Course,
    Docs,
    Exercises,
    Tutorial,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotableUser {
    pub name: String,
    /// What they use it for.
    pub what: String,
    pub source: Url,
}

/// Corrections to facts the engine collects, for when a source is wrong.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    pub inception: Option<Override<PartialDate>>,
    pub designers: Option<Override<Vec<String>>>,
    pub developer: Option<Override<Vec<String>>>,
    pub license: Option<Override<Vec<String>>>,
    pub influenced_by: Option<Override<Vec<String>>>,
    pub written_in: Option<Override<Vec<String>>>,
}

impl Overrides {
    /// The reason given for each override that is set.
    pub fn reasons(&self) -> impl Iterator<Item = (&'static str, &str)> {
        fn reason<T>(o: &Option<Override<T>>) -> Option<&str> {
            o.as_ref().map(|o| o.reason.as_str())
        }
        let Overrides {
            inception,
            designers,
            developer,
            license,
            influenced_by,
            written_in,
        } = self;
        [
            ("inception", reason(inception)),
            ("designers", reason(designers)),
            ("developer", reason(developer)),
            ("license", reason(license)),
            ("influenced_by", reason(influenced_by)),
            ("written_in", reason(written_in)),
        ]
        .into_iter()
        .filter_map(|(field, reason)| Some((field, reason?)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override<T> {
    pub value: T,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survey_names_apply_from_their_year_until_the_next_one() {
        let one = SurveyName::One("Rust".into());
        assert_eq!(one.for_year(2011), Some("Rust"));
        assert_eq!(one.for_year(2025), Some("Rust"));

        let by_year = SurveyName::ByYear(BTreeMap::from([
            (2022, "React.js".to_owned()),
            (2023, "React".to_owned()),
        ]));
        assert_eq!(by_year.for_year(2021), None);
        assert_eq!(by_year.for_year(2022), Some("React.js"));
        assert_eq!(by_year.for_year(2023), Some("React"));
        assert_eq!(by_year.for_year(2025), Some("React"));
    }
}
