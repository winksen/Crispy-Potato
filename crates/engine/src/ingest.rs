//! Source contract and per-entity transactional collection runner.
use crate::{
    http::Client,
    http_cache,
    observations::{self, Observation},
};
use model::Entity;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{future::Future, pin::Pin};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type FetchFuture<'a> = Pin<Box<dyn Future<Output = Result<Batch, &'static str>> + Send + 'a>>;

pub struct SourceInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub license: &'static str,
    pub attribution: &'static str,
    pub schedule_seconds: i64,
}
pub struct Context<'a> {
    pub pool: &'a PgPool,
    pub http: &'a Client,
}
pub trait Source: Send + Sync {
    fn info(&self) -> SourceInfo;
    fn applies(&self, entity: &Entity) -> bool;
    fn fetch<'a>(&'a self, context: &'a Context<'a>, entity: &'a Entity) -> FetchFuture<'a>;
}

/// Concrete adapters arrive in track F; unknown names never start a run.
pub fn registered_sources() -> Vec<Box<dyn Source>> {
    Vec::new()
}

fn entity_from_document(document: serde_json::Value) -> Result<Entity, serde_json::Error> {
    use model::{FrameworkTraits, Kind, LanguageTraits, RuntimeTraits, Traits};
    match serde_json::from_value::<Kind>(document["kind"].clone())? {
        Kind::Language => serde_json::from_value::<Entity<LanguageTraits>>(document)
            .map(|e| e.map_traits(Traits::Language)),
        Kind::Framework => serde_json::from_value::<Entity<FrameworkTraits>>(document)
            .map(|e| e.map_traits(Traits::Framework)),
        Kind::Runtime => serde_json::from_value::<Entity<RuntimeTraits>>(document)
            .map(|e| e.map_traits(Traits::Runtime)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub version: String,
    pub released_at: Option<String>,
    pub eol_at: Option<String>,
    pub lts: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub field: String,
    pub value: serde_json::Value,
}
#[derive(Default, Serialize)]
pub struct Batch {
    pub observations: Vec<Observation>,
    pub releases: Vec<Release>,
    pub facts: Vec<Fact>,
    pub unchanged: bool,
    #[serde(skip)]
    pub cache: Vec<http_cache::Update>,
}
#[derive(Serialize)]
pub struct Failure {
    pub entity: String,
    pub reason: &'static str,
}
#[derive(Serialize)]
pub struct Report {
    pub run_id: Option<i64>,
    pub status: &'static str,
    pub changed: i32,
    pub unchanged: i32,
    pub errors: Vec<Failure>,
    pub preview: Vec<(String, Batch)>,
}

pub async fn run(
    pool: &PgPool,
    source: &dyn Source,
    entity_id: Option<&str>,
    dry_run: bool,
) -> Result<Report, Error> {
    let info = source.info();
    let rows = sqlx::query!("SELECT document FROM entities WHERE NOT retired AND ($1::text IS NULL OR id=$1) ORDER BY id", entity_id).fetch_all(pool).await?;
    if entity_id.is_some() && rows.is_empty() {
        return Err("Unknown or retired entity".into());
    }
    let entities = rows
        .into_iter()
        .map(|row| entity_from_document(row.document))
        .collect::<Result<Vec<_>, _>>()?;
    let run_id = if dry_run {
        None
    } else {
        sqlx::query!("INSERT INTO sources (id,name,url,license,attribution,schedule_seconds) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO UPDATE SET name=EXCLUDED.name,url=EXCLUDED.url,license=EXCLUDED.license,attribution=EXCLUDED.attribution,schedule_seconds=EXCLUDED.schedule_seconds", info.id, info.name, info.url, info.license, info.attribution, info.schedule_seconds).execute(pool).await?;
        Some(
            sqlx::query_scalar!(
                "INSERT INTO ingestion_runs (source_id,status) VALUES ($1,'running') RETURNING id",
                info.id
            )
            .fetch_one(pool)
            .await?,
        )
    };
    let http = Client::new()?;
    let context = Context { pool, http: &http };
    let mut report = Report {
        run_id,
        status: "success",
        changed: 0,
        unchanged: 0,
        errors: Vec::new(),
        preview: Vec::new(),
    };
    for entity in entities.iter().filter(|e| source.applies(e)) {
        let mut batch = match source.fetch(&context, entity).await {
            Ok(batch) => batch,
            Err(_) => {
                report.errors.push(Failure {
                    entity: entity.id.clone(),
                    reason: "fetch failed",
                });
                continue;
            }
        };
        if crate::untrusted::clean(&mut batch).is_err() {
            report.errors.push(Failure {
                entity: entity.id.clone(),
                reason: "invalid source response",
            });
            continue;
        }
        if batch
            .observations
            .iter()
            .any(|o| o.entity_id != entity.id || o.source_id != info.id)
            || (batch.unchanged
                && (!batch.observations.is_empty()
                    || !batch.releases.is_empty()
                    || !batch.facts.is_empty()))
        {
            report.errors.push(Failure {
                entity: entity.id.clone(),
                reason: "invalid batch identity or unchanged payload",
            });
            continue;
        }
        if !dry_run && store(pool, entity, info.id, &batch).await.is_err() {
            report.errors.push(Failure {
                entity: entity.id.clone(),
                reason: "storage failed; entity rolled back",
            });
            continue;
        }
        if batch.unchanged {
            report.unchanged += 1;
        } else {
            report.changed += 1;
        }
        if dry_run {
            report.preview.push((entity.id.clone(), batch));
        }
    }
    if !report.errors.is_empty() {
        report.status = if report.changed + report.unchanged == 0 {
            "failed"
        } else {
            "partial"
        };
    }
    if let Some(id) = run_id {
        let errors = serde_json::to_value(&report.errors)?;
        sqlx::query!("UPDATE ingestion_runs SET finished_at=now(),status=$2,changed=$3,unchanged=$4,errors=$5 WHERE id=$1", id, report.status, report.changed, report.unchanged, errors).execute(pool).await?;
    }
    Ok(report)
}

async fn store(pool: &PgPool, entity: &Entity, source: &str, batch: &Batch) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    for observation in &batch.observations {
        observations::upsert(&mut tx, observation).await?;
    }
    for release in &batch.releases {
        sqlx::query!("INSERT INTO releases (entity_id,version,released_at,eol_at,lts,source_id) VALUES ($1,$2,$3::text::date,$4::text::date,$5,$6) ON CONFLICT (entity_id,version,source_id) DO UPDATE SET released_at=EXCLUDED.released_at,eol_at=EXCLUDED.eol_at,lts=EXCLUDED.lts", entity.id, release.version, release.released_at, release.eol_at, release.lts, source).execute(&mut *tx).await?;
    }
    for fact in &batch.facts {
        sqlx::query!("INSERT INTO facts (entity_id,field,value,source_id) VALUES ($1,$2,$3,$4) ON CONFLICT (entity_id,field,source_id) DO UPDATE SET value=EXCLUDED.value,collected_at=now()", entity.id, fact.field, fact.value, source).execute(&mut *tx).await?;
    }
    for update in &batch.cache {
        http_cache::persist(&mut tx, update).await?;
    }
    tx.commit().await?;
    Ok(())
}
