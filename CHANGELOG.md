# Changelog

What changed in each release, newest first, with what to do when upgrading. Versions follow [semantic versioning](https://semver.org): the 0.1.x releases lead to `0.2.0`, the public preview. Every release, shipped and planned, has a row in the [release index](docs/RELEASES_0.1.md), and the [mindmap](docs/MINDMAP.md) describes what is built.

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
