use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use crate::smtp::SmtpProbeResult;

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct CatchAllCacheValue {
    pub catch_all: bool,
    pub mx_host: Option<String>,
}

pub struct SmtpCache {
    ttl: Duration,
    email_results: RwLock<HashMap<String, CacheEntry<SmtpProbeResult>>>,
    catch_all_results: RwLock<HashMap<String, CacheEntry<CatchAllCacheValue>>>,
}

impl SmtpCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            email_results: RwLock::new(HashMap::new()),
            catch_all_results: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_email(&self, email: &str) -> Option<SmtpProbeResult> {
        let now = Instant::now();
        self.email_results
            .read()
            .await
            .get(email)
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.value.clone())
    }

    pub async fn set_email(&self, email: String, result: SmtpProbeResult) {
        self.email_results.write().await.insert(
            email,
            CacheEntry {
                value: result,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub async fn get_catch_all(&self, domain: &str) -> Option<CatchAllCacheValue> {
        let now = Instant::now();
        self.catch_all_results
            .read()
            .await
            .get(domain)
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.value.clone())
    }

    pub async fn set_catch_all(
        &self,
        domain: String,
        catch_all: bool,
        mx_host: Option<String>,
    ) {
        self.catch_all_results.write().await.insert(
            domain,
            CacheEntry {
                value: CatchAllCacheValue { catch_all, mx_host },
                expires_at: Instant::now() + self.ttl,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp::SmtpStatus;

    #[tokio::test]
    async fn cache_restores_email_and_catch_all_entries() {
        let cache = SmtpCache::new(Duration::from_secs(5));
        cache
            .set_email(
                "person@example.com".to_string(),
                SmtpProbeResult {
                    email: "person@example.com".to_string(),
                    outcome: SmtpStatus::Accepted,
                    ..Default::default()
                },
            )
            .await;
        cache
            .set_catch_all(
                "example.com".to_string(),
                true,
                Some("mx.example.com".to_string()),
            )
            .await;

        assert_eq!(
            cache.get_email("person@example.com").await.map(|value| value.outcome),
            Some(SmtpStatus::Accepted)
        );
        assert_eq!(
            cache.get_catch_all("example.com").await.map(|value| value.catch_all),
            Some(true)
        );
    }
}
