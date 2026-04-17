use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::smtp_status::{SmtpProbeRecord, SmtpStatus};

#[derive(Clone)]
pub struct SmtpApiClient {
    client: Client,
    base_url: String,
    api_key: String,
    base_timeout: Duration,
    per_target_timeout: Duration,
    max_timeout: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmtpVerifyTarget {
    pub email: String,
    pub normalized_domain: String,
}

#[derive(Serialize)]
struct SmtpVerifyV2Request {
    targets: Vec<SmtpVerifyTarget>,
}

#[derive(Deserialize)]
struct SmtpVerifyV2Response {
    results: Vec<SmtpVerifyResponseItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct SmtpVerifyResponseItem {
    email: String,
    outcome: SmtpStatus,
    smtp_basic_code: Option<u16>,
    smtp_enhanced_code: Option<String>,
    smtp_reply_text: Option<String>,
    mx_host: Option<String>,
    catch_all: bool,
    cached: bool,
    duration_ms: u64,
}

impl SmtpApiClient {
    pub fn new(base_url: String, api_key: String) -> Option<Self> {
        Self::new_with_timeouts(
            base_url,
            api_key,
            Duration::from_secs(20),
            Duration::from_secs(20),
            Duration::from_secs(180),
        )
    }

    #[cfg(test)]
    fn new_with_timeout(base_url: String, api_key: String, timeout: Duration) -> Option<Self> {
        Self::new_with_timeouts(base_url, api_key, timeout, Duration::ZERO, timeout)
    }

    fn new_with_timeouts(
        base_url: String,
        api_key: String,
        base_timeout: Duration,
        per_target_timeout: Duration,
        max_timeout: Duration,
    ) -> Option<Self> {
        let trimmed_url = base_url.trim().trim_end_matches('/').to_string();
        let trimmed_key = api_key.trim().to_string();
        if trimmed_url.is_empty() || trimmed_key.is_empty() {
            return None;
        }

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .ok()?;
        Some(Self {
            client,
            base_url: trimmed_url,
            api_key: trimmed_key,
            base_timeout,
            per_target_timeout,
            max_timeout,
        })
    }

    pub async fn verify_batch(
        &self,
        targets: &[SmtpVerifyTarget],
    ) -> HashMap<String, SmtpProbeRecord> {
        if targets.is_empty() {
            return HashMap::new();
        }

        let request = SmtpVerifyV2Request {
            targets: targets.to_vec(),
        };
        let fallback = targets
            .iter()
            .map(|target| {
                (
                    target.email.clone(),
                    SmtpProbeRecord {
                        email: target.email.clone(),
                        outcome: SmtpStatus::Inconclusive,
                        ..Default::default()
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let response = match self
            .client
            .post(format!("{}/verify/smtp/v2", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(self.request_timeout_for_batch(targets.len()))
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            _ => return fallback,
        };

        let parsed = match response.json::<SmtpVerifyV2Response>().await {
            Ok(parsed) => parsed,
            Err(_) => return fallback,
        };

        let items = parsed
            .results
            .into_iter()
            .map(|item| {
                (
                    item.email.clone(),
                    SmtpProbeRecord {
                        email: item.email,
                        outcome: item.outcome,
                        smtp_basic_code: item.smtp_basic_code,
                        smtp_enhanced_code: item.smtp_enhanced_code,
                        smtp_reply_text: item.smtp_reply_text,
                        mx_host: item.mx_host,
                        catch_all: item.catch_all,
                        cached: item.cached,
                        duration_ms: item.duration_ms,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        targets
            .iter()
            .map(|target| {
                let record = items.get(&target.email).cloned().unwrap_or(SmtpProbeRecord {
                    email: target.email.clone(),
                    outcome: SmtpStatus::Inconclusive,
                    ..Default::default()
                });
                (target.email.clone(), record)
            })
            .collect()
    }

    fn request_timeout_for_batch(&self, target_count: usize) -> Duration {
        let target_count = u32::try_from(target_count).unwrap_or(u32::MAX);
        let extra = self
            .per_target_timeout
            .checked_mul(target_count)
            .unwrap_or(self.max_timeout);
        self.base_timeout
            .saturating_add(extra)
            .min(self.max_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn verify_batch_returns_mocked_statuses() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer).await.unwrap();
            let body = r#"{"results":[{"email":"person@gmail.com","outcome":"Accepted","smtp_basic_code":250,"smtp_enhanced_code":"2.1.5","smtp_reply_text":"OK","mx_host":"gmail-smtp-in.l.google.com","catch_all":false,"cached":false,"duration_ms":321}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let client = SmtpApiClient::new_with_timeout(
            format!("http://{}", addr),
            "test-key".to_string(),
            Duration::from_secs(2),
        )
        .unwrap();

        let results = client
            .verify_batch(&[SmtpVerifyTarget {
                email: "person@gmail.com".to_string(),
                normalized_domain: "gmail.com".to_string(),
            }])
            .await;

        let record = results.get("person@gmail.com").unwrap();
        assert_eq!(record.outcome, SmtpStatus::Accepted);
        assert_eq!(record.smtp_basic_code, Some(250));
        assert_eq!(record.smtp_enhanced_code.as_deref(), Some("2.1.5"));
    }

    #[tokio::test]
    async fn verify_batch_falls_back_to_inconclusive_when_vps_is_down() {
        let client = SmtpApiClient::new_with_timeout(
            "http://127.0.0.1:9".to_string(),
            "test-key".to_string(),
            Duration::from_millis(200),
        )
        .unwrap();

        let results = client
            .verify_batch(&[SmtpVerifyTarget {
                email: "person@gmail.com".to_string(),
                normalized_domain: "gmail.com".to_string(),
            }])
            .await;

        assert_eq!(
            results.get("person@gmail.com").map(|record| &record.outcome),
            Some(&SmtpStatus::Inconclusive)
        );
    }

    #[test]
    fn request_timeout_scales_with_batch_size() {
        let client = SmtpApiClient::new("https://smtp.example.com".to_string(), "key".to_string())
            .expect("client");

        assert_eq!(client.request_timeout_for_batch(1), Duration::from_secs(40));
        assert_eq!(client.request_timeout_for_batch(5), Duration::from_secs(120));
        assert_eq!(client.request_timeout_for_batch(10), Duration::from_secs(180));
        assert_eq!(client.request_timeout_for_batch(100), Duration::from_secs(180));
    }
}
