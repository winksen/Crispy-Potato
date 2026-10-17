# Changelog

What changed in each release, newest first, with what to do when upgrading. Versions follow [semantic versioning](https://semver.org): the 0.1.x releases lead to `0.2.0`, the public preview. Every release, shipped and planned, has a row in the [release index](docs/RELEASES_0.1.md), and the [mindmap](docs/MINDMAP.md) describes what is built.

## V0.1.27 (2026-10-17)

A source contract and transactional collection runner.

### Added
- **Collection.** Source metadata, applicability and fetch batches drive cli ingest with entity filters and dry-run JSON previews; real runs persist provenance and isolate each entity transaction, recording partial failures without stopping other entities

### Fixed
- **Survey year maps.** JSON round-trips now accept string year keys as well as YAML numeric keys, so React can be read back from JSONB without losing its year-specific survey names.

### Not done
- cargo fmt, clippy, test (56 tests), validate, schema --check and sqlx prepare --check pass; a fake source verifies partial runs and a dry run with no writes; concrete providers remain scheduled for track F

### Upgrading
- Run cli sync before ingestion; concrete source names become available as their adapter releases ship.

## V0.1.26 (2026-10-17)

Shared host quotas and bounded concurrency.

### Added
- **Rate limits.** Process-wide capacity-one token buckets and concurrency caps cover every attempt and redirect; quotas live in one code table, crates.io is limited to one request per second and Stack Exchange backoff delays its shared host bucket

### Fixed
- **HEAD responses.** A large advertised resource size no longer trips the body cap when no response body is transferred.

### Not done
- cargo fmt, clippy, test (55 tests), validate and schema --check pass; fake-clock tests verify spacing, shared buckets, concurrency and Stack Exchange backoff; no provider load test was run

### Upgrading
- No new settings; quotas are defined in engine::limits::QUOTAS.

## V0.1.25 (2026-10-17)

Conditional HTTP requests preserve unchanged data.

### Added
- **HTTP cache.** ETag and Last-Modified validators use hashed request keys, conditional headers and explicit acceptance transactions; a 304 increments the run unchanged count without rewriting observations

### Not done
- cargo fmt, clippy, test (53 tests), validate, schema --check and sqlx prepare --check pass; a wiremock/Postgres test verifies conditional headers, delayed cache acceptance, unchanged row identity and run count

### Upgrading
- No migration needed; callers accept cache validators only with successfully stored data.

## V0.1.24 (2026-10-17)

A bounded retrying HTTP client for sources.

### Added
- **HTTP.** reqwest 0.12 with rustls, gzip and brotli, connect and body deadlines, a 20 MiB decoded cap, identifying User-Agent and three attempts with jitter and Retry-After; httpdate 1 and wiremock 0.6 added

### Not done
- cargo fmt, clippy, test (52 tests), validate, schema --check and cargo deny pass; wiremock covers 503 recovery, attempt limits, Retry-After, oversized bodies and timeouts; no live source adapter is built yet

### Upgrading
- Nothing to do; transport settings are code constants documented in DEVELOPMENT.md.

## V0.1.23 (2026-10-17)

Separate reader, writer and migration privileges.

### Added
- **Database roles.** Idempotent owner-run roles.sql creates cp_reader and cp_writer, grants only required table and sequence permissions, denies DDL and migration-history edits, and introduces API_DATABASE_URL without a writer fallback

### Not done
- cargo fmt, clippy, test (49 tests), validate and schema --check pass; Postgres role tests deny reader writes and writer DDL; GitHub CI awaits a push

### Upgrading
- Apply scripts/db/roles.sql as the migration owner, set role passwords with psql and use separate reader/writer URLs.

## V0.1.22 (2026-10-17)

SQL checked at compile time, with offline builds.

### Added
- **Checked queries.** Application SQL uses sqlx macros, .sqlx metadata is committed, SQLX_OFFLINE defaults to true and CI checks metadata against migrated Postgres

### Not done
- cargo fmt, clippy, test (48 tests), validate, schema --check, sqlx prepare --check and offline cargo build pass; a deliberately misspelled column fails compilation; GitHub CI awaits a push

### Upgrading
- Install sqlx-cli 0.8.6 and regenerate .sqlx metadata whenever a query changes; normal builds need no database.

## V0.1.21 (2026-10-17)

Validated content sync in one transaction.

### Added
- **Sync.** cli sync writes a stable validated snapshot, derives reverse relations, imports samples and vocabularies, hashes all content with sha2 0.10, skips unchanged runs and retires removed entities without deleting collected history

### Not done
- cargo fmt, clippy, test (48 tests), validate, schema --check and cargo deny pass; sync repeatability, invalid-input rollback and retirement are tested in Postgres; GitHub CI awaits a push

### Upgrading
- Run cli migrate then cli sync with DATABASE_URL set.

## V0.1.20 (2026-10-17)

Storage for collected data and provenance.

### Added
- **Collected data.** Migration 0002 adds sources, observations, quarantine, releases, facts, ingestion runs and HTTP validators; observations upsert by entity, metric, period, date and source; the data model documents all fourteen tables

### Not done
- cargo fmt, clippy, test (47 tests), validate and schema --check pass, including a real Postgres observation upsert test; GitHub CI awaits a push

### Upgrading
- Run cli migrate as the database owner.

## V0.1.19 (2026-10-17)

Postgres storage and explicit migrations.

### Added
- **Database.** sqlx 0.8 and Tokio 1, a five-connection pool with acquire and statement timeouts, cli migrate, seven curated-data tables and isolated database tests with Postgres in CI. The reviewed CDLA-Permissive-2.0 license for webpki-roots trust data is allowed; retain its agreement when redistributing

### Not done
- cargo fmt, clippy, test (46 tests), validate, schema --check and cargo deny pass; migrations run twice locally; GitHub CI awaits a push

### Upgrading
- Set DATABASE_URL with owner credentials, run cargo run -p cli -- migrate; tests require a disposable database owner with CREATEDB.

## V0.1.18 (2026-10-17)

A reproducible local Postgres environment.

### Added
- **Development environment.** Digest-pinned Postgres 17, a localhost port, persistent volume, health check, ignored local settings and a development guide with the settings table

### Not done
- Docker Compose startup and psql connection, cargo fmt, clippy, test (45 tests), validate and schema --check pass; GitHub CI was not run because nothing is pushed

### Upgrading
- Copy .env.example to .env, choose the password and run docker compose up -d db --wait.

## V0.1.17 (2026-10-17)

A contributing guide, a pull request template and two issue forms, so contributors know the rules before they write and reviewers check the same things every time.

### Added
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: the tools and commands; adding an entity step by step from `cargo new-entity`; editing one (ids never change, numbers are reported rather than written, wrong collected facts go under `overrides`); the sourcing rules (a source for anything a reader could dispute, which sources to prefer and avoid, facts read against Wikidata and the project's site, no collected numbers, neutral tone with the marketing words to leave out, your own words, no unreviewed AI text, stable links); the fit-rating rubric with an example of each rating from Rust and React; the writing style for each field; the review checklist; and how to contribute code.
- **A pull request template** with the kind of change, the main checklist items and a place for sources and AI use.
- **Two issue forms**: *Request an entity* (name, kind, website, why it belongs, a check against `planned.yaml`) and *Report a data problem* (entity, page, kind of problem, what is wrong, the correction with its source). The report's `entity` and `page` fields can be filled from the address (`?template=report-data-problem.yml&entity=rust&page=…`), which the website's "Report a data problem" link will use (V0.1.117). A config file adds a link to private security reporting next to them.
- **Links to the guide** from the README, the data model's `fit` section, `CLAUDE.md`, and the template `cli new` writes, whose fit comment now points at the rubric.

### Not done
- **The templates have not been seen on GitHub**, because nothing is pushed. The two forms and their config were validated against SchemaStore's schemas for GitHub issue forms, which also refused a form with a missing label or an unknown field type.

### Upgrading
- **The owner creates the labels** `entity request` and `data problem` on GitHub; until then the forms work but add no label.

## V0.1.16 (2026-10-17)

`cli new` starts an entity file from a commented template, and validation lists every `TODO` left in it.

### Added
- **`cli new <kind> <id>`**, run as `cargo new-entity language go`, writes `content/<kind>/<id>.yaml`: the schema line, every field with a comment explaining it, the allowed values of each trait and domain taken from `content/vocab/`, `TODO` where a value is needed, and the optional fields empty. It takes the id off `planned.yaml` line by line, so the file's comments stay, whether the id is on a `- id` line or in a `[a, b]` list. It refuses an invalid id, an id that already has a file of any kind, and an id planned as another kind (`"python" is planned as a language; run cli new language python`), and then writes nothing.
- **Validation refuses any `TODO` left** in an entity file, by field and in the file's order (`strengths[0].title: replace the TODO placeholder`). While placeholders remain, nothing else is reported for that file, so a new file fails only on them; once they are filled, the usual checks apply. The id still counts as known, so other files can refer to it.
- **`content::id_problem`**, the id rule (a slug, without `vs` as a word) shared by validation and `cli new`, and `Kind` parses from `language`, `framework` or `runtime`.
- **Tests**: a new file of each kind fails only on its placeholders (36 for a language) and passes once they are filled; `planned.yaml` loses the ids from both list forms and keeps its comments; each refusal leaves the folder as it was; a `TODO` left in an otherwise finished file is the only problem reported; the finder reads fields in order, including integer keys, and ignores comments.

### Upgrading
- Nothing to do. Use `cargo new-entity <kind> <id>` to start an entity instead of copying an existing file.

## V0.1.15 (2026-10-17)

Entities name their logo by its Simple Icons slug, so the web app can show it without the repository storing any SVG.

### Added
- **`ids.simple_icons`**, the slug of the entity's logo in [Simple Icons](https://simpleicons.org), whose icons are CC0. The web app will import the icon from the simple-icons package (V0.1.102), so no SVG is stored here and there is none to sanitize. The slug is validated like the other ids: lowercase letters and digits with single underscores between words, the format all 3,466 slugs Simple Icons lists follow. It only names a logo, so an entity still needs another id to collect data with.
- **Rust and React have one**: `rust` and `react`, both in Simple Icons' slug list.
- **The data model and the JSON Schemas** describe the field.
- **Tests**: real slugs and malformed ones in the id table, a malformed slug through the validator, and an entity whose only id is its logo.

### Fixed
- **Six test strings spanned a line break** (in the V0.1.9 and V0.1.10 tests) instead of using `\n`; they read the same now as the rest. Nothing they test changed.

### Changed
- **`ExternalIds::is_empty` is now `has_source`**, which leaves out `simple_icons`.

### Upgrading
- Nothing to do. Add `simple_icons` to an entity when Simple Icons has its logo; the slug is in the URL of the icon's page on simpleicons.org.

## V0.1.14 (2026-10-17)

`ids.so_survey` takes one name or a name per survey year, because the Stack Overflow survey renames technologies between years.

### Added
- **A name per survey year.** `so_survey` is either one name, as before, or a map of years to names, each applying from its year until the next one listed (`model::SurveyName`, with `for_year` to look a name up). Validation reports a year outside the survey's range (2011, its first year, to this year), an empty map, and a name in the wrong format with its year (`ids.so_survey.2023`). The JSON Schemas accept both forms.
- **React's survey names by year**: "React.js" in the 2022 survey and "React" from 2023, as the survey's own result pages for 2022 to 2025 show. Rust is "Rust" in every year and keeps one name.
- **The data model** explains both forms, with React as the example.
- **Tests**: `for_year` before, at and after each listed year; one name, a map, and the range's first and last years parse; 2010, next year, an empty map and a malformed name are each reported once.

### Not done
- **The CSV headers themselves were not read.** React's and Rust's names were checked against the survey's published result pages for 2022 to 2025 (2023 to 2025 are the years decision D8 imports); the import in V0.1.70 lists any name that matches no column.

### Upgrading
- Nothing to do: a single name still works. Code reading `ExternalIds::so_survey` gets a `SurveyName` instead of a `String`, and `ExternalIds::iter` no longer lists it.

## V0.1.13 (2026-10-17)

Every entity says when to choose it, in 2 to 4 short statements that compare pages will show side by side.

### Added
- **`choose_when`**, a required list of 2 to 4 statements, each completing the sentence "Choose <name> when …" and at most 140 characters. Validation reports a list that is too short or too long, an empty or repeated statement, and a statement over the limit with its length. Compare pages will show them side by side (comparison rule 5 in the data model).
- **Rust and React** have three statements each: for Rust, C-level control with memory and data-race safety, a fast single binary, and a team ready for the learning curve; for React, the widest ecosystem for interactive web interfaces, one component model on the web and mobile, and freedom to pick libraries or a framework on top.
- **The data model** describes the field, how to write a statement, and the new rule; the JSON Schemas list it as required.
- **A test** for one, five, an over-long and a missing list, and four statements with one at exactly 140 characters.

### Upgrading
- Every entity file needs a `choose_when` list now; validation names the files that lack one. Run `cargo schema` if you have local schema changes.

## V0.1.12 (2026-10-17)

Editors complete fields and allowed values in entity files and flag mistakes while you type, from JSON Schemas generated from the model.

### Added
- **`cli schema`** (`cargo schema`) writes a JSON Schema per entity kind to `schemas/` (`language.schema.json`, `framework.schema.json`, `runtime.schema.json`), generated from the model types. Unknown fields are refused, `kind` is fixed to the folder's kind, and every vocabulary value (traits, domains, ecosystems) is an enum with its label and description shown next to each completion. They use JSON Schema draft-07, which the YAML language server reads.
- **Each entity file's first line points at its schema** (`# yaml-language-server: $schema=../../schemas/language.schema.json`), so VS Code with the YAML extension, and any editor using the YAML language server, picks it up with no setup. A test checks that every entity file has the line.
- **Stale schemas fail CI.** `cargo schema --check` writes nothing and fails when a committed schema differs from what the model and vocabularies produce; it runs as a CI step and as a test, so changing a model type or a vocabulary without regenerating fails both.
- **`content::load_vocab`** reads only the vocabularies, so schemas can be written while other files still have problems.
- **New dependencies**: `schemars` 1.2.2 (MIT, held a version behind the newest, released five days ago) with its `url2` feature, and `serde_json` 1.0 (MIT OR Apache-2.0). They bring `dyn-clone`, `itoa`, `ref-cast`, `serde_derive_internals` and `zmij`, all under licenses `cargo deny` allows.

### Not done
- **The editor itself was not opened.** Instead, the committed schemas were checked with a JSON Schema validator (Python `jsonschema`, draft-07): `rust.yaml` and `react.yaml` are valid, and `stars: 100`, `memory: [magic]`, `kind: framework` in a language and `ecosystem: web` are each flagged.

### Upgrading
- Install the YAML extension in VS Code (or another editor with the YAML language server) to get completions. Run `cargo schema` after changing a model type or a vocabulary, and commit the result.

## V0.1.11 (2026-10-17)

The validator closes five gaps: external ids are checked against their source's format, years must be plausible, timelines must be in order, and ids can't break compare URLs.

### Fixed
- **Every external id is checked against its source's format.** Only Wikidata and GitHub ids were checked, and loosely: `npm: ""` passed. Each kind now has its registry's rules: npm names and scopes, PyPI project names, crates.io names, NuGet ids, Packagist `vendor/package`, gem names, pub.dev and Hex names, Maven `group:artifact`, Stack Overflow tags, the survey name and endoflife.date slugs. The message says what a valid id looks like, with an example. The table is in the data model.
- **GitHub ids refuse `.`, `..` and names ending in `.git`**, and owners follow GitHub's rule of letters, digits and single hyphens.
- **Years must be from 1940 to next year.** A timeline date or an overridden inception of `0000` passed; the bound is the first programmable computers, and next year leaves room for announced milestones. `model::current_year()` reads the year from the system clock.
- **Timelines must be in date order.** A milestone dated before the one above it is reported with both dates; a year sorts before the months in it.
- **Ids may not contain `vs` as a word** (`rust-vs-go`, `vs-code`, `testlang-vs`), in entity files and in `planned.yaml`, because compare URLs use `-vs-` to separate ids. `canvas` is fine.

### Added
- **Table-driven tests**: real and malformed ids for each of the 14 id kinds, one malformed id per kind through the whole validator, GitHub's dot names and `.git` suffix, years on both sides of each bound, timelines in and out of order, and `vs` in entity and planned ids.

### Upgrading
- Nothing to do: Rust and React pass every new rule. A content file with an id in a registry's wrong format, a year outside 1940 to next year, or milestones out of order now fails validation.

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
