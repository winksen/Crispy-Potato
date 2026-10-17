use engine::freshness;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn freshness_separates_failure_attempts_and_success(
    pool: PgPool,
) -> Result<(), engine::ingest::Error> {
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO sources VALUES ('test','Test','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    freshness::attempt(&pool, "rust", "test").await?;
    freshness::failure(&pool, "rust", "test", "fetch failed").await?;
    let status = freshness::status(&pool).await?;
    assert_eq!(status[0]["entities"][0]["stale"], true);
    assert!(status[0]["entities"][0]["last_success"].is_null());
    assert_eq!(status[0]["entities"][0]["failure"], "fetch failed");
    let mut tx = pool.begin().await?;
    freshness::success(&mut tx, "rust", "test").await?;
    tx.commit().await?;
    assert_eq!(
        freshness::status(&pool).await?[0]["entities"][0]["stale"],
        false
    );
    sqlx::query("UPDATE source_entity_state SET last_success=now()-interval '3 days'")
        .execute(&pool)
        .await?;
    freshness::attempt(&pool, "rust", "test").await?;
    freshness::failure(&pool, "rust", "test", "new failure").await?;
    assert_eq!(
        freshness::status(&pool).await?[0]["entities"][0]["stale"],
        true
    );
    sqlx::query("INSERT INTO ingestion_runs (source_id,status,errors) VALUES ('test','partial','[{\"entity\":\"rust\",\"reason\":\"fetch failed\"}]')").execute(&pool).await?;
    assert_eq!(freshness::status(&pool).await?[0]["status"], "partial");
    Ok(())
}
