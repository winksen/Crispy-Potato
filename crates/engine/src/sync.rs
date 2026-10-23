//! Validate first, then replace the curated snapshot in one transaction.
use crate::content;
use crate::hashing::sha256_hex;
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
    let version = sha256_hex(serde_json::to_vec(&snapshot)?);
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
    let mut kinds = Vec::new();
    let mut names = Vec::new();
    let mut documents = Vec::new();
    let mut hashes = Vec::new();
    let mut aliases = Vec::new();
    let mut alias_entities = Vec::new();
    let mut relation_entities = Vec::new();
    let mut relation_kinds = Vec::new();
    let mut relation_targets = Vec::new();
    let mut sample_entities = Vec::new();
    let mut sample_paths = Vec::new();
    let mut sample_texts = Vec::new();
    for entity in &content.entities {
        let document = serde_json::to_value(entity)?;
        hashes.push(sha256_hex(serde_json::to_vec(&document)?));
        kinds.push(entity.kind.to_string());
        names.push(entity.name.clone());
        documents.push(document);
        for alias in &entity.aliases {
            aliases.push(alias.clone());
            alias_entities.push(entity.id.clone());
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
                    relation_entities.push(from.clone());
                    relation_kinds.push(kind.to_owned());
                    relation_targets.push(to.clone());
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
            sample_entities.push(entity.id.clone());
            sample_paths.push(sample.clone());
            sample_texts.push(text);
        }
    }
    let changed = sqlx::query!("INSERT INTO entities (id,kind,name,document,content_hash) SELECT * FROM UNNEST($1::text[],$2::text[],$3::text[],$4::jsonb[],$5::text[]) ON CONFLICT (id) DO UPDATE SET kind=EXCLUDED.kind,name=EXCLUDED.name,document=EXCLUDED.document,content_hash=EXCLUDED.content_hash,retired=false WHERE entities.content_hash <> EXCLUDED.content_hash OR entities.retired",&ids,&kinds,&names,&documents,&hashes).execute(&mut *tx).await?.rows_affected();
    sqlx::query!(
        "INSERT INTO aliases (alias,entity_id) SELECT * FROM UNNEST($1::text[],$2::text[])",
        &aliases,
        &alias_entities
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("INSERT INTO relations (entity_id,relation,target_id) SELECT * FROM UNNEST($1::text[],$2::text[],$3::text[]) ON CONFLICT DO NOTHING",&relation_entities,&relation_kinds,&relation_targets).execute(&mut *tx).await?;
    sqlx::query!("INSERT INTO samples (entity_id,path,text) SELECT * FROM UNNEST($1::text[],$2::text[],$3::text[])",&sample_entities,&sample_paths,&sample_texts).execute(&mut *tx).await?;
    let mut planned_ids = Vec::new();
    let mut planned_kinds = Vec::new();
    for kind in model::Kind::ALL {
        for id in content.planned.ids(kind) {
            planned_ids.push(id.clone());
            planned_kinds.push(kind.to_string());
        }
    }
    sqlx::query!(
        "INSERT INTO planned_ids (id,kind) SELECT * FROM UNNEST($1::text[],$2::text[])",
        &planned_ids,
        &planned_kinds
    )
    .execute(&mut *tx)
    .await?;
    let mut vocab = serde_json::to_value(&content.vocab.traits)?;
    vocab["domains"] = serde_json::to_value(&content.vocab.domains)?;
    let mut groups_out = Vec::new();
    let mut terms_out = Vec::new();
    let mut definitions = Vec::new();
    if let Some(groups) = vocab.as_object() {
        for (group, terms) in groups {
            if let Some(terms) = terms.as_object() {
                for (term, document) in terms {
                    groups_out.push(group.clone());
                    terms_out.push(term.clone());
                    definitions.push(document.clone());
                }
            }
        }
    }
    sqlx::query!("INSERT INTO vocabulary_terms (vocabulary,term,document) SELECT * FROM UNNEST($1::text[],$2::text[],$3::jsonb[])",&groups_out,&terms_out,&definitions).execute(&mut *tx).await?;
    sqlx::query!("INSERT INTO meta (key,value) VALUES ('content_version',$1) ON CONFLICT (key) DO UPDATE SET value=EXCLUDED.value" ,&version).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Report {
        changed,
        retired,
        unchanged: false,
    })
}
