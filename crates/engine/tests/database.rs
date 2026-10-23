#![allow(clippy::disallowed_methods)]
use engine::db;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn database_roles_enforce_least_privilege(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::raw_sql(include_str!("../../../scripts/db/roles.sql"))
        .execute(&pool)
        .await?;
    let mut reader = pool.acquire().await?;
    sqlx::query("SET ROLE cp_reader")
        .execute(&mut *reader)
        .await?;
    sqlx::query("SELECT id,kind,name,document,retired FROM entities")
        .fetch_all(&mut *reader)
        .await?;
    let denied = sqlx::query("INSERT INTO meta VALUES ('forbidden','value')")
        .execute(&mut *reader)
        .await
        .unwrap_err();
    assert_eq!(
        denied.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("42501")
    );
    sqlx::query("RESET ROLE").execute(&mut *reader).await?;
    let mut writer = pool.acquire().await?;
    sqlx::query("SET ROLE cp_writer")
        .execute(&mut *writer)
        .await?;
    sqlx::query("INSERT INTO meta VALUES ('allowed','value')")
        .execute(&mut *writer)
        .await?;
    for statement in [
        "CREATE TABLE forbidden (id integer)",
        "CREATE TEMP TABLE forbidden (id integer)",
        "DELETE FROM _sqlx_migrations",
    ] {
        let denied = sqlx::query(statement)
            .execute(&mut *writer)
            .await
            .unwrap_err();
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
    }
    sqlx::query("RESET ROLE").execute(&mut *writer).await?;
    let expected = [
        ("entities", "id,kind,name,document,retired"),
        ("aliases", "alias,entity_id"),
        ("relations", "entity_id,relation,target_id"),
        ("planned_ids", "id,kind"),
        ("vocabulary_terms", "vocabulary,term,document"),
        ("samples", "entity_id,path,text"),
        ("meta", "key,value"),
        (
            "sources",
            "id,name,url,license,attribution,schedule_seconds",
        ),
        (
            "observations",
            "entity_id,metric,value,unit,period,observed_at,source_id",
        ),
        (
            "releases",
            "entity_id,version,released_at,eol_at,lts,source_id",
        ),
        ("facts", "entity_id,field,value,source_id,collected_at"),
        (
            "ingestion_runs",
            "id,source_id,started_at,finished_at,status,changed,unchanged",
        ),
        (
            "source_entity_state",
            "entity_id,source_id,last_attempt,last_success",
        ),
    ];
    let expected: std::collections::BTreeSet<(String, String)> = expected
        .into_iter()
        .flat_map(|(table, columns)| {
            columns
                .split(',')
                .map(move |column| (table.into(), column.into()))
        })
        .collect();
    let actual: Vec<(String,String)> = sqlx::query_as("SELECT c.relname,a.attname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_attribute a ON a.attrelid=c.oid WHERE n.nspname='public' AND c.relkind IN ('r','p') AND a.attnum>0 AND NOT a.attisdropped AND has_column_privilege('cp_reader',c.oid,a.attnum,'SELECT')").fetch_all(&pool).await?;
    assert_eq!(
        actual
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    sqlx::query("CREATE TABLE future_private (secret text)")
        .execute(&pool)
        .await?;
    let readable: (bool,) = sqlx::query_as(
        "SELECT has_column_privilege('cp_reader','future_private','secret','SELECT')",
    )
    .fetch_one(&pool)
    .await?;
    assert!(!readable.0);
    let role: (i32, Vec<String>) =
        sqlx::query_as("SELECT rolconnlimit,rolconfig FROM pg_roles WHERE rolname='cp_reader'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(role.0, 20);
    for setting in [
        "default_transaction_read_only=on",
        "statement_timeout=5s",
        "idle_in_transaction_session_timeout=30s",
        "lock_timeout=1s",
    ] {
        assert!(role.1.iter().any(|s| s == setting), "missing {setting}");
    }
    // SET ROLE does not apply login defaults. Load the declared timeout to
    // exercise a real reader query without changing the operator's password.
    let timeout = role
        .1
        .iter()
        .find_map(|s| s.strip_prefix("statement_timeout="))
        .unwrap();
    sqlx::query("SELECT set_config('statement_timeout',$1,false)")
        .bind(timeout)
        .execute(&mut *reader)
        .await?;
    sqlx::query("SET ROLE cp_reader")
        .execute(&mut *reader)
        .await?;
    let timeout = sqlx::query("SELECT pg_sleep(6)")
        .execute(&mut *reader)
        .await
        .unwrap_err();
    assert_eq!(
        timeout
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("57014")
    );
    sqlx::query("RESET ROLE").execute(&mut *reader).await?;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn pools_set_application_and_read_only_deadlines(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = pool.connect_options();
    // Reuse the disposable database, never the base database's tables.
    let mut url = model::Url::parse(&std::env::var("DATABASE_URL")?)?;
    url.set_path(options.get_database().ok_or("test database missing")?);
    let reader = db::connect_named(url.as_str(), "cp-reader-test", true).await?;
    let settings: (String,String,String) = sqlx::query_as("SELECT current_setting('application_name'),current_setting('statement_timeout'),current_setting('default_transaction_read_only')").fetch_one(&reader).await?;
    assert_eq!(
        settings,
        ("cp-reader-test".into(), "5s".into(), "on".into())
    );
    reader.close().await;
    let writer = db::connect(url.as_str()).await?;
    let settings: (String, String) = sqlx::query_as(
        "SELECT current_setting('application_name'),current_setting('statement_timeout')",
    )
    .fetch_one(&writer)
    .await?;
    assert_eq!(settings, ("cp-cli".into(), "30s".into()));
    writer.close().await;
    drop(options);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn observation_import_updates_instead_of_duplicating(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO sources VALUES ('test','Test','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    let mut o = engine::observations::Observation {
        entity_id: "rust".into(),
        metric: "test.count".into(),
        value: 1.0,
        unit: "count".into(),
        period: "instant".into(),
        observed_at: "2026-10-17".into(),
        source_id: "test".into(),
    };
    let mut tx = pool.begin().await?;
    engine::observations::upsert(&mut tx, &o).await?;
    o.value = 2.0;
    engine::observations::upsert(&mut tx, &o).await?;
    tx.commit().await?;
    let rows: Vec<(f64,)> = sqlx::query_as("SELECT value FROM observations")
        .fetch_all(&pool)
        .await?;
    assert_eq!(rows, vec![(2.0,)]);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn migrations_are_repeatable(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    db::migrate(&pool).await?;
    db::migrate(&pool).await?;
    let tables: (i64,) = sqlx::query_as("SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN ('entities','aliases','relations','planned_ids','vocabulary_terms','samples','meta')")
        .fetch_one(&pool).await?;
    assert_eq!(tables.0, 7);
    Ok(())
}
