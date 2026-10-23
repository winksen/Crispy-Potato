use engine::{observations::Observation, quality};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn migration_preserves_bad_baselines_and_merges_old_duplicates(
    pool: PgPool,
) -> Result<(), engine::ingest::Error> {
    // Recreate the pre-0005 shape inside this disposable test database.
    sqlx::raw_sql("ALTER TABLE observations DROP CONSTRAINT observation_date_bounds, DROP CONSTRAINT observation_count_bound; DROP INDEX quarantine_measurement; DROP INDEX quarantine_newest;").execute(&pool).await?;
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO sources VALUES ('github','GitHub','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    sqlx::raw_sql("INSERT INTO observations VALUES ('rust','good',1,'count','instant','2026-10-17','github'), ('rust','huge',1e15,'count','instant','2026-10-17','github'), ('rust','future',1,'count','instant','9999-12-31','github'); INSERT INTO observations_quarantine (entity_id,source_id,document,reason) SELECT entity_id,source_id,to_jsonb(o),'old' FROM observations o WHERE metric='future'; INSERT INTO observations_quarantine (entity_id,source_id,document,reason) SELECT entity_id,source_id,document,'new' FROM observations_quarantine;").execute(&pool).await?;
    sqlx::raw_sql(include_str!("../migrations/0005_quarantine_identity.sql"))
        .execute(&pool)
        .await?;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM observations),(SELECT count(*) FROM observations_quarantine)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(counts, (1, 2));
    for (date, value) in [
        ("2007-12-31", 1.0),
        ("9999-12-31", 1.0),
        ("2026-10-17", 1e15),
    ] {
        assert!(sqlx::query("INSERT INTO observations VALUES ('rust','bad',$1,'count','instant',$2::text::date,'github')").bind(value).bind(date).execute(&pool).await.is_err());
    }
    Ok(())
}

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
    let mut tx = pool.begin().await?;
    o.value = 1002.0;
    assert!(quality::store(&mut tx, &o).await?.is_some());
    tx.commit().await?;
    assert_eq!(quality::list(&pool).await?.as_array().unwrap().len(), 1);
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

#[sqlx::test(migrations = "./migrations")]
async fn quarantine_filters_pages_and_newest_order(
    pool: PgPool,
) -> Result<(), engine::ingest::Error> {
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false), ('react','framework','React','{}','hash',false)").execute(&pool).await?;
    sqlx::query("INSERT INTO sources VALUES ('one','One','https://example.com','CC0','Test',86400), ('two','Two','https://example.com','CC0','Test',86400)").execute(&pool).await?;
    for (entity, source) in [("rust", "one"), ("react", "one"), ("rust", "two")] {
        let o = Observation {
            entity_id: entity.into(),
            source_id: source.into(),
            metric: "test.count".into(),
            value: 1e15,
            unit: "count".into(),
            period: "instant".into(),
            observed_at: "2026-10-17".into(),
        };
        let mut tx = pool.begin().await?;
        assert!(quality::store(&mut tx, &o).await?.is_some());
        tx.commit().await?;
    }
    let page = quality::list_filtered(&pool, Some("rust"), None, 1, 1).await?;
    assert_eq!(page[0]["source"], "two");
    let page = quality::list_filtered(&pool, Some("rust"), None, 2, 1).await?;
    assert_eq!(page[0]["source"], "one");
    assert_eq!(
        quality::list_filtered(&pool, None, Some("one"), 1, 100)
            .await?
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        quality::list_filtered(&pool, Some("react"), Some("two"), 1, 100).await?,
        serde_json::json!([])
    );
    assert!(
        quality::list_filtered(&pool, None, None, 0, 10)
            .await
            .is_err()
    );
    assert!(
        quality::list_filtered(&pool, None, None, 1, 101)
            .await
            .is_err()
    );
    let id = page[0]["id"].as_i64().unwrap();
    assert!(quality::resolve(&pool, id, true).await.is_err());
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM observations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count.0, 0);
    Ok(())
}
