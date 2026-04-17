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

use crate::{
    cache::SmtpCache,
    catch_all::detect_catch_all,
    rate_limiter::RateLimiter,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub enum SmtpStatus {
    Accepted,
    AcceptedForwarded,
    CatchAll,
    BadMailbox,
    BadDomain,
    PolicyBlocked,
    MailboxFull,
    MailboxDisabled,
    TempFailure,
    NetworkError,
    ProtocolError,
    Timeout,
    #[default]
    Inconclusive,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct SmtpProbeResult {
    pub email: String,
    pub outcome: SmtpStatus,
    pub smtp_basic_code: Option<u16>,
    pub smtp_enhanced_code: Option<String>,
    pub smtp_reply_text: Option<String>,
    pub mx_host: Option<String>,
    pub catch_all: bool,
    pub cached: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
struct SmtpReply {
    code: u16,
    reply_text: String,
    enhanced_code: Option<String>,
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
        let started = std::time::Instant::now();
        let mx_hosts = match self.resolve_mx_hosts(domain).await {
            Ok(mx_hosts) if !mx_hosts.is_empty() => mx_hosts,
            Err(error) => {
                return SmtpProbeResult {
                    email: email.to_string(),
                    outcome: SmtpStatus::Inconclusive,
                    smtp_reply_text: Some(error.to_string()),
                    duration_ms: started.elapsed().as_millis() as u64,
                    ..Default::default()
                };
            }
            _ => {
                return SmtpProbeResult {
                    email: email.to_string(),
                    outcome: SmtpStatus::Inconclusive,
                    smtp_reply_text: Some("No MX host available".to_string()),
                    duration_ms: started.elapsed().as_millis() as u64,
                    ..Default::default()
                };
            }
        };

        if let Some(catch_all) = self.cache.get_catch_all(domain).await
            && catch_all.catch_all
        {
            return SmtpProbeResult {
                email: email.to_string(),
                outcome: SmtpStatus::CatchAll,
                mx_host: catch_all.mx_host.or_else(|| mx_hosts.first().cloned()),
                catch_all: true,
                cached: true,
                duration_ms: started.elapsed().as_millis() as u64,
                ..Default::default()
            };
        };

        if let Some(mut cached) = self.cache.get_email(email).await {
            cached.cached = true;
            cached.duration_ms = started.elapsed().as_millis() as u64;
            if cached.mx_host.is_none() {
                cached.mx_host = mx_hosts.first().cloned();
            }
            return cached;
        }

        let mail_from = format!("verify@{}", self.from_domain);
        let mut last_result = None;

        for mx_host in mx_hosts {
            if !self.limiter.acquire(&mx_host).await {
                last_result = Some(SmtpProbeResult {
                    email: email.to_string(),
                    outcome: SmtpStatus::Inconclusive,
                    smtp_reply_text: Some("host cooldown active".to_string()),
                    mx_host: Some(mx_host.clone()),
                    duration_ms: started.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                continue;
            }

            let catch_all = detect_catch_all(&mx_host, domain, &self.from_domain, self.timeout)
                .await
                .unwrap_or(false);
            self.cache
                .set_catch_all(domain.to_string(), catch_all, Some(mx_host.clone()))
                .await;
            if catch_all {
                let result = SmtpProbeResult {
                    email: email.to_string(),
                    outcome: SmtpStatus::CatchAll,
                    mx_host: Some(mx_host.clone()),
                    catch_all: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    ..Default::default()
                };
                self.cache.set_email(email.to_string(), result.clone()).await;
                return result;
            }

            let mut attempt = 0u8;
            let host_result = loop {
                let mut probe = smtp_rcpt_check(&mx_host, email, &mail_from, self.timeout).await;
                probe.email = email.to_string();
                probe.mx_host = Some(mx_host.clone());
                probe.duration_ms = started.elapsed().as_millis() as u64;

                if !matches!(
                    probe.outcome,
                    SmtpStatus::TempFailure | SmtpStatus::Timeout | SmtpStatus::NetworkError
                ) || attempt >= 2
                {
                    break probe;
                }

                attempt += 1;
                let base = 150u64 * (1u64 << (attempt as u64 - 1));
                let jitter =
                    (email.len() as u64 * 17 + mx_host.len() as u64 * 11 + attempt as u64 * 7)
                        % 60;
                tokio::time::sleep(Duration::from_millis(base + jitter)).await;
            };

            self.limiter
                .record_outcome(&mx_host, &host_result.outcome)
                .await;

            if should_stop_on_host_result(&host_result.outcome) {
                self.cache
                    .set_email(email.to_string(), host_result.clone())
                    .await;
                return host_result;
            }

            last_result = Some(host_result);
        }

        let final_result = last_result.unwrap_or_else(|| SmtpProbeResult {
            email: email.to_string(),
            outcome: SmtpStatus::Inconclusive,
            smtp_reply_text: Some("No MX host produced a conclusive result".to_string()),
            duration_ms: started.elapsed().as_millis() as u64,
            ..Default::default()
        });
        self.cache
            .set_email(email.to_string(), final_result.clone())
            .await;
        final_result
    }

    async fn resolve_mx_hosts(&self, domain: &str) -> Result<Vec<String>> {
        let lookup = self.resolver.mx_lookup(domain).await?;
        let mut records = lookup
            .iter()
            .map(|record| {
                (
                    record.preference(),
                    record.exchange().to_string().trim_end_matches('.').to_string(),
                )
            })
            .filter(|(_, host)| !host.is_empty())
            .collect::<Vec<_>>();
        records.sort_by_key(|(preference, host)| (*preference, host.clone()));
        let hosts = records.into_iter().map(|(_, host)| host).collect::<Vec<_>>();
        if hosts.is_empty() {
            return Err(anyhow::anyhow!("No MX host available"));
        }
        Ok(hosts)
    }
}

fn should_stop_on_host_result(status: &SmtpStatus) -> bool {
    matches!(
        status,
        SmtpStatus::Accepted
            | SmtpStatus::AcceptedForwarded
            | SmtpStatus::CatchAll
            | SmtpStatus::BadMailbox
            | SmtpStatus::BadDomain
            | SmtpStatus::PolicyBlocked
            | SmtpStatus::MailboxFull
            | SmtpStatus::MailboxDisabled
    )
}

pub async fn smtp_rcpt_check(
    mx_host: &str,
    recipient: &str,
    mail_from: &str,
    timeout_duration: Duration,
) -> SmtpProbeResult {
    let started = std::time::Instant::now();
    match timeout(
        timeout_duration,
        smtp_rcpt_check_inner(mx_host, recipient, mail_from),
    )
    .await
    {
        Ok(Ok(result)) => SmtpProbeResult {
            email: recipient.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            ..result
        },
        Ok(Err(error)) if error.kind == ProbeErrorKind::Network => SmtpProbeResult {
            email: recipient.to_string(),
            outcome: SmtpStatus::NetworkError,
            smtp_reply_text: Some(error.message),
            mx_host: Some(mx_host.to_string()),
            duration_ms: started.elapsed().as_millis() as u64,
            ..Default::default()
        },
        Ok(Err(error)) => SmtpProbeResult {
            email: recipient.to_string(),
            outcome: SmtpStatus::ProtocolError,
            smtp_reply_text: Some(error.message),
            mx_host: Some(mx_host.to_string()),
            duration_ms: started.elapsed().as_millis() as u64,
            ..Default::default()
        },
        Err(_) => SmtpProbeResult {
            email: recipient.to_string(),
            outcome: SmtpStatus::Timeout,
            smtp_reply_text: Some("timed out".to_string()),
            mx_host: Some(mx_host.to_string()),
            duration_ms: started.elapsed().as_millis() as u64,
            ..Default::default()
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeErrorKind {
    Network,
    Protocol,
}

#[derive(Debug, Clone)]
struct ProbeError {
    kind: ProbeErrorKind,
    message: String,
}

async fn smtp_rcpt_check_inner(
    mx_host: &str,
    recipient: &str,
    mail_from: &str,
) -> Result<SmtpProbeResult, ProbeError> {
    let stream = TcpStream::connect((mx_host, 25))
        .await
        .map_err(network_error)?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let greeting = read_smtp_response(&mut reader).await?;
    if greeting.code != 220 {
        return Ok(reply_to_probe_result(mx_host, recipient, greeting));
    }

    send_command(&mut writer, "EHLO verify.local\r\n").await?;
    let ehlo = read_smtp_response(&mut reader).await?;
    if ehlo.code != 250 {
        let _ = send_command(&mut writer, "QUIT\r\n").await;
        return Ok(reply_to_probe_result(mx_host, recipient, ehlo));
    }

    send_command(&mut writer, &format!("MAIL FROM:<{}>\r\n", mail_from)).await?;
    let mail_from_reply = read_smtp_response(&mut reader).await?;
    if mail_from_reply.code / 100 != 2 {
        let _ = send_command(&mut writer, "QUIT\r\n").await;
        return Ok(reply_to_probe_result(mx_host, recipient, mail_from_reply));
    }

    send_command(&mut writer, &format!("RCPT TO:<{}>\r\n", recipient)).await?;
    let rcpt_reply = read_smtp_response(&mut reader).await?;
    let _ = send_command(&mut writer, "QUIT\r\n").await;

    Ok(reply_to_probe_result(mx_host, recipient, rcpt_reply))
}

async fn send_command(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    command: &str,
) -> Result<(), ProbeError> {
    writer
        .write_all(command.as_bytes())
        .await
        .map_err(network_error)?;
    writer.flush().await.map_err(network_error)?;
    Ok(())
}

async fn read_smtp_response(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<SmtpReply, ProbeError> {
    let mut line = String::new();
    let mut last_code = None;
    let mut collected = Vec::new();

    loop {
        line.clear();
        if reader.read_line(&mut line).await.map_err(network_error)? == 0 {
            break;
        }
        if line.len() < 3 {
            continue;
        }
        let code = line[0..3]
            .parse::<u16>()
            .map_err(|error| protocol_error(error.to_string()))?;
        let continuation = line.as_bytes().get(3).copied() == Some(b'-');
        let content = line
            .get(4..)
            .unwrap_or_default()
            .trim()
            .to_string();
        collected.push(content);
        last_code = Some(code);
        if !continuation {
            break;
        }
    }

    let Some(code) = last_code else {
        return Err(protocol_error("SMTP server closed without response".to_string()));
    };
    let reply_text = collected.join(" | ");
    let enhanced_code = find_enhanced_status_code(&reply_text);

    Ok(SmtpReply {
        code,
        reply_text,
        enhanced_code,
    })
}

fn reply_to_probe_result(mx_host: &str, recipient: &str, reply: SmtpReply) -> SmtpProbeResult {
    SmtpProbeResult {
        email: recipient.to_string(),
        outcome: map_reply_to_status(&reply),
        smtp_basic_code: Some(reply.code),
        smtp_enhanced_code: reply.enhanced_code,
        smtp_reply_text: Some(reply.reply_text),
        mx_host: Some(mx_host.to_string()),
        ..Default::default()
    }
}

fn map_reply_to_status(reply: &SmtpReply) -> SmtpStatus {
    let enhanced = reply.enhanced_code.as_deref().unwrap_or_default();
    let text = reply.reply_text.to_lowercase();

    if reply.code == 251 || enhanced == "2.1.5" {
        return SmtpStatus::AcceptedForwarded;
    }
    if reply.code == 250 {
        return SmtpStatus::Accepted;
    }
    if reply.code == 521 || reply.code == 556 || text.contains("null mx") {
        return SmtpStatus::BadDomain;
    }
    if enhanced == "5.2.2" {
        return SmtpStatus::MailboxFull;
    }
    if enhanced == "5.2.1" {
        return SmtpStatus::MailboxDisabled;
    }
    if enhanced.starts_with("5.7.") || contains_policy_text(&text) {
        return SmtpStatus::PolicyBlocked;
    }
    if matches!(enhanced, "5.1.1" | "5.1.6" | "5.1.10")
        || contains_mailbox_text(&text)
    {
        return SmtpStatus::BadMailbox;
    }
    if matches!(enhanced, "5.1.2" | "5.1.3") || contains_bad_domain_text(&text) {
        return SmtpStatus::BadDomain;
    }
    if reply.code == 421 || matches!(reply.code, 450..=452) || enhanced.starts_with("4.") {
        return SmtpStatus::TempFailure;
    }
    if reply.code / 100 == 5 {
        return SmtpStatus::Inconclusive;
    }
    SmtpStatus::Inconclusive
}

fn contains_policy_text(text: &str) -> bool {
    [
        "policy",
        "blocked",
        "block listed",
        "blocklisted",
        "blacklist",
        "spam",
        "access denied",
        "not authorized",
        "throttle",
        "rate limit",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn contains_mailbox_text(text: &str) -> bool {
    [
        "user unknown",
        "unknown user",
        "no such user",
        "does not exist",
        "recipient does not exist",
        "address does not exist",
        "invalid recipient",
        "mailbox unavailable",
        "recipient address rejected",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn contains_bad_domain_text(text: &str) -> bool {
    [
        "domain not found",
        "host or domain name not found",
        "bad destination mailbox address",
        "mail to domain not accepted",
        "domain does not exist",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn find_enhanced_status_code(text: &str) -> Option<String> {
    for window in text.as_bytes().windows(5) {
        if window[0].is_ascii_digit()
            && window[1] == b'.'
            && window[2].is_ascii_digit()
            && window[3] == b'.'
            && window[4].is_ascii_digit()
        {
            return Some(format!(
                "{}.{}.{}",
                window[0] as char, window[2] as char, window[4] as char
            ));
        }
    }
    None
}

fn network_error(error: impl ToString) -> ProbeError {
    ProbeError {
        kind: ProbeErrorKind::Network,
        message: error.to_string(),
    }
}

fn protocol_error(message: String) -> ProbeError {
    ProbeError {
        kind: ProbeErrorKind::Protocol,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_mapping_matches_expected_buckets() {
        let accepted = SmtpReply {
            code: 250,
            reply_text: "OK".to_string(),
            enhanced_code: None,
        };
        let forwarded = SmtpReply {
            code: 251,
            reply_text: "User not local; will forward".to_string(),
            enhanced_code: Some("2.1.5".to_string()),
        };
        let bad_mailbox = SmtpReply {
            code: 550,
            reply_text: "5.1.1 user unknown".to_string(),
            enhanced_code: Some("5.1.1".to_string()),
        };
        let policy = SmtpReply {
            code: 550,
            reply_text: "5.7.1 access denied".to_string(),
            enhanced_code: Some("5.7.1".to_string()),
        };
        let greeting_policy = SmtpReply {
            code: 554,
            reply_text: "Service not available | IP address is block listed".to_string(),
            enhanced_code: None,
        };
        let temp = SmtpReply {
            code: 451,
            reply_text: "4.7.0 try again later".to_string(),
            enhanced_code: Some("4.7.0".to_string()),
        };
        let bad_mailbox_text = SmtpReply {
            code: 550,
            reply_text: "Recipient does not exist".to_string(),
            enhanced_code: None,
        };

        assert_eq!(map_reply_to_status(&accepted), SmtpStatus::Accepted);
        assert_eq!(map_reply_to_status(&forwarded), SmtpStatus::AcceptedForwarded);
        assert_eq!(map_reply_to_status(&bad_mailbox), SmtpStatus::BadMailbox);
        assert_eq!(map_reply_to_status(&bad_mailbox_text), SmtpStatus::BadMailbox);
        assert_eq!(map_reply_to_status(&policy), SmtpStatus::PolicyBlocked);
        assert_eq!(
            map_reply_to_status(&greeting_policy),
            SmtpStatus::PolicyBlocked
        );
        assert_eq!(map_reply_to_status(&temp), SmtpStatus::TempFailure);
    }
}
