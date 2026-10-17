# Local development

Install Rust through rustup and Docker with Compose. Run commands from the repository root.

```powershell
Copy-Item .env.example .env
# Edit .env and choose POSTGRES_PASSWORD, then:
docker compose up -d db --wait
docker compose exec db psql -U cp_owner -d crispy_potato -c 'SELECT version();'
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo validate
cargo schema --check
cargo deny check
```

Postgres 17 is pinned by its multi-platform image digest. Only localhost can reach the published port. `docker compose stop db` stops it; `docker compose down` removes the container but keeps the named data volume. The password initializes a new volume only: changing `.env` does not change an existing database password. Use `psql`'s `\password` to rotate it. Do not commit `.env`.

`docker compose exec db psql -U cp_owner -d crispy_potato` opens a SQL shell without installing psql on the host. Application commands will be added as their releases ship. Cargo does not automatically load `.env`; Compose does.

## Settings

Every new application setting must be documented here when introduced.

| Setting | Read by | Default / purpose |
|---|---|---|
| `POSTGRES_PASSWORD` | Compose / Postgres image | Required; local database owner's password |
| `POSTGRES_PORT` | Compose | `5432`; localhost port published for Postgres |
| `DATABASE_URL` | CLI / sqlx tests | Required for database commands; Postgres connection URL; use owner credentials for migrations and a disposable local owner database for tests |
| `API_DATABASE_URL` | API reader connection | Required for reader connections; use `cp_reader`, never the writer or owner; no fallback |
| `SQLX_OFFLINE` | sqlx macros | `true` in `.cargo/config.toml`; builds use committed `.sqlx/` metadata. Set `false` only when preparing/checking against a migrated database |

The container's user and database are fixed as `cp_owner` and `crispy_potato`. Content-only commands need no database.

## Content sync (V0.1.21)

After migrations, run `cargo run -p cli -- sync` (optionally `--content <folder>`). It validates a stable snapshot, then writes everything in one transaction guarded by an advisory lock. Invalid content writes nothing. A second identical run prints `0 changed, 0 retired; content unchanged`. A sample-only or vocabulary-only edit still updates the content version even when no entity document changes. Removed entities retire; their observations remain. Sync needs data-write privileges, not schema ownership.

## Migrations (V0.1.19)

Set `$env:DATABASE_URL = 'postgres://cp_owner:<password>@localhost:5432/crispy_potato'` in PowerShell (URL-encode special password characters), then run `cargo run -p cli -- migrate`. Running it again is safe: sqlx records applied migration checksums. Never edit a shipped migration; add another. `cargo test` uses this URL to create isolated test databases, so the test user needs CREATEDB. Never point tests at production.

The engine pool allows five connections, waits at most five seconds to acquire one, drops idle connections after five minutes, and gives each statement 30 seconds. The CLI reports connection and migration failures without printing connection strings. New dependencies: sqlx 0.8 (Postgres, rustls, migrations, macros, JSON) and Tokio 1.

## Checked queries (V0.1.22)

All application SQL uses sqlx macros; test fixtures use runtime queries. Install `cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features rustls,postgres`, set the local owner `DATABASE_URL`, then run:

```powershell
cargo sqlx migrate run --source crates/engine/migrations
$env:SQLX_OFFLINE = 'false'
cargo sqlx prepare --workspace -- --all-targets
cargo sqlx prepare --check --workspace -- --all-targets
Remove-Item Env:SQLX_OFFLINE
```

Commit `.sqlx/` with every query change. Ordinary `cargo build` needs no database. CI checks metadata against a fresh migrated Postgres service; a bad column fails macro expansion.

## Collection runner (V0.1.27)

`cargo run -p cli -- ingest <source> [--entity <id>] [--dry-run]` invokes registered adapters. Concrete providers arrive in track F (V0.1.56 onward); until then unknown source names fail explicitly. The `Source` contract supplies provenance, schedule, applicability and a typed batch of observations, releases and facts, plus accepted HTTP validators. The runner reads active synced entities, records real runs, commits each entity atomically and continues after fetch or storage failures. Mixed success gives `partial`; all failures give `failed`. Counts are entities accepted or unchanged, not individual rows.

Dry runs print the batches they would store and write nothing, including no run or source rows. The Postgres fake-source test verifies a successful measurement, a failed second entity and a recorded partial run. Diagnostics store entity ids and fixed error categories, never response bodies or URLs. No settings added.

Since V0.1.28 adapters parse response bytes with `untrusted::decode<T>` into source-specific structs. It enforces the 20 MiB cap and JSON recursion limit and returns fixed errors for missing fields and malformed JSON. The runner always sanitizes batches before either storage or dry-run output: source strings become plain text capped at 256 characters, with controls, bidi controls and angle brackets removed; identifiers are bounded; numbers must be finite and in range; dates must be full dates. Fact JSON is limited to depth 16 and 10,000 values; batches to 10,000 records. Bad responses fail only their entity. Future UI code must still render these fields as text.

## HTTP transport (V0.1.24)

Since V0.1.26 all clients share a capacity-one token bucket and concurrency semaphore per hostname, including retries and redirects. `engine::limits::QUOTAS` holds the source/host table: conservative one-per-second defaults for known APIs, one concurrent request for crates.io, two for other hosts; unlisted hosts use 100 ms spacing and two slots. These are application limits, not advertised provider allowances. Cross-origin redirects drop authorization and cookies.

Stack Exchange's typed `backoff` value delays the entire host (stricter than its per-method requirement), including callers already waiting. See [Stack Exchange throttling](https://api.stackexchange.com/docs/throttle) and [crates.io policies](https://crates.io/policies). Fake-clock tests cover spacing, sharing and concurrency. No settings were added.

Since V0.1.25 `http_cache::get` sends stored ETag and Last-Modified validators and returns `Changed` or `Unchanged`. A caller must persist the returned cache update in the same transaction as accepted data; malformed or rolled-back imports therefore cannot poison the cache. `record_unchanged` commits a 304's validators and run counter without touching observations. Cache keys hash the URL and request headers, keeping credential-bearing URLs out of storage. No new settings.

`engine::http::Client` uses reqwest 0.12 with rustls only, gzip and brotli decoding, a five-second connect timeout and a 30-second deadline per attempt (including body reads). Responses are limited to 20 MiB after decoding, both by declared length when available and while streaming. The User-Agent includes the crate version and repository URL.

There are three attempts total (the roadmap's “giving up after 3 tries” criterion), with exponential backoff starting at 500 ms plus jitter for 429, 5xx and timeouts. `Retry-After` seconds and HTTP dates are honored; delays over five minutes return a deferred error instead of retrying early. Other transport errors are not retried. Errors contain neither URLs nor response bodies. No new environment settings. Dependencies: reqwest 0.12, httpdate 1 and wiremock 0.6 for tests.

## Database roles (V0.1.23)

After owner migrations, run `Get-Content scripts/db/roles.sql -Raw | docker compose exec -T db psql -U cp_owner -d crispy_potato -v ON_ERROR_STOP=1`. This idempotent script provisions `cp_reader` (SELECT) and `cp_writer` (SELECT/INSERT/UPDATE/DELETE and sequences), denies schema creation and temporary tables, and protects migration history. Run it in every database as the same owner that runs migrations so default privileges apply to future tables.

In the owner's interactive psql shell, use `\password cp_reader` and `\password cp_writer`. Set `DATABASE_URL` to the writer for sync/collection; `API_DATABASE_URL` to the reader for API access. Keep owner credentials only for migrations and isolated tests. Roles are cluster-wide and must not own objects or inherit other roles. The integration test switches dedicated connections into each role, checks reader SELECT and writer INSERT, and verifies permission-denied errors for reader writes, writer DDL and migration-history changes.
