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

pub(crate) async fn upsert_batch(
    tx: &mut Transaction<'_, Postgres>,
    values: &[Observation],
) -> Result<(), sqlx::Error> {
    let entities: Vec<_> = values.iter().map(|o| o.entity_id.clone()).collect();
    let metrics: Vec<_> = values.iter().map(|o| o.metric.clone()).collect();
    let numbers: Vec<_> = values.iter().map(|o| o.value).collect();
    let units: Vec<_> = values.iter().map(|o| o.unit.clone()).collect();
    let periods: Vec<_> = values.iter().map(|o| o.period.clone()).collect();
    let dates: Vec<_> = values.iter().map(|o| o.observed_at.clone()).collect();
    let sources: Vec<_> = values.iter().map(|o| o.source_id.clone()).collect();
    sqlx::query!("INSERT INTO observations (entity_id,metric,value,unit,period,observed_at,source_id) SELECT entity,metric,value,unit,period,date::date,source FROM UNNEST($1::text[],$2::text[],$3::float8[],$4::text[],$5::text[],$6::text[],$7::text[]) AS v(entity,metric,value,unit,period,date,source) ON CONFLICT (entity_id,metric,period,observed_at,source_id) DO UPDATE SET value=EXCLUDED.value,unit=EXCLUDED.unit", &entities,&metrics,&numbers,&units,&periods,&dates,&sources).execute(&mut **tx).await?;
    Ok(())
}
