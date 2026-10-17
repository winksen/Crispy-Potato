# Data model

The Rust types for everything below live in [`crates/model`](../crates/model/src/entity.rs).

## Two kinds of data

|              | Curated | Collected |
|--------------|---------|-----------|
| Lives in     | `content/` YAML files in git | Postgres, written by the engine |
| Written by   | contributors, through PRs | engine ingestion |
| Examples     | summary, strengths, weaknesses, use cases, relations, external IDs | stars, downloads, survey results, releases, creation date, designers |
| Changes      | when someone edits a file | on a schedule |

The rules:

- Curated files never contain a number the engine can collect. The schema rejects unknown fields, so a `stars:` field in YAML fails validation.
- Collected data never contains opinions.
- When a collected fact is wrong (Wikidata is sometimes inconsistent), the curated file can override it under `overrides`, with a reason.

## Entities

Every page is an entity.

- `id`: a lowercase slug, stable forever, used in URLs. Examples: `rust`, `csharp`, `cpp`, `nextjs`, `aspnet-core`. `vs` may not be one of its words, because compare URLs use it to separate ids (`/compare/go-vs-rust`).
- `kind`: `language` | `framework` | `runtime`. Libraries will be a fourth kind after v1.

`kind` describes the entity's role on the site, not what its authors call it. React calls itself a library, but it is a `framework` here because it sits alongside Vue and Angular.

### Content layout

```
content/
  languages/rust.yaml        one file per entity, named after its id
  frameworks/react.yaml
  runtimes/nodejs.yaml
  planned.yaml               ids in scope that don't have a file yet
  samples/rust/hello.rs      code samples, referenced from the YAML
  vocab/                     allowed values, with labels and descriptions
    domains.yaml
    traits.yaml
```

Each entity's `kind` must match its folder.

Each entity file starts with a line that points editors at its JSON Schema:

```yaml
# yaml-language-server: $schema=../../schemas/language.schema.json
```

The schemas in `schemas/` (`language`, `framework` and `runtime`) are generated from the model types by `cargo schema`, with the vocabulary values as enums, so editors complete fields and values and flag unknown fields. `cargo schema --check`, run in CI and by `cargo test`, fails when they are out of date.

### Examples

- [`content/languages/rust.yaml`](../content/languages/rust.yaml)
- [`content/frameworks/react.yaml`](../content/frameworks/react.yaml)

### Planned entities

[`content/planned.yaml`](../content/planned.yaml) lists the ids in the v1 scope that don't have a file yet, grouped by kind. Other files may refer to a planned id, so Rust can list `go` as an alternative before `go.yaml` exists. The UI can show planned entities as plain text instead of links.

When you add a file, remove its id from `planned.yaml`. Validation fails while an id is in both places.

## Curated fields

### All kinds

| Field | Required | Notes |
|-------|----------|-------|
| `id`, `kind`, `name` | yes | |
| `aliases` | no | for search: `golang`, `js`, `c#`; each alias must be unique across entities |
| `tagline` | yes | one sentence, at most 100 characters |
| `summary` | yes | 2–4 sentences, neutral tone |
| `links` | yes | `website` is required; `docs` and `repo` are optional |
| `ids` | yes | how the engine finds this entity in each source; at least one |
| `traits` | yes | depends on the kind (see below) |
| `domains` | yes | how well it fits each kind of project, with a note when the fit isn't obvious |
| `strengths`, `weaknesses` | yes | 2–6 each; `title`, `detail`, optional `sources` |
| `choose_when` | yes | 2–4 situations, each completing "Choose <name> when …", at most 140 characters (see below) |
| `relations` | no | typed links to other entities |
| `timeline` | no | milestones that aren't releases (releases are collected); `date`, `title`, optional `url` |
| `learning` | yes | `curve` plus at least one resource |
| `notable_users` | no | `name`, `what`, `source` |
| `samples` | no | paths under `content/samples/` |
| `overrides` | no | corrections to collected facts, each with a `reason` |

Dates may be a year, a month or a day: `2013`, `2013-05` or `2015-05-15`. The year must be from 1940 to next year, and `timeline` milestones are listed in date order (a year sorts before the months in it).

### `choose_when`

Each statement completes the sentence "Choose Rust when …", so it starts in lower case and leaves out those words:

```yaml
choose_when:
  - you need the performance and control of C or C++ and want the compiler to rule out memory errors and data races
  - you are building a command-line tool, a network service or a WebAssembly module that should ship as one fast binary
```

Write 2 to 4, each at most 140 characters. They describe situations in which a reader should pick this entity over its alternatives, rather than repeating its strengths. Compare pages show them side by side (comparison rule 5).

### `ids`: the link to automation

| Key | Used by |
|-----|---------|
| `wikidata` | creation date, designers, developer, license, influences |
| `github` | stars, contributors, commit activity, releases (`owner/repo`) |
| `npm`, `pypi`, `crates`, `nuget`, `packagist`, `rubygems`, `pub`, `hex`, `maven` | registry downloads and versions |
| `stackoverflow_tag` | question counts over time |
| `so_survey` | the name used in the Stack Overflow Developer Survey: one name, or a name per survey year (see below) |
| `endoflife` | the endoflife.date product slug: support cycles, LTS dates |

Each id is checked against its source's format, so a typo fails validation rather than a lookup:

| Key | Format | Example |
|-----|--------|---------|
| `wikidata` | `Q` and a number without leading zeros | `Q575650` |
| `github` | `owner/repo`: an owner of letters, digits and single hyphens (at most 39), a name of letters, digits, `-`, `_` and `.` (at most 100) that is not `.` or `..` and doesn't end in `.git` | `rust-lang/rust` |
| `npm` | lowercase letters, digits, `-`, `.`, `_` and `~`, not starting with `.` or `_`, optionally scoped; at most 214 characters | `@angular/core` |
| `pypi` | letters, digits, `-`, `_` and `.`, starting and ending with a letter or digit | `django` |
| `crates` | up to 64 letters, digits, `-` and `_`, starting with a letter | `axum` |
| `nuget` | words of letters, digits and `_` joined by single `.` or `-`; at most 100 characters | `Microsoft.Maui.Controls` |
| `packagist` | `vendor/package` in lowercase letters and digits with single `-`, `_` or `.` between them | `laravel/framework` |
| `rubygems` | letters, digits, `-`, `_` and `.`, starting with a letter or digit and holding a letter | `rails` |
| `pub` | lowercase letters, digits and `_`, not starting with a digit; at most 64 characters | `flutter` |
| `hex` | lowercase letters, digits and `_`, starting with a letter | `phoenix` |
| `maven` | `group:artifact`, the group in dotted words | `org.springframework.boot:spring-boot` |
| `stackoverflow_tag` | up to 35 lowercase letters, digits and `+ # . -` | `c#` |
| `so_survey` | the survey's name, up to 60 characters, no leading or trailing spaces; or a map of survey years (2011 to this year) to such names | `Rust` |
| `endoflife` | lowercase letters and digits joined by single hyphens | `spring-boot` |

### Survey names by year

The survey sometimes renames a technology between years: React was "React.js" in the 2022 survey and "React" from 2023. Check the column headers of each year's results CSV, then write either one name, used for every year:

```yaml
so_survey: Rust
```

or each name with the first survey year that used it. A name applies until the next year listed, and years before the first one have no name:

```yaml
so_survey:
  2022: React.js
  2023: React
```

Years must be survey years: from 2011, when the survey started, to this year.

### Traits

Every trait is a list, even when it usually has one value (`memory: [ownership]`). The allowed values, with labels and descriptions, are in [`content/vocab/traits.yaml`](../content/vocab/traits.yaml). To use a new value, add it there first.

**Languages** (all required):

| Field | Values |
|-------|--------|
| `typing` | `static`, `dynamic`, `strong`, `weak`, `inferred`, `gradual` |
| `memory` | `gc`, `ownership`, `manual`, `arc` |
| `execution` | `compiled-native`, `bytecode-vm`, `interpreted`, `jit`, `transpiled` |
| `paradigms` | `imperative`, `procedural`, `object-oriented`, `functional`, `generic`, `concurrent`, `declarative` |
| `concurrency` | `threads`, `async`, `actors`, `green-threads`, `event-loop`, `csp`, `coroutines` |
| `targets` | `linux`, `macos`, `windows`, `browser`, `wasm`, `ios`, `android`, `embedded` |

**Frameworks:**

| Field | Required | Values |
|-------|----------|--------|
| `ecosystem` | yes | a single value from `domains`: `web-frontend`, `web-backend` or `mobile` in v1 |
| `languages` | yes | language ids, such as `[javascript, typescript]` |
| `architecture` | no | `component`, `mvc`, `minimal`, `batteries-included`, `meta-framework` |
| `rendering` | no | `csr`, `ssr`, `ssg`, `native`, `own-renderer` |
| `platforms` | no | `ios`, `android`, `web`, `desktop` |

**Runtimes** (all required):

| Field | Values |
|-------|--------|
| `languages` | ids of the languages it runs, such as `[java, kotlin, scala]` for the JVM |
| `targets` | same values as for languages |

### Other vocabularies

**`domains`** ([`content/vocab/domains.yaml`](../content/vocab/domains.yaml)):

- `web-frontend`, `web-backend`, `mobile`, `desktop`
- `systems`, `embedded`, `cli`, `scripting`, `devops`
- `data-science`, `machine-learning`, `scientific`
- `games`

**`fit`** is one of `excellent` | `good` | `possible` | `poor`. A `note` is required for `possible` and `poor`, and for any rating a reviewer questions.

**`learning.curve`** is one of `gentle` | `moderate` | `steep`.

**`learning.resources[].kind`** is one of `book` | `course` | `docs` | `exercises` | `tutorial` | `video`.

### Relations

Each relation is stored on one side only. The engine will derive the reverse direction on sync.

| Relation | Direction | Example | Rule |
|----------|-----------|---------|------|
| `alternatives` | symmetric | `vue` ↔ `react` | same kind |
| `built_on` | A is built on B | `nextjs` → `react` | |
| `runs_on` | A runs on B | `express` → `nodejs` | B is a runtime |
| `often_with` | symmetric | `react` ↔ `typescript` | |
| `succeeded_by` | A was replaced by B | `angularjs` → `angular` | |

`written_in` and `influenced_by` come from Wikidata and can be overridden.

### Overrides

```yaml
overrides:
  inception:
    value: 2012
    reason: Wikidata gives the date of the first commit; we use the first public release.
```

Fields that can be overridden: `inception`, `designers`, `developer`, `license`, `influenced_by`, `written_in`.

## Collected data

### Observations

One row per measurement:

| Column | Example |
|--------|---------|
| `entity_id` | `react` |
| `metric` | `npm.downloads` |
| `value` | number |
| `unit` | `count`, `pct` |
| `period` | `instant`, `week`, `month`, `year` |
| `observed_at` | date the value applies to |
| `source_id` | `npm` |

Metric names are namespaced by source:

| Metric | Source |
|--------|--------|
| `github.stars`, `github.forks`, `github.contributors`, `github.commits_30d` | GitHub API |
| `npm.downloads`, `pypi.downloads`, `crates.downloads`, `nuget.downloads`, … | package registries |
| `so.questions_total`, `so.questions_30d` | Stack Exchange API |
| `so_survey.used_pct`, `so_survey.admired_pct`, `so_survey.desired_pct` | Stack Overflow Developer Survey (yearly) |

### Releases

`entity_id`, `version`, `released_at`, `eol_at`, `lts`, `source_id`. The sources are GitHub releases and endoflife.date.

### Sources and provenance

Every observation points to a row in `sources`: `name`, `url`, `license` and `attribution`. The UI shows the source and date on every number (for example, "GitHub · 17 Oct 2026"), and `/about/data` lists every source with its license.

License notes:

- Wikidata is CC0, free to reuse.
- The Stack Overflow survey is ODbL, which requires attribution and requires databases built from it to use the same license.
- TIOBE and StackShare data are not used, because they can't be redistributed.

## Comparison rules

Raw numbers mislead when compared carelessly, so the engine enforces these rules:

1. **Compare like with like.** Languages are compared with languages, and frameworks only within the same ecosystem.
2. **Same source only.** A metric is shown side by side only when every entity has it from the same source. npm downloads and PyPI downloads are never placed next to each other.
3. **Missing is not zero.** Some ecosystems publish no download counts at all (Maven Central, for example). The UI shows "not published", not an empty bar.
4. **Trends travel better than totals.** Where totals can't be compared, the engine compares 12-month growth instead.
5. **No single overall score.** A comparison shows the signals plus each entity's curated `choose_when` statements, side by side.

## Validation

Run `cargo validate` from the repository root. It reports every problem at once, each with its file and field, as `languages/rust.yaml: traits.memory[0]: …`: the path is relative to the content folder and uses forward slashes on every OS. The rules are in [`crates/engine/src/content/check.rs`](../crates/engine/src/content/check.rs).

- **Schema:** required fields are present, there are no unknown fields, dates are real dates from 1940 to next year and URLs use http or https.
- **External ids:** each one matches its source's format (see the table under `ids`).
- **Files:** each file name matches its `id`, the `kind` matches the folder, and ids are unique slugs without `vs` as a word. Extensions are matched without regard to case: `.yml` is refused, and `Rust.YAML` is read and asked to use a lowercase `.yaml`.
- **Vocabularies:** every trait and domain value is listed in `content/vocab/`.
- **References:** every relation and every `traits.languages` entry points to an entity with a file or one listed in `planned.yaml`, following the rules in the relations table. Nothing refers to itself.
- **Style:**
  - `strengths` and `weaknesses` have 2–6 entries each;
  - `choose_when` has 2–4 statements of at most 140 characters;
  - the tagline is at most 100 characters;
  - `possible` and `poor` fits have a note;
  - no value is listed twice;
  - timeline milestones are in date order;
  - aliases are unique.
- **Samples:** every path exists under `content/samples/`.
- **Overrides:** every override has a non-empty `reason`.
- **Limits:** content comes from pull requests and is read as untrusted input. Nothing under `content/` may be a symbolic link (or a Windows junction), no file may be larger than 256 KiB, and each YAML file must hold one document that nests at most 16 levels deep, with at most 16 anchors, 64 aliases and 50,000 values; alias expansion is capped while the file is read.
