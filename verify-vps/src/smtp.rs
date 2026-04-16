use std::{sync::Arc, time::Duration};

use anyhow::Result;
use hickory_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

use crate::{cache::SmtpCache, catch_all::detect_catch_all, rate_limiter::RateLimiter};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum SmtpStatus {
    Deliverable,
    Rejected,
    CatchAll,
    Inconclusive,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SmtpProbeResult {
    pub status: SmtpStatus,
    pub mx_host: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpRcptResult {
    Accepted,
    Rejected,
    TempFail,
    Inconclusive,
}

#[derive(Clone)]
pub struct SmtpVerifier {
    resolver: TokioAsyncResolver,
    cache: Arc<SmtpCache>,
    limiter: Arc<RateLimiter>,
    from_domain: String,
    timeout: Duration,
}

impl SmtpVerifier {
    pub fn new(from_domain: String, timeout: Duration) -> Self {
        let mut opts = ResolverOpts::default();
        opts.timeout = timeout;
        opts.attempts = 2;
        opts.validate = false;
        Self {
            resolver: TokioAsyncResolver::tokio(ResolverConfig::default(), opts),
            cache: Arc::new(SmtpCache::new(Duration::from_secs(2 * 60 * 60))),
            limiter: Arc::new(RateLimiter::new()),
            from_domain,
            timeout,
        }
    }

    pub async fn verify_email(&self, domain: &str, email: &str) -> SmtpProbeResult {
        let mx_host = match self.resolve_mx_host(domain).await {
            Ok(mx_host) => mx_host,
            Err(_) => {
                return SmtpProbeResult {
                    status: SmtpStatus::Inconclusive,
                    mx_host: None,
                };
            }
        };

        self.limiter.acquire(&mx_host).await;

        if let Some(catch_all) = self.cache.get_catch_all(domain).await {
            if catch_all {
                return SmtpProbeResult {
                    status: SmtpStatus::CatchAll,
                    mx_host: Some(mx_host),
                };
            }
        } else if detect_catch_all(&mx_host, domain, &self.from_domain, self.timeout)
            .await
            .unwrap_or(false)
        {
            self.cache.set_catch_all(domain.to_string(), true).await;
            return SmtpProbeResult {
                status: SmtpStatus::CatchAll,
                mx_host: Some(mx_host),
            };
        } else {
            self.cache.set_catch_all(domain.to_string(), false).await;
        }

        if let Some(cached) = self.cache.get_email(email).await {
            return SmtpProbeResult {
                status: cached,
                mx_host: Some(mx_host),
            };
        }

        let mail_from = format!("verify@{}", self.from_domain);
        let status = match smtp_rcpt_check(&mx_host, email, &mail_from, self.timeout).await {
            SmtpRcptResult::Accepted => SmtpStatus::Deliverable,
            SmtpRcptResult::Rejected => SmtpStatus::Rejected,
            SmtpRcptResult::TempFail | SmtpRcptResult::Inconclusive => SmtpStatus::Inconclusive,
        };

        self.cache
            .set_email(email.to_string(), status.clone())
            .await;

        SmtpProbeResult {
            status,
            mx_host: Some(mx_host),
        }
    }

    async fn resolve_mx_host(&self, domain: &str) -> Result<String> {
        let lookup = self.resolver.mx_lookup(domain).await?;
        let record = lookup
            .iter()
            .min_by_key(|record| record.preference())
            .ok_or_else(|| anyhow::anyhow!("No MX host available"))?;
        Ok(record
            .exchange()
            .to_string()
            .trim_end_matches('.')
            .to_string())
    }
}

pub async fn smtp_rcpt_check(
    mx_host: &str,
    recipient: &str,
    mail_from: &str,
    timeout_duration: Duration,
) -> SmtpRcptResult {
    match timeout(
        timeout_duration,
        smtp_rcpt_check_inner(mx_host, recipient, mail_from),
    )
    .await
    {
        Ok(Ok(result)) => result,
        _ => SmtpRcptResult::Inconclusive,
    }
}

async fn smtp_rcpt_check_inner(
    mx_host: &str,
    recipient: &str,
    mail_from: &str,
) -> Result<SmtpRcptResult> {
    let stream = TcpStream::connect((mx_host, 25)).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let greeting = read_smtp_response(&mut reader).await?;
    if greeting != 220 {
        return Ok(SmtpRcptResult::Inconclusive);
    }

    send_command(&mut writer, "EHLO verify.local\r\n").await?;
    let ehlo = read_smtp_response(&mut reader).await?;
    if ehlo != 250 {
        let _ = send_command(&mut writer, "QUIT\r\n").await;
        return Ok(SmtpRcptResult::Inconclusive);
    }

    send_command(&mut writer, &format!("MAIL FROM:<{}>\r\n", mail_from)).await?;
    let mail_from_code = read_smtp_response(&mut reader).await?;
    if mail_from_code / 100 != 2 {
        let _ = send_command(&mut writer, "QUIT\r\n").await;
        return Ok(SmtpRcptResult::Inconclusive);
    }

    send_command(&mut writer, &format!("RCPT TO:<{}>\r\n", recipient)).await?;
    let rcpt_code = read_smtp_response(&mut reader).await?;
    let _ = send_command(&mut writer, "QUIT\r\n").await;

    Ok(map_rcpt_code(rcpt_code))
}

async fn send_command(writer: &mut tokio::net::tcp::OwnedWriteHalf, command: &str) -> Result<()> {
    writer.write_all(command.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_smtp_response(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Result<u16> {
    let mut line = String::new();
    let mut last_code = 0u16;

    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        if line.len() < 3 {
            continue;
        }
        let code = line[0..3].parse::<u16>()?;
        last_code = code;
        let continuation = line.as_bytes().get(3).copied() == Some(b'-');
        if !continuation {
            break;
        }
    }

    if last_code == 0 {
        anyhow::bail!("SMTP server closed without response");
    }

    Ok(last_code)
}

fn map_rcpt_code(code: u16) -> SmtpRcptResult {
    match code {
        250 | 251 => SmtpRcptResult::Accepted,
        550 | 551 | 553 | 554 => SmtpRcptResult::Rejected,
        421 | 450 | 451 | 452 => SmtpRcptResult::TempFail,
        _ => SmtpRcptResult::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcpt_code_mapping_matches_expected_buckets() {
        assert_eq!(map_rcpt_code(250), SmtpRcptResult::Accepted);
        assert_eq!(map_rcpt_code(251), SmtpRcptResult::Accepted);
        assert_eq!(map_rcpt_code(550), SmtpRcptResult::Rejected);
        assert_eq!(map_rcpt_code(451), SmtpRcptResult::TempFail);
        assert_eq!(map_rcpt_code(999), SmtpRcptResult::Inconclusive);
    }
}
