# Crispy-Potato

An open-source reference for programming languages and frameworks: what each one is good at, where it falls short, how it is trending, and how it compares to the alternatives. Every number comes with its source and date.

## Status

Early. The data model and content validation exist; the data collection engine, the API and the web app don't yet.

| Part | Where | State |
|------|-------|-------|
| Data types | `crates/model` | done |
| Content loading and validation | `crates/engine` | done |
| `validate` command | `crates/cli` | done |
| Content | `content/` | Rust and React written; the rest of the v1 scope is in `content/planned.yaml` |
| Data collection, API, web app | | not started |

## Contributing content

Entities are YAML files in `content/`. After editing one, run:

```bash
cargo validate
```

It reports every problem with its file and field. See [docs/data-model.md](docs/data-model.md) for what each field means.

## Docs

- [Architecture](docs/architecture.md)
- [Data model](docs/data-model.md)
- [Release plan to V0.2.0](docs/RELEASES_0.1.md)
