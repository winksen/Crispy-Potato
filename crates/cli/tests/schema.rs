//! The committed schemas under `schemas/` match the model and the vocabularies.

use std::path::Path;
use std::process::Command;

#[test]
fn committed_schemas_are_up_to_date() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_cli"))
        .arg("schema")
        .arg("--check")
        .arg("--content")
        .arg(repo.join("content"))
        .arg("--out")
        .arg(repo.join("schemas"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
