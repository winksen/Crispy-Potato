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
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO cp_reader, cp_writer;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM cp_reader, cp_writer;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO cp_reader;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO cp_writer;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM cp_reader, cp_writer;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO cp_writer;
-- Only the owner may change migration history.
REVOKE INSERT, UPDATE, DELETE ON _sqlx_migrations FROM cp_writer;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO cp_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO cp_writer;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO cp_writer;
COMMIT;
