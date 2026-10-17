//! Bounded Postgres connections; migrations are run explicitly by the owner.
use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// The API must use its separate read-only credential, never the writer URL.
pub async fn connect_reader() -> Result<PgPool, String> {
    let url = std::env::var("API_DATABASE_URL")
        .map_err(|_| "API_DATABASE_URL is required".to_string())?;
    connect(&url)
        .await
        .map_err(|_| "Cannot connect to the reader database".to_string())
}

pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET statement_timeout = '30s'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}
