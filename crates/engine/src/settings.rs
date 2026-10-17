//! Credentials never implement ordinary printable or serializable strings.
use reqwest::{
    Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use std::{fmt, fs::File, io::Read};

#[derive(Debug, Default)]
pub struct Settings {
    pub github_token: Option<SecretString>,
    pub stackexchange_key: Option<SecretString>,
}
#[derive(Clone, Copy)]
pub enum RequiredSecret {
    GitHub,
    StackExchange,
}

impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Settings {{ github_token: {}, stackexchange_key: {} }}",
            if self.github_token.is_some() {
                "[REDACTED]"
            } else {
                "missing"
            },
            if self.stackexchange_key.is_some() {
                "[REDACTED]"
            } else {
                "missing"
            }
        )
    }
}
impl Settings {
    pub fn load() -> Result<Self, String> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        Ok(Self {
            github_token: read("GITHUB_TOKEN", &lookup)?,
            stackexchange_key: read("STACKEXCHANGE_KEY", &lookup)?,
        })
    }
    pub fn has(&self, secret: RequiredSecret) -> bool {
        match secret {
            RequiredSecret::GitHub => self.github_token.is_some(),
            RequiredSecret::StackExchange => self.stackexchange_key.is_some(),
        }
    }
    pub fn github_headers(&self) -> Result<HeaderMap, &'static str> {
        let token = self
            .github_token
            .as_ref()
            .ok_or("GITHUB_TOKEN is missing")?;
        let mut value = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
            .map_err(|_| "Invalid GitHub credential")?;
        value.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, value);
        Ok(headers)
    }
    /// Stack Exchange requires a query parameter. Never format the returned URL.
    pub fn stackexchange_url(&self, mut url: Url) -> Result<Url, &'static str> {
        let key = self
            .stackexchange_key
            .as_ref()
            .ok_or("STACKEXCHANGE_KEY is missing")?;
        url.query_pairs_mut()
            .append_pair("key", key.expose_secret());
        Ok(url)
    }
}

fn read(
    name: &str,
    lookup: &impl Fn(&str) -> Option<String>,
) -> Result<Option<SecretString>, String> {
    let value = if let Some(value) = lookup(name).filter(|v| !v.trim().is_empty()) {
        value
    } else if let Some(path) = lookup(&format!("{name}_FILE")).filter(|v| !v.is_empty()) {
        let mut value = String::new();
        File::open(path)
            .and_then(|file| file.take(16_385).read_to_string(&mut value))
            .map_err(|_| format!("Cannot read {name}_FILE"))?;
        value
    } else {
        return Ok(None);
    };
    if value.len() > 16_384 {
        return Err(format!("{name} exceeds the credential size limit"));
    }
    let value = value.trim().to_owned();
    Ok(if value.is_empty() {
        None
    } else {
        Some(SecretString::from(value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_display_and_headers_redact_credentials() {
        let settings = Settings::from_lookup(|name| match name {
            "GITHUB_TOKEN" => Some("github-sentinel-secret".into()),
            "STACKEXCHANGE_KEY" => Some("stack-sentinel-secret".into()),
            _ => None,
        })
        .unwrap();
        let printed = format!(
            "{settings:?} {settings} {:?}",
            settings.github_headers().unwrap()
        );
        assert!(!printed.contains("github-sentinel-secret"));
        assert!(!printed.contains("stack-sentinel-secret"));
        assert!(printed.contains("REDACTED"));
    }
    #[test]
    fn reads_files_and_environment_takes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        std::fs::write(&path, "file-secret\n").unwrap();
        let settings = Settings::from_lookup(|name| {
            (name == "GITHUB_TOKEN_FILE").then(|| path.display().to_string())
        })
        .unwrap();
        assert_eq!(
            settings.github_token.as_ref().unwrap().expose_secret(),
            "file-secret"
        );
        let settings = Settings::from_lookup(|name| match name {
            "GITHUB_TOKEN" => Some("env-secret".into()),
            "GITHUB_TOKEN_FILE" => Some("nonexistent".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(
            settings.github_token.as_ref().unwrap().expose_secret(),
            "env-secret"
        );
        assert!(Settings::default().github_token.is_none());
    }
}
