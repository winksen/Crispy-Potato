//! Measurements retain their source, period and date; repeated imports upsert.
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub entity_id: String,
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub period: String,
    pub observed_at: String,
    pub source_id: String,
}

pub async fn upsert(
    tx: &mut Transaction<'_, Postgres>,
    o: &Observation,
) -> Result<(), sqlx::Error> {
    sqlx::query!("INSERT INTO observations (entity_id, metric, value, unit, period, observed_at, source_id) VALUES ($1,$2,$3,$4,$5,$6::text::date,$7) ON CONFLICT (entity_id,metric,period,observed_at,source_id) DO UPDATE SET value=EXCLUDED.value, unit=EXCLUDED.unit" ,&o.entity_id, &o.metric, o.value, &o.unit, &o.period, &o.observed_at, &o.source_id).execute(&mut **tx).await?;
    Ok(())
}
