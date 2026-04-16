use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, RwLock};

struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_per_second: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(per_minute: u32) -> Self {
        let capacity = per_minute as f64;
        Self {
            capacity,
            tokens: capacity,
            refill_per_second: capacity / 60.0,
            last_refill: Instant::now(),
        }
    }

    fn try_take(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        self.tokens = (self.tokens + elapsed * self.refill_per_second).min(self.capacity);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub struct RateLimiter {
    buckets: RwLock<HashMap<String, Arc<Mutex<TokenBucket>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
        }
    }

    pub async fn acquire(&self, mx_host: &str) {
        let key = mx_host.trim_end_matches('.').to_lowercase();
        let bucket = if let Some(existing) = self.buckets.read().await.get(&key) {
            Arc::clone(existing)
        } else {
            let mut write = self.buckets.write().await;
            Arc::clone(write.entry(key.clone()).or_insert_with(|| {
                Arc::new(Mutex::new(TokenBucket::new(per_minute_for_host(&key))))
            }))
        };

        loop {
            let mut guard = bucket.lock().await;
            if guard.try_take() {
                break;
            }
            drop(guard);
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

fn per_minute_for_host(mx_host: &str) -> u32 {
    let host = mx_host.to_lowercase();
    if host.contains("google.com") || host.contains("gmail") {
        20
    } else if host.contains("outlook")
        || host.contains("protection.outlook.com")
        || host.contains("hotmail")
    {
        15
    } else if host.contains("yahoo") || host.contains("yahoodns") {
        10
    } else {
        30
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_limits_match_expected_profiles() {
        assert_eq!(per_minute_for_host("gmail-smtp-in.l.google.com"), 20);
        assert_eq!(
            per_minute_for_host("example.mail.protection.outlook.com"),
            15
        );
        assert_eq!(per_minute_for_host("mta5.am0.yahoodns.net"), 10);
        assert_eq!(per_minute_for_host("mx1.custom-host.net"), 30);
    }
}
