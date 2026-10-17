CREATE TABLE source_entity_state (
    entity_id text NOT NULL REFERENCES entities(id),
    source_id text NOT NULL REFERENCES sources(id),
    last_attempt timestamptz NOT NULL DEFAULT now(),
    last_success timestamptz,
    last_error text,
    PRIMARY KEY (entity_id, source_id)
);
CREATE INDEX source_entity_freshness ON source_entity_state (source_id, last_success);
