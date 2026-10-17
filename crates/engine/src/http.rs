//! Shared bounded HTTP transport. Errors never contain URLs or response bodies.
use reqwest::{
    Method, Url,
    header::{HeaderMap, RETRY_AFTER},
};
use std::{
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const MAX_BODY: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Transport,
    Timeout,
    TooLarge,
    InvalidUrl,
    Deferred,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Transport => "HTTP transport failed",
            Self::Timeout => "HTTP request timed out",
            Self::TooLarge => "HTTP response exceeds 20 MiB",
            Self::InvalidUrl => "Invalid HTTP URL",
            Self::Deferred => "Server requested a retry more than five minutes from now",
        })
    }
}
impl std::error::Error for Error {}

pub struct Response {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub url: Url,
}

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    retry_base: Duration,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        Self::with_timeout(Duration::from_secs(30), Duration::from_millis(500))
    }

    /// Custom deadlines are used by bounded content checks and deterministic tests.
    pub fn with_timeout(timeout: Duration, retry_base: Duration) -> Result<Self, Error> {
        let inner = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(timeout)
            .gzip(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!(
                "Crispy-Potato/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/winksen/Crispy-Potato)"
            ))
            .build()
            .map_err(|_| Error::Transport)?;
        Ok(Self { inner, retry_base })
    }

    pub async fn get(&self, url: &str) -> Result<Response, Error> {
        self.request(Method::GET, url, HeaderMap::new()).await
    }

    pub async fn request(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
    ) -> Result<Response, Error> {
        let url = Url::parse(url).map_err(|_| Error::InvalidUrl)?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::InvalidUrl);
        }
        // Three attempts total, matching the release's acceptance criterion.
        for attempt in 0..3 {
            let result = self
                .inner
                .request(method.clone(), url.clone())
                .headers(headers.clone())
                .send()
                .await;
            match result {
                Ok(mut response) => {
                    let status = response.status().as_u16();
                    if (status == 429 || status >= 500) && attempt < 2 {
                        self.wait(attempt, response.headers()).await?;
                        continue;
                    }
                    if response
                        .content_length()
                        .is_some_and(|n| n > MAX_BODY as u64)
                    {
                        return Err(Error::TooLarge);
                    }
                    let mut body = Vec::new();
                    let response_headers = response.headers().clone();
                    let final_url = response.url().clone();
                    let mut timeout = false;
                    loop {
                        match response.chunk().await {
                            Ok(Some(chunk)) => {
                                if body.len().saturating_add(chunk.len()) > MAX_BODY {
                                    return Err(Error::TooLarge);
                                }
                                body.extend_from_slice(&chunk);
                            }
                            Ok(None) => break,
                            Err(error) if error.is_timeout() => {
                                timeout = true;
                                break;
                            }
                            Err(_) => return Err(Error::Transport),
                        }
                    }
                    if !timeout {
                        return Ok(Response {
                            status,
                            headers: response_headers,
                            body,
                            url: final_url,
                        });
                    }
                    if attempt == 2 {
                        return Err(Error::Timeout);
                    }
                    self.wait(attempt, &HeaderMap::new()).await?;
                }
                Err(error) if error.is_timeout() && attempt < 2 => {
                    self.wait(attempt, &HeaderMap::new()).await?
                }
                Err(error) => {
                    return Err(if error.is_timeout() {
                        Error::Timeout
                    } else {
                        Error::Transport
                    });
                }
            }
        }
        Err(Error::Transport)
    }

    async fn wait(&self, attempt: u32, headers: &HeaderMap) -> Result<(), Error> {
        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
            % 100;
        let backoff =
            self.retry_base * 2_u32.pow(attempt) + Duration::from_millis(u64::from(jitter));
        let retry = headers
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(retry_after)
            .unwrap_or_default();
        if retry > Duration::from_secs(300) {
            return Err(Error::Deferred);
        }
        tokio::time::sleep(backoff.max(retry)).await;
        Ok(())
    }
}

fn retry_after(value: &str) -> Option<Duration> {
    value
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .or_else(|| {
            httpdate::parse_http_date(value)
                .ok()
                .map(|time| time.duration_since(SystemTime::now()).unwrap_or_default())
        })
}
