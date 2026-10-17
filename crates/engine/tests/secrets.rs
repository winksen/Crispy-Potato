use engine::{
    ingest::{self, Context, FetchFuture, Source, SourceInfo},
    settings::{RequiredSecret, Settings},
};
use sqlx::PgPool;
struct Protected;
impl Source for Protected {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "protected",
            name: "Protected",
            url: "https://example.com",
            license: "CC0",
            attribution: "Test",
            schedule_seconds: 86400,
        }
    }
    fn required_secret(&self) -> Option<RequiredSecret> {
        Some(RequiredSecret::GitHub)
    }
    fn applies(&self, _: &model::Entity) -> bool {
        true
    }
    fn fetch<'a>(&'a self, _: &'a Context<'a>, _: &'a model::Entity) -> FetchFuture<'a> {
        Box::pin(async { panic!("must not fetch without credentials") })
    }
}
#[sqlx::test(migrations = "./migrations")]
async fn missing_secret_is_recorded_as_skipped(pool: PgPool) -> Result<(), ingest::Error> {
    let report =
        ingest::run_with_settings(&pool, &Protected, None, false, &Settings::default()).await?;
    assert_eq!(report.status, "skipped");
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(
        engine::freshness::status(&pool).await?[0]["status"],
        "skipped"
    );
    Ok(())
}
