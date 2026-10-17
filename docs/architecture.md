# Architecture

## Goals

- A reference site for programming languages and frameworks: well designed, backed by data, honest about trade-offs.
- Every number on the site shows its source and the date it was collected.
- Easy to contribute to: hand-written content lives in git as plain YAML files.
- Open source and cheap to run.

## v1 scope

About 50 entities, done well, before adding more.

**Languages (18):** JavaScript, TypeScript, Python, Java, C#, C, C++, Go, Rust, PHP, Kotlin, Swift, Ruby, Dart, Scala, R, Lua, Elixir.

**Framework ecosystems (3):**

| Ecosystem    | Frameworks |
|--------------|------------|
| Web frontend | React, Vue, Angular, Svelte, SolidJS, Next.js, Nuxt, SvelteKit, Astro |
| Web backend  | Express, NestJS, Django, FastAPI, Flask, Spring Boot, ASP.NET Core, Laravel, Ruby on Rails, Gin, axum, Phoenix |
| Mobile       | React Native, Flutter, SwiftUI, Jetpack Compose, .NET MAUI, Kotlin Multiplatform |

**Runtimes (5)**, needed because frameworks run on them: Node.js, Deno, Bun, JVM, .NET.

Libraries come after v1, as a fourth entity kind.

The ids for everything in scope are listed in [`content/planned.yaml`](../content/planned.yaml) until their files are written.

## Components

```
┌──────────────┐  HTTP/JSON   ┌──────────────┐
│ web          │ ───────────▶ │ api          │  thin: routing, validation,
│ React + SSR  │  (OpenAPI)   │ axum binary  │  rate limits, caching headers
└──────────────┘              └──────┬───────┘
                                     │ calls
                                     ▼
┌──────────────┐    calls     ┌──────────────┐        ┌────────────────────┐
│ cli          │ ───────────▶ │ engine       │ ◀───── │ content/ (git)     │
│ validate,    │              │ library      │        │ hand-written YAML  │
│ sync, worker │              │ does the work│ ◀───── │ external sources   │
└──────────────┘              └──────┬───────┘        │ GitHub, Wikidata…  │
                                     │                └────────────────────┘
                                     ▼
                              ┌──────────────┐
                              │ Postgres     │
                              └──────────────┘
```

- **engine**: all the domain logic. It loads and validates content, pulls data from external sources, stores it, computes trends, and answers queries (entity pages, comparisons, search).
- **api**: a live HTTP server that only translates requests into engine calls and engine results into JSON. It contains no business logic and never queries the database directly.
- **cli**: the same engine, run from the command line. Contributors and CI use `validate`; deploys use `sync`; `worker` runs scheduled data collection in production.
- **web**: the React frontend. It talks only to the api.

### Why the engine is a library, not its own server

The api and the cli are two binaries built on the same `engine` crate. This keeps the split we want (the api is only a communication layer, and the engine does the work) without a network hop or an RPC protocol between them:

- one codebase, one set of types, no service-to-service API to version;
- the engine can be tested without HTTP;
- if the engine ever needs to run as a separate service, its public Rust API is already the boundary, and it can be wrapped in gRPC or HTTP then.

## Repository layout

```
Cargo.toml            Cargo workspace
crates/
  model/              shared types: Entity, Observation, Source… (serde, schemars, utoipa)
  engine/             content loading and validation, data collection, storage, queries
  api/                axum HTTP server (binary)
  cli/                validate | sync | ingest <source> | worker (binary)
web/                  React app
content/              hand-written data (see data-model.md)
docs/
```

## Data flow

1. A contributor edits `content/frameworks/svelte.yaml` and opens a PR. CI runs `cargo validate` (an alias for `cli validate`), which checks the schema, the vocabularies and that every cross-reference exists.
2. On merge, the deploy runs `cli sync`, which writes the hand-written data to Postgres.
3. `cli worker` collects external data on a schedule:
   - daily: GitHub, package registries, Stack Exchange
   - weekly: Wikidata, endoflife.date
   - once a year: the Stack Overflow Developer Survey (a CSV import)
4. The api serves the results. Data changes at most daily, so responses get `Cache-Control` and `ETag` headers and can sit behind a CDN.

## Engine

**Content.** Reads `content/**/*.yaml` into `model` types (`deny_unknown_fields`), validates them, and upserts them. Validation runs without a database, so contributors only need `cargo`.

**Ingestion.** One module per source, all behind a common `Source` trait:

- fetch the data for each entity that has an ID for that source;
- normalize it into `Observation`s;
- store them with provenance (which source, when, and under what license).

Each source handles its own rate limits, retries, conditional requests and an identifying User-Agent.

**Derivation.** Computes values from the raw data:

- trends (12-month change);
- rank within an ecosystem;
- activity status (active, maintenance, declining);
- freshness, so the UI can flag stale numbers.

**Queries.** Builds:

- the full payload for an entity page;
- comparison payloads that only line up comparable numbers (see data-model.md);
- filtered lists and search.

Recommendations ("I want to build…") come later.

## Database

Postgres through `sqlx`, with migrations in the engine crate.

| Table            | Holds |
|------------------|-------|
| `entities`       | hand-written data: filterable fields as columns, prose sections as JSONB |
| `relations`      | typed edges between entities; inverse edges derived on sync |
| `observations`   | time series: entity, metric, value, period, observed_at, source |
| `releases`       | versions, release dates, end-of-life and LTS dates |
| `sources`        | name, URL, license, attribution text |
| `ingestion_runs` | when each source ran, status, error counts |

Search starts with Postgres full-text search. A few hundred entities don't need a separate search engine.

## API

- REST + JSON, versioned under `/v1`, read-only, no accounts.
- The OpenAPI spec is generated from Rust types with `utoipa`. The web app's TypeScript client is generated from the same spec, so Rust and TypeScript types can't drift apart. The same spec documents the public API.

| Endpoint | Purpose |
|----------|---------|
| `GET /v1/entities?kind=&ecosystem=&domain=` | lists and filters |
| `GET /v1/entities/{id}` | full entity page payload |
| `GET /v1/entities/{id}/metrics?metric=&from=&to=` | time series for charts |
| `GET /v1/compare?ids=rust,go,cpp` | side-by-side payload |
| `GET /v1/search?q=` | search |
| `GET /v1/sources` | sources, licenses and attribution |

## Web

- React with server-side rendering, so pages are indexed by search engines and load fast. Route loaders call the api from the server.
- Routes:
  - `/`, `/languages`, `/frameworks/:ecosystem`
  - `/languages/:id`, `/frameworks/:id`
  - `/compare/:a-vs-:b`, which matches how people search
  - `/explore` (ecosystem graph)
  - `/about/data` (sources and methodology)

## Later

- **Decision guides**: "I want to build a REST API / a mobile app / a CLI" narrows the catalog to a few candidates with trade-offs. Needs a `guide` content type.
- **AI assistant**: answers questions only from the curated dataset and cites the entries it used. Rate-limited, possibly with users bringing their own API key.
- **Libraries** as an entity kind.
- **Public dataset downloads** (JSON/SQLite) for anyone to reuse.

## Open decisions

- SSR framework for the web app: React Router v7 (framework mode) or Next.js.
- Hosting for the api, worker and Postgres.
- Project name.
