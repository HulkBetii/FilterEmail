use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, RwLock};

use crate::smtp::SmtpStatus;

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

#[derive(Default)]
struct HostHealth {
    consecutive_bad: usize,
    recent_bad: VecDeque<bool>,
    cooldown_until: Option<Instant>,
    cooldown_seconds: u64,
}

impl HostHealth {
    fn in_cooldown(&self, now: Instant) -> bool {
        self.cooldown_until.is_some_and(|until| until > now)
    }

    fn record(&mut self, is_bad: bool) {
        if is_bad {
            self.consecutive_bad += 1;
        } else {
            self.consecutive_bad = 0;
        }

        self.recent_bad.push_back(is_bad);
        while self.recent_bad.len() > 20 {
            self.recent_bad.pop_front();
        }
    }

    fn should_open_cooldown(&self) -> bool {
        if self.consecutive_bad >= 5 {
            return true;
        }

        if self.recent_bad.len() < 20 {
            return false;
        }

        let bad_count = self.recent_bad.iter().filter(|value| **value).count();
        (bad_count as f64 / self.recent_bad.len() as f64) > 0.5
    }

    fn open_cooldown(&mut self, now: Instant) {
        let current = self.cooldown_seconds.max(60);
        self.cooldown_until = Some(now + Duration::from_secs(current));
        self.cooldown_seconds = (current * 2).min(300);
        self.consecutive_bad = 0;
        self.recent_bad.clear();
    }

    fn mark_healthy(&mut self) {
        self.cooldown_seconds = 60;
    }
}

pub struct RateLimiter {
    buckets: RwLock<HashMap<String, Arc<Mutex<TokenBucket>>>>,
    health: RwLock<HashMap<String, Arc<Mutex<HostHealth>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            health: RwLock::new(HashMap::new()),
        }
    }

    pub async fn acquire(&self, mx_host: &str) -> bool {
        let key = mx_host.trim_end_matches('.').to_lowercase();
        let health = self.health_for(&key).await;
        {
            let guard = health.lock().await;
            if guard.in_cooldown(Instant::now()) {
                return false;
            }
        }

        let bucket = self.bucket_for(&key).await;
        loop {
            let mut guard = bucket.lock().await;
            if guard.try_take() {
                return true;
            }
            drop(guard);
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn record_outcome(&self, mx_host: &str, status: &SmtpStatus) {
        let key = mx_host.trim_end_matches('.').to_lowercase();
        let health = self.health_for(&key).await;
        let mut guard = health.lock().await;
        let is_bad = matches!(status, SmtpStatus::TempFailure | SmtpStatus::PolicyBlocked);
        guard.record(is_bad);
        if guard.should_open_cooldown() {
            guard.open_cooldown(Instant::now());
        } else if !is_bad {
            guard.mark_healthy();
        }
    }

    async fn bucket_for(&self, key: &str) -> Arc<Mutex<TokenBucket>> {
        if let Some(existing) = self.buckets.read().await.get(key) {
            Arc::clone(existing)
        } else {
            let mut write = self.buckets.write().await;
            Arc::clone(write.entry(key.to_string()).or_insert_with(|| {
                Arc::new(Mutex::new(TokenBucket::new(per_minute_for_host(key))))
            }))
        }
    }

    async fn health_for(&self, key: &str) -> Arc<Mutex<HostHealth>> {
        if let Some(existing) = self.health.read().await.get(key) {
            Arc::clone(existing)
        } else {
            let mut write = self.health.write().await;
            Arc::clone(write.entry(key.to_string()).or_insert_with(|| {
                Arc::new(Mutex::new(HostHealth {
                    cooldown_seconds: 60,
                    ..HostHealth::default()
                }))
            }))
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

    #[tokio::test]
    async fn cooldown_opens_on_repeated_transient_or_policy_responses() {
        let limiter = RateLimiter::new();
        let host = "mx.example.com";
        for _ in 0..5 {
            limiter.record_outcome(host, &SmtpStatus::TempFailure).await;
        }

        assert!(!limiter.acquire(host).await);
    }
}
