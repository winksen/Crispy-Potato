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

The container's user and database are fixed as `cp_owner` and `crispy_potato`. Content-only commands need no database.
