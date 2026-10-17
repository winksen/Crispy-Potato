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

The container's user and database are fixed as `cp_owner` and `crispy_potato`. Content-only commands need no database.

## Migrations (V0.1.19)

Set `$env:DATABASE_URL = 'postgres://cp_owner:<password>@localhost:5432/crispy_potato'` in PowerShell (URL-encode special password characters), then run `cargo run -p cli -- migrate`. Running it again is safe: sqlx records applied migration checksums. Never edit a shipped migration; add another. `cargo test` uses this URL to create isolated test databases, so the test user needs CREATEDB. Never point tests at production.

The engine pool allows five connections, waits at most five seconds to acquire one, drops idle connections after five minutes, and gives each statement 30 seconds. The CLI reports connection and migration failures without printing connection strings. New dependencies: sqlx 0.8 (Postgres, rustls, migrations, macros, JSON) and Tokio 1.
