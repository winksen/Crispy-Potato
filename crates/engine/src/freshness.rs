//! Freshness tracks attempts separately from successful collection.
use sqlx::{PgPool, Postgres, Transaction};

pub async fn attempt(pool: &PgPool, entity: &str, source: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("INSERT INTO source_entity_state (entity_id,source_id) VALUES ($1,$2) ON CONFLICT (entity_id,source_id) DO UPDATE SET last_attempt=now(),last_error=NULL", entity, source).execute(pool).await?;
    Ok(())
}
pub async fn success(
    tx: &mut Transaction<'_, Postgres>,
    entity: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE source_entity_state SET last_success=now(),last_error=NULL WHERE entity_id=$1 AND source_id=$2", entity, source).execute(&mut **tx).await?;
    Ok(())
}
pub async fn failure(
    pool: &PgPool,
    entity: &str,
    source: &str,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE source_entity_state SET last_error=$3 WHERE entity_id=$1 AND source_id=$2",
        entity,
        source,
        reason
    )
    .execute(pool)
    .await?;
    Ok(())
}
pub async fn status(pool: &PgPool) -> Result<serde_json::Value, sqlx::Error> {
    let sources = sqlx::query!("SELECT s.id,s.schedule_seconds,r.status,r.started_at::text AS last_run,r.finished_at::text AS finished_at,r.errors FROM sources s LEFT JOIN LATERAL (SELECT status,started_at,finished_at,errors FROM ingestion_runs WHERE source_id=s.id ORDER BY id DESC LIMIT 1) r ON true ORDER BY s.id").fetch_all(pool).await?;
    let states = sqlx::query!("SELECT st.entity_id,st.source_id,st.last_attempt::text AS last_attempt,st.last_success::text AS last_success,st.last_error,(st.last_success IS NULL OR st.last_success < now() - s.schedule_seconds * interval '2 seconds') AS stale FROM source_entity_state st JOIN sources s ON s.id=st.source_id JOIN entities e ON e.id=st.entity_id WHERE NOT e.retired ORDER BY st.source_id,st.entity_id").fetch_all(pool).await?;
    Ok(serde_json::Value::Array(sources.into_iter().map(|s| {
        let entities: Vec<_> = states.iter().filter(|state| state.source_id == s.id).map(|state| serde_json::json!({"entity":state.entity_id,"last_attempt":state.last_attempt,"last_success":state.last_success,"failure":state.last_error,"stale":state.stale.unwrap_or(true)})).collect();
        serde_json::json!({"source":s.id,"schedule_seconds":s.schedule_seconds,"last_run":s.last_run,"finished_at":s.finished_at,"status":s.status,"failures":s.errors,"entities":entities})
    }).collect()))
}
