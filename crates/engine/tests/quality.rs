use engine::{observations::Observation, quality};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn held_values_are_not_served_and_release_moves_them(
    pool: PgPool,
) -> Result<(), engine::ingest::Error> {
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO sources VALUES ('github','GitHub','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    let mut o = Observation {
        entity_id: "rust".into(),
        source_id: "github".into(),
        metric: "github.stars".into(),
        value: 100.0,
        unit: "count".into(),
        period: "instant".into(),
        observed_at: "2026-10-16".into(),
    };
    let mut tx = pool.begin().await?;
    assert!(quality::store(&mut tx, &o).await?.is_none());
    o.value = 1001.0;
    o.observed_at = "2026-10-17".into();
    assert!(quality::store(&mut tx, &o).await?.is_some());
    tx.commit().await?;
    let values: Vec<(f64,)> = sqlx::query_as("SELECT value FROM observations")
        .fetch_all(&pool)
        .await?;
    assert_eq!(values, vec![(100.0,)]);
    let id: (i64,) = sqlx::query_as("SELECT id FROM observations_quarantine")
        .fetch_one(&pool)
        .await?;
    quality::resolve(&pool, id.0, true).await?;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM observations),(SELECT count(*) FROM observations_quarantine)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(counts, (2, 0));
    o.value = -1.0;
    let mut tx = pool.begin().await?;
    quality::store(&mut tx, &o).await?;
    tx.commit().await?;
    let id: (i64,) = sqlx::query_as("SELECT id FROM observations_quarantine")
        .fetch_one(&pool)
        .await?;
    assert!(quality::resolve(&pool, id.0, true).await.is_err());
    quality::resolve(&pool, id.0, false).await?;
    assert_eq!(quality::list(&pool).await?, serde_json::json!([]));
    Ok(())
}
