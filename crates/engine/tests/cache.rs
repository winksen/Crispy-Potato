use engine::{
    http::Client,
    http_cache::{self, Fetch},
};
use reqwest::header::HeaderMap;
use sqlx::PgPool;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method},
};

#[sqlx::test(migrations = "./migrations")]
async fn conditional_request_preserves_observations_and_counts_unchanged(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .insert_header("Last-Modified", "Sat, 17 Oct 2026 00:00:00 GMT")
                .set_body_string("accepted"),
        )
        .mount(&server)
        .await;
    let client = Client::new()?;
    let Fetch::Changed(_, update) =
        http_cache::get(&client, &pool, &server.uri(), HeaderMap::new()).await?
    else {
        panic!("expected data")
    };
    let before_accept: (i64,) = sqlx::query_as("SELECT count(*) FROM http_cache")
        .fetch_one(&pool)
        .await?;
    assert_eq!(before_accept.0, 0);
    let mut tx = pool.begin().await?;
    http_cache::persist(&mut tx, &update).await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO entities VALUES ('rust','language','Rust','{}','hash',false)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO sources VALUES ('test','Test','https://example.com','CC0','Test',86400)",
    )
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO observations VALUES ('rust','test.count',7,'count','instant','2026-10-17','test')").execute(&pool).await?;
    let run: (i64,) = sqlx::query_as(
        "INSERT INTO ingestion_runs (source_id,status) VALUES ('test','running') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let before: (String,) = sqlx::query_as("SELECT xmin::text FROM observations")
        .fetch_one(&pool)
        .await?;
    server.reset().await;
    Mock::given(method("GET"))
        .and(header("If-None-Match", "\"v1\""))
        // HeaderExactMatcher splits commas, which is wrong for HTTP dates.
        .and(|request: &wiremock::Request| {
            request
                .headers
                .get("if-modified-since")
                .is_some_and(|v| v == "Sat, 17 Oct 2026 00:00:00 GMT")
        })
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let Fetch::Unchanged(update) =
        http_cache::get(&client, &pool, &server.uri(), HeaderMap::new()).await?
    else {
        panic!("expected unchanged")
    };
    http_cache::record_unchanged(&pool, run.0, &update).await?;
    let after: (String,) = sqlx::query_as("SELECT xmin::text FROM observations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(before, after);
    let count: (i32,) = sqlx::query_as("SELECT unchanged FROM ingestion_runs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count.0, 1);
    Ok(())
}
