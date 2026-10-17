use std::path::{Path, PathBuf};
use std::process::ExitCode;

use std::fs;

use clap::{Parser, Subcommand};
use engine::content::{self, Content};
use engine::schema;
use model::Kind;

#[derive(Parser)]
#[command(version, about = "Tools for the catalog's content and data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check every file in the content folder.
    Validate {
        /// Path to the content folder.
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },
    /// Write the JSON Schemas editors use for entity files, one per kind.
    Schema {
        /// Path to the content folder, for the vocabularies.
        #[arg(long, default_value = "content")]
        content: PathBuf,
        /// Folder to write the schemas to.
        #[arg(long, default_value = "schemas")]
        out: PathBuf,
        /// Write nothing; fail if a schema in the folder is missing or out of date.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { content } => validate(&content),
        Command::Schema {
            content,
            out,
            check,
        } => write_schemas(&content, &out, check),
    }
}

fn write_schemas(content: &Path, out: &Path, check: bool) -> ExitCode {
    let vocab = match content::load_vocab(content) {
        Ok(vocab) => vocab,
        Err(diags) => {
            for diag in &diags {
                eprintln!("{diag}\n");
            }
            eprintln!("the vocabularies have problems; fix them first");
            return ExitCode::FAILURE;
        }
    };

    let mut stale = Vec::new();
    for kind in Kind::ALL {
        let json = match schema::entity_schema(kind, &vocab) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("cannot build the {kind} schema: {err}");
                return ExitCode::FAILURE;
            }
        };
        let path = out.join(schema::file_name(kind));
        if check {
            if fs::read_to_string(&path).ok().as_deref() != Some(json.as_str()) {
                stale.push(path);
            }
        } else if let Err(err) = fs::create_dir_all(out).and_then(|()| fs::write(&path, json)) {
            eprintln!("cannot write {}: {err}", path.display());
            return ExitCode::FAILURE;
        }
    }

    if stale.is_empty() {
        let verb = if check { "are up to date" } else { "written" };
        println!("{} schemas {verb} in {}", Kind::ALL.len(), out.display());
        ExitCode::SUCCESS
    } else {
        for path in &stale {
            eprintln!("{} is missing or out of date", path.display());
        }
        eprintln!("run `cargo schema` and commit the result");
        ExitCode::FAILURE
    }
}

fn validate(root: &Path) -> ExitCode {
    match content::load(root) {
        Ok(content) => {
            println!("Content is valid: {}", summary(&content));
            ExitCode::SUCCESS
        }
        Err(diags) => {
            for diag in &diags {
                eprintln!("{diag}\n");
            }
            eprintln!(
                "{} found in {}",
                plural(diags.len(), "problem"),
                root.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn summary(content: &Content) -> String {
    let count = |kind| content.entities.iter().filter(|e| e.kind == kind).count();
    let planned: usize = Kind::ALL
        .into_iter()
        .map(|kind| content.planned.ids(kind).len())
        .sum();
    format!(
        "{}, {}, {} ({planned} more planned)",
        plural(count(Kind::Language), "language"),
        plural(count(Kind::Framework), "framework"),
        plural(count(Kind::Runtime), "runtime"),
    )
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
