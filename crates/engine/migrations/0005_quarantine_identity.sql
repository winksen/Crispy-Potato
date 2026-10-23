-- Keep the newest review record for each measurement before enforcing identity.
DELETE FROM observations_quarantine WHERE id IN (
    SELECT id FROM (
        SELECT id, row_number() OVER (
            PARTITION BY entity_id, source_id, document->>'metric',
                document->>'period', document->>'observed_at'
            ORDER BY held_at DESC, id DESC
        ) AS position FROM observations_quarantine
    ) AS duplicates WHERE position > 1
);
CREATE UNIQUE INDEX quarantine_measurement ON observations_quarantine (
    entity_id, source_id, (document->>'metric'), (document->>'period'),
    (document->>'observed_at')
);
CREATE INDEX quarantine_newest ON observations_quarantine (held_at DESC, id DESC);

-- Preserve old out-of-range measurements for review, removing them as baselines.
INSERT INTO observations_quarantine (entity_id, source_id, document, reason)
SELECT entity_id, source_id, to_jsonb(o), 'outside observation date or count bounds'
FROM observations AS o
WHERE observed_at < DATE '2008-01-01'
    OR observed_at > (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::date + 1
    OR (unit = 'count' AND value >= 1e15)
ON CONFLICT (entity_id, source_id, (document->>'metric'), (document->>'period'), (document->>'observed_at'))
DO UPDATE SET document=EXCLUDED.document, reason=EXCLUDED.reason, held_at=now();
DELETE FROM observations
WHERE observed_at < DATE '2008-01-01'
    OR observed_at > (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::date + 1
    OR (unit = 'count' AND value >= 1e15);
ALTER TABLE observations ADD CONSTRAINT observation_date_bounds CHECK (
    observed_at >= DATE '2008-01-01'
    AND observed_at <= (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::date + 1
);
ALTER TABLE observations ADD CONSTRAINT observation_count_bound
    CHECK (unit <> 'count' OR value < 1e15);
