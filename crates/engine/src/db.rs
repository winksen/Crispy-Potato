//! Bounded Postgres connections; migrations are run explicitly by the owner.
use std::{str::FromStr, time::Duration};

pub use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// The API must use its separate read-only credential, never the writer URL.
pub async fn connect_reader() -> Result<PgPool, String> {
    let url = std::env::var("API_DATABASE_URL")
        .map_err(|_| "API_DATABASE_URL is required".to_string())?;
    connect_named(&url, "cp-api", true)
        .await
        .map_err(|error| crate::diagnostics::connection_cause(&error).to_string())
}

pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    connect_named(url, "cp-cli", false).await
}

pub async fn connect_named(
    url: &str,
    application: &str,
    reader: bool,
) -> Result<PgPool, sqlx::Error> {
    let settings = crate::settings::DatabaseSettings::load(reader).map_err(|error| {
        sqlx::Error::Configuration(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, error).into(),
        )
    })?;
    let options = PgConnectOptions::from_str(url)?.application_name(application);
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(Duration::from_millis(settings.acquire_timeout_ms))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(move |connection, _| {
            Box::pin(async move {
                let timeout = settings.statement_timeout_ms as i64;
                sqlx::query!("SELECT set_config('statement_timeout',(CASE WHEN current_user='cp_reader' THEN LEAST($1::bigint,5000) ELSE $1 END)::text || 'ms',false) AS timeout_setting, set_config('default_transaction_read_only',CASE WHEN $2 THEN 'on' ELSE current_setting('default_transaction_read_only') END,false) AS read_only_setting", timeout, reader)
                    .fetch_one(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}
