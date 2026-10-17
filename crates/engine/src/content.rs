//! Loading and validating the hand-written files in `content/`.

mod check;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use model::{
    Entity, FrameworkTraits, Kind, LanguageTraits, Planned, RuntimeTraits, Terms, TraitVocab,
    Traits, Vocab,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

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
    pub file: PathBuf,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.message)
    }
}

/// Reads and validates the content folder at `root`.
///
/// Returns every problem found rather than stopping at the first one.
pub fn load(root: &Path) -> Result<Content, Vec<Diagnostic>> {
    let mut diags = Vec::new();

    let domains = read_yaml::<Terms>(&root.join("vocab/domains.yaml"), &mut diags);
    let traits = read_yaml::<TraitVocab>(&root.join("vocab/traits.yaml"), &mut diags);
    let planned_path = root.join("planned.yaml");
    let planned = if planned_path.exists() {
        read_yaml::<Planned>(&planned_path, &mut diags)
    } else {
        Some(Planned::default())
    };

    let mut entities = Vec::new();
    // Files that failed to parse. Their ids still count as known, so other
    // files referring to them don't get misleading "unknown entity" errors.
    let mut unparsed = Vec::new();
    for kind in Kind::ALL {
        for file in entity_files(&root.join(kind.dir()), &mut diags) {
            match read_entity(&file, kind, &mut diags) {
                Some(entity) => entities.push((file, entity)),
                None => unparsed.push((file, kind)),
            }
        }
    }

    let (Some(domains), Some(traits), Some(planned)) = (domains, traits, planned) else {
        return Err(diags);
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
        Err(diags)
    }
}

fn read_yaml<T: DeserializeOwned>(path: &Path, diags: &mut Vec<Diagnostic>) -> Option<T> {
    let source = read(path, diags)?;
    parse(path, &source, diags)
}

fn read(path: &Path, diags: &mut Vec<Diagnostic>) -> Option<String> {
    fs::read_to_string(path)
        .map_err(|err| diags.push(diagnostic(path, format!("cannot read file: {err}"))))
        .ok()
}

fn parse<T: DeserializeOwned>(path: &Path, source: &str, diags: &mut Vec<Diagnostic>) -> Option<T> {
    serde_saphyr::from_str(source)
        .map_err(|err| diags.push(diagnostic(path, err.to_string())))
        .ok()
}

/// The `.yaml` files directly inside `dir`, sorted. A missing folder has no files.
fn entity_files(dir: &Path, diags: &mut Vec<Diagnostic>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            diags.push(diagnostic(dir, format!("cannot read folder: {err}")));
            return Vec::new();
        }
    };

    let mut files = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(err) => {
                diags.push(diagnostic(dir, format!("cannot read folder: {err}")));
                continue;
            }
        };
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yaml") if path.is_file() => files.push(path),
            Some("yml") => diags.push(diagnostic(&path, "use the .yaml extension")),
            _ => {}
        }
    }
    files.sort();
    files
}

fn read_entity(path: &Path, folder_kind: Kind, diags: &mut Vec<Diagnostic>) -> Option<Entity> {
    let source = read(path, diags)?;

    // Check `kind` first: a framework file in `languages/` would otherwise fail
    // with confusing errors about its traits.
    #[derive(Deserialize)]
    struct KindOnly {
        kind: Kind,
    }
    if let Ok(KindOnly { kind }) = serde_saphyr::from_str::<KindOnly>(&source)
        && kind != folder_kind
    {
        diags.push(diagnostic(
            path,
            format!(
                "kind is {kind}, but the file is in {}/; move it to {}/",
                folder_kind.dir(),
                kind.dir()
            ),
        ));
        return None;
    }

    Some(match folder_kind {
        Kind::Language => {
            parse::<Entity<LanguageTraits>>(path, &source, diags)?.map_traits(Traits::Language)
        }
        Kind::Framework => {
            parse::<Entity<FrameworkTraits>>(path, &source, diags)?.map_traits(Traits::Framework)
        }
        Kind::Runtime => {
            parse::<Entity<RuntimeTraits>>(path, &source, diags)?.map_traits(Traits::Runtime)
        }
    })
}

fn diagnostic(file: &Path, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        file: file.to_path_buf(),
        message: message.into(),
    }
}
