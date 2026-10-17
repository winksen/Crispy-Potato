CREATE TABLE sources (
    id text PRIMARY KEY,
    name text NOT NULL,
    url text NOT NULL,
    license text NOT NULL,
    attribution text NOT NULL,
    schedule_seconds bigint NOT NULL CHECK (schedule_seconds > 0)
);
CREATE TABLE observations (
    entity_id text NOT NULL REFERENCES entities(id),
    metric text NOT NULL,
    value double precision NOT NULL CHECK (value >= 0 AND value < 'Infinity'::float8),
    unit text NOT NULL CHECK (unit IN ('count', 'pct')),
    period text NOT NULL CHECK (period IN ('instant', 'week', 'month', 'year')),
    observed_at date NOT NULL,
    source_id text NOT NULL REFERENCES sources(id),
    PRIMARY KEY (entity_id, metric, period, observed_at, source_id),
    CHECK (unit <> 'pct' OR value <= 100)
);
CREATE INDEX observations_time ON observations (entity_id, metric, observed_at DESC);
CREATE TABLE observations_quarantine (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    entity_id text NOT NULL REFERENCES entities(id),
    source_id text NOT NULL REFERENCES sources(id),
    document jsonb NOT NULL,
    reason text NOT NULL,
    held_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE releases (
    entity_id text NOT NULL REFERENCES entities(id),
    version text NOT NULL,
    released_at date,
    eol_at date,
    lts boolean NOT NULL,
    source_id text NOT NULL REFERENCES sources(id),
    PRIMARY KEY (entity_id, version, source_id)
);
CREATE TABLE facts (
    entity_id text NOT NULL REFERENCES entities(id),
    field text NOT NULL,
    value jsonb NOT NULL,
    source_id text NOT NULL REFERENCES sources(id),
    collected_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (entity_id, field, source_id)
);
CREATE TABLE ingestion_runs (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source_id text NOT NULL REFERENCES sources(id),
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    status text NOT NULL CHECK (status IN ('running', 'success', 'partial', 'failed', 'skipped')),
    changed integer NOT NULL DEFAULT 0,
    unchanged integer NOT NULL DEFAULT 0,
    errors jsonb NOT NULL DEFAULT '[]'
);
CREATE TABLE http_cache (
    cache_key text PRIMARY KEY,
    etag text,
    last_modified text,
    checked_at timestamptz NOT NULL DEFAULT now()
);
