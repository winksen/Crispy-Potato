//! Starting a new entity file: a template with every field, a comment on each
//! and `TODO` placeholders, and the id taken off `planned.yaml`.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use model::{Kind, Terms, Vocab};

use crate::content::{self, PLACEHOLDER};
use crate::schema;

/// What [`new_entity`] did.
#[derive(Debug)]
pub struct Created {
    /// The file written: `<root>/<kind folder>/<id>.yaml`.
    pub path: PathBuf,
    /// Whether the id was taken off `planned.yaml`; `false` when it wasn't listed.
    pub was_planned: bool,
}

/// Writes a new entity file of `kind` for `id` under the content folder `root`
/// and removes the id from `planned.yaml`, keeping that file's comments.
///
/// Refuses an invalid id, an id that already has a file, and an id planned
/// as another kind.
pub fn new_entity(root: &Path, kind: Kind, id: &str) -> Result<Created, String> {
    if let Some(problem) = content::id_problem(id) {
        return Err(problem);
    }
    let vocab = content::load_vocab(root).map_err(|diags| {
        let problems: Vec<String> = diags.iter().map(ToString::to_string).collect();
        format!(
            "the vocabularies have problems; fix them first:\n{}",
            problems.join("\n")
        )
    })?;
    for other in Kind::ALL {
        let existing = root.join(other.dir()).join(format!("{id}.yaml"));
        if existing.exists() {
            return Err(format!(
                "\"{id}\" already has a file: {}/{id}.yaml",
                other.dir()
            ));
        }
    }

    let planned_path = root.join("planned.yaml");
    let planned = match fs::read_to_string(&planned_path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("cannot read planned.yaml: {err}")),
    };
    let without = match &planned {
        Some(text) => remove_planned(text, kind, id)?,
        None => None,
    };

    let dir = root.join(kind.dir());
    let path = dir.join(format!("{id}.yaml"));
    fs::create_dir_all(&dir)
        .and_then(|()| OpenOptions::new().write(true).create_new(true).open(&path))
        .and_then(|mut file| file.write_all(template(kind, id, &vocab).as_bytes()))
        .map_err(|err| format!("cannot write {}/{id}.yaml: {err}", kind.dir()))?;

    let was_planned = without.is_some();
    if let Some(text) = without {
        fs::write(&planned_path, text)
            .map_err(|err| format!("cannot update planned.yaml: {err}"))?;
    }
    Ok(Created { path, was_planned })
}

/// `planned` without `id`, or `None` when it doesn't list `id`. Edits the text
/// line by line so comments stay, and handles both `- id` lines and a
/// `[a, b]` list on the section's own line.
fn remove_planned(planned: &str, kind: Kind, id: &str) -> Result<Option<String>, String> {
    let mut section = "";
    let mut out = String::with_capacity(planned.len());
    let mut found = false;
    for line in planned.split_inclusive('\n') {
        let content = line.split('#').next().unwrap_or_default();
        if !line.starts_with([' ', '\t', '#']) {
            if let Some((name, rest)) = content.split_once(':') {
                section = name.trim();
                let flow = rest.trim();
                if let Some(items) = flow.strip_prefix('[').and_then(|f| f.strip_suffix(']')) {
                    let items: Vec<&str> = items.split(',').map(str::trim).collect();
                    if items.contains(&id) {
                        planned_as(section, kind, id)?;
                        found = true;
                        let kept: Vec<&str> =
                            items.into_iter().filter(|item| *item != id).collect();
                        let ending = if line.ends_with('\n') { "\n" } else { "" };
                        out.push_str(&format!("{section}: [{}]{ending}", kept.join(", ")));
                        continue;
                    }
                }
            }
        } else if content.trim().strip_prefix("- ").map(str::trim) == Some(id) {
            planned_as(section, kind, id)?;
            found = true;
            continue;
        }
        out.push_str(line);
    }
    Ok(found.then_some(out))
}

/// Refuses to create `id` as `kind` when `planned.yaml` lists it under another kind.
fn planned_as(section: &str, kind: Kind, id: &str) -> Result<(), String> {
    match Kind::ALL.into_iter().find(|k| k.dir() == section) {
        Some(planned) if planned != kind => Err(format!(
            "\"{id}\" is planned as a {planned}; run `cli new {planned} {id}`"
        )),
        _ => Ok(()),
    }
}

/// The text of a new entity file. Every value to write is `TODO`; optional
/// fields are empty, with a comment saying what goes in them.
pub fn template(kind: Kind, id: &str, vocab: &Vocab) -> String {
    let todo = PLACEHOLDER;
    let list = |terms: &Terms| terms.keys().cloned().collect::<Vec<_>>().join(", ");
    let traits = &vocab.traits;
    let domains = list(&vocab.domains);
    let kind_traits = match kind {
        Kind::Language => format!(
            "\
# Each trait is a list of values from content/vocab/traits.yaml.
traits:
  # One or more of: {typing}
  typing: [{todo}]
  # One or more of: {memory}
  memory: [{todo}]
  # One or more of: {execution}
  execution: [{todo}]
  # One or more of: {paradigms}
  paradigms: [{todo}]
  # One or more of: {concurrency}
  concurrency: [{todo}]
  # One or more of: {targets}
  targets: [{todo}]
",
            typing = list(&traits.typing),
            memory = list(&traits.memory),
            execution = list(&traits.execution),
            paradigms = list(&traits.paradigms),
            concurrency = list(&traits.concurrency),
            targets = list(&traits.targets),
        ),
        Kind::Framework => format!(
            "\
# Each trait is a list of values from content/vocab/traits.yaml, except the
# ecosystem and the language ids.
traits:
  # The ecosystem it belongs to, one of: {domains}
  ecosystem: {todo}
  # The ids of the languages it is used from, such as [javascript, typescript].
  languages: [{todo}]
  # Optional. Any of: {architecture}
  architecture: []
  # Optional. Any of: {rendering}
  rendering: []
  # Optional. Any of: {platforms}
  platforms: []
",
            architecture = list(&traits.architecture),
            rendering = list(&traits.rendering),
            platforms = list(&traits.platforms),
        ),
        Kind::Runtime => format!(
            "\
traits:
  # The ids of the languages it runs, such as [java, kotlin, scala].
  languages: [{todo}]
  # One or more of: {targets}
  targets: [{todo}]
",
            targets = list(&traits.targets),
        ),
    };

    format!(
        "\
{modeline}
#
# A new {kind}, written by `cli new`. Replace every {todo}, delete the optional
# lines you don't need, then run `cargo validate`: it lists the placeholders
# left. docs/data-model.md describes every field, and CONTRIBUTING.md how to
# source and word them.

# The id is the page's URL and never changes.
id: {id}
kind: {kind}
# The name as its own project writes it.
name: {todo}
# Other names people search for, such as golang. Optional.
aliases: []
# One sentence of at most 100 characters.
tagline: {todo}
# Two to four sentences in a neutral tone: what it is and what it is used for.
summary: >
  {todo}

links:
  # The official website.
  website: https://example.com/{todo}
  # The documentation's home page. Optional: delete the line if there is none.
  docs: https://example.com/{todo}
  # The main source repository. Optional: delete the line if there is none.
  repo: https://example.com/{todo}

# Where data about it is collected. Keep the ids that exist and delete the
# others; at least one besides simple_icons. docs/data-model.md has the formats.
ids:
  # The Wikidata item, such as Q575650.
  wikidata: {todo}
  # The main repository on GitHub, as owner/repo.
  github: {todo}
  # The Stack Overflow tag, such as rust.
  stackoverflow_tag: {todo}
  # The name in the Stack Overflow Developer Survey, or a name per survey year.
  so_survey: {todo}
  # The endoflife.date product, such as nodejs.
  endoflife: {todo}
  # The Simple Icons slug of the logo, such as nodedotjs.
  simple_icons: {todo}
  # Package registries it publishes to, if any:
  # npm, pypi, crates, nuget, packagist, rubygems, pub, hex, maven.

{kind_traits}
# How well it suits each kind of project. Rate the domains that matter.
domains:
  # One of: {domains}
  - domain: {todo}
    # excellent, good, possible or poor, following the rubric in CONTRIBUTING.md.
    fit: {todo}
    # Required when the fit is possible or poor; otherwise optional.
    note: {todo}

# Two to six of each, with a source for any claim a reader could dispute.
strengths:
  - title: {todo}
    detail: {todo}
    sources: []
  - title: {todo}
    detail: {todo}
    sources: []

weaknesses:
  - title: {todo}
    detail: {todo}
    sources: []
  - title: {todo}
    detail: {todo}
    sources: []

# Two to four situations that each complete \"Choose <name> when …\", in at
# most 140 characters, such as: you need a single fast binary.
choose_when:
  - {todo}
  - {todo}

# Links to other entities by id, each written on one side only. Optional.
relations:
  # Entities of the same kind a reader would choose between.
  alternatives: []
  # What it is built on, such as react for nextjs.
  built_on: []
  # The runtimes it runs on, such as nodejs for express.
  runs_on: []
  # What it is often used with, such as typescript for react.
  often_with: []
  # What replaced it, such as angular for angularjs.
  succeeded_by: []

# Milestones that aren't releases, oldest first: a date (2015, 2015-05 or
# 2015-05-15), a title and an optional url. Optional.
timeline: []

learning:
  # gentle, moderate or steep.
  curve: {todo}
  # At least one.
  resources:
    - title: {todo}
      url: https://example.com/{todo}
      # book, course, docs, exercises, tutorial or video.
      kind: {todo}
      # true or false.
      free: {todo}

# Well-known users: a name, what they use it for and a source. Optional.
notable_users: []

# Code samples, as paths under content/samples/, such as {id}/hello.txt. Optional.
samples: []

# Corrections to collected facts (inception, designers, developer, license,
# influenced_by, written_in), each with a value and a reason. Optional.
overrides: {{}}
",
        modeline = schema::modeline(kind),
    )
}
