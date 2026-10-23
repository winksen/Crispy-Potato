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
    Dns,
    Connect,
    Tls,
    Protocol,
    Blocked,
    Timeout,
    TooLarge,
    InvalidUrl,
    Deferred,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dns => "HTTP DNS resolution failed",
            Self::Connect => "HTTP connection failed",
            Self::Tls => "HTTP TLS verification failed",
            Self::Protocol => "HTTP protocol failed",
            Self::Blocked => "HTTP egress destination refused",
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
    policy: crate::egress::Policy,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        Self::with_timeout(Duration::from_secs(30), Duration::from_millis(500))
    }

    /// Custom deadlines are used by bounded content checks and deterministic tests.
    pub fn with_timeout(timeout: Duration, retry_base: Duration) -> Result<Self, Error> {
        Self::build(timeout, retry_base, crate::egress::Policy::Source)
    }

    pub fn for_links(timeout: Duration) -> Result<Self, Error> {
        Self::build(
            timeout,
            Duration::from_millis(100),
            crate::egress::Policy::Links,
        )
    }

    #[cfg(test)]
    pub(crate) fn test_local(
        origin: &str,
        timeout: Duration,
        retry_base: Duration,
    ) -> Result<Self, Error> {
        let origin = Url::parse(origin).map_err(|_| Error::InvalidUrl)?;
        if !matches!(origin.host(), Some(url::Host::Ipv4(ip)) if ip.is_loopback()) {
            return Err(Error::InvalidUrl);
        }
        Self::build(
            timeout,
            retry_base,
            crate::egress::Policy::LocalTest(origin),
        )
    }

    fn build(
        timeout: Duration,
        retry_base: Duration,
        policy: crate::egress::Policy,
    ) -> Result<Self, Error> {
        // Concurrent client constructors may race; a previously installed
        // provider is fine. The workspace enables only ring, never aws-lc.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let inner = reqwest::Client::builder()
            .no_proxy()
            .dns_resolver(std::sync::Arc::new(crate::egress::PublicResolver))
            .connect_timeout(Duration::from_secs(5))
            .timeout(timeout)
            .gzip(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!(
                "Crispy-Potato/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/winksen/Crispy-Potato)"
            ))
            .build()
            .map_err(|error| classify(&error))?;
        Ok(Self {
            inner,
            retry_base,
            policy,
        })
    }

    pub async fn get(&self, url: &str) -> Result<Response, Error> {
        self.request(Method::GET, url, HeaderMap::new()).await
    }

    pub async fn request(
        &self,
        mut method: Method,
        url: &str,
        mut headers: HeaderMap,
    ) -> Result<Response, Error> {
        let mut url = Url::parse(url).map_err(|_| Error::InvalidUrl)?;
        for _ in 0..=5 {
            let response = self
                .request_once(method.clone(), url.as_str(), headers.clone())
                .await?;
            if !matches!(response.status, 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            let Some(location) = response
                .headers
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
            else {
                return Ok(response);
            };
            let next = url.join(location).map_err(|_| Error::InvalidUrl)?;
            if next.origin() != url.origin() {
                headers.remove(reqwest::header::AUTHORIZATION);
                headers.remove(reqwest::header::COOKIE);
            }
            if response.status == 303 && method != Method::HEAD {
                method = Method::GET;
            }
            url = next;
        }
        Err(Error::Protocol)
    }

    async fn request_once(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
    ) -> Result<Response, Error> {
        let url = Url::parse(url).map_err(|_| Error::InvalidUrl)?;
        self.policy.validate(&url)?;
        // Three attempts total, matching the release's acceptance criterion.
        for attempt in 0..3 {
            let host = crate::limits::host(url.host_str().ok_or(Error::InvalidUrl)?).await;
            let permit = host.acquire().await.map_err(|_| Error::Protocol)?;
            let result = self
                .inner
                .request(method.clone(), url.clone())
                .headers(headers.clone())
                .send()
                .await;
            match result {
                Ok(mut response) => {
                    let status = response.status().as_u16();
                    if method != Method::HEAD
                        && response
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
                            Err(error) => return Err(classify(&error)),
                        }
                    }
                    drop(response);
                    if !timeout {
                        if url.host_str() == Some("api.stackexchange.com") {
                            crate::limits::stackexchange_backoff(&host, &body).await?;
                        }
                        if (status == 429 || status >= 500) && attempt < 2 {
                            drop(permit);
                            self.wait(attempt, &response_headers).await?;
                            continue;
                        }
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
                    drop(permit);
                    self.wait(attempt, &HeaderMap::new()).await?;
                }
                Err(error) if error.is_timeout() && attempt < 2 => {
                    drop(permit);
                    self.wait(attempt, &HeaderMap::new()).await?
                }
                Err(error) => {
                    return Err(classify(&error));
                }
            }
        }
        Err(Error::Protocol)
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

fn classify(error: &reqwest::Error) -> Error {
    use std::error::Error as _;
    let mut source = error.source();
    while let Some(cause) = source {
        if cause.is::<crate::egress::Denied>() {
            return Error::Blocked;
        }
        if cause.is::<rustls::Error>() {
            return Error::Tls;
        }
        source = cause.source();
    }
    if error.is_timeout() {
        Error::Timeout
    } else if error.is_dns() {
        Error::Dns
    } else if error.is_connect() {
        Error::Connect
    } else {
        Error::Protocol
    }
}
