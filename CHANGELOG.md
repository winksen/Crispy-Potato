# Changelog

What changed in each release, newest first, with what to do when upgrading. Versions follow [semantic versioning](https://semver.org): the 0.1.x releases lead to `0.2.0`, the public preview. Every release, shipped and planned, has a row in the [release index](docs/RELEASES_0.1.md), and the [mindmap](docs/MINDMAP.md) describes what is built.

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
