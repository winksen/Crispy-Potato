//! Live identifier checks. Survey labels and build-time logos have separate validators.
use crate::{
    content,
    http::Client,
    links::{Problem, Report},
    settings::Settings,
    untrusted,
};
use reqwest::{Method, Url, header::HeaderMap};
use serde_json::Value;
use std::{
    collections::{BTreeSet, VecDeque},
    path::Path,
    time::Duration,
};

const DEADLINE: Duration = Duration::from_secs(30);
// Programming languages are included explicitly: Wikidata does not classify the
// language itself as software. Libraries and frameworks are software subclasses.
const SOFTWARE: &[&str] = &["Q7397", "Q9143", "Q188860", "Q271680"];

pub async fn check(root: &Path, settings: &Settings) -> Result<Report, crate::ingest::Error> {
    let content = content::load(root).map_err(|d| {
        d.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let client = Client::new()?;
    let mut report = Report::default();
    for entity in content.entities {
        for (provider, id) in entity.ids.iter().filter(|(p, _)| *p != "simple_icons") {
            let file = format!("{}/{}.yaml", entity.kind.dir(), entity.id);
            let field = format!("ids.{provider}");
            let missing = match provider {
                "github" if settings.github_token.is_none() => Some("GITHUB_TOKEN is missing"),
                "stackoverflow_tag" if settings.stackexchange_key.is_none() => {
                    Some("STACKEXCHANGE_KEY is missing")
                }
                _ => None,
            };
            if let Some(message) = missing {
                report.skipped.push(Problem {
                    file,
                    field,
                    message: message.into(),
                });
                continue;
            }
            report.checked += 1;
            let result =
                tokio::time::timeout(DEADLINE, verify(&client, settings, provider, id)).await;
            let error = match result {
                Ok(result) => result.err(),
                Err(_) => Some("verification deadline exceeded".into()),
            };
            if let Some(message) = error {
                report.problems.push(problem(file, provider, message));
            }
        }
    }
    Ok(report)
}

fn problem(file: String, provider: &str, message: String) -> Problem {
    Problem {
        file,
        field: format!("ids.{provider}"),
        message: format!("{provider}: {message}"),
    }
}

fn endpoint(provider: &str, id: &str) -> Result<Url, String> {
    let (base, parts): (&str, Vec<String>) = match provider {
        "github" => (
            "https://api.github.com/",
            [
                vec!["repos".into()],
                id.split('/').map(str::to_owned).collect(),
            ]
            .concat(),
        ),
        "npm" => ("https://registry.npmjs.org/", vec![id.into()]),
        "pypi" => (
            "https://pypi.org/",
            vec!["pypi".into(), id.into(), "json".into()],
        ),
        "crates" => (
            "https://crates.io/",
            vec!["api".into(), "v1".into(), "crates".into(), id.into()],
        ),
        "nuget" => (
            "https://api.nuget.org/",
            vec![
                "v3-flatcontainer".into(),
                id.to_lowercase(),
                "index.json".into(),
            ],
        ),
        "packagist" => (
            "https://repo.packagist.org/",
            vec![
                "p2".into(),
                id.split('/').next().ok_or("Invalid package")?.into(),
                format!("{}.json", id.split('/').nth(1).ok_or("Invalid package")?),
            ],
        ),
        "rubygems" => (
            "https://rubygems.org/",
            vec![
                "api".into(),
                "v1".into(),
                "gems".into(),
                format!("{id}.json"),
            ],
        ),
        "pub" => (
            "https://pub.dev/",
            vec!["api".into(), "packages".into(), id.into()],
        ),
        "hex" => (
            "https://hex.pm/",
            vec!["api".into(), "packages".into(), id.into()],
        ),
        "maven" => (
            "https://search.maven.org/",
            vec!["solrsearch".into(), "select".into()],
        ),
        "wikidata" => (
            "https://www.wikidata.org/",
            vec![
                "wiki".into(),
                "Special:EntityData".into(),
                format!("{id}.json"),
            ],
        ),
        "stackoverflow_tag" => (
            "https://api.stackexchange.com/",
            vec!["2.3".into(), "tags".into(), id.into(), "info".into()],
        ),
        "endoflife" => (
            "https://endoflife.date/",
            vec!["api".into(), "v1".into(), "products".into(), id.into()],
        ),
        _ => return Err("Unsupported provider".into()),
    };
    let mut url = Url::parse(base).map_err(|_| "Invalid provider URL")?;
    url.path_segments_mut()
        .map_err(|_| "Invalid provider URL")?
        .clear()
        .extend(parts);
    if provider == "maven" {
        let (group, artifact) = id.split_once(':').ok_or("Invalid Maven coordinates")?;
        url.query_pairs_mut()
            .append_pair("q", &format!("g:\"{group}\" AND a:\"{artifact}\""))
            .append_pair("rows", "1")
            .append_pair("wt", "json");
    } else if provider == "stackoverflow_tag" {
        url.query_pairs_mut().append_pair("site", "stackoverflow");
    }
    Ok(url)
}

async fn fetch(client: &Client, url: &Url, headers: HeaderMap) -> Result<Value, String> {
    let response = client
        .request(Method::GET, url.as_str(), headers)
        .await
        .map_err(|e| e.to_string())?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "HTTP {} (identifier not verified)",
            response.status
        ));
    }
    untrusted::decode(&response.body).map_err(str::to_owned)
}

async fn verify(
    client: &Client,
    settings: &Settings,
    provider: &str,
    id: &str,
) -> Result<(), String> {
    let mut url = endpoint(provider, id)?;
    let headers = if provider == "github" {
        settings.github_headers()?
    } else {
        HeaderMap::new()
    };
    if provider == "stackoverflow_tag" {
        url = settings.stackexchange_url(url)?;
    }
    let value = fetch(client, &url, headers).await?;
    if provider == "wikidata" {
        return software_class(client, id, &value).await;
    }
    validate(provider, id, &value)
}

fn validate(provider: &str, id: &str, value: &Value) -> Result<(), String> {
    if provider == "github" {
        let name = value["full_name"]
            .as_str()
            .ok_or("Invalid repository response")?;
        let archived = value["archived"]
            .as_bool()
            .ok_or("Invalid repository response")?;
        let mut problems = Vec::new();
        if archived {
            problems.push("repository is archived".into());
        }
        if !name.eq_ignore_ascii_case(id) {
            problems.push(format!("repository renamed to {}", untrusted::text(name)));
        }
        return if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; "))
        };
    }
    let found = match provider {
        "npm" | "rubygems" | "pub" | "hex" => value["name"].as_str() == Some(id),
        "pypi" => value["info"]["name"]
            .as_str()
            .is_some_and(|name| normalize_pypi(name) == normalize_pypi(id)),
        "crates" => value["crate"]["id"].as_str() == Some(id),
        "nuget" => value["versions"].as_array().is_some_and(|v| !v.is_empty()),
        "packagist" => value["packages"][id]
            .as_array()
            .is_some_and(|v| !v.is_empty()),
        "maven" => value["response"]["docs"]
            .as_array()
            .is_some_and(|docs| docs.iter().any(|d| d["id"].as_str() == Some(id))),
        "stackoverflow_tag" => value["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["name"].as_str() == Some(id))),
        "endoflife" => value["result"]["name"].as_str() == Some(id),
        _ => false,
    };
    if found {
        Ok(())
    } else {
        Err("identifier missing or unexpected API response".into())
    }
}

fn normalize_pypi(name: &str) -> String {
    name.to_ascii_lowercase()
        .split(['-', '_', '.'])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn claims(value: &Value, id: &str, property: &str) -> Vec<String> {
    value["entities"][id]["claims"][property]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|claim| claim["rank"] != "deprecated")
        .filter_map(|claim| claim["mainsnak"]["datavalue"]["value"]["id"].as_str())
        .filter(|id| {
            id.starts_with('Q') && id.len() > 1 && id[1..].bytes().all(|b| b.is_ascii_digit())
        })
        .map(str::to_owned)
        .collect()
}

async fn software_class(client: &Client, id: &str, value: &Value) -> Result<(), String> {
    let mut pending: VecDeque<_> = claims(value, id, "P31")
        .into_iter()
        .map(|id| (id, 0))
        .collect();
    let mut visited = BTreeSet::new();
    // Check all direct classes first, so an unrelated first P31 cannot obscure a valid class.
    if pending
        .iter()
        .any(|(id, _)| SOFTWARE.contains(&id.as_str()))
    {
        return Ok(());
    }
    while let Some((id, depth)) = pending.pop_front() {
        if SOFTWARE.contains(&id.as_str()) {
            return Ok(());
        }
        if !visited.insert(id.clone()) {
            continue;
        }
        if visited.len() > 32 || depth >= 5 {
            return Err("software classification exceeds verification limits".into());
        }
        let class = fetch(client, &endpoint("wikidata", &id)?, HeaderMap::new()).await?;
        pending.extend(
            claims(&class, &id, "P279")
                .into_iter()
                .map(|id| (id, depth + 1)),
        );
    }
    Err("item missing or not an instance of a software/programming-language class".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    #[test]
    fn repository_rename_archive_and_malformed_responses() {
        assert!(
            validate(
                "github",
                "old/repo",
                &json!({"full_name":"new/repo", "archived":true})
            )
            .unwrap_err()
            .contains("renamed to new/repo")
        );
        assert!(
            validate(
                "github",
                "old/repo",
                &json!({"full_name":"old/repo", "archived":true})
            )
            .unwrap_err()
            .contains("archived")
        );
        assert!(validate("github", "old/repo", &json!({})).is_err());
        assert!(
            validate(
                "github",
                "old/repo",
                &json!({"full_name":"OLD/repo", "archived":false})
            )
            .is_ok()
        );
    }

    #[test]
    fn every_registry_and_named_service_requires_identity_or_versions() {
        for (provider, id, response) in [
            ("npm", "@scope/pkg", json!({"name":"@scope/pkg"})),
            ("pypi", "My_Package", json!({"info":{"name":"my-package"}})),
            ("crates", "serde", json!({"crate":{"id":"serde"}})),
            ("nuget", "Example", json!({"versions":["1.0"]})),
            (
                "packagist",
                "vendor/pkg",
                json!({"packages":{"vendor/pkg":[{"version":"1.0"}]}}),
            ),
            ("rubygems", "rails", json!({"name":"rails"})),
            ("pub", "flutter", json!({"name":"flutter"})),
            ("hex", "phoenix", json!({"name":"phoenix"})),
            (
                "maven",
                "org.example:pkg",
                json!({"response":{"docs":[{"id":"org.example:pkg"}]}}),
            ),
            (
                "stackoverflow_tag",
                "rust",
                json!({"items":[{"name":"rust"}]}),
            ),
            ("endoflife", "rust", json!({"result":{"name":"rust"}})),
        ] {
            assert!(validate(provider, id, &response).is_ok(), "{provider}");
            assert!(validate(provider, id, &json!({})).is_err(), "{provider}");
            assert_eq!(endpoint(provider, id).unwrap().scheme(), "https");
        }
        assert!(
            endpoint("npm", "@scope/pkg")
                .unwrap()
                .as_str()
                .contains("@scope%2Fpkg")
        );
        assert!(
            endpoint("stackoverflow_tag", "c++")
                .unwrap()
                .as_str()
                .contains("c++/info?site=stackoverflow")
        );
    }

    #[tokio::test]
    async fn missing_package_is_reported_with_registry_and_field() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let error = fetch(
            &Client::new().unwrap(),
            &Url::parse(&server.uri()).unwrap(),
            HeaderMap::new(),
        )
        .await
        .unwrap_err();
        assert!(error.contains("404"));
        let diagnostic = problem("frameworks/react.yaml".into(), "npm", error);
        assert_eq!(diagnostic.field, "ids.npm");
        assert_eq!(diagnostic.file, "frameworks/react.yaml");
        assert!(diagnostic.message.starts_with("npm: HTTP 404"));
    }

    #[tokio::test]
    async fn wikidata_requires_a_software_instance_and_ignores_deprecated_claims() {
        let client = Client::new().unwrap();
        let value = json!({"entities":{"Q1":{"claims":{"P31":[{"mainsnak":{"datavalue":{"value":{"id":"Q188860"}}}}]}}}});
        assert!(software_class(&client, "Q1", &value).await.is_ok());
        assert!(software_class(&client, "Q2", &value).await.is_err());
        let deprecated = json!({"entities":{"Q1":{"claims":{"P31":[{"rank":"deprecated","mainsnak":{"datavalue":{"value":{"id":"Q188860"}}}}]}}}});
        assert!(software_class(&client, "Q1", &deprecated).await.is_err());
    }
}
