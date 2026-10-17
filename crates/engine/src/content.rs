//! Loading and validating the hand-written files in `content/`.
//!
//! Content files come from pull requests, so they are read as untrusted input:
//! nothing under `content/` may be a symbolic link, no file may be larger than
//! [`MAX_FILE_BYTES`], and the YAML parser works within a budget that bounds
//! nesting, anchors and alias expansion.

mod check;

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use model::{
    Entity, FrameworkTraits, Kind, LanguageTraits, Planned, RuntimeTraits, Terms, TraitVocab,
    Traits, Vocab,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_saphyr::Budget;
use serde_saphyr::budget::{BudgetBreach, EnforcingPolicy};

/// The largest file allowed anywhere under `content/`: 256 KiB.
pub const MAX_FILE_BYTES: u64 = 256 * 1024;

/// Everything in `content/`, parsed and validated.
#[derive(Debug)]
pub struct Content {
    pub entities: Vec<Entity>,
    pub planned: Planned,
    pub vocab: Vocab,
}

/// A problem in one content file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The file, relative to the content folder and with forward slashes on
    /// every OS: `languages/rust.yaml`.
    pub file: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// The problems found so far, with paths written relative to the content folder.
struct Diags<'a> {
    root: &'a Path,
    list: Vec<Diagnostic>,
}

impl Diags<'_> {
    fn push(&mut self, file: &Path, message: impl Into<String>) {
        let file = self.relative(file);
        self.list.push(Diagnostic {
            file,
            message: message.into(),
        });
    }

    /// `path` relative to the content folder, with forward slashes.
    fn relative(&self, path: &Path) -> String {
        match path.strip_prefix(self.root) {
            Ok(relative) => relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
            Err(_) => path.to_string_lossy().replace('\\', "/"),
        }
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// Reads and validates the content folder at `root`.
///
/// Returns every problem found rather than stopping at the first one.
pub fn load(root: &Path) -> Result<Content, Vec<Diagnostic>> {
    let mut loader = Loader {
        diags: Diags {
            root,
            list: Vec::new(),
        },
        refused: HashSet::new(),
    };
    loader.check_tree(root);

    let domains = loader.read_yaml::<Terms>(&root.join("vocab/domains.yaml"));
    let traits = loader.read_yaml::<TraitVocab>(&root.join("vocab/traits.yaml"));
    let planned_path = root.join("planned.yaml");
    let planned = if fs::symlink_metadata(&planned_path).is_ok() {
        loader.read_yaml::<Planned>(&planned_path)
    } else {
        Some(Planned::default())
    };

    let mut entities = Vec::new();
    // Files that failed to parse. Their ids still count as known, so other
    // files referring to them don't get misleading "unknown entity" errors.
    let mut unparsed = Vec::new();
    for kind in Kind::ALL {
        for file in loader.entity_files(&root.join(kind.dir())) {
            match loader.read_entity(&file, kind) {
                Some(entity) => entities.push((file, entity)),
                None => unparsed.push((file, kind)),
            }
        }
    }

    let mut diags = loader.diags;
    let (Some(domains), Some(traits), Some(planned)) = (domains, traits, planned) else {
        return Err(diags.list);
    };
    let vocab = Vocab { domains, traits };

    check::check(root, &entities, &unparsed, &planned, &vocab, &mut diags);

    if diags.is_empty() {
        Ok(Content {
            entities: entities.into_iter().map(|(_, entity)| entity).collect(),
            planned,
            vocab,
        })
    } else {
        Err(diags.list)
    }
}

// The parser's limits for one content file. A hand-written entity nests about
// four levels deep and needs no anchors, so these leave plenty of room while an
// alias bomb or a deeply nested file is refused before it is deserialized.
const MAX_DEPTH: usize = 16;
const MAX_ANCHORS: usize = 16;
const MAX_ALIASES: usize = 64;
const MAX_NODES: usize = 50_000;
const MAX_SCALAR_BYTES: usize = 1024 * 1024;

fn budget() -> Budget {
    let mut budget = Budget::default();
    budget.max_documents = 1;
    budget.max_depth = MAX_DEPTH;
    budget.max_anchors = MAX_ANCHORS;
    budget.max_aliases = MAX_ALIASES;
    budget.max_nodes = MAX_NODES;
    budget.max_events = 2 * MAX_NODES;
    budget.max_total_scalar_bytes = MAX_SCALAR_BYTES;
    budget.max_inclusion_depth = 0;
    budget
}

/// The budget again while deserializing, plus limits on alias expansion, which
/// the scan in [`Loader::within_budget`] cannot see.
fn yaml_options() -> serde_saphyr::Options {
    serde_saphyr::options! {
        budget: Some(budget()),
        alias_limits: serde_saphyr::alias_limits! {
            max_total_replayed_events: 10_000,
            max_replay_stack_depth: 8,
            max_alias_expansions_per_anchor: 100,
        },
    }
}

fn describe(breach: &BudgetBreach) -> String {
    match breach {
        BudgetBreach::Depth { .. } => format!("it nests deeper than {MAX_DEPTH} levels"),
        BudgetBreach::Anchors { .. } => format!("it defines more than {MAX_ANCHORS} anchors"),
        BudgetBreach::Aliases { .. } | BudgetBreach::AliasAnchorRatio { .. } => {
            format!("it uses more than {MAX_ALIASES} aliases")
        }
        BudgetBreach::Nodes { .. } | BudgetBreach::Events { .. } => {
            format!("it holds more than {MAX_NODES} values")
        }
        BudgetBreach::Documents { .. } => {
            "it holds more than one YAML document; remove the extra ---".to_owned()
        }
        BudgetBreach::ScalarBytes { .. } => {
            format!("its text is longer than {} KiB", MAX_SCALAR_BYTES / 1024)
        }
        BudgetBreach::InclusionDepth { .. } => "includes are not supported".to_owned(),
        other => format!("{other:?}"),
    }
}

struct Loader<'a> {
    diags: Diags<'a>,
    /// Links and oversized files, already reported. Nothing is read from them
    /// or from inside a linked folder.
    refused: HashSet<PathBuf>,
}

impl Loader<'_> {
    /// Reports every symbolic link and every file over [`MAX_FILE_BYTES`] under
    /// `dir`, without following links.
    fn check_tree(&mut self, dir: &Path) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            // A missing content folder is reported when its files are read.
            Err(err) if err.kind() == io::ErrorKind::NotFound => return,
            Err(err) => {
                self.diags.push(dir, format!("cannot read folder: {err}"));
                return;
            }
        };
        let mut paths = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => paths.push(entry.path()),
                Err(err) => self.diags.push(dir, format!("cannot read folder: {err}")),
            }
        }
        paths.sort();

        for path in paths {
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(err) => {
                    self.diags.push(&path, format!("cannot read file: {err}"));
                    continue;
                }
            };
            if meta.file_type().is_symlink() {
                self.diags.push(
                    &path,
                    "is a symbolic link; links are not allowed under content/, so nothing \
                     outside it can be read through them",
                );
                self.refused.insert(path);
            } else if meta.is_dir() {
                self.check_tree(&path);
            } else if meta.len() > MAX_FILE_BYTES {
                self.diags.push(&path, too_large(meta.len()));
                self.refused.insert(path);
            }
        }
    }

    /// Whether `path` or a folder above it was refused by [`Self::check_tree`].
    fn is_refused(&self, path: &Path) -> bool {
        path.ancestors().any(|p| self.refused.contains(p))
    }

    fn read_yaml<T: DeserializeOwned>(&mut self, path: &Path) -> Option<T> {
        let source = self.read(path)?;
        if !self.within_budget(path, &source) {
            return None;
        }
        self.parse(path, &source)
    }

    /// Scans `source` against the parser's budget before anything is
    /// deserialized, and reports a breach in plain words.
    fn within_budget(&mut self, path: &Path, source: &str) -> bool {
        match serde_saphyr::budget::check_yaml_budget(source, budget(), EnforcingPolicy::AllContent)
        {
            Ok(report) => match report.breached {
                None => true,
                Some(breach) => {
                    let reason = describe(&breach);
                    self.diags.push(
                        path,
                        format!("over the parser's limits for content files: {reason}"),
                    );
                    false
                }
            },
            // A syntax error: the parse that follows reports it with its line.
            Err(_) => true,
        }
    }

    /// The text of `path`, read up to [`MAX_FILE_BYTES`].
    fn read(&mut self, path: &Path) -> Option<String> {
        if self.is_refused(path) {
            return None;
        }
        let mut bytes = Vec::new();
        let read = fs::File::open(path)
            .and_then(|file| file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes));
        if let Err(err) = read {
            self.diags.push(path, format!("cannot read file: {err}"));
            return None;
        }
        if bytes.len() as u64 > MAX_FILE_BYTES {
            // Only the first bytes were read; the file's own size is more telling.
            let size = fs::metadata(path).map_or(bytes.len() as u64, |meta| meta.len());
            self.diags.push(path, too_large(size));
            return None;
        }
        String::from_utf8(bytes)
            .map_err(|_| {
                self.diags
                    .push(path, "cannot read file: it is not valid UTF-8")
            })
            .ok()
    }

    fn parse<T: DeserializeOwned>(&mut self, path: &Path, source: &str) -> Option<T> {
        serde_saphyr::from_str_with_options(source, yaml_options())
            .map_err(|err| self.diags.push(path, err.to_string()))
            .ok()
    }

    /// The `.yaml` files directly inside `dir`, sorted. A missing folder has no files.
    ///
    /// Extensions are matched without regard to case, so `Rust.YAML` is read and
    /// reported rather than silently skipped.
    fn entity_files(&mut self, dir: &Path) -> Vec<PathBuf> {
        if self.is_refused(dir) {
            return Vec::new();
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(err) => {
                self.diags.push(dir, format!("cannot read folder: {err}"));
                return Vec::new();
            }
        };

        let mut files = Vec::new();
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(err) => {
                    self.diags.push(dir, format!("cannot read folder: {err}"));
                    continue;
                }
            };
            let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if ext.eq_ignore_ascii_case("yaml") && path.is_file() {
                if ext != "yaml" {
                    self.diags
                        .push(&path, "use the .yaml extension, in lowercase");
                }
                files.push(path);
            } else if ext.eq_ignore_ascii_case("yml") {
                self.diags.push(&path, "use the .yaml extension");
            }
        }
        files.sort();
        files
    }

    fn read_entity(&mut self, path: &Path, folder_kind: Kind) -> Option<Entity> {
        let source = self.read(path)?;
        if !self.within_budget(path, &source) {
            return None;
        }

        // Check `kind` first: a framework file in `languages/` would otherwise fail
        // with confusing errors about its traits.
        #[derive(Deserialize)]
        struct KindOnly {
            kind: Kind,
        }
        if let Ok(KindOnly { kind }) =
            serde_saphyr::from_str_with_options::<KindOnly>(&source, yaml_options())
            && kind != folder_kind
        {
            self.diags.push(
                path,
                format!(
                    "kind is {kind}, but the file is in {}/; move it to {}/",
                    folder_kind.dir(),
                    kind.dir()
                ),
            );
            return None;
        }

        Some(match folder_kind {
            Kind::Language => self
                .parse::<Entity<LanguageTraits>>(path, &source)?
                .map_traits(Traits::Language),
            Kind::Framework => self
                .parse::<Entity<FrameworkTraits>>(path, &source)?
                .map_traits(Traits::Framework),
            Kind::Runtime => self
                .parse::<Entity<RuntimeTraits>>(path, &source)?
                .map_traits(Traits::Runtime),
        })
    }
}

fn too_large(bytes: u64) -> String {
    format!(
        "is {} KiB; files under content/ may be at most {} KiB",
        bytes.div_ceil(1024),
        MAX_FILE_BYTES / 1024
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bomb small enough for the scan (45 aliases) that would still expand to
    /// 9^6 values: the alias limits stop it while it is deserialized.
    #[test]
    fn alias_expansion_is_limited_while_deserializing() {
        let mut yaml = String::from(
            "l0: &l0 [a, a, a, a, a, a, a, a, a]
",
        );
        for level in 1..6 {
            let aliases = vec![format!("*l{}", level - 1); 9].join(", ");
            yaml.push_str(&format!(
                "l{level}: &l{level} [{aliases}]
"
            ));
        }
        let report =
            serde_saphyr::budget::check_yaml_budget(&yaml, budget(), EnforcingPolicy::AllContent);
        assert!(report.is_ok_and(|report| report.breached.is_none()));

        let parsed =
            serde_saphyr::from_str_with_options::<serde::de::IgnoredAny>(&yaml, yaml_options());
        assert!(parsed.is_err());
    }
}
