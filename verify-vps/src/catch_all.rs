use std::time::Duration;

use uuid::Uuid;

use crate::smtp::{SmtpRcptResult, smtp_rcpt_check};

pub async fn detect_catch_all(
    mx_host: &str,
    domain: &str,
    from_domain: &str,
    timeout: Duration,
) -> anyhow::Result<bool> {
    let probe = format!("zz-noexist-{}@{}", &Uuid::new_v4().to_string()[..8], domain);
    let mail_from = format!("verify@{}", from_domain);
    let result = smtp_rcpt_check(mx_host, &probe, &mail_from, timeout).await;
    Ok(matches!(result, SmtpRcptResult::Accepted))
}
