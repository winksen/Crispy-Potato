use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use engine::content::{self, Content};
use model::Kind;

#[derive(Parser)]
#[command(about = "Tools for the catalog's content and data")]
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
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { content } => validate(&content),
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
