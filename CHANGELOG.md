# Changelog

What changed in each release, newest first, with what to do when upgrading. Versions follow [semantic versioning](https://semver.org): the 0.1.x releases lead to `0.2.0`, the public preview. Every release, shipped and planned, has a row in the [release index](docs/RELEASES_0.1.md), and the [mindmap](docs/MINDMAP.md) describes what is built.

## V0.1.10 (2026-10-17)

Content files come from pull requests, so loading them now has hard limits: no links, no file over 256 KiB, and a YAML parser that refuses bombs before they expand.

### Security
- **No symbolic links under `content/`.** The loader walks the whole folder without following links and reports every symbolic link (and, on Windows, every junction) with "is a symbolic link; links are not allowed under content/…". Nothing is read through a refused link or from inside a linked folder, so a sample pointing outside the repository can never be stored by sync or served by the API.
- **No file over 256 KiB.** Every file under `content/` is checked by size, and files are read at most 256 KiB at a time, so a file that grows while it is read is still refused (`MAX_FILE_BYTES` in the engine).
- **A budget for the YAML parser.** Each file is scanned before it is deserialized: one document, at most 16 levels of nesting, 16 anchors, 64 aliases, 50,000 values and 1 MiB of text. A breach is reported in plain words ("over the parser's limits for content files: it uses more than 64 aliases"). While a file is deserialized, alias expansion is capped too (10,000 replayed values, 100 per anchor), which also covers the check of `kind` that reads a file before its full parse.

### Added
- **Five tests**: an alias bomb (nine levels of nine aliases), a file nested 40 levels deep, a 258 KiB file, a symbolic link to a sample next to a linked folder, each failing in well under a second with one clear message; and a smaller bomb that passes the scan but is stopped by the alias limits while being deserialized.

### Not done
- **The symbolic-link test skipped on this Windows machine**, which lacks the privilege to create links (os error 1314); it runs wherever links can be made, including CI on Linux. A Windows junction to a folder outside `content/` was checked by hand: it was refused and the file behind it was not read.

### Upgrading
- Nothing to do: the repository's content is far inside every limit. Content that used YAML anchors heavily, or a sample over 256 KiB, would now be refused.

## V0.1.9 (2026-10-17)

Diagnostics print the same paths on every OS, and entity files with an uppercase extension are no longer skipped.

### Fixed
- **Paths in diagnostics are relative to the content folder, with forward slashes**: `languages/rust.yaml: traits.memory[0]: …` instead of the folder given on the command line joined with a mix of `/` and `\` on Windows. Paths inside messages (a duplicate id's other file, a planned id's file, a missing sample) are written the same way. `Diagnostic::file` is now a `String` holding that path.
- **Extensions are matched without regard to case.** A file named `Rust.YAML` was silently skipped; it is now read, validated, and reported with "use the .yaml extension, in lowercase". `.yml` in any case is still refused.

### Added
- **Two tests**: the exact diagnostic lines for a duplicate id, a planned id that has a file and a missing sample, with no backslash anywhere; and `.YAML` and `.Yml` files.

### Upgrading
- Nothing to do, unless a script parses the output of `cargo validate`: file paths now start at the content folder (`languages/rust.yaml`, not `content/languages/rust.yaml`).

## V0.1.8 (2026-10-17)

A threat model says what the project protects and which release deals with each threat, and `SECURITY.md` says how to report a vulnerability.

### Security
- **[The threat model](docs/security/threat-model.md)**: five assets (data integrity, availability, secrets, visitors' browsers, the code that ships), five trust boundaries (contributor pull requests, external sources, the public API and website, the operator, third-party code), and 24 threats, each with its mitigation, the release in the index that delivers it and whether that release has shipped. A last table maps every 🔒 row of the index to its threats.
- **`SECURITY.md`**: report a vulnerability privately through GitHub's private vulnerability reporting, what a useful report contains, that only the next release gets fixes, and what is in and out of scope. The README links it and the threat model.

### Upgrading
- **The owner turns on private vulnerability reporting** in the repository's settings (*Security* → *Private vulnerability reporting*); until then the route `SECURITY.md` describes is not available.

## V0.1.7 (2026-10-17)

The version is written in one place, and a test keeps it equal to the newest changelog entry.

### Added
- **One version for the workspace.** `workspace.package.version` in the root `Cargo.toml` is the only place the version is written; every crate inherits it with `version.workspace = true`.
- **`cli --version`** prints it (`cli 0.1.7`).
- **A version test** (`crates/cli/tests/version.rs`): `cargo test` fails when the newest `## V…` heading in this changelog differs from the crate version, when a crate sets its own version, or when `--version` prints something else.

### Changed
- **The release rules** in the index and in `CLAUDE.md` say where the version lives and that the test checks it.

### Upgrading
- Nothing to do. From now on a release bumps the version in the root `Cargo.toml` and adds its changelog entry in the same commit.

## V0.1.6 (2026-10-17)

Workspace-wide lints and formatting rules, so every crate follows the same ones.

### Added
- **Workspace lints** in `Cargo.toml`, which every crate opts into with `[lints] workspace = true`: `unsafe_code` is forbidden, clippy's `unwrap_used` and `expect_used` warn (and so fail CI, which denies warnings), and `dbg_macro` and `todo` are denied.
- **`clippy.toml`** allows `unwrap` and `expect` in tests.
- **`rustfmt.toml`** sets the 2024 style edition and LF line endings, matching `.gitattributes`.

### Changed
- **The engine's integration tests** allow `unwrap` at the top of the file: clippy's test exemption covers `#[test]` functions but not the helpers they share. There was no `unwrap`, `expect`, `unsafe`, `dbg!` or `todo!` outside tests to remove.

### Upgrading
- Nothing to do. New code that unwraps outside tests now fails `cargo clippy -- -D warnings`; return the error or handle the `None` instead.

## V0.1.5 (2026-10-17)

Dependencies are checked for advisories, licenses and sources, and kept up to date.

### Security
- **`deny.toml`**, the cargo-deny policy: RustSec vulnerabilities, unsound code and notices fail the check, and so do unmaintained crates the workspace depends on directly; licenses must be on an allow-list of permissive licenses that fit decision D1 (MIT, MIT-0, Apache-2.0 with or without the LLVM exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, CC0-1.0, Unicode-3.0), and the workspace's own crates are checked too; crates.io is the only allowed source, with no git dependencies; `*` version requirements are refused and duplicate versions warned.
- **A `cargo deny check` job in CI**, using `EmbarkStudios/cargo-deny-action` pinned by commit SHA (v2.1.1).
- **Dependabot** opens weekly pull requests for Cargo dependencies and for the pinned GitHub Actions, and only offers a release once it is a week old.

### Added
- **`rust-toolchain.toml`** pins the stable channel with rustfmt and clippy, so local builds and CI use the same toolchain.

### Not done
- **The CI job has not run on GitHub**, because nothing is pushed. Locally, `cargo deny check` passes (advisories, bans, licenses and sources ok), and setting a crate's license to GPL-3.0-only made it fail with "license is not explicitly allowed".

### Upgrading
- Install cargo-deny to run the check locally: `cargo install cargo-deny --locked`, then `cargo deny check`.

## V0.1.4 (2026-10-17)

Every push to main and every pull request is checked on GitHub Actions.

### Added
- **A CI workflow** (`.github/workflows/ci.yml`) that runs on pushes to `main` and on pull requests: `cargo fmt --check`, clippy with `-D warnings`, `cargo validate`, then `cargo test`. Validation runs before the tests, so a pull request with a broken content file fails at the validate step with the validator's report; format, lints and validation each report even when an earlier one failed.
- **Safe for untrusted pull requests.** Content pull requests come from contributors, so the workflow only runs on `pull_request`, never `pull_request_target`; its token is read-only (`permissions: contents: read`) and not kept in the checkout (`persist-credentials: false`).
- **Pinned actions and a build cache.** `actions/checkout` and `Swatinem/rust-cache` are pinned by commit SHA (v6.1.0 and v2.9.2); the toolchain is installed with the runner's own `rustup` rather than a third-party action. Cargo commands use `--locked`, so a stale `Cargo.lock` fails.

### Not done
- **The workflow has not run on GitHub**, because nothing is pushed yet. Its steps were run locally with the same commands, and a copy of `content/` with a misspelled trait value made `cargo validate` exit with an error. The first push to `main` is the real check.

### Upgrading
- Nothing to do.

## V0.1.3 (2026-10-17)

The code and the content get their licenses (decision D1).

### Added
- **Code is dual-licensed MIT OR Apache-2.0**, the Rust convention: `LICENSE-MIT` and `LICENSE-APACHE` at the root, and `license = "MIT OR Apache-2.0"` in the workspace manifest, which every crate inherits.
- **Content is licensed CC BY-SA 4.0**: `content/LICENSE` holds the legal code, and it covers the entity files, the vocabularies and the code samples.
- **A License section in the README** saying which license applies where, that collected data keeps its source's license, and that contributions come in under the same terms.

### Fixed
- **The architecture document** still listed the web framework, hosting and project name as open; it now points to the decisions agreed in V0.1.2.

### Upgrading
- Nothing to do.

## V0.1.2 (2026-10-17)

The repository gets its conventions: a changelog, the release rules for whoever implements the next release, a mindmap, and one line ending everywhere.

### Added
- **This changelog**, in the format of the release rules, with the V0.1.1 entry written after the fact.
- **`CLAUDE.md`**, holding the release rules from the [release index](docs/RELEASES_0.1.md#how-each-release-is-made) and linking it: one row per commit, staying inside the row, the changelog and commit formats, the checks to run and the security rules that always apply.
- **[The mindmap](docs/MINDMAP.md)**: what the project is, what is built (content, crates, commands, validation rules, repository conventions, checks) and what is not built yet. Every release keeps it current.
- **`.gitattributes`** stores and checks out every text file with LF line endings whatever the OS or `core.autocrlf` says, and marks images, fonts and archives as binary. **`.editorconfig`** gives editors the same settings: UTF-8, LF, a final newline, spaces (4, or 2 for YAML, JSON and web files).

### Changed
- **The release index** (first committed as V0.1.1 Bis) has decisions D1 to D9 agreed as recommended, and its release rules now include the mindmap.
- **Crate versions** move from 0.1.0 to 0.1.2, as every release bumps them from now on.

### Upgrading
- Nothing to do. Every file was already stored with LF, so `git add --renormalize .` changed none.

## V0.1.1 (2026-10-17)

The first code: the data model, validation of hand-written content, and the first two entities.

### Added
- **A Cargo workspace with three crates**: `model` (the data types), `engine` (loading and validating content) and `cli` (the `validate` command, run with `cargo validate`).
- **The entity model** for languages, frameworks and runtimes: links, external ids, traits, domain fit, strengths and weaknesses, relations, timeline, learning resources, notable users, code samples and overrides of collected facts.
- **Validation that reports every problem at once**, each with its file and field: schema and unknown fields, file names and folders, vocabulary values, references between entities, style limits, samples and override reasons.
- **Content for Rust and React** with a code sample each, the domains and traits vocabularies, and `content/planned.yaml` listing the 48 other v1 entities so files can refer to them before they exist.
- **The architecture and data model documents**: the engine behind a thin axum API, curated YAML kept apart from collected metrics, and a v1 scope of 18 languages and three framework ecosystems.

### Not done
- **No license yet.** It comes in V0.1.3.
- **Crate versions stayed at 0.1.0.** V0.1.2 starts bumping them with each release.

### Upgrading
- Nothing to do.
