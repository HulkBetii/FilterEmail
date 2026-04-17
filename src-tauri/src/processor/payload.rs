use super::types::{ProcessingPayload, Stats};
use super::DOMAIN_SCAN_PROGRESS_END;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_processing_payload(
    output_dir: &str,
    processed_lines: u64,
    progress_percent: f64,
    stats: &Stats,
    smtp_enabled: bool,
    smtp_elapsed_ms: u64,
    elapsed_ms: u128,
    current_domain: Option<String>,
    current_email: Option<String>,
) -> ProcessingPayload {
    let smtp_coverage_percent = if stats.mx_has_mx == 0 {
        0.0
    } else {
        ((stats.smtp_attempted_emails as f64 / stats.mx_has_mx as f64) * 100.0).clamp(0.0, 100.0)
    };

    ProcessingPayload {
        processed_lines,
        progress_percent: progress_percent.clamp(0.0, 100.0),
        invalid: stats.invalid,
        public: stats.public,
        edu: stats.edu,
        targeted: stats.targeted,
        custom: stats.custom,
        duplicates: stats.duplicates,
        mx_dead: stats.mx_dead,
        mx_has_mx: stats.mx_has_mx,
        mx_a_fallback: stats.mx_a_fallback,
        mx_inconclusive: stats.mx_inconclusive,
        mx_parked: stats.mx_parked,
        mx_disposable: stats.mx_disposable,
        mx_typo: stats.mx_typo,
        smtp_deliverable: stats.smtp_deliverable,
        smtp_rejected: stats.smtp_rejected,
        smtp_catchall: stats.smtp_catchall,
        smtp_unknown: stats.smtp_unknown,
        smtp_enabled,
        smtp_elapsed_ms,
        cache_hits: stats.cache_hits,
        final_alive: stats.final_alive,
        final_dead: stats.final_dead,
        final_unknown: stats.final_unknown,
        smtp_attempted_emails: stats.smtp_attempted_emails,
        smtp_cache_hits: stats.smtp_cache_hits,
        smtp_coverage_percent,
        smtp_policy_blocked: stats.smtp_policy_blocked,
        smtp_temp_failure: stats.smtp_temp_failure,
        smtp_mailbox_full: stats.smtp_mailbox_full,
        smtp_mailbox_disabled: stats.smtp_mailbox_disabled,
        smtp_bad_mailbox: stats.smtp_bad_mailbox,
        smtp_bad_domain: stats.smtp_bad_domain,
        smtp_network_error: stats.smtp_network_error,
        smtp_protocol_error: stats.smtp_protocol_error,
        smtp_timeout: stats.smtp_timeout,
        elapsed_ms,
        output_dir: Some(output_dir.to_string()),
        current_domain,
        current_email,
    }
}

pub(crate) fn scale_progress(total_bytes: u64, bytes_read: u64, max_progress: f64) -> f64 {
    if total_bytes == 0 {
        max_progress
    } else {
        ((bytes_read as f64 / total_bytes as f64) * max_progress).clamp(0.0, max_progress)
    }
}

pub(crate) fn scale_second_pass_progress(
    total_bytes: u64,
    bytes_read: u64,
    check_mx: bool,
    smtp_enabled: bool,
) -> f64 {
    if total_bytes == 0 {
        if check_mx && smtp_enabled {
            88.0
        } else {
            100.0
        }
    } else if check_mx {
        let target_end = if smtp_enabled { 88.0 } else { 100.0 };
        let remaining = target_end - DOMAIN_SCAN_PROGRESS_END;
        (DOMAIN_SCAN_PROGRESS_END + ((bytes_read as f64 / total_bytes as f64) * remaining))
            .clamp(DOMAIN_SCAN_PROGRESS_END, target_end)
    } else {
        ((bytes_read as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
    }
}

pub(crate) fn smtp_phase_progress(processed_targets: usize, total_targets: usize) -> f64 {
    if total_targets == 0 {
        100.0
    } else {
        (88.0 + ((processed_targets as f64 / total_targets as f64) * 12.0)).clamp(88.0, 100.0)
    }
}
