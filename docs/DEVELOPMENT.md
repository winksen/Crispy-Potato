# Local development

## Dependency checks (V0.1.34)

Workspace packages are `cp-model`, `cp-engine` and `cp-cli`; their libraries remain `model` and `engine`, and the binary is still `cli`. The cargo aliases keep their names. Run `cargo deny check` for the active dependency graph, and `osv-scanner scan source --lockfile Cargo.lock` for the whole lockfile. At V0.1.34 the latter reports only the inactive sqlx MySQL `rsa` dependency; see the [audit follow-up](security/audit-2026-10-21.md#v0134-follow-up-2026-10-22).

CI checks dependencies on pushes and pull requests and daily at 04:20 UTC on main. The scheduled report job has only issue-write permission and does not check out the repository or run project code. It updates one marked bot issue and closes it after recovery. Run `python -m unittest discover -s scripts/checks -p 'test_*.py'` to test the reporting transitions locally without contacting GitHub.

## Live content checks (V0.1.32–V0.1.33)

`cargo run -p cp-cli -- check-links` validates content then checks every URL-valued field, including nested sources and learning resources. Each distinct URL is checked once, with diagnostics retained for every referencing field; `checked` counts fields. It uses HEAD with GET fallback, eight concurrent checks, the shared two-per-host cap (or stricter provider limit), and a ten-second total deadline per URL. It reports HTTP failures, timeouts and redirects to a different hostname with the content-relative file and field. Output is JSON (`checked`, `problems`, `skipped`); any problem exits nonzero. URLs and response bodies are not logged. No database or credentials are needed.

`.github/workflows/content-health.yml` runs weekly and by manual dispatch on main. Link and identifier jobs have only `contents: read`; only the separate reporter has `issues: write`. It creates or updates one bot-owned **Content health checks** issue, closes it when clean and reopens the same issue on regression. It does not run on pull requests. The workflow has not run remotely until pushed; local checks never create issues.

`cargo run -p cp-cli -- verify-ids` validates content then verifies each source identifier against its official API: GitHub, npm, PyPI, crates.io, NuGet, Packagist, RubyGems, pub.dev, Hex, Maven Central, Wikidata, Stack Overflow and endoflife.date v1. Requests run eight identifiers at a time through shared host limits, with a 30-second total deadline per identifier. Missing ids, unexpected responses, archived GitHub repositories and renamed repositories (including the new name) report the content file and `ids.<provider>` field. PyPI names are normalized per its package-name rules; scoped npm names and tags are URL-encoded. npm checks fetch the latest version document rather than the full package history.

Wikidata must have a non-deprecated `instance of` (`P31`) software class. The checker follows `subclass of` (`P279`) with cycle detection, at most 32 classes and five levels; exceeding these limits reports an unverified classification. Software (`Q7397`), software library (`Q188860`), software framework (`Q271680`) and programming language (`Q9143`) are accepted roots. Programming languages are explicit because Wikidata models languages separately from their software implementations. Survey labels are checked against survey data in V0.1.80; Simple Icons slugs remain format-validated until the web package check in V0.1.116.

The command uses the same JSON report and exit convention as `check-links`. Missing GitHub/Stack Exchange credentials produce `skipped` entries (decision D5), not verified ids; `checked` counts attempted ids only. The weekly issue includes these skips even if all attempted checks pass. Supply the existing environment/file settings locally. In GitHub, set secrets `COLLECTION_GITHUB_TOKEN` (public read-only token) and `STACKEXCHANGE_KEY` in the `content-collection` environment, restricted to main (existing repository secrets remain fallback values); the issue-writing workflow token is kept separate. Neither command needs a database, stores observations, or prints credentials or response bodies.

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
| `GITHUB_TOKEN` | Settings / GitHub sources | Optional fine-grained public read-only token; missing means skipped |
| `GITHUB_TOKEN_FILE` | Settings | Optional UTF-8 secret file, at most 16 KiB; used when the environment token is empty or absent |
| `STACKEXCHANGE_KEY` | Settings / Stack Exchange sources | Optional API key; missing means skipped |
| `STACKEXCHANGE_KEY_FILE` | Settings | Optional UTF-8 secret file, at most 16 KiB; used when the environment key is empty or absent |
| `SQLX_OFFLINE` | sqlx macros | `true` in `.cargo/config.toml`; builds use committed `.sqlx/` metadata. Set `false` only when preparing/checking against a migrated database |
| `DB_POOL_MAX_CONNECTIONS` | Database pools | Default 5, allowed 1–20 per pool; account for the role's 20-connection limit across instances |
| `DB_ACQUIRE_TIMEOUT_MS` | Database pools | Default 5,000 ms, allowed 100–60,000 ms |
| `DB_STATEMENT_TIMEOUT_MS` | Database pools | Writer default 30,000 ms, allowed 100–300,000 ms; reader default and maximum 5,000 ms |

The container's user and database are fixed as `cp_owner` and `crispy_potato`. Content-only commands need no database.

## Content sync (V0.1.21)

After migrations, run `cargo run -p cp-cli -- sync` (optionally `--content <folder>`). It validates a stable snapshot, then writes everything in one transaction guarded by an advisory lock. Invalid content writes nothing. A second identical run prints `0 changed, 0 retired; content unchanged`. A sample-only or vocabulary-only edit still updates the content version even when no entity document changes. Removed entities retire; their observations remain. Sync needs data-write privileges, not schema ownership.

Since V0.1.41 every curated table uses one bulk `UNNEST` insert/upsert per changed
snapshot, including entities and inverse relations. Database statement count is
independent of the number of entities; validation and serialization still scale
with content size. Existing atomicity, unchanged-snapshot skips and retirement
behavior are preserved.

## Migrations (V0.1.19)

Set `$env:DATABASE_URL = 'postgres://cp_owner:<password>@localhost:5432/crispy_potato'` in PowerShell (URL-encode special password characters), then run `cargo run -p cp-cli -- migrate`. Running it again is safe: sqlx records applied migration checksums. Never edit a shipped migration; add another. `cargo test` uses this URL to create isolated test databases, so the test user needs CREATEDB. Never point tests at production.

The pool defaults to five connections and a five-second acquire deadline, drops idle connections after five minutes, and uses the settings above. Reader connections cap statement timeouts at five seconds and enable read-only transactions; writer connections preserve the role's read-only setting instead of disabling it. `application_name` is `cp-cli` for the current CLI and `cp-api` for the reader helper; future binaries use `connect_named` with their own name. The CLI reports connection and configuration failures without printing connection strings or supplied setting values.

## Checked queries (V0.1.22)

All application SQL uses sqlx macros; test fixtures use runtime queries. Install `cargo install sqlx-cli --version 0.9.0 --no-default-features --features rustls,postgres`, set the local owner `DATABASE_URL`, then run:

```powershell
cargo sqlx migrate run --source crates/engine/migrations
$env:SQLX_OFFLINE = 'false'
cargo sqlx prepare --workspace -- --all-targets
cargo sqlx prepare --check --workspace -- --all-targets
Remove-Item Env:SQLX_OFFLINE
```

Commit `.sqlx/` with every query change. Ordinary `cargo build` needs no database. CI checks metadata against a fresh migrated Postgres service; a bad column fails macro expansion.

## Freshness status (V0.1.30)

Since V0.1.31 credentials use `secrecy` 0.10 containers. Settings Debug/Display and sensitive GitHub headers redact them. Nonempty environment values take precedence over files; surrounding whitespace is trimmed, unreadable files are configuration errors with no file contents or paths printed. The runner records a missing-credential source as `skipped` with a warning, visible in `cli status`, without attempting entities. Stack Exchange keys are query parameters by its API design: never log whole request URLs. HTTP diagnostics are fixed categories; cache keys hash URLs and headers.

After applying migration 0003, `cli status` prints JSON grouped by source: `source`, `schedule_seconds`, `last_run`, `finished_at`, `status`, `failures` and `entities`. Each entity has `last_attempt`, `last_success`, `failure` and `stale`. A missing success or a success older than twice the schedule is stale; a later failed attempt does not refresh it. Retired entities are omitted. Sources with no run have null run fields; only sources registered by a real run and entities attempted by that source are listed. A successful 304 refreshes success without rewriting observations. Dry runs change neither timestamp. No new settings.

## Quarantine (V0.1.40)

`cli quarantine list` prints the newest 100 held observations with ids and reasons. Optional `--entity rust`, `--source github`, `--page 2` and `--page-size 20` filter and page the list; pages start at one and page size is 1–100. Ordering uses held time, then id, both descending. Concurrent holds can change page membership. `cli quarantine release <id>` atomically moves an approved trend outlier into observations; `cli quarantine discard <id>` removes it. Invalid numeric or date ranges cannot be approved. These commands use the writer URL.

Apply migration 0005 as the owner before upgrading. It retains the newest old hold for each measurement, moves out-of-range observations into quarantine, adds a unique identity index and enforces observation date/count constraints. Repeated holds for the same entity/source/metric/period/date update their existing candidate, reason and timestamp. Dates must be from 2008-01-01 through tomorrow UTC, and counts must be below 10^15. Review held values after migrating; recollect corrected source data instead of manually approving invalid ranges.

Counts below zero and percentages outside 0–100 are now held rather than rejecting the whole entity. Non-finite numbers still fail before storage. Cumulative metrics (`*.stars`, `*.downloads_total`, `*.total_downloads`, `crates.downloads`) are held if they fall more than 20% or grow more than tenfold against the latest accepted value for the same entity/source/metric/period at or before that date. A zero baseline has no defined growth ratio. Entity row locks serialize baseline checks. Reports and dry runs include held candidates and reasons; normal observation reads never include quarantine.

## Collection runner (V0.1.27)

`cargo run -p cp-cli -- ingest <source> [--entity <id>] [--dry-run]` invokes registered adapters. Concrete providers arrive in track F (V0.1.66 onward); until then unknown source names fail explicitly. The `Source` contract supplies provenance, schedule, applicability and a typed batch of observations, releases and facts, plus accepted HTTP validators. The runner reads active synced entities, records real runs, commits each entity atomically and continues after fetch or storage failures. Mixed success gives `partial`; all failures give `failed`. Counts are entities accepted or unchanged, not individual rows.

Dry runs print the batches they would store and write nothing, including no run or source rows. The Postgres fake-source test verifies a successful measurement, a failed second entity and a recorded partial run. Diagnostics store entity ids and fixed error categories, never response bodies or URLs. No settings added.

Since V0.1.41 collection locks each entity once per batch. One query fetches the
baseline before each affected series' date range and existing points within that
range; candidates are checked in date order in memory. Rejected candidates do not
advance the baseline. Historical corrections preserve the prior at-or-before-date
comparison. Observations, quarantine, releases and facts each use a bulk upsert,
with repeated keys preserving the last applicable input. Dry runs share this
evaluation. Cache validators and freshness still commit with accepted data in the
same entity transaction. The statement-budget test counts real SQLx query events,
including a positive control, without retaining SQL text or parameters.

Since V0.1.28 adapters parse response bytes with `untrusted::decode<T>` into source-specific structs. It enforces the 20 MiB cap and JSON recursion limit and returns fixed errors for missing fields and malformed JSON. The runner sanitizes batches before storage or dry-run output. Since V0.1.40 versions and fact strings over 256 Unicode characters are rejected before sanitization, rather than truncated; controls, bidi/invisible characters and angle brackets are removed from accepted strings. Identifiers are bounded; numbers must be finite; dates must be full dates. Release dates must be from 1940 through next year, while announced end-of-life dates may extend through year 9999. Fact JSON is limited to depth 16 and 10,000 values; batches to 10,000 records. Bad responses fail only their entity. Future UI code must still render these fields as text.

## HTTP transport (V0.1.24)

Since V0.1.26 all clients share a capacity-one token bucket and concurrency semaphore per hostname, including retries and redirects. `engine::limits::QUOTAS` holds the source/host table: conservative one-per-second defaults for known APIs, one concurrent request for crates.io, two for other hosts; unlisted hosts use 100 ms spacing and two slots. These are application limits, not advertised provider allowances. Cross-origin redirects drop authorization and cookies.

Stack Exchange's typed `backoff` value delays the entire host (stricter than its per-method requirement), including callers already waiting. See [Stack Exchange throttling](https://api.stackexchange.com/docs/throttle) and [crates.io policies](https://crates.io/policies). Fake-clock tests cover spacing, sharing and concurrency. No settings were added.

Since V0.1.25 `http_cache::get` sends stored ETag and Last-Modified validators and returns `Changed` or `Unchanged`. A caller must persist the returned cache update in the same transaction as accepted data; malformed or rolled-back imports therefore cannot poison the cache. `record_unchanged` commits a 304's validators and run counter without touching observations. Cache keys hash the URL and request headers, keeping credential-bearing URLs out of storage. No new settings.

`engine::http::Client` uses reqwest 0.13 with rustls/ring and the platform certificate verifier, gzip and brotli decoding, a five-second connect timeout and a 30-second deadline per attempt (including body reads). Responses are limited to 20 MiB after decoding, both by declared length when available and while streaming. The User-Agent includes the crate version and repository URL.

There are three attempts total (the roadmap's “giving up after 3 tries” criterion), with exponential backoff starting at 500 ms plus jitter for 429, 5xx and timeouts. `Retry-After` seconds and HTTP dates are honored; delays over five minutes return a deferred error instead of retrying early. Other transport errors are not retried. Errors contain neither URLs nor response bodies. No new environment settings. Dependencies: reqwest 0.13, rustls 0.23 (ring), httpdate 1 and wiremock 0.6 for tests.

## Database roles (V0.1.23)

After owner migrations, run `Get-Content scripts/db/roles.sql -Raw | docker compose exec -T db psql -U cp_owner -d crispy_potato -v ON_ERROR_STOP=1`. This idempotent script provisions `cp_reader` (explicit SELECT column grants) and `cp_writer` (SELECT/INSERT/UPDATE/DELETE and sequences), denies schema creation and temporary tables, and protects migration history. Run it in every database as the same owner that runs migrations. Since V0.1.42 reader default grants are revoked: each release adding a public table or column must grant it explicitly. Reapplying also clears prior column grants.

Reader access excludes `http_cache`, `observations_quarantine`, `_sqlx_migrations`,
`entities.content_hash`, `ingestion_runs.errors` and `source_entity_state.last_error`.
Its allowed columns are spelled out in `roles.sql` and compared with an independent
allow-list in the database test. Both roles have a 20-connection limit and a
30-second idle-in-transaction timeout. Reader defaults are read-only, a five-second
statement timeout and a one-second lock timeout; writer lock timeout is five
seconds. Restart pools after provisioning because role defaults apply at login.
Tests use `SET ROLE` plus the configured timeout to cancel a reader query without
changing any operator password, and separately verify real pool session settings.

The workspace denies non-macro `sqlx::query*`, `raw_sql` and `QueryBuilder`
constructors through clippy. Test-only allowances permit fixtures. Run
`python scripts/checks/sql_guard.py` to compile a temporary example that uses
`format!`-built SQL and prove the lint rejects it; CI runs this check too. The
fixture is never executed and is removed afterward.

In the owner's interactive psql shell, use `\password cp_reader` and `\password cp_writer`. Set `DATABASE_URL` to the writer for sync/collection; `API_DATABASE_URL` to the reader for API access. Keep owner credentials only for migrations and isolated tests. Roles are cluster-wide and must not own objects or inherit other roles. The integration test switches dedicated connections into each role, checks reader SELECT and writer INSERT, and verifies permission-denied errors for reader writes, writer DDL and migration-history changes.

## Major dependency updates (V0.1.35)

The workspace uses sqlx 0.9.0 (minimum Rust 1.94), reqwest 0.13.5 and sha2 0.11.1. sqlx-cli installation follows upstream guidance and omits --locked to avoid relying on its published snapshot of yanked dependencies; application builds remain locked. HTTPS uses the operating system trust store through rustls-platform-verifier, with ring installed before constructing clients. Linux runtime images need ca-certificates. No aws-lc dependency or new license allowance is needed.

On memory-constrained Windows systems, set CARGO_BUILD_JOBS=1 and CARGO_PROFILE_DEV_DEBUG=0 in the shell before checks to reduce compiler memory and debug artifact size. The same settings work with tests and sqlx prepare.

## Workflow hardening (V0.1.36)

Rust 1.96.0 is pinned in rust-toolchain.toml. CI and health jobs read that file when installing the compiler; upgrades are reviewed changes. CI caches sqlx-cli 0.9.0 separately, builds it only on a cache miss, and only main push builds save the shared Rust cache. Health jobs restore but never save that cache.

The identifier job builds before receiving collection credentials. The reporter downloads only named JSON artifacts from the current run, caps files at 1 MiB and lists at 1,000 entries, treats absent or malformed reports as failures, and never checks out or executes project or artifact code. Marker and bot status identify its issue independently of the gh login spelling. Its inline code is checked against scripts/checks/weekly.py by the report tests.

Install zizmor 1.31.1 with pipx, then run: zizmor --offline --strict-collection --persona auditor .github/workflows. CI runs that same audit; all findings, including auditor-only checks, are addressed. No GitHub issues are written by local tests.

## Run failures and CLI diagnostics (V0.1.37)

Apply migration 0004 as the owner before collecting. A guard covers all exits after a run is created: failures finish it as failed, and dropping a collection future schedules an interrupted update. Terminal updates retry three times with a five-second timeout each and short backoff; they update only a running record. A lost connection can be replaced by the pool. The future worker in V0.1.82 will recover records left by process death, runtime shutdown or a database outage longer than the retry budget; no in-process guard can persist during a complete outage.

Database commands print fixed categories: unreachable, authentication, permission, timeout, constraint, TLS, configuration, missing record or generic database failure. Errors are classified from types and SQLSTATE, never from raw messages. Initial pool timeouts report an unreachable database because SQLx retries connection refusals until the connection deadline; acquired-pool and statement timeouts remain timeout errors. Content validation keeps its detailed file diagnostics. Ingest checks database connectivity before adapter selection, without creating a run for unknown sources.
