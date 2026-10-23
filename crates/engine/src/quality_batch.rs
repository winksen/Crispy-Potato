//! One baseline read and one write per table for an observation batch.
use crate::{
    ingest::Error,
    observations::Observation,
    quality::{Held, reason},
};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;

type Series = (String, String, String, String);
fn series(o: &Observation) -> Series {
    (
        o.entity_id.clone(),
        o.source_id.clone(),
        o.metric.clone(),
        o.period.clone(),
    )
}

pub(crate) async fn process(
    tx: &mut Transaction<'_, Postgres>,
    values: &[Observation],
    persist: bool,
) -> Result<Vec<Held>, Error> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<_> = values
        .iter()
        .map(|o| o.entity_id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if persist {
        let locked = sqlx::query!(
            "SELECT id FROM entities WHERE id=ANY($1) ORDER BY id FOR UPDATE",
            &ids
        )
        .fetch_all(&mut **tx)
        .await?;
        if locked.len() != ids.len() {
            return Err("unknown batch entity".into());
        }
    }
    let mut ranges: BTreeMap<Series, (String, String)> = BTreeMap::new();
    for o in values.iter().filter(|o| reason(o, None).is_none()) {
        let range = ranges
            .entry(series(o))
            .or_insert_with(|| (o.observed_at.clone(), o.observed_at.clone()));
        range.0 = range.0.clone().min(o.observed_at.clone());
        range.1 = range.1.clone().max(o.observed_at.clone());
    }
    let mut entities = Vec::new();
    let mut sources = Vec::new();
    let mut metrics = Vec::new();
    let mut periods = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for ((entity, source, metric, period), (start, end)) in &ranges {
        entities.push(entity.clone());
        sources.push(source.clone());
        metrics.push(metric.clone());
        periods.push(period.clone());
        starts.push(start.clone());
        ends.push(end.clone());
    }
    // Read the baseline before each range plus existing points within it. This
    // preserves historical corrections without using a later point as baseline.
    let rows = sqlx::query!(r#"SELECT s.entity AS "entity!", s.source AS "source!", s.metric AS "metric!", s.period AS "period!", o.observed_at::text AS "date!", o.value AS "value!"
        FROM UNNEST($1::text[],$2::text[],$3::text[],$4::text[],$5::text[],$6::text[]) AS s(entity,source,metric,period,start_date,end_date)
        CROSS JOIN LATERAL (
            (SELECT observed_at,value FROM observations WHERE entity_id=s.entity AND source_id=s.source AND metric=s.metric AND period=s.period AND observed_at < s.start_date::date ORDER BY observed_at DESC LIMIT 1)
            UNION ALL
            SELECT observed_at,value FROM observations WHERE entity_id=s.entity AND source_id=s.source AND metric=s.metric AND period=s.period AND observed_at BETWEEN s.start_date::date AND s.end_date::date
        ) o"#, &entities, &sources, &metrics, &periods, &starts, &ends).fetch_all(&mut **tx).await?;
    let mut history: BTreeMap<Series, BTreeMap<String, f64>> = BTreeMap::new();
    for row in rows {
        history
            .entry((row.entity, row.source, row.metric, row.period))
            .or_default()
            .insert(row.date, row.value);
    }
    let mut ordered: Vec<_> = values.iter().collect();
    ordered.sort_by_key(|o| (series(o), o.observed_at.clone()));
    let mut accepted = BTreeMap::new();
    let mut held_rows = BTreeMap::new();
    let mut held = Vec::new();
    for o in ordered {
        let key = series(o);
        let points = history.entry(key.clone()).or_default();
        let last = points
            .range(..=o.observed_at.clone())
            .next_back()
            .map(|(_, value)| *value);
        if let Some(why) = reason(o, last) {
            held_rows.insert((key, o.observed_at.clone()), (o, why));
            held.push(Held {
                observation: o.clone(),
                reason: why.into(),
            });
        } else {
            points.insert(o.observed_at.clone(), o.value);
            accepted.insert((key, o.observed_at.clone()), o);
        }
    }
    if persist {
        let accepted: Vec<_> = accepted.into_values().cloned().collect();
        crate::observations::upsert_batch(tx, &accepted).await?;
        let mut entities = Vec::new();
        let mut sources = Vec::new();
        let mut documents = Vec::new();
        let mut reasons = Vec::new();
        for (o, reason) in held_rows.into_values() {
            entities.push(o.entity_id.clone());
            sources.push(o.source_id.clone());
            documents.push(serde_json::to_value(o)?);
            reasons.push(reason.to_owned());
        }
        sqlx::query!("INSERT INTO observations_quarantine (entity_id,source_id,document,reason) SELECT * FROM UNNEST($1::text[],$2::text[],$3::jsonb[],$4::text[]) ON CONFLICT (entity_id,source_id,(document->>'metric'),(document->>'period'),(document->>'observed_at')) DO UPDATE SET document=EXCLUDED.document,reason=EXCLUDED.reason,held_at=now()", &entities,&sources,&documents,&reasons).execute(&mut **tx).await?;
    }
    Ok(held)
}
