//! Bounded content URL checks with file/field diagnostics, never full URLs.
use crate::{content, http::Client};
use reqwest::{Method, Url, header::HeaderMap};
use serde::Serialize;
use serde_json::Value;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Clone)]
pub struct Link {
    pub file: String,
    pub field: String,
    pub url: String,
}
#[derive(Debug, Serialize)]
pub struct Problem {
    pub file: String,
    pub field: String,
    pub message: String,
}
#[derive(Default, Serialize)]
pub struct Report {
    pub checked: usize,
    pub problems: Vec<Problem>,
    pub skipped: Vec<Problem>,
}

pub fn collect(root: &Path) -> Result<Vec<Link>, String> {
    let content = content::load(root).map_err(|d| {
        d.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut links = Vec::new();
    for entity in content.entities {
        let file = format!("{}/{}.yaml", entity.kind.dir(), entity.id);
        walk(
            &serde_json::to_value(entity).map_err(|_| "Cannot serialize content")?,
            &file,
            "",
            &mut links,
        );
    }
    walk(
        &serde_json::to_value(content.vocab.domains).map_err(|_| "Cannot serialize vocabulary")?,
        "vocab/domains.yaml",
        "",
        &mut links,
    );
    walk(
        &serde_json::to_value(content.vocab.traits).map_err(|_| "Cannot serialize vocabulary")?,
        "vocab/traits.yaml",
        "",
        &mut links,
    );
    Ok(links)
}
fn walk(value: &Value, file: &str, field: &str, links: &mut Vec<Link>) {
    match value {
        Value::String(url) if url.starts_with("https://") || url.starts_with("http://") => links
            .push(Link {
                file: file.into(),
                field: field.into(),
                url: url.clone(),
            }),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk(item, file, &format!("{field}[{i}]"), links);
            }
        }
        Value::Object(items) => {
            for (name, item) in items {
                walk(
                    item,
                    file,
                    &if field.is_empty() {
                        name.clone()
                    } else {
                        format!("{field}.{name}")
                    },
                    links,
                );
            }
        }
        _ => {}
    }
}

pub async fn check(links: Vec<Link>, timeout: Duration) -> Result<Report, crate::ingest::Error> {
    let client = Client::with_timeout(timeout, Duration::from_millis(100))?;
    let slots = Arc::new(Semaphore::new(8));
    let mut tasks = JoinSet::new();
    let mut report = Report {
        checked: links.len(),
        ..Report::default()
    };
    for link in links {
        let permit = slots.clone().acquire_owned().await?;
        let client = client.clone();
        tasks.spawn(async move {
            let _permit = permit;
            let message = match tokio::time::timeout(timeout, check_one(&client, &link.url)).await {
                Ok(message) => message,
                Err(_) => Some("timeout after link-check deadline".into()),
            };
            message.map(|message| Problem {
                file: link.file,
                field: link.field,
                message,
            })
        });
    }
    while let Some(result) = tasks.join_next().await {
        if let Some(problem) = result? {
            report.problems.push(problem);
        }
    }
    report
        .problems
        .sort_by(|a, b| (&a.file, &a.field).cmp(&(&b.file, &b.field)));
    Ok(report)
}

async fn check_one(client: &Client, url: &str) -> Option<String> {
    let original = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => return Some("invalid URL".into()),
    };
    let response = match client.request(Method::HEAD, url, HeaderMap::new()).await {
        Ok(response) if response.status < 400 => Ok(response),
        _ => client.get(url).await,
    };
    match response {
        Err(error) => Some(error.to_string()),
        Ok(response) if response.status >= 400 => Some(format!("HTTP {}", response.status)),
        Ok(response) if response.url.host_str() != original.host_str() => Some(format!(
            "redirects to another host ({})",
            response.url.host_str().unwrap_or("unknown")
        )),
        Ok(response) if (300..400).contains(&response.status) => {
            Some(format!("unresolved HTTP {} redirect", response.status))
        }
        Ok(_) => None,
    }
}
