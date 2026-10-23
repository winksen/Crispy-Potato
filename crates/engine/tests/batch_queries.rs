#![allow(clippy::unwrap_used, clippy::expect_used)]
use engine::{observations::Observation, quality};
use sqlx::PgPool;
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

// This integration executable contains one test, so other tests cannot pollute
// its SQLx event count. SQL text and bound values are never printed or retained.
struct Counter;
static COUNT: AtomicUsize = AtomicUsize::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);
impl log::Log for Counter {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.target() == "sqlx::query"
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) && ENABLED.load(Ordering::SeqCst) {
            COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn flush(&self) {}
}
fn start() {
    COUNT.store(0, Ordering::SeqCst);
    ENABLED.store(true, Ordering::SeqCst);
}
fn stop() -> usize {
    ENABLED.store(false, Ordering::SeqCst);
    COUNT.load(Ordering::SeqCst)
}
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn statement_counts_stay_bounded_for_large_batches_and_catalogs(
    pool: PgPool,
) -> Result<(), engine::ingest::Error> {
    log::set_logger(&Counter).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    // Prove the observer sees actual statements before trusting its budgets.
    start();
    sqlx::query("SELECT 1").execute(&pool).await?;
    assert_eq!(stop(), 1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let temp = tempfile::tempdir()?;
    copy(&root, temp.path());
    start();
    let small = engine::sync::sync(&pool, temp.path()).await?;
    let small_queries = stop();
    assert_eq!(small.changed, 2);
    let catalog = engine::content::load(&root).unwrap();
    let original = catalog.entities.iter().find(|e| e.id == "rust").unwrap();
    for n in 0..100 {
        let mut entity = original.clone();
        entity.id = format!("benchmark-{n}");
        entity.name = format!("Benchmark {n}");
        entity.aliases.clear();
        entity.relations = Default::default();
        fs::write(
            temp.path().join(format!("languages/{}.yaml", entity.id)),
            serde_json::to_vec_pretty(&entity)?,
        )?;
    }
    start();
    let large = engine::sync::sync(&pool, temp.path()).await?;
    let large_queries = stop();
    assert_eq!(large.changed, 100);
    assert_eq!(small_queries, large_queries);
    assert!(
        small_queries > 0 && small_queries <= 20,
        "sync statements: {small_queries}"
    );
    sqlx::query(
        "INSERT INTO sources VALUES ('batch','Batch','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    let mut date: model::PartialDate = "2020-01-01".parse()?;
    let mut values = Vec::new();
    for n in 0..1000 {
        values.push(Observation {
            entity_id: "rust".into(),
            source_id: "batch".into(),
            metric: "batch.stars".into(),
            value: 1000.0 + f64::from(n),
            unit: "count".into(),
            period: "instant".into(),
            observed_at: date.to_string(),
        });
        date = date.next_day().unwrap();
    }
    values.reverse(); // The source may return newest first.
    start();
    let mut tx = pool.begin().await?;
    assert!(quality::store_batch(&mut tx, &values).await?.is_empty());
    tx.commit().await?;
    let observation_queries = stop();
    assert!(
        observation_queries > 0 && observation_queries <= 10,
        "batch statements: {observation_queries}"
    );
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM observations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count.0, 1000);
    // A historical correction uses the stored value at that date, not the
    // latest value; a held point must not become the next point's baseline.
    let mut correction = values.last().unwrap().clone();
    correction.value = 1100.0;
    let mut held = correction.clone();
    held.observed_at = "2023-01-01".into();
    held.value = 30000.0;
    let mut after = held.clone();
    after.observed_at = "2023-01-02".into();
    after.value = 2000.0;
    let mut tx = pool.begin().await?;
    let held =
        quality::store_batch(&mut tx, &[after, held, correction.clone(), correction]).await?;
    assert_eq!(held.len(), 1);
    tx.commit().await?;
    println!(
        "Observed SQL statements: sync {small_queries} for 2 and 102 entities; 1,000-observation transaction {observation_queries}"
    );
    Ok(())
}
