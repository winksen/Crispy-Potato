# Crispy-Potato

An open-source reference for programming languages and frameworks: what each one is good at, where it falls short, how it is trending, and how it compares to the alternatives. Every number comes with its source and date.

## Status

Early. Content tooling, Postgres storage, sync and the collection framework exist. Provider adapters, the API and the web app are still planned.

| Part | Where | State |
|------|-------|-------|
| Data types | `crates/model` | done |
| Content loading and validation | `crates/engine` | done |
| `validate` command | `crates/cli` | done |
| Content | `content/` | Rust and React written; the rest of the v1 scope is in `content/planned.yaml` |
| Storage and sync | `crates/engine`, `compose.yaml` | migrations, offline checked SQL, reader/writer roles and transactional sync |
| Collection framework | `crates/engine` | bounded HTTP, conditional caching, host quotas and a per-entity runner; provider adapters arrive in track F |
| API and web app | | not started |

## Contributing content

[CONTRIBUTING.md](CONTRIBUTING.md) has the whole process: adding and editing an entity, the sourcing rules, how domain fit is rated and what reviewers check.

Entities are YAML files in `content/`. To start a new one, run `cargo new-entity <kind> <id>`, for example:

```bash
cargo new-entity language go
```

It writes `content/languages/go.yaml` with every field, a comment on each and `TODO` placeholders, and takes `go` off `content/planned.yaml`. After filling it in, or after editing any entity, run:

```bash
cargo validate
```

It reports every problem with its file and field, including each `TODO` left. See [docs/data-model.md](docs/data-model.md) for what each field means.

Editors that use the YAML language server (VS Code with the Red Hat YAML extension, for example) complete fields and allowed values and flag mistakes as you type: each entity file's first line points at its JSON Schema in `schemas/`. After changing a model type or a vocabulary, regenerate them with `cargo schema`.

## Docs

- [Mindmap](docs/MINDMAP.md): what is built
- [Architecture](docs/architecture.md)
- [Development](docs/DEVELOPMENT.md): local Postgres, settings, migrations, sync and collection commands
- [Data model](docs/data-model.md)
- [Release plan to V0.2.0](docs/RELEASES_0.1.md)
- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)
- [Threat model](docs/security/threat-model.md)

## Security

To report a vulnerability, use GitHub's private vulnerability reporting, as described in [SECURITY.md](SECURITY.md). Please don't open a public issue for it.

## License

- **Code and documentation** (everything outside `content/`) are dual-licensed under the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.
- **Content** (everything in `content/`: entity files, vocabularies and code samples) is licensed under [Creative Commons Attribution-ShareAlike 4.0 International](content/LICENSE) (CC BY-SA 4.0).
- **Data collected from external sources** keeps its source's license and attribution; see [the data model](docs/data-model.md#sources-and-provenance).

Unless you explicitly state otherwise, a contribution you submit for inclusion is licensed the same way, without any additional terms or conditions: code under MIT OR Apache-2.0, content under CC BY-SA 4.0.
