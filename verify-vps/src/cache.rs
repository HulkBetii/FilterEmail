use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use crate::smtp::SmtpStatus;

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

pub struct SmtpCache {
    ttl: Duration,
    email_results: RwLock<HashMap<String, CacheEntry<SmtpStatus>>>,
    catch_all_results: RwLock<HashMap<String, CacheEntry<bool>>>,
}

impl SmtpCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            email_results: RwLock::new(HashMap::new()),
            catch_all_results: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_email(&self, email: &str) -> Option<SmtpStatus> {
        let now = Instant::now();
        self.email_results
            .read()
            .await
            .get(email)
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.value.clone())
    }

    pub async fn set_email(&self, email: String, status: SmtpStatus) {
        self.email_results.write().await.insert(
            email,
            CacheEntry {
                value: status,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub async fn get_catch_all(&self, domain: &str) -> Option<bool> {
        let now = Instant::now();
        self.catch_all_results
            .read()
            .await
            .get(domain)
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.value)
    }

    pub async fn set_catch_all(&self, domain: String, catch_all: bool) {
        self.catch_all_results.write().await.insert(
            domain,
            CacheEntry {
                value: catch_all,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_restores_email_and_catch_all_entries() {
        let cache = SmtpCache::new(Duration::from_secs(5));
        cache
            .set_email("person@example.com".to_string(), SmtpStatus::Deliverable)
            .await;
        cache.set_catch_all("example.com".to_string(), true).await;

        assert_eq!(
            cache.get_email("person@example.com").await,
            Some(SmtpStatus::Deliverable)
        );
        assert_eq!(cache.get_catch_all("example.com").await, Some(true));
    }
}
