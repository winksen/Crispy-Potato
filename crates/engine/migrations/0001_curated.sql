CREATE TABLE entities (
    id text PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('language', 'framework', 'runtime')),
    name text NOT NULL,
    document jsonb NOT NULL,
    content_hash text NOT NULL,
    retired boolean NOT NULL DEFAULT false
);
CREATE TABLE aliases (
    alias text PRIMARY KEY,
    entity_id text NOT NULL REFERENCES entities(id) ON DELETE CASCADE
);
-- Targets may be planned ids. Sync validates both ends before writing.
CREATE TABLE relations (
    entity_id text NOT NULL,
    relation text NOT NULL,
    target_id text NOT NULL,
    PRIMARY KEY (entity_id, relation, target_id)
);
CREATE TABLE planned_ids (
    id text PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('language', 'framework', 'runtime'))
);
CREATE TABLE vocabulary_terms (
    vocabulary text NOT NULL,
    term text NOT NULL,
    document jsonb NOT NULL,
    PRIMARY KEY (vocabulary, term)
);
CREATE TABLE samples (
    entity_id text NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    path text NOT NULL,
    text text NOT NULL,
    PRIMARY KEY (entity_id, path)
);
CREATE TABLE meta (
    key text PRIMARY KEY,
    value text NOT NULL
);
