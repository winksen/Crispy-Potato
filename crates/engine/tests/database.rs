use engine::db;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn migrations_are_repeatable(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    db::migrate(&pool).await?;
    db::migrate(&pool).await?;
    let tables: (i64,) = sqlx::query_as("SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN ('entities','aliases','relations','planned_ids','vocabulary_terms','samples','meta')")
        .fetch_one(&pool).await?;
    assert_eq!(tables.0, 7);
    Ok(())
}
