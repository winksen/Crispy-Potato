//! Rules that go beyond what the types enforce: vocabularies, cross-references,
//! naming and the house style for content.

mod ids;

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};

use model::{Entity, Fit, Kind, PartialDate, Planned, Point, Terms, Traits, Url, Vocab};

use super::Diags;

const TAGLINE_MAX: usize = 100;
const POINTS: RangeInclusive<usize> = 2..=6;
const SLUG_HINT: &str = "use lowercase letters, digits and single hyphens, like spring-boot";
const VS_HINT: &str = "compare URLs use it to separate ids, as in /compare/go-vs-rust";
/// No programming language or framework predates the first programmable computers.
const EARLIEST_YEAR: u16 = 1940;

/// An id that other entities may refer to: an entity with a file, or a planned one.
struct Known<'a> {
    kind: Kind,
    file: Option<&'a Path>,
}

struct Context<'a> {
    known: &'a HashMap<&'a str, Known<'a>>,
    vocab: &'a Vocab,
    samples: PathBuf,
    /// Dates may be at most a year ahead, for announced milestones.
    latest_year: u16,
}

/// Collects problems for one file, prefixed with the field they concern.
struct Report<'a, 'r> {
    file: &'a Path,
    diags: &'a mut Diags<'r>,
}

impl Report<'_, '_> {
    fn error(&mut self, field: impl Display, message: impl Display) {
        self.diags.push(self.file, format!("{field}: {message}"));
    }
}

pub(super) fn check(
    root: &Path,
    entities: &[(PathBuf, Entity)],
    unparsed: &[(PathBuf, Kind)],
    planned: &Planned,
    vocab: &Vocab,
    diags: &mut Diags,
) {
    let mut known = HashMap::new();
    for (file, entity) in entities {
        match known.get(entity.id.as_str()) {
            Some(Known {
                file: Some(other), ..
            }) => {
                let other = diags.relative(other);
                diags.push(
                    file,
                    format!("id: \"{}\" is already used by {other}", entity.id),
                );
            }
            _ => {
                known.insert(
                    entity.id.as_str(),
                    Known {
                        kind: entity.kind,
                        file: Some(file),
                    },
                );
            }
        }
    }
    for (file, kind) in unparsed {
        if let Some(id) = file.file_stem().and_then(|s| s.to_str()) {
            known.entry(id).or_insert(Known {
                kind: *kind,
                file: Some(file),
            });
        }
    }

    let planned_file = root.join("planned.yaml");
    let mut report = Report {
        file: &planned_file,
        diags,
    };
    for kind in Kind::ALL {
        for (i, id) in planned.ids(kind).iter().enumerate() {
            let field = format!("{}[{i}]", kind.dir());
            if !is_slug(id) {
                report.error(&field, format!("\"{id}\" is not a valid id; {SLUG_HINT}"));
            } else if has_vs(id) {
                report.error(
                    &field,
                    format!("\"{id}\" contains \"vs\" as a word; {VS_HINT}"),
                );
            }
            match known.get(id.as_str()) {
                Some(Known {
                    file: Some(file), ..
                }) => {
                    let file = report.diags.relative(file);
                    report.error(
                        &field,
                        format!(
                            "\"{id}\" already has a file ({file}); remove it from planned.yaml"
                        ),
                    );
                }
                Some(Known { file: None, .. }) => {
                    report.error(&field, format!("\"{id}\" is listed more than once"))
                }
                None => {
                    known.insert(id.as_str(), Known { kind, file: None });
                }
            }
        }
    }

    let ctx = Context {
        known: &known,
        vocab,
        samples: root.join("samples"),
        latest_year: model::current_year().saturating_add(1),
    };
    for (file, entity) in entities {
        check_entity(entity, &ctx, &mut Report { file, diags });
    }

    check_aliases(entities, planned, diags);
}

fn check_entity(e: &Entity, ctx: &Context, r: &mut Report) {
    if !is_slug(&e.id) {
        r.error("id", format!("\"{}\" is not a valid id; {SLUG_HINT}", e.id));
    } else if has_vs(&e.id) {
        r.error(
            "id",
            format!("\"{}\" contains \"vs\" as a word; {VS_HINT}", e.id),
        );
    }
    let stem = r
        .file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if stem != e.id {
        r.error(
            "id",
            format!(
                "\"{}\" does not match the file name; rename the file to {}.yaml",
                e.id, e.id
            ),
        );
    }

    required_text(r, "name", &e.name);
    required_text(r, "tagline", &e.tagline);
    let tagline_len = e.tagline.chars().count();
    if tagline_len > TAGLINE_MAX {
        r.error(
            "tagline",
            format!("is {tagline_len} characters; the limit is {TAGLINE_MAX}"),
        );
    }
    required_text(r, "summary", &e.summary);
    for (i, alias) in e.aliases.iter().enumerate() {
        required_text(r, format!("aliases[{i}]"), alias);
    }

    http_url(r, "links.website", &e.links.website);
    if let Some(url) = &e.links.docs {
        http_url(r, "links.docs", url);
    }
    if let Some(url) = &e.links.repo {
        http_url(r, "links.repo", url);
    }

    check_ids(e, r);
    check_traits(e, ctx, r);
    check_domains(e, ctx, r);
    check_points(r, "strengths", &e.strengths);
    check_points(r, "weaknesses", &e.weaknesses);
    check_relations(e, ctx, r);

    for (i, milestone) in e.timeline.iter().enumerate() {
        plausible_date(r, ctx, format!("timeline[{i}].date"), milestone.date);
        required_text(r, format!("timeline[{i}].title"), &milestone.title);
        if let Some(url) = &milestone.url {
            http_url(r, format!("timeline[{i}].url"), url);
        }
    }
    for (i, pair) in e.timeline.windows(2).enumerate() {
        if let [before, after] = pair
            && after.date < before.date
        {
            r.error(
                format!("timeline[{}].date", i + 1),
                format!(
                    "{} comes before the milestone above it ({}); list milestones in date order",
                    after.date, before.date
                ),
            );
        }
    }

    if e.learning.resources.is_empty() {
        r.error("learning.resources", "list at least one resource");
    }
    for (i, resource) in e.learning.resources.iter().enumerate() {
        required_text(r, format!("learning.resources[{i}].title"), &resource.title);
        http_url(r, format!("learning.resources[{i}].url"), &resource.url);
    }

    for (i, user) in e.notable_users.iter().enumerate() {
        required_text(r, format!("notable_users[{i}].name"), &user.name);
        required_text(r, format!("notable_users[{i}].what"), &user.what);
        http_url(r, format!("notable_users[{i}].source"), &user.source);
    }

    check_samples(e, ctx, r);

    if let Some(inception) = &e.overrides.inception {
        plausible_date(r, ctx, "overrides.inception.value", inception.value);
    }
    for (field, reason) in e.overrides.reasons() {
        if reason.trim().is_empty() {
            r.error(
                format!("overrides.{field}.reason"),
                "explain why the collected value is wrong",
            );
        }
    }
}

fn check_ids(e: &Entity, r: &mut Report) {
    let ids = &e.ids;
    if ids.is_empty() {
        r.error(
            "ids",
            "set at least one external id so data can be collected",
        );
    }
    for (field, id) in ids.iter() {
        if let Some(expected) = ids::problem(field, id) {
            r.error(
                format!("ids.{field}"),
                format!("\"{id}\" is not {expected}"),
            );
        }
    }
}

fn plausible_date(r: &mut Report, ctx: &Context, field: impl Display, date: PartialDate) {
    if !(EARLIEST_YEAR..=ctx.latest_year).contains(&date.year) {
        r.error(
            field,
            format!(
                "{date} is not a plausible date; the year must be from {EARLIEST_YEAR} to {}",
                ctx.latest_year
            ),
        );
    }
}

fn check_traits(e: &Entity, ctx: &Context, r: &mut Report) {
    let vocab = &ctx.vocab.traits;
    match &e.traits {
        Traits::Language(t) => {
            terms(r, "traits.typing", &t.typing, &vocab.typing, true);
            terms(r, "traits.memory", &t.memory, &vocab.memory, true);
            terms(r, "traits.execution", &t.execution, &vocab.execution, true);
            terms(r, "traits.paradigms", &t.paradigms, &vocab.paradigms, true);
            terms(
                r,
                "traits.concurrency",
                &t.concurrency,
                &vocab.concurrency,
                true,
            );
            terms(r, "traits.targets", &t.targets, &vocab.targets, true);
        }
        Traits::Framework(t) => {
            term(r, "traits.ecosystem", &t.ecosystem, &ctx.vocab.domains);
            languages(r, ctx, &t.languages);
            terms(
                r,
                "traits.architecture",
                &t.architecture,
                &vocab.architecture,
                false,
            );
            terms(r, "traits.rendering", &t.rendering, &vocab.rendering, false);
            terms(r, "traits.platforms", &t.platforms, &vocab.platforms, false);
        }
        Traits::Runtime(t) => {
            languages(r, ctx, &t.languages);
            terms(r, "traits.targets", &t.targets, &vocab.targets, true);
        }
    }
}

fn languages(r: &mut Report, ctx: &Context, ids: &[String]) {
    if ids.is_empty() {
        r.error("traits.languages", "list at least one language");
    }
    for (i, id) in unique(r, "traits.languages", ids) {
        let field = format!("traits.languages[{i}]");
        if let Some(target) = reference(r, ctx, &field, id)
            && target.kind != Kind::Language
        {
            r.error(
                field,
                format!("\"{id}\" is a {}, not a language", target.kind),
            );
        }
    }
}

fn check_domains(e: &Entity, ctx: &Context, r: &mut Report) {
    if e.domains.is_empty() {
        r.error("domains", "rate at least one domain");
    }
    let mut seen = HashSet::new();
    for (i, d) in e.domains.iter().enumerate() {
        let field = format!("domains[{i}]");
        term(r, format!("{field}.domain"), &d.domain, &ctx.vocab.domains);
        if !seen.insert(&d.domain) {
            r.error(
                format!("{field}.domain"),
                format!("\"{}\" is rated twice", d.domain),
            );
        }
        match &d.note {
            Some(note) => required_text(r, format!("{field}.note"), note),
            None if matches!(d.fit, Fit::Possible | Fit::Poor) => r.error(
                format!("{field}.note"),
                "a note is required when fit is possible or poor",
            ),
            None => {}
        }
    }
}

fn check_points(r: &mut Report, field: &str, points: &[Point]) {
    if !POINTS.contains(&points.len()) {
        r.error(
            field,
            format!(
                "has {} entries; write between {} and {}",
                points.len(),
                POINTS.start(),
                POINTS.end()
            ),
        );
    }
    for (i, point) in points.iter().enumerate() {
        required_text(r, format!("{field}[{i}].title"), &point.title);
        required_text(r, format!("{field}[{i}].detail"), &point.detail);
        for (j, url) in point.sources.iter().enumerate() {
            http_url(r, format!("{field}[{i}].sources[{j}]"), url);
        }
    }
}

fn check_relations(e: &Entity, ctx: &Context, r: &mut Report) {
    for (name, ids) in e.relations.iter() {
        let list = format!("relations.{name}");
        for (i, id) in unique(r, &list, ids) {
            let field = format!("{list}[{i}]");
            if *id == e.id {
                r.error(field, "an entity cannot refer to itself");
                continue;
            }
            let Some(target) = reference(r, ctx, &field, id) else {
                continue;
            };
            match name {
                "alternatives" if target.kind != e.kind => r.error(
                    field,
                    format!(
                        "\"{id}\" is a {}; alternatives must also be {}",
                        target.kind,
                        e.kind.dir()
                    ),
                ),
                "runs_on" if target.kind != Kind::Runtime => r.error(
                    field,
                    format!(
                        "\"{id}\" is a {}; runs_on must point to a runtime",
                        target.kind
                    ),
                ),
                _ => {}
            }
        }
    }
}

fn check_samples(e: &Entity, ctx: &Context, r: &mut Report) {
    for (i, sample) in e.samples.iter().enumerate() {
        let field = format!("samples[{i}]");
        let path = Path::new(sample);
        let inside =
            !sample.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)));
        if !inside {
            r.error(
                field,
                format!("\"{sample}\" must be a relative path inside the samples/ folder"),
            );
        } else if !ctx.samples.join(path).is_file() {
            let missing = r.diags.relative(&ctx.samples.join(path));
            r.error(field, format!("file not found: {missing}"));
        }
    }
}

/// Aliases are used for search, so each one must point to a single entity.
fn check_aliases(entities: &[(PathBuf, Entity)], planned: &Planned, diags: &mut Diags) {
    let planned_ids = Kind::ALL.into_iter().flat_map(|kind| planned.ids(kind));
    let entity_ids = entities.iter().map(|(_, e)| &e.id);
    let mut names: HashMap<String, &str> = entity_ids
        .chain(planned_ids)
        .map(|id| (id.to_lowercase(), id.as_str()))
        .collect();

    for (file, e) in entities {
        for alias in &e.aliases {
            let key = alias.to_lowercase();
            match names.get(key.as_str()) {
                Some(owner) if *owner != e.id => diags.push(
                    file,
                    format!("aliases: \"{alias}\" is already used by \"{owner}\""),
                ),
                Some(_) => {}
                None => {
                    names.insert(key, &e.id);
                }
            }
        }
    }
}

/// Reports values listed twice and returns the rest with their index.
fn unique<'v>(r: &mut Report, field: &str, values: &'v [String]) -> Vec<(usize, &'v String)> {
    let mut seen = HashSet::new();
    let mut first = Vec::new();
    for (i, value) in values.iter().enumerate() {
        if seen.insert(value) {
            first.push((i, value));
        } else {
            r.error(
                format!("{field}[{i}]"),
                format!("\"{value}\" is listed twice"),
            );
        }
    }
    first
}

fn reference<'c>(r: &mut Report, ctx: &'c Context, field: &str, id: &str) -> Option<&'c Known<'c>> {
    let found = ctx.known.get(id);
    if found.is_none() {
        r.error(
            field,
            format!("unknown entity \"{id}\"; add a file for it or list it in planned.yaml"),
        );
    }
    found
}

fn terms(r: &mut Report, field: &str, values: &[String], allowed: &Terms, required: bool) {
    if required && values.is_empty() {
        r.error(field, "list at least one value");
    }
    for (i, value) in unique(r, field, values) {
        term(r, format!("{field}[{i}]"), value, allowed);
    }
}

fn term(r: &mut Report, field: impl Display, value: &str, allowed: &Terms) {
    if !allowed.contains_key(value) {
        let names: Vec<&str> = allowed.keys().map(String::as_str).collect();
        r.error(
            field,
            format!(
                "unknown value \"{value}\"; use one of {} or add it to content/vocab/",
                names.join(", ")
            ),
        );
    }
}

fn required_text(r: &mut Report, field: impl Display, value: &str) {
    if value.trim().is_empty() {
        r.error(field, "must not be empty");
    }
}

fn http_url(r: &mut Report, field: impl Display, url: &Url) {
    if !matches!(url.scheme(), "http" | "https") {
        r.error(field, format!("\"{url}\" must be an http or https URL"));
    }
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Whether a slug has `vs` as one of its words, which would make a compare URL
/// such as `/compare/a-vs-b` ambiguous.
fn has_vs(slug: &str) -> bool {
    slug.split('-').any(|word| word == "vs")
}
