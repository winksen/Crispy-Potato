//! Hold implausible measurements outside the served observation table.
use crate::{
    ingest::Error,
    observations::{self, Observation},
};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Serialize)]
pub struct Held {
    pub observation: Observation,
    pub reason: String,
}

pub fn reason(o: &Observation, previous: Option<f64>) -> Option<&'static str> {
    if !crate::untrusted::observation_date(&o.observed_at) {
        return Some("observation date is outside 2008 through tomorrow UTC");
    }
    if !o.value.is_finite() {
        return Some("non-finite value");
    }
    if o.value < 0.0 {
        return Some("negative count or percentage");
    }
    if o.unit == "count" && o.value >= 1e15 {
        return Some("count must be below 10^15");
    }
    if o.unit == "pct" && o.value > 100.0 {
        return Some("percentage exceeds 100");
    }
    let cumulative = o.metric.ends_with(".stars")
        || o.metric.ends_with(".downloads_total")
        || o.metric.ends_with(".total_downloads")
        || o.metric == "crates.downloads";
    if cumulative && let Some(last) = previous {
        if o.value < last * 0.8 {
            return Some("cumulative value fell more than 20 percent");
        }
        if last > 0.0 && o.value > last * 10.0 {
            return Some("cumulative value grew more than tenfold");
        }
    }
    None
}

async fn previous(
    tx: &mut Transaction<'_, Postgres>,
    o: &Observation,
) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar!("SELECT value FROM observations WHERE entity_id=$1 AND source_id=$2 AND metric=$3 AND period=$4 AND observed_at <= $5::text::date ORDER BY observed_at DESC LIMIT 1", o.entity_id, o.source_id, o.metric, o.period, o.observed_at).fetch_optional(&mut **tx).await
}

pub async fn inspect(pool: &PgPool, o: &Observation) -> Result<Option<Held>, Error> {
    let mut tx = pool.begin().await?;
    let last = if reason(o, None).is_some() {
        None
    } else {
        previous(&mut tx, o).await?
    };
    tx.rollback().await?;
    Ok(reason(o, last).map(|reason| Held {
        observation: o.clone(),
        reason: reason.into(),
    }))
}

pub async fn store(
    tx: &mut Transaction<'_, Postgres>,
    o: &Observation,
) -> Result<Option<Held>, Error> {
    // Serialize writers for this entity before reading the baseline.
    sqlx::query!(
        "SELECT id FROM entities WHERE id=$1 FOR UPDATE",
        o.entity_id
    )
    .fetch_one(&mut **tx)
    .await?;
    let last = if reason(o, None).is_some() {
        None
    } else {
        previous(tx, o).await?
    };
    if let Some(reason) = reason(o, last) {
        let document = serde_json::to_value(o)?;
        sqlx::query!("INSERT INTO observations_quarantine (entity_id,source_id,document,reason) VALUES ($1,$2,$3,$4) ON CONFLICT (entity_id,source_id,(document->>'metric'),(document->>'period'),(document->>'observed_at')) DO UPDATE SET document=EXCLUDED.document,reason=EXCLUDED.reason,held_at=now()", o.entity_id, o.source_id, document, reason).execute(&mut **tx).await?;
        Ok(Some(Held {
            observation: o.clone(),
            reason: reason.into(),
        }))
    } else {
        observations::upsert(tx, o).await?;
        Ok(None)
    }
}

pub async fn list(pool: &PgPool) -> Result<serde_json::Value, Error> {
    list_filtered(pool, None, None, 1, 100).await
}

/// One-based pages, bounded to 100 rows, newest holds first with stable ties.
pub async fn list_filtered(
    pool: &PgPool,
    entity: Option<&str>,
    source: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<serde_json::Value, Error> {
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err("page must be positive and page size between 1 and 100".into());
    }
    let offset = i64::from(page - 1) * i64::from(page_size);
    let rows = sqlx::query!("SELECT id,entity_id,source_id,document,reason,held_at::text AS held_at FROM observations_quarantine WHERE ($1::text IS NULL OR entity_id=$1) AND ($2::text IS NULL OR source_id=$2) ORDER BY held_at DESC,id DESC LIMIT $3 OFFSET $4", entity, source, i64::from(page_size), offset).fetch_all(pool).await?;
    Ok(serde_json::Value::Array(rows.into_iter().map(|r| serde_json::json!({"id":r.id,"entity":r.entity_id,"source":r.source_id,"observation":r.document,"reason":r.reason,"held_at":r.held_at})).collect()))
}

pub async fn resolve(pool: &PgPool, id: i64, release: bool) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query!(
        "SELECT document FROM observations_quarantine WHERE id=$1 FOR UPDATE",
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    if release {
        let observation: Observation = serde_json::from_value(row.document)?;
        // Manual approval may override trend rules, never invalid ranges.
        if reason(&observation, None).is_some() {
            return Err("Invalid numeric or date range cannot be released".into());
        }
        observations::upsert(&mut tx, &observation).await?;
    }
    sqlx::query!("DELETE FROM observations_quarantine WHERE id=$1", id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(value: f64, unit: &str) -> Observation {
        Observation {
            entity_id: "rust".into(),
            source_id: "github".into(),
            metric: "github.stars".into(),
            value,
            unit: unit.into(),
            period: "instant".into(),
            observed_at: "2026-10-17".into(),
        }
    }
    #[test]
    fn negative_count_is_held() {
        assert!(reason(&observation(-1.0, "count"), None).is_some());
    }
    #[test]
    fn percentage_range_is_enforced() {
        for value in [-1.0, 101.0] {
            assert!(reason(&observation(value, "pct"), None).is_some());
        }
    }
    #[test]
    fn cumulative_drop_is_held() {
        assert!(reason(&observation(79.0, "count"), Some(100.0)).is_some());
        assert!(reason(&observation(80.0, "count"), Some(100.0)).is_none());
    }
    #[test]
    fn cumulative_jump_is_held() {
        assert!(reason(&observation(1001.0, "count"), Some(100.0)).is_some());
        assert!(reason(&observation(1000.0, "count"), Some(100.0)).is_none());
    }
    #[test]
    fn first_value_and_zero_baseline_are_valid() {
        assert!(reason(&observation(1.0, "count"), None).is_none());
        assert!(reason(&observation(1.0, "count"), Some(0.0)).is_none());
    }

    #[test]
    fn first_counts_and_dates_cannot_poison_a_baseline() {
        assert!(reason(&observation(1e15 - 1.0, "count"), None).is_none());
        for value in [1e15, f64::MAX] {
            assert!(reason(&observation(value, "count"), None).is_some());
        }
        let mut o = observation(1.0, "count");
        for date in ["2007-12-31", "9999-12-31", "0000-01-01", "2026-02-30"] {
            o.observed_at = date.into();
            assert!(reason(&o, None).is_some(), "{date}");
        }
        for date in [
            "2008-01-01".into(),
            model::current_date().next_day().unwrap().to_string(),
        ] {
            o.observed_at = date;
            assert!(reason(&o, None).is_none());
        }
    }
}
