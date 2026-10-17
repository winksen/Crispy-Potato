use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use engine::content::{self, Content};
use engine::{scaffold, schema};
use model::Kind;

#[derive(Parser)]
#[command(version, about = "Tools for the catalog's content and data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show last runs, failures and entity freshness for each source.
    Status,
    /// Inspect, release or discard held observations.
    Quarantine {
        #[command(subcommand)]
        action: QuarantineAction,
    },
    /// Apply database migrations (use the database owner's credentials).
    Migrate,
    /// Collect a registered source; source adapters arrive in track F.
    Ingest {
        source: String,
        #[arg(long)]
        entity: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate and synchronize curated content in one transaction.
    Sync {
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },
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
    /// Start a new entity file from a template and take its id off planned.yaml.
    New {
        /// language, framework or runtime.
        kind: Kind,
        /// The new entity's id, such as spring-boot.
        id: String,
        /// Path to the content folder.
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },
}

#[derive(Subcommand)]
enum QuarantineAction {
    List,
    Release { id: i64 },
    Discard { id: i64 },
}

#[tokio::main]
async fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Status => {
            let Ok(url) = std::env::var("DATABASE_URL") else {
                eprintln!("DATABASE_URL is required");
                return ExitCode::FAILURE;
            };
            let Ok(pool) = engine::db::connect(&url).await else {
                eprintln!("Cannot connect to the database");
                return ExitCode::FAILURE;
            };
            match engine::freshness::status(&pool).await {
                Ok(value) => {
                    println!("{value:#}");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!("Cannot read collection status");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Quarantine { action } => {
            let Ok(url) = std::env::var("DATABASE_URL") else {
                eprintln!("DATABASE_URL is required");
                return ExitCode::FAILURE;
            };
            let Ok(pool) = engine::db::connect(&url).await else {
                eprintln!("Cannot connect to the database");
                return ExitCode::FAILURE;
            };
            let result = match action {
                QuarantineAction::List => engine::quality::list(&pool)
                    .await
                    .map(|value| println!("{value:#}")),
                QuarantineAction::Release { id } => engine::quality::resolve(&pool, id, true).await,
                QuarantineAction::Discard { id } => {
                    engine::quality::resolve(&pool, id, false).await
                }
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => {
                    eprintln!("Quarantine action failed; invalid ranges cannot be released");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Ingest {
            source,
            entity,
            dry_run,
        } => {
            let sources = engine::ingest::registered_sources();
            let Some(adapter) = sources.iter().find(|s| s.info().id == source) else {
                eprintln!("Unknown source; concrete source adapters arrive in track F");
                return ExitCode::FAILURE;
            };
            let Ok(url) = std::env::var("DATABASE_URL") else {
                eprintln!("DATABASE_URL is required");
                return ExitCode::FAILURE;
            };
            let Ok(pool) = engine::db::connect(&url).await else {
                eprintln!("Cannot connect to the database");
                return ExitCode::FAILURE;
            };
            match engine::ingest::run(&pool, adapter.as_ref(), entity.as_deref(), dry_run).await {
                Ok(report) => match serde_json::to_string_pretty(&report) {
                    Ok(json) => {
                        println!("{json}");
                        if report.errors.is_empty() {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        }
                    }
                    Err(_) => {
                        eprintln!("Cannot format ingestion report");
                        ExitCode::FAILURE
                    }
                },
                Err(_) => {
                    eprintln!("Ingestion failed");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Sync { content } => {
            if validate(&content) != ExitCode::SUCCESS {
                return ExitCode::FAILURE;
            }
            let Ok(url) = std::env::var("DATABASE_URL") else {
                eprintln!("DATABASE_URL is required");
                return ExitCode::FAILURE;
            };
            let Ok(pool) = engine::db::connect(&url).await else {
                eprintln!("Cannot connect to the database");
                return ExitCode::FAILURE;
            };
            match engine::sync::sync(&pool, &content).await {
                Ok(report) => {
                    println!(
                        "{} changed, {} retired; content {}",
                        report.changed,
                        report.retired,
                        if report.unchanged {
                            "unchanged"
                        } else {
                            "updated"
                        }
                    );
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!("Content sync failed; transaction rolled back");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Migrate => {
            let Ok(url) = std::env::var("DATABASE_URL") else {
                eprintln!("DATABASE_URL is required");
                return ExitCode::FAILURE;
            };
            match engine::db::connect(&url).await {
                Ok(pool) => match engine::db::migrate(&pool).await {
                    Ok(()) => {
                        println!("Migrations are up to date");
                        ExitCode::SUCCESS
                    }
                    Err(_) => {
                        eprintln!("Database migration failed");
                        ExitCode::FAILURE
                    }
                },
                Err(_) => {
                    eprintln!("Cannot connect to the database");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Validate { content } => validate(&content),
        Command::Schema {
            content,
            out,
            check,
        } => write_schemas(&content, &out, check),
        Command::New { kind, id, content } => new_entity(&content, kind, &id),
    }
}

fn new_entity(root: &Path, kind: Kind, id: &str) -> ExitCode {
    match scaffold::new_entity(root, kind, id) {
        Ok(created) => {
            let file = format!("{}/{}/{id}.yaml", root.display(), kind.dir());
            if created.was_planned {
                println!("Wrote {file} and took \"{id}\" off planned.yaml.");
            } else {
                println!("Wrote {file}; \"{id}\" was not in planned.yaml.");
            }
            println!(
                "Replace every {} in it, then run `cargo validate` to see which are left.",
                content::PLACEHOLDER
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
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
