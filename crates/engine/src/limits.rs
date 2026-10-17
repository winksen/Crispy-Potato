//! One process-wide capacity-one token bucket and concurrency cap per host.
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::{
    sync::{Mutex, OwnedSemaphorePermit, Semaphore},
    time::Instant,
};

/// Conservative application quotas, not claims about each provider's maximum.
/// crates.io policy: https://crates.io/policies (at most one request/second).
pub const QUOTAS: &[(&str, &str, u64, usize)] = &[
    ("github", "api.github.com", 1000, 2),
    ("crates", "crates.io", 1000, 1),
    ("npm", "registry.npmjs.org", 1000, 2),
    ("npm downloads", "api.npmjs.org", 1000, 2),
    ("pypi", "pypi.org", 1000, 2),
    ("nuget", "api.nuget.org", 1000, 2),
    ("packagist", "repo.packagist.org", 1000, 2),
    ("rubygems", "rubygems.org", 1000, 2),
    ("pub", "pub.dev", 1000, 2),
    ("hex", "hex.pm", 1000, 2),
    ("maven", "search.maven.org", 1000, 2),
    ("wikidata", "www.wikidata.org", 1000, 2),
    ("stackoverflow", "api.stackexchange.com", 1000, 2),
    ("endoflife", "endoflife.date", 1000, 2),
    ("scorecard", "api.securityscorecards.dev", 1000, 2),
];

pub struct Host {
    next: Mutex<Instant>,
    interval: Duration,
    slots: Arc<Semaphore>,
}
impl Host {
    pub fn new(interval: Duration, concurrency: usize) -> Self {
        Self {
            next: Mutex::new(Instant::now()),
            interval,
            slots: Arc::new(Semaphore::new(concurrency.max(1))),
        }
    }
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit, tokio::sync::AcquireError> {
        let permit = self.slots.clone().acquire_owned().await?;
        loop {
            let mut next = self.next.lock().await;
            let now = Instant::now();
            if now >= *next {
                *next = now + self.interval;
                return Ok(permit);
            }
            let due = *next;
            drop(next);
            tokio::time::sleep_until(due).await;
        }
    }
    pub async fn defer(&self, delay: Duration) {
        let mut next = self.next.lock().await;
        // A malformed unrepresentable duration blocks for a century, never retries early.
        let due = Instant::now()
            .checked_add(delay)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(3_153_600_000));
        *next = (*next).max(due);
    }
}

static HOSTS: OnceLock<Mutex<HashMap<String, Arc<Host>>>> = OnceLock::new();
pub async fn host(name: &str) -> Arc<Host> {
    let name = name.to_ascii_lowercase();
    let mut hosts = HOSTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .await;
    hosts
        .entry(name.clone())
        .or_insert_with(|| {
            let quota = QUOTAS.iter().find(|q| q.1 == name);
            Arc::new(Host::new(
                Duration::from_millis(quota.map_or(100, |q| q.2)),
                quota.map_or(2, |q| q.3),
            ))
        })
        .clone()
}

pub async fn stackexchange_backoff(host: &Host, body: &[u8]) {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(default)]
        backoff: u64,
    }
    if let Ok(wrapper) = serde_json::from_slice::<Wrapper>(body) {
        host.defer(Duration::from_secs(wrapper.backoff)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn spaces_requests_and_honors_backoff() {
        let host = Host::new(Duration::from_secs(1), 2);
        let start = Instant::now();
        drop(host.acquire().await.unwrap());
        drop(host.acquire().await.unwrap());
        assert_eq!(start.elapsed(), Duration::from_secs(1));
        stackexchange_backoff(&host, br#"{"backoff":7}"#).await;
        drop(host.acquire().await.unwrap());
        assert_eq!(start.elapsed(), Duration::from_secs(8));
    }
    #[tokio::test(start_paused = true)]
    async fn two_sources_share_host_and_concurrency() {
        let a = host("shared-quota.example").await;
        let b = host("SHARED-QUOTA.EXAMPLE").await;
        assert!(Arc::ptr_eq(&a, &b));
        let one = a.acquire().await.unwrap();
        let two = b.acquire().await.unwrap();
        let task = tokio::spawn(async move { b.acquire().await.unwrap() });
        tokio::time::advance(Duration::from_secs(5)).await;
        assert!(!task.is_finished());
        drop(one);
        drop(task.await.unwrap());
        drop(two);
    }
}
