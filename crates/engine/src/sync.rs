//! Validate first, then replace the curated snapshot in one transaction.
use crate::content;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{collections::BTreeMap, fs, io::Read, path::Path};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub changed: u64,
    pub retired: u64,
    pub unchanged: bool,
}

fn files(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("linked content refused".into());
            }
        }
        if metadata.is_symlink() {
            return Err("linked content refused".into());
        }
        if metadata.is_dir() {
            files(root, &path, out)?;
        } else {
            let mut bytes = Vec::new();
            fs::File::open(&path)?
                .take(content::MAX_FILE_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > content::MAX_FILE_BYTES {
                return Err("oversized content refused".into());
            }
            out.insert(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                bytes,
            );
        }
    }
    Ok(())
}

pub async fn sync(pool: &PgPool, root: &Path) -> Result<Report, Error> {
    // Validate before reading samples or beginning any write transaction.
    content::load(root).map_err(|diags| {
        diags
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut snapshot = BTreeMap::new();
    files(root, root, &mut snapshot)?;
    // Parse exactly the bytes hashed and stored, even if the working tree changes.
    let stable = tempfile::tempdir()?;
    for (path, bytes) in &snapshot {
        let dest = stable.path().join(path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, bytes)?;
    }
    let content = content::load(stable.path()).map_err(|diags| {
        diags
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let version = format!("{:x}", Sha256::digest(serde_json::to_vec(&snapshot)?));
    let mut tx = pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock(1129337689)::text")
        .fetch_one(&mut *tx)
        .await?;
    let previous: Option<String> =
        sqlx::query_scalar!("SELECT value FROM meta WHERE key='content_version'")
            .fetch_optional(&mut *tx)
            .await?;
    if previous.as_ref().is_some_and(|v| *v == version) {
        return Ok(Report {
            changed: 0,
            retired: 0,
            unchanged: true,
        });
    }
    let ids: Vec<String> = content.entities.iter().map(|e| e.id.clone()).collect();
    let retired = sqlx::query!(
        "UPDATE entities SET retired=true WHERE NOT retired AND NOT (id = ANY($1))",
        &ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    // Delete/reinsert dependent curated data so removed aliases and edges disappear.
    sqlx::query!("DELETE FROM aliases")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM relations")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM samples")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM vocabulary_terms")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM planned_ids")
        .execute(&mut *tx)
        .await?;
    let mut changed = 0;
    for entity in &content.entities {
        let document = serde_json::to_value(entity)?;
        let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&document)?));
        changed += sqlx::query!("INSERT INTO entities (id,kind,name,document,content_hash) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (id) DO UPDATE SET kind=EXCLUDED.kind,name=EXCLUDED.name,document=EXCLUDED.document,content_hash=EXCLUDED.content_hash,retired=false WHERE entities.content_hash <> EXCLUDED.content_hash OR entities.retired" ,&entity.id, entity.kind.to_string(), &entity.name, &document, &hash).execute(&mut *tx).await?.rows_affected();
        for alias in &entity.aliases {
            sqlx::query!(
                "INSERT INTO aliases (alias,entity_id) VALUES ($1,$2)",
                alias,
                &entity.id
            )
            .execute(&mut *tx)
            .await?;
        }
        for (relation, targets) in entity.relations.iter() {
            let reverse = match relation {
                "built_on" => "foundation_of",
                "runs_on" => "runs",
                "succeeded_by" => "succeeds",
                other => other,
            };
            for target in targets {
                for (from, kind, to) in [
                    (&entity.id, relation, target),
                    (target, reverse, &entity.id),
                ] {
                    sqlx::query!("INSERT INTO relations (entity_id,relation,target_id) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING" ,from, kind, to).execute(&mut *tx).await?;
                }
            }
        }
        for sample in &entity.samples {
            let text = String::from_utf8(
                snapshot
                    .get(&format!("samples/{sample}"))
                    .ok_or("sample missing from snapshot")?
                    .clone(),
            )?;
            sqlx::query!(
                "INSERT INTO samples (entity_id,path,text) VALUES ($1,$2,$3)",
                &entity.id,
                sample,
                text
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    for kind in model::Kind::ALL {
        for id in content.planned.ids(kind) {
            sqlx::query!(
                "INSERT INTO planned_ids (id,kind) VALUES ($1,$2)",
                id,
                kind.to_string()
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    let mut vocab = serde_json::to_value(&content.vocab.traits)?;
    vocab["domains"] = serde_json::to_value(&content.vocab.domains)?;
    if let Some(groups) = vocab.as_object() {
        for (group, terms) in groups {
            if let Some(terms) = terms.as_object() {
                for (term, document) in terms {
                    sqlx::query!(
                        "INSERT INTO vocabulary_terms (vocabulary,term,document) VALUES ($1,$2,$3)",
                        group,
                        term,
                        document
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }
    sqlx::query!("INSERT INTO meta (key,value) VALUES ('content_version',$1) ON CONFLICT (key) DO UPDATE SET value=EXCLUDED.value" ,&version).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Report {
        changed,
        retired,
        unchanged: false,
    })
}
