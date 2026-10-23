-- Run as the migration owner in EACH database, after migrations.
-- No passwords in this file. Set them interactively with psql \password.
BEGIN;
SELECT pg_advisory_xact_lock(1129337690);
DO $$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'cp_reader') THEN
        CREATE ROLE cp_reader LOGIN;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'cp_writer') THEN
        CREATE ROLE cp_writer LOGIN;
    END IF;
    EXECUTE format('REVOKE CREATE, TEMP ON DATABASE %I FROM PUBLIC', current_database());
    EXECUTE format('GRANT CONNECT ON DATABASE %I TO cp_reader, cp_writer', current_database());
END $$;
ALTER ROLE cp_reader NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE cp_writer NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE cp_reader CONNECTION LIMIT 20;
ALTER ROLE cp_writer CONNECTION LIMIT 20;
ALTER ROLE cp_reader SET default_transaction_read_only = on;
ALTER ROLE cp_reader SET statement_timeout = '5s';
ALTER ROLE cp_reader SET idle_in_transaction_session_timeout = '30s';
ALTER ROLE cp_reader SET lock_timeout = '1s';
ALTER ROLE cp_writer SET idle_in_transaction_session_timeout = '30s';
ALTER ROLE cp_writer SET lock_timeout = '5s';
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO cp_reader, cp_writer;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM cp_reader, cp_writer;
-- Revoke any earlier column grants as well as table-wide grants.
DO $$ DECLARE columns record; BEGIN
    FOR columns IN SELECT c.relname, string_agg(quote_ident(a.attname), ',') AS names
        FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        JOIN pg_attribute a ON a.attrelid=c.oid
        WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m','f')
            AND a.attnum > 0 AND NOT a.attisdropped
        GROUP BY c.relname
    LOOP
        EXECUTE format('REVOKE SELECT (%s) ON TABLE %I FROM cp_reader', columns.names, columns.relname);
    END LOOP;
END $$;
GRANT SELECT (id,kind,name,document,retired) ON entities TO cp_reader;
GRANT SELECT (alias,entity_id) ON aliases TO cp_reader;
GRANT SELECT (entity_id,relation,target_id) ON relations TO cp_reader;
GRANT SELECT (id,kind) ON planned_ids TO cp_reader;
GRANT SELECT (vocabulary,term,document) ON vocabulary_terms TO cp_reader;
GRANT SELECT (entity_id,path,text) ON samples TO cp_reader;
GRANT SELECT (key,value) ON meta TO cp_reader;
GRANT SELECT (id,name,url,license,attribution,schedule_seconds) ON sources TO cp_reader;
GRANT SELECT (entity_id,metric,value,unit,period,observed_at,source_id) ON observations TO cp_reader;
GRANT SELECT (entity_id,version,released_at,eol_at,lts,source_id) ON releases TO cp_reader;
GRANT SELECT (entity_id,field,value,source_id,collected_at) ON facts TO cp_reader;
GRANT SELECT (id,source_id,started_at,finished_at,status,changed,unchanged) ON ingestion_runs TO cp_reader;
GRANT SELECT (entity_id,source_id,last_attempt,last_success) ON source_entity_state TO cp_reader;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO cp_writer;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM cp_reader, cp_writer;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO cp_writer;
-- Only the owner may change migration history.
REVOKE INSERT, UPDATE, DELETE ON _sqlx_migrations FROM cp_writer;
ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM cp_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO cp_writer;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO cp_writer;
COMMIT;
