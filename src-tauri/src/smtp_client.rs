use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::smtp_status::SmtpStatus;

#[derive(Clone)]
pub struct SmtpApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

#[derive(Serialize)]
struct SmtpVerifyRequest {
    domains: Vec<String>,
    emails: HashMap<String, String>,
}

#[derive(Deserialize)]
struct SmtpVerifyResponse {
    results: HashMap<String, SmtpVerifyResponseItem>,
}

#[derive(Deserialize)]
struct SmtpVerifyResponseItem {
    status: SmtpStatus,
}

impl SmtpApiClient {
    pub fn new(base_url: String, api_key: String) -> Option<Self> {
        Self::new_with_timeout(base_url, api_key, Duration::from_secs(30))
    }

    fn new_with_timeout(base_url: String, api_key: String, timeout: Duration) -> Option<Self> {
        let trimmed_url = base_url.trim().trim_end_matches('/').to_string();
        let trimmed_key = api_key.trim().to_string();
        if trimmed_url.is_empty() || trimmed_key.is_empty() {
            return None;
        }

        let client = Client::builder().timeout(timeout).build().ok()?;
        Some(Self {
            client,
            base_url: trimmed_url,
            api_key: trimmed_key,
        })
    }

    pub async fn verify_batch(
        &self,
        mx_domains: &[(String, String)],
    ) -> HashMap<String, SmtpStatus> {
        if mx_domains.is_empty() {
            return HashMap::new();
        }

        let mut emails = HashMap::with_capacity(mx_domains.len());
        for (domain, email) in mx_domains {
            emails
                .entry(domain.clone())
                .or_insert_with(|| email.clone());
        }
        let domains: Vec<String> = emails.keys().cloned().collect();
        let request = SmtpVerifyRequest {
            domains: domains.clone(),
            emails,
        };
        let fallback = domains
            .iter()
            .cloned()
            .map(|domain| (domain, SmtpStatus::Inconclusive))
            .collect::<HashMap<_, _>>();

        let response = match self
            .client
            .post(format!("{}/verify/smtp", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            _ => return fallback,
        };

        let parsed = match response.json::<SmtpVerifyResponse>().await {
            Ok(parsed) => parsed,
            Err(_) => return fallback,
        };

        domains
            .into_iter()
            .map(|domain| {
                let status = parsed
                    .results
                    .get(&domain)
                    .map(|item| item.status.clone())
                    .unwrap_or(SmtpStatus::Inconclusive);
                (domain, status)
            })
            .collect()
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
            let body = r#"{"results":{"gmail.com":{"status":"Deliverable"}}}"#;
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
            .verify_batch(&[("gmail.com".to_string(), "person@gmail.com".to_string())])
            .await;

        assert_eq!(results.get("gmail.com"), Some(&SmtpStatus::Deliverable));
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
            .verify_batch(&[("gmail.com".to_string(), "person@gmail.com".to_string())])
            .await;

        assert_eq!(results.get("gmail.com"), Some(&SmtpStatus::Inconclusive));
    }
}
