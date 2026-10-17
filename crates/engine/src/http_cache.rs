//! Validators are committed only after the caller accepts the corresponding data.
use crate::http::{Client, Response};
use reqwest::{
    Method,
    header::{ETAG, HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};

pub struct Update {
    pub key: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}
pub enum Fetch {
    Changed(Box<Response>, Update),
    Unchanged(Update),
}
type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn get(
    client: &Client,
    pool: &PgPool,
    url: &str,
    mut headers: HeaderMap,
) -> Result<Fetch, Error> {
    let mut identity: Vec<_> = headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_bytes()))
        .collect();
    identity.sort_unstable();
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(url, identity))?)
    );
    let previous = sqlx::query!(
        "SELECT etag,last_modified FROM http_cache WHERE cache_key=$1",
        key
    )
    .fetch_optional(pool)
    .await?;
    if let Some(previous) = &previous {
        for (name, value) in [
            (IF_NONE_MATCH, &previous.etag),
            (IF_MODIFIED_SINCE, &previous.last_modified),
        ] {
            if let Some(value) = value {
                headers.insert(name, HeaderValue::from_str(value)?);
            }
        }
    }
    let response = client.request(Method::GET, url, headers).await?;
    let header = |name| {
        response
            .headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .filter(|v| v.len() <= 1024)
            .map(str::to_owned)
    };
    let mut update = Update {
        key,
        etag: header(ETAG),
        last_modified: header(LAST_MODIFIED),
    };
    if response.status == 304 {
        let previous = previous.ok_or("Unexpected 304 without cached validators")?;
        if previous.etag.is_none() && previous.last_modified.is_none() {
            return Err("Unexpected 304 without cached validators".into());
        }
        update.etag = update.etag.or(previous.etag);
        update.last_modified = update.last_modified.or(previous.last_modified);
        Ok(Fetch::Unchanged(update))
    } else if (200..300).contains(&response.status) {
        Ok(Fetch::Changed(Box::new(response), update))
    } else {
        Err(format!("HTTP status {}", response.status).into())
    }
}

pub async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    update: &Update,
) -> Result<(), sqlx::Error> {
    sqlx::query!("INSERT INTO http_cache (cache_key,etag,last_modified) VALUES ($1,$2,$3) ON CONFLICT (cache_key) DO UPDATE SET etag=EXCLUDED.etag,last_modified=EXCLUDED.last_modified,checked_at=now()", update.key, update.etag, update.last_modified).execute(&mut **tx).await?;
    Ok(())
}

/// A 304 updates validators and the run counter, never observations.
pub async fn record_unchanged(
    pool: &PgPool,
    run_id: i64,
    update: &Update,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query!(
        "UPDATE ingestion_runs SET unchanged=unchanged+1 WHERE id=$1 AND status='running'",
        run_id
    )
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    persist(&mut tx, update).await?;
    tx.commit().await
}
