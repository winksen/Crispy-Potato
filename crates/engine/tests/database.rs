use engine::db;
use sqlx::PgPool;

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
