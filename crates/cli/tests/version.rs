//! The version is written once, in the workspace manifest, and the newest
//! changelog entry names it. Bumping one without the other fails here.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn newest_changelog_entry_is_the_crate_version() {
    let changelog = fs::read_to_string(repo().join("CHANGELOG.md")).unwrap();
    let newest = changelog
        .lines()
        .find_map(|line| line.strip_prefix("## V"))
        .expect("CHANGELOG.md has no `## V…` entry");
    let newest = newest.split_whitespace().next().unwrap_or_default();
    assert_eq!(
        newest, VERSION,
        "the newest CHANGELOG.md entry is V{newest} but workspace.package.version is {VERSION}; \
         bump both together"
    );
}

#[test]
fn every_crate_inherits_the_workspace_version() {
    for entry in fs::read_dir(repo().join("crates")).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        let Ok(source) = fs::read_to_string(&manifest) else {
            continue;
        };
        assert!(
            source
                .lines()
                .any(|line| line.trim() == "version.workspace = true"),
            "{} must use `version.workspace = true`",
            manifest.display()
        );
    }
}

#[test]
fn version_flag_prints_the_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_cli"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("cli {VERSION}")
    );
}
