//! Credentials never implement ordinary printable or serializable strings.
use reqwest::{
    Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use std::{fmt, fs::File, io::Read};

#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub github_token: Option<SecretString>,
    pub stackexchange_key: Option<SecretString>,
}
#[derive(Clone, Copy)]
pub enum RequiredSecret {
    GitHub,
    StackExchange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseSettings {
    pub max_connections: u32,
    pub acquire_timeout_ms: u64,
    pub statement_timeout_ms: u64,
}

impl DatabaseSettings {
    pub fn load(reader: bool) -> Result<Self, String> {
        Self::from_lookup(|name| std::env::var(name).ok(), reader)
    }
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
        reader: bool,
    ) -> Result<Self, String> {
        fn number(
            lookup: &impl Fn(&str) -> Option<String>,
            name: &str,
            default: u64,
            min: u64,
            max: u64,
        ) -> Result<u64, String> {
            match lookup(name) {
                None => Ok(default),
                Some(value) => value
                    .parse::<u64>()
                    .ok()
                    .filter(|value| (min..=max).contains(value))
                    .ok_or_else(|| format!("{name} must be between {min} and {max}")),
            }
        }
        let statement = number(
            &lookup,
            "DB_STATEMENT_TIMEOUT_MS",
            if reader { 5000 } else { 30000 },
            100,
            300000,
        )?;
        Ok(Self {
            max_connections: number(&lookup, "DB_POOL_MAX_CONNECTIONS", 5, 1, 20)? as u32,
            acquire_timeout_ms: number(&lookup, "DB_ACQUIRE_TIMEOUT_MS", 5000, 100, 60000)?,
            statement_timeout_ms: if reader {
                statement.min(5000)
            } else {
                statement
            },
        })
    }
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
    fn database_settings_validate_bounds_and_cap_reader_deadlines() {
        let writer = DatabaseSettings::from_lookup(|_| None, false).unwrap();
        assert_eq!(
            writer,
            DatabaseSettings {
                max_connections: 5,
                acquire_timeout_ms: 5000,
                statement_timeout_ms: 30000
            }
        );
        assert_eq!(
            DatabaseSettings::from_lookup(|_| None, true)
                .unwrap()
                .statement_timeout_ms,
            5000
        );
        let reader = DatabaseSettings::from_lookup(
            |name| (name == "DB_STATEMENT_TIMEOUT_MS").then(|| "300000".into()),
            true,
        )
        .unwrap();
        assert_eq!(reader.statement_timeout_ms, 5000);
        for (name, value) in [
            ("DB_POOL_MAX_CONNECTIONS", "0"),
            ("DB_POOL_MAX_CONNECTIONS", "21"),
            ("DB_ACQUIRE_TIMEOUT_MS", "60001"),
            ("DB_STATEMENT_TIMEOUT_MS", "0"),
            ("DB_STATEMENT_TIMEOUT_MS", "private-invalid-value"),
        ] {
            let error =
                DatabaseSettings::from_lookup(|key| (key == name).then(|| value.into()), false)
                    .unwrap_err();
            assert!(error.starts_with(name));
            assert!(!error.contains("private-invalid-value"));
        }
    }
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
