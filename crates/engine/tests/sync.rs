#![allow(clippy::unwrap_used, clippy::expect_used)]
use sqlx::PgPool;
use std::{fs, path::Path};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sync_is_atomic_repeatable_and_preserves_history(pool: PgPool) {
    let temp = tempfile::tempdir().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        temp.path(),
    );
    let first = engine::sync::sync(&pool, temp.path()).await.unwrap();
    assert_eq!(first.changed, 2);
    assert!(
        engine::sync::sync(&pool, temp.path())
            .await
            .unwrap()
            .unchanged
    );
    let samples: (i64,) = sqlx::query_as("SELECT count(*) FROM samples")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(samples.0, 2);
    let reverse: (i64,) = sqlx::query_as("SELECT count(*) FROM relations WHERE target_id='rust'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(reverse.0 > 0);
    let version: (String,) = sqlx::query_as("SELECT value FROM meta WHERE key='content_version'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let rust = temp.path().join("languages/rust.yaml");
    let original = fs::read_to_string(&rust).unwrap();
    fs::write(&rust, "invalid: true").unwrap();
    assert!(engine::sync::sync(&pool, temp.path()).await.is_err());
    let after: (String,) = sqlx::query_as("SELECT value FROM meta WHERE key='content_version'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, after);
    fs::write(&rust, original).unwrap();
    sqlx::query(
        "INSERT INTO sources VALUES ('test','Test','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO observations VALUES ('rust','test.count',1,'count','instant','2026-10-17','test')").execute(&pool).await.unwrap();
    fs::remove_file(&rust).unwrap();
    let retired = engine::sync::sync(&pool, temp.path()).await.unwrap();
    assert_eq!(retired.retired, 1);
    let kept: (bool, i64) = sqlx::query_as(
        "SELECT retired,(SELECT count(*) FROM observations) FROM entities WHERE id='rust'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, (true, 1));
}
