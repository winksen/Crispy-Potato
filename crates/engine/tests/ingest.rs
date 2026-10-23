#![allow(clippy::disallowed_methods)]
use engine::{
    ingest::{self, Batch, Context, FetchFuture, Source, SourceInfo},
    observations::Observation,
};
use sqlx::PgPool;
use std::path::Path;

struct Fake;

struct RepeatedHold;
impl Source for RepeatedHold {
    fn info(&self) -> SourceInfo {
        Fake.info()
    }
    fn applies(&self, entity: &model::Entity) -> bool {
        entity.id == "rust"
    }
    fn fetch<'a>(&'a self, context: &'a Context<'a>, entity: &'a model::Entity) -> FetchFuture<'a> {
        Box::pin(async move {
            let mut batch = Fake.fetch(context, entity).await?;
            batch.observations[0].value = 1e15;
            Ok(batch)
        })
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn two_runs_update_the_same_held_measurement(pool: PgPool) -> Result<(), ingest::Error> {
    engine::sync::sync(
        &pool,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    )
    .await?;
    for _ in 0..2 {
        let report = ingest::run(&pool, &RepeatedHold, None, false).await?;
        assert_eq!(report.held.len(), 1);
    }
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM observations),(SELECT count(*) FROM observations_quarantine),(SELECT count(*) FROM ingestion_runs)").fetch_one(&pool).await?;
    assert_eq!(counts, (0, 1, 2));
    Ok(())
}
impl Source for Fake {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "fake",
            name: "Fake",
            url: "https://example.com",
            license: "CC0",
            attribution: "Fixture",
            schedule_seconds: 86400,
        }
    }
    fn applies(&self, _: &model::Entity) -> bool {
        true
    }
    fn fetch<'a>(&'a self, _: &'a Context<'a>, entity: &'a model::Entity) -> FetchFuture<'a> {
        Box::pin(async move {
            if entity.id == "react" {
                return Err("deliberate failure");
            }
            Ok(Batch {
                releases: vec![ingest::Release {
                    version: "1.0".into(),
                    released_at: Some("2026-10-17".into()),
                    eol_at: None,
                    lts: false,
                }],
                facts: vec![ingest::Fact {
                    field: "developer".into(),
                    value: serde_json::json!("<b>Example\u{202e}\n</b>"),
                }],
                observations: vec![Observation {
                    entity_id: entity.id.clone(),
                    source_id: "fake".into(),
                    metric: "fake.count".into(),
                    value: 42.0,
                    unit: "count".into(),
                    period: "instant".into(),
                    observed_at: "2026-10-17".into(),
                }],
                ..Batch::default()
            })
        })
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn runner_isolates_failure_and_dry_run_writes_nothing(
    pool: PgPool,
) -> Result<(), ingest::Error> {
    engine::sync::sync(
        &pool,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    )
    .await?;
    let preview = ingest::run(&pool, &Fake, None, true).await?;
    assert_eq!(preview.preview.len(), 1);
    assert_eq!(preview.preview[0].1.observations[0].value, 42.0);
    let rows: (i64,) = sqlx::query_as("SELECT (SELECT count(*) FROM sources)+(SELECT count(*) FROM observations)+(SELECT count(*) FROM ingestion_runs)").fetch_one(&pool).await?;
    assert_eq!(rows.0, 0);
    let report = ingest::run(&pool, &Fake, None, false).await?;
    assert_eq!(report.status, "partial");
    assert_eq!(report.changed, 1);
    assert_eq!(report.errors[0].entity, "react");
    let run: (String, i32) = sqlx::query_as("SELECT status,changed FROM ingestion_runs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(run, ("partial".into(), 1));
    let state: Vec<(String, bool, bool)> = sqlx::query_as("SELECT entity_id,last_success IS NOT NULL,last_error IS NOT NULL FROM source_entity_state ORDER BY entity_id").fetch_all(&pool).await?;
    assert_eq!(
        state,
        vec![("react".into(), false, true), ("rust".into(), true, false)]
    );
    let observation: (f64,) = sqlx::query_as("SELECT value FROM observations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(observation.0, 42.0);
    let extra: (i64,) =
        sqlx::query_as("SELECT (SELECT count(*) FROM releases)+(SELECT count(*) FROM facts)")
            .fetch_one(&pool)
            .await?;
    assert_eq!(extra.0, 2);
    let fact: (serde_json::Value,) = sqlx::query_as("SELECT value FROM facts")
        .fetch_one(&pool)
        .await?;
    assert_eq!(fact.0, serde_json::json!("bExample/b"));
    assert!(
        ingest::run(&pool, &Fake, Some("missing"), false)
            .await
            .is_err()
    );
    Ok(())
}
