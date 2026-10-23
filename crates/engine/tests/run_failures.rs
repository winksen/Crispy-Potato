use engine::ingest::{self, Batch, Context, FetchFuture, Source, SourceInfo};
use sqlx::PgPool;
use std::{path::Path, sync::Arc, time::Duration};

struct FailingSource;
impl Source for FailingSource {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "failure-test",
            name: "Failure test",
            url: "https://example.com",
            license: "CC0",
            attribution: "Fixture",
            schedule_seconds: 86400,
        }
    }
    fn applies(&self, _: &model::Entity) -> bool {
        true
    }
    fn fetch<'a>(&'a self, _: &'a Context<'a>, _: &'a model::Entity) -> FetchFuture<'a> {
        Box::pin(async { Err("secret-bearing provider diagnostic must not be stored") })
    }
}

async fn seed(pool: &PgPool) -> Result<(), ingest::Error> {
    engine::sync::sync(
        pool,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    )
    .await?;
    sqlx::raw_sql("CREATE FUNCTION kill_test_connection() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_terminate_backend(pg_backend_pid()); RETURN NEW; END $$").execute(pool).await?;
    Ok(())
}

async fn assert_failed(pool: &PgPool) -> Result<(), ingest::Error> {
    let result = ingest::run(pool, &FailingSource, Some("rust"), false).await;
    assert!(result.is_err());
    let (status, finished, errors): (String, bool, serde_json::Value) = sqlx::query_as(
        "SELECT status,finished_at IS NOT NULL,errors FROM ingestion_runs ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(status, "failed");
    assert!(finished);
    assert!(errors.to_string().contains("database unreachable"));
    assert!(!errors.to_string().contains("secret-bearing"));
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn connection_lost_during_attempt_finalizes_run(pool: PgPool) -> Result<(), ingest::Error> {
    seed(&pool).await?;
    sqlx::raw_sql("CREATE TRIGGER kill_attempt BEFORE INSERT ON source_entity_state FOR EACH ROW EXECUTE FUNCTION kill_test_connection()")
        .execute(&pool).await?;
    assert_failed(&pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn connection_lost_recording_failure_finalizes_run(
    pool: PgPool,
) -> Result<(), ingest::Error> {
    seed(&pool).await?;
    sqlx::raw_sql("CREATE TRIGGER kill_failure BEFORE UPDATE ON source_entity_state FOR EACH ROW WHEN (NEW.last_error IS NOT NULL) EXECUTE FUNCTION kill_test_connection()")
        .execute(&pool).await?;
    assert_failed(&pool).await
}

struct WaitingSource(Arc<tokio::sync::Notify>);
impl Source for WaitingSource {
    fn info(&self) -> SourceInfo {
        FailingSource.info()
    }
    fn applies(&self, _: &model::Entity) -> bool {
        true
    }
    fn fetch<'a>(&'a self, _: &'a Context<'a>, _: &'a model::Entity) -> FetchFuture<'a> {
        Box::pin(async {
            self.0.notify_one();
            std::future::pending::<Result<Batch, &'static str>>().await
        })
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelled_future_records_interrupted_status(pool: PgPool) -> Result<(), ingest::Error> {
    seed(&pool).await?;
    let notify = Arc::new(tokio::sync::Notify::new());
    let source = WaitingSource(notify.clone());
    let task_pool = pool.clone();
    let task =
        tokio::spawn(async move { ingest::run(&task_pool, &source, Some("rust"), false).await });
    tokio::time::timeout(Duration::from_secs(5), notify.notified()).await?;
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ingestion_runs WHERE status='interrupted' AND finished_at IS NOT NULL")
                .fetch_one(&pool).await?;
            if count == 1 { return Ok::<(), sqlx::Error>(()); }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await??;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn sqlstate_diagnostics_are_classified_without_error_text(
    pool: PgPool,
) -> Result<(), ingest::Error> {
    sqlx::raw_sql(include_str!("../../../scripts/db/roles.sql"))
        .execute(&pool)
        .await?;
    let mut connection = pool.acquire().await?;
    sqlx::query("SET ROLE cp_reader")
        .execute(&mut *connection)
        .await?;
    let error = sqlx::query("INSERT INTO meta VALUES ('private-value','private-value')")
        .execute(&mut *connection)
        .await
        .unwrap_err();
    assert_eq!(
        engine::diagnostics::cause(&error),
        "database permission denied"
    );
    sqlx::query("RESET ROLE").execute(&mut *connection).await?;
    sqlx::query("SET statement_timeout='5ms'")
        .execute(&mut *connection)
        .await?;
    let error = sqlx::query("SELECT pg_sleep(0.1)")
        .execute(&mut *connection)
        .await
        .unwrap_err();
    assert_eq!(engine::diagnostics::cause(&error), "database timeout");
    sqlx::query("SET statement_timeout='30s'")
        .execute(&mut *connection)
        .await?;
    let error = sqlx::query("INSERT INTO sources VALUES ('invalid','private-value','https://private-value','CC0','test',-1)").execute(&mut *connection).await.unwrap_err();
    assert_eq!(
        engine::diagnostics::cause(&error),
        "database constraint violation"
    );
    Ok(())
}
