use std::time::Duration;

use uuid::Uuid;

use crate::smtp::{SmtpStatus, smtp_rcpt_check};

pub async fn detect_catch_all(
    mx_host: &str,
    domain: &str,
    from_domain: &str,
    timeout: Duration,
) -> anyhow::Result<bool> {
    let first_probe = format!("zz-noexist-{}@{}", &Uuid::new_v4().to_string()[..8], domain);
    let second_probe = format!("zz-noexist-{}@{}", &Uuid::new_v4().to_string()[..8], domain);
    let mail_from = format!("verify@{}", from_domain);

    let first = smtp_rcpt_check(mx_host, &first_probe, &mail_from, timeout).await;
    let second = smtp_rcpt_check(mx_host, &second_probe, &mail_from, timeout).await;

    Ok(
        matches!(first.outcome, SmtpStatus::Accepted | SmtpStatus::AcceptedForwarded)
            && matches!(second.outcome, SmtpStatus::Accepted | SmtpStatus::AcceptedForwarded),
    )
}
