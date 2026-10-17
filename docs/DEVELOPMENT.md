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
