// This file is test code; clippy's allow-unwrap-in-tests covers only the
// #[test] functions, not the helpers they share.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine::{content, schema};
use model::Kind;
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

#[test]
fn entity_files_point_at_their_schema() {
    for kind in Kind::ALL {
        let Ok(entries) = fs::read_dir(repo_content().join(kind.dir())) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let source = fs::read_to_string(&path).unwrap();
            assert_eq!(
                source.lines().next(),
                Some(schema::modeline(kind).as_str()),
                "{} must start with the line that points editors at its schema",
                path.display()
            );
        }
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
choose_when:
  - you need a language that only exists in tests.
  - you want to check the validator.
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

/// Every diagnostic for the fixture as the cli prints it: `file: message`.
fn report(planned: &str, files: &[(&str, &str)]) -> Vec<String> {
    let dir = fixture(planned, files);
    match content::load(dir.path()) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.iter().map(ToString::to_string).collect(),
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

#[test]
fn paths_are_relative_to_the_content_folder_with_forward_slashes() {
    let with_sample = format!("{LANGUAGE}samples: [testlang/missing.txt]\n");
    let report = report(
        "languages: [testlang]\n",
        &[
            ("languages/copy.yaml", LANGUAGE),
            ("languages/testlang.yaml", &with_sample),
        ],
    );
    for expected in [
        "languages/copy.yaml: id: \"testlang\" does not match the file name; rename the file to testlang.yaml",
        "languages/testlang.yaml: id: \"testlang\" is already used by languages/copy.yaml",
        "planned.yaml: languages[0]: \"testlang\" already has a file (languages/copy.yaml); remove it from planned.yaml",
        "languages/testlang.yaml: samples[0]: file not found: samples/testlang/missing.txt",
    ] {
        assert!(
            report.iter().any(|line| line == expected),
            "expected {expected:?} in {report:#?}"
        );
    }
    assert!(
        !report.iter().any(|line| line.contains('\\')),
        "{report:#?}"
    );
}

#[test]
fn extensions_are_matched_without_regard_to_case() {
    let broken = LANGUAGE.replace("memory: [gc]", "memory: [magic]");
    let report = report(
        "",
        &[
            ("languages/testlang.YAML", &broken),
            ("languages/other.Yml", LANGUAGE),
        ],
    );
    assert_problem(
        &report,
        "languages/testlang.YAML: use the .yaml extension, in lowercase",
    );
    assert_problem(
        &report,
        "languages/testlang.YAML: traits.memory[0]: unknown value \"magic\"",
    );
    assert_problem(&report, "languages/other.Yml: use the .yaml extension");
}

/// Nine levels of nine aliases: about 387 million strings once expanded.
fn alias_bomb() -> String {
    let mut yaml = String::from("lol0: &l0 [lol, lol, lol, lol, lol, lol, lol, lol, lol]\n");
    for level in 1..9 {
        let prev = level - 1;
        let aliases = vec![format!("*l{prev}"); 9].join(", ");
        yaml.push_str(&format!("lol{level}: &l{level} [{aliases}]\n"));
    }
    yaml
}

#[test]
fn alias_bombs_are_refused_before_they_expand() {
    let bomb = format!("{LANGUAGE}{}", alias_bomb());
    let start = Instant::now();
    let problems = problems("", &[("languages/testlang.yaml", &bomb)]);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "took {:?}",
        start.elapsed()
    );
    assert_eq!(
        problems,
        ["over the parser's limits for content files: it uses more than 64 aliases"]
    );
}

#[test]
fn deeply_nested_files_are_refused() {
    let deep = format!(
        "{LANGUAGE}notable_users: {}{}\n",
        "[".repeat(40),
        "]".repeat(40)
    );
    let problems = problems("", &[("languages/testlang.yaml", &deep)]);
    assert_eq!(
        problems,
        ["over the parser's limits for content files: it nests deeper than 16 levels"]
    );
}

#[test]
fn files_over_256_kib_are_refused() {
    let line = format!("# {}\n", "x".repeat(1021));
    let big = format!("{LANGUAGE}{}", line.repeat(257));
    let start = Instant::now();
    let report = report("", &[("languages/testlang.yaml", &big)]);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "took {:?}",
        start.elapsed()
    );
    assert_eq!(
        report,
        ["languages/testlang.yaml: is 258 KiB; files under content/ may be at most 256 KiB"]
    );
}

#[test]
fn symbolic_links_under_content_are_refused() {
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "not content").unwrap();
    fs::write(outside.path().join("evil.yaml"), LANGUAGE).unwrap();

    let source = format!("{LANGUAGE}samples: [testlang/hello.txt]\n");
    let dir = fixture("", &[("languages/testlang.yaml", &source)]);
    let link = dir.path().join("samples/testlang/hello.txt");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    if let Err(err) = symlink(&secret, &link, false) {
        eprintln!("skipped: this system cannot create symbolic links ({err})");
        return;
    }
    symlink(outside.path(), &dir.path().join("runtimes"), true).unwrap();

    let start = Instant::now();
    let report = match content::load(dir.path()) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.iter().map(ToString::to_string).collect(),
    };
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "took {:?}",
        start.elapsed()
    );
    let link_message = "is a symbolic link; links are not allowed under content/, so nothing outside it can be read through them";
    assert_eq!(
        report,
        [
            format!("runtimes: {link_message}"),
            format!("samples/testlang/hello.txt: {link_message}"),
        ]
    );
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path, _dir: bool) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink(target: &Path, link: &Path, dir: bool) -> io::Result<()> {
    if dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// The fixture with one more line under `ids:`, next to its valid GitHub id.
fn with_id(line: &str) -> String {
    LANGUAGE.replace(
        "  github: example/testlang\n",
        &format!("  github: example/testlang\n  {line}\n"),
    )
}

#[test]
fn each_external_id_kind_is_checked_against_its_format() {
    let cases = [
        (
            r#"wikidata: "575650""#,
            r#"ids.wikidata: "575650" is not a Wikidata item id"#,
        ),
        (r#"npm: """#, r#"ids.npm: "" is not an npm package name"#),
        (
            r#"pypi: "-django""#,
            r#"ids.pypi: "-django" is not a PyPI project name"#,
        ),
        (
            r#"crates: "1password""#,
            r#"ids.crates: "1password" is not a crates.io name"#,
        ),
        (
            r#"nuget: "Foo..Bar""#,
            r#"ids.nuget: "Foo..Bar" is not a NuGet package id"#,
        ),
        (
            "packagist: laravel",
            r#"ids.packagist: "laravel" is not a Packagist name"#,
        ),
        (
            r#"rubygems: "123""#,
            r#"ids.rubygems: "123" is not a gem name"#,
        ),
        (
            "pub: flutter-bloc",
            r#"ids.pub: "flutter-bloc" is not a pub.dev package name"#,
        ),
        (
            "hex: Phoenix",
            r#"ids.hex: "Phoenix" is not a Hex package name"#,
        ),
        (
            "maven: spring-boot",
            r#"ids.maven: "spring-boot" is not Maven coordinates"#,
        ),
        (
            "stackoverflow_tag: React JS",
            r#"ids.stackoverflow_tag: "React JS" is not a Stack Overflow tag"#,
        ),
        (
            r#"so_survey: " Rust""#,
            r#"ids.so_survey: " Rust" is not the name the Stack Overflow survey uses"#,
        ),
        (
            "endoflife: node.js",
            r#"ids.endoflife: "node.js" is not an endoflife.date product"#,
        ),
        (
            "simple_icons: node.js",
            r#"ids.simple_icons: "node.js" is not a Simple Icons slug"#,
        ),
    ];
    for (line, expected) in cases {
        let problems = problems("", &[("languages/testlang.yaml", &with_id(line))]);
        assert_eq!(problems.len(), 1, "{line}: {problems:#?}");
        assert_problem(&problems, expected);
    }

    let valid = [
        "npm: \"@angular/core\"",
        "maven: org.springframework.boot:spring-boot",
        "stackoverflow_tag: c#",
        "so_survey: ASP.NET Core",
    ];
    for line in valid {
        let problems = problems("", &[("languages/testlang.yaml", &with_id(line))]);
        assert_eq!(problems, Vec::<String>::new(), "{line}");
    }
}

#[test]
fn github_ids_refuse_dot_names_and_git_suffixes() {
    for repo in [
        "example/.",
        "example/..",
        "example/testlang.git",
        "-example/testlang",
    ] {
        let source = LANGUAGE.replace("github: example/testlang", &format!("github: \"{repo}\""));
        let problems = problems("", &[("languages/testlang.yaml", &source)]);
        assert_problem(
            &problems,
            &format!("ids.github: \"{repo}\" is not a GitHub repository"),
        );
    }
}

#[test]
fn dates_must_have_a_plausible_year() {
    let next = model::current_year() + 1;
    let cases = [
        ("0000".to_owned(), false),
        ("1939-12".to_owned(), false),
        ("1940".to_owned(), true),
        (next.to_string(), true),
        ((next + 1).to_string(), false),
        ("9999-01-01".to_owned(), false),
    ];
    for (date, valid) in cases {
        let source = format!("{LANGUAGE}timeline:\n  - date: \"{date}\"\n    title: A milestone\n");
        let problems = problems("", &[("languages/testlang.yaml", &source)]);
        if valid {
            assert_eq!(problems, Vec::<String>::new(), "{date}");
        } else {
            assert_problem(
                &problems,
                &format!(
                    "timeline[0].date: {date} is not a plausible date; the year must be from 1940 to {next}"
                ),
            );
        }
    }

    let source = format!(
        "{LANGUAGE}overrides:\n  inception:\n    value: \"0000\"\n    reason: The source is wrong.\n"
    );
    let problems = problems("", &[("languages/testlang.yaml", &source)]);
    assert_problem(
        &problems,
        "overrides.inception.value: 0000 is not a plausible date",
    );
}

#[test]
fn timelines_must_be_in_date_order() {
    let source = format!(
        "{LANGUAGE}timeline:\n  - date: 2015-05-15\n    title: Later\n  - date: 2013-05\n    title: Earlier\n"
    );
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &source)]),
        [
            "timeline[1].date: 2013-05 comes before the milestone above it (2015-05-15); list milestones in date order"
        ]
    );

    // A year sorts before the months in it, and equal dates may follow each other.
    let source = format!(
        "{LANGUAGE}timeline:\n  - date: 2013\n    title: A\n  - date: 2013-05\n    title: B\n  - date: 2013-05\n    title: C\n"
    );
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &source)]),
        Vec::<String>::new()
    );
}

#[test]
fn ids_must_not_contain_vs_as_a_word() {
    for id in ["rust-vs-go", "vs", "vs-code", "testlang-vs"] {
        let source = LANGUAGE.replace("id: testlang", &format!("id: {id}"));
        let problems = problems("", &[(&format!("languages/{id}.yaml"), &source)]);
        assert_eq!(
            problems,
            [format!(
                "id: \"{id}\" contains \"vs\" as a word; compare URLs use it to separate ids, as in /compare/go-vs-rust"
            )]
        );
    }

    assert_problem(
        &problems("languages: [go-vs-rust]\n", &[]),
        "languages[0]: \"go-vs-rust\" contains \"vs\" as a word",
    );

    // Inside a word it is fine.
    let source = LANGUAGE.replace("id: testlang", "id: canvas");
    assert_eq!(
        problems("", &[("languages/canvas.yaml", &source)]),
        Vec::<String>::new()
    );
}

#[test]
fn choose_when_has_two_to_four_short_statements() {
    let statements = |list: &[&str]| {
        let lines: Vec<String> = list.iter().map(|s| format!("  - {s}\n")).collect();
        let start = LANGUAGE.find("choose_when:\n").unwrap();
        let end = LANGUAGE.find("learning:\n").unwrap();
        format!(
            "{}choose_when:\n{}{}",
            &LANGUAGE[..start],
            lines.concat(),
            &LANGUAGE[end..]
        )
    };

    let one = statements(&["you need one reason."]);
    assert_problem(
        &problems("", &[("languages/testlang.yaml", &one)]),
        "choose_when: has 1 entries; write between 2 and 4",
    );

    let five = statements(&["one.", "two.", "three.", "four.", "five."]);
    assert_problem(
        &problems("", &[("languages/testlang.yaml", &five)]),
        "choose_when: has 5 entries; write between 2 and 4",
    );

    let long = "a".repeat(141);
    let too_long = statements(&["you need a short reason.", &long]);
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &too_long)]),
        ["choose_when[1]: is 141 characters; the limit is 140"]
    );

    let at_limit = "a".repeat(140);
    let four = statements(&["one.", "two.", "three.", &at_limit]);
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &four)]),
        Vec::<String>::new()
    );

    let missing = statements(&[]).replace("choose_when:\n", "");
    assert_problem(
        &problems("", &[("languages/testlang.yaml", &missing)]),
        "missing field `choose_when`",
    );
}

#[test]
fn survey_names_are_one_name_or_a_name_per_year() {
    let this_year = model::current_year();
    let valid = [
        "so_survey: Testlang".to_owned(),
        "so_survey:\n    2022: Testlang.js\n    2023: Testlang".to_owned(),
        format!("so_survey:\n    2011: Old Testlang\n    {this_year}: Testlang"),
    ];
    for line in &valid {
        let problems = problems("", &[("languages/testlang.yaml", &with_id(line))]);
        assert_eq!(problems, Vec::<String>::new(), "{line}");
    }

    let next_year = this_year + 1;
    let invalid = [
        (
            "so_survey:\n    2010: Testlang".to_owned(),
            format!(
                "ids.so_survey.2010: 2010 is not a survey year; the survey has run every year from 2011 to {this_year}"
            ),
        ),
        (
            format!("so_survey:\n    {next_year}: Testlang"),
            format!("ids.so_survey.{next_year}: {next_year} is not a survey year"),
        ),
        (
            "so_survey: {}".to_owned(),
            "ids.so_survey: list at least one survey year, or write the name alone".to_owned(),
        ),
        (
            "so_survey:\n    2023: \" Testlang\"".to_owned(),
            r#"ids.so_survey.2023: " Testlang" is not the name the Stack Overflow survey uses"#
                .to_owned(),
        ),
    ];
    for (line, expected) in &invalid {
        let problems = problems("", &[("languages/testlang.yaml", &with_id(line))]);
        assert_eq!(problems.len(), 1, "{line}: {problems:#?}");
        assert_problem(&problems, expected);
    }
}

#[test]
fn a_logo_slug_alone_is_not_an_id_to_collect_data_with() {
    let only_logo = LANGUAGE.replace("github: example/testlang", "simple_icons: testlang");
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &only_logo)]),
        ["ids: set at least one external id so data can be collected"]
    );
    let both = with_id("simple_icons: testlang");
    assert_eq!(
        problems("", &[("languages/testlang.yaml", &both)]),
        Vec::<String>::new()
    );
}
