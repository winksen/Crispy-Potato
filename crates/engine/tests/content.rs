// This file is test code; clippy's allow-unwrap-in-tests covers only the
// #[test] functions, not the helpers they share.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use engine::content;
use tempfile::TempDir;

fn repo_content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn repository_content_is_valid() {
    if let Err(diags) = content::load(&repo_content()) {
        let report: Vec<String> = diags.iter().map(ToString::to_string).collect();
        panic!("content has problems:\n\n{}", report.join("\n\n"));
    }
}

const LANGUAGE: &str = "\
id: testlang
kind: language
name: Testlang
tagline: A language that only exists in tests.
summary: Testlang is used to check the validator.
links:
  website: https://example.com
ids:
  github: example/testlang
traits:
  typing: [static]
  memory: [gc]
  execution: [compiled-native]
  paradigms: [imperative]
  concurrency: [threads]
  targets: [linux]
domains:
  - domain: systems
    fit: good
strengths:
  - title: One
    detail: The first strength.
  - title: Two
    detail: The second strength.
weaknesses:
  - title: One
    detail: The first weakness.
  - title: Two
    detail: The second weakness.
learning:
  curve: gentle
  resources:
    - title: Docs
      url: https://example.com/docs
      kind: docs
      free: true
";

/// A content folder with the real vocabularies, the given planned list and
/// one file per `(path, source)` pair.
fn fixture(planned: &str, files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("vocab")).unwrap();
    for vocab in ["domains.yaml", "traits.yaml"] {
        fs::copy(
            repo_content().join("vocab").join(vocab),
            dir.path().join("vocab").join(vocab),
        )
        .unwrap();
    }
    fs::write(dir.path().join("planned.yaml"), planned).unwrap();
    for (path, source) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    dir
}

/// The messages reported for the fixture, or an empty list if it is valid.
fn problems(planned: &str, files: &[(&str, &str)]) -> Vec<String> {
    let dir = fixture(planned, files);
    match content::load(dir.path()) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.into_iter().map(|d| d.message).collect(),
    }
}

fn assert_problem(problems: &[String], expected: &str) {
    assert!(
        problems.iter().any(|p| p.contains(expected)),
        "expected a problem containing {expected:?}, got {problems:#?}"
    );
}

#[test]
fn minimal_entity_is_valid() {
    assert_eq!(
        problems("", &[("languages/testlang.yaml", LANGUAGE)]),
        Vec::<String>::new()
    );
}

#[test]
fn unknown_fields_are_rejected() {
    let source = format!("{LANGUAGE}stars: 100\n");
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(&problems, "stars");
}

#[test]
fn unknown_vocabulary_values_are_rejected() {
    let source = LANGUAGE.replace("memory: [gc]", "memory: [magic]");
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(&problems, "traits.memory[0]: unknown value \"magic\"");
}

#[test]
fn relations_must_point_to_known_entities() {
    let source = format!("{LANGUAGE}relations:\n  alternatives: [nope]\n");
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(
        &problems,
        "relations.alternatives[0]: unknown entity \"nope\"",
    );
}

#[test]
fn planned_entities_can_be_referenced() {
    let source = format!("{LANGUAGE}relations:\n  alternatives: [otherlang]\n");
    let problems = problems(
        "languages: [otherlang]\n",
        &[("languages/testlang.yaml", &source)],
    );
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn broken_files_still_count_as_known_entities() {
    let broken = format!("{LANGUAGE}stars: 100\n");
    let other = format!(
        "{}relations:\n  alternatives: [testlang]\n",
        LANGUAGE.replace("id: testlang", "id: otherlang")
    );
    let problems = problems(
        "",
        &[
            ("languages/testlang.yaml", &broken),
            ("languages/otherlang.yaml", &other),
        ],
    );
    assert_problem(&problems, "stars");
    assert!(
        !problems.iter().any(|p| p.contains("unknown entity")),
        "{problems:#?}"
    );
}

#[test]
fn alternatives_must_have_the_same_kind() {
    let source = format!("{LANGUAGE}relations:\n  alternatives: [someframework]\n");
    let problems = problems(
        "frameworks: [someframework]\n",
        &[("languages/testlang.yaml", &source)],
    );
    assert_problem(&problems, "alternatives must also be languages");
}

#[test]
fn poor_fit_needs_a_note() {
    let source = LANGUAGE.replace("fit: good", "fit: poor");
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(&problems, "domains[0].note: a note is required");
}

#[test]
fn file_name_must_match_id() {
    let problems = problems("", &[("languages/other.yaml", LANGUAGE)]);
    assert_problem(&problems, "rename the file to testlang.yaml");
}

#[test]
fn kind_must_match_folder() {
    let problems = problems("", &[("frameworks/testlang.yaml", LANGUAGE)]);
    assert_problem(
        &problems,
        "kind is language, but the file is in frameworks/",
    );
}

#[test]
fn planned_ids_must_not_have_files() {
    let problems = problems(
        "languages: [testlang]\n",
        &[("languages/testlang.yaml", LANGUAGE)],
    );
    assert_problem(&problems, "\"testlang\" already has a file");
}

#[test]
fn missing_samples_are_reported() {
    let source = format!("{LANGUAGE}samples: [testlang/hello.txt]\n");
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(&problems, "samples[0]: file not found");
}
