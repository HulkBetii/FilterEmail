use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MxStatus {
    HasMx,
    ARecordFallback,
    Dead,
    NullMx,
    Parked,
    Disposable,
    TypoSuggestion(String),
    Inconclusive,
}

#[derive(Clone, Serialize, Debug)]
pub struct ProcessingPayload {
    pub processed_lines: u64,
    pub progress_percent: f64,
    pub invalid: u64,
    pub public: u64,
    pub edu: u64,
    pub targeted: u64,
    pub custom: u64,
    pub duplicates: u64,
    pub mx_dead: u64,
    pub mx_has_mx: u64,
    pub mx_a_fallback: u64,
    pub mx_inconclusive: u64,
    pub mx_parked: u64,
    pub mx_disposable: u64,
    pub mx_typo: u64,
    pub smtp_deliverable: u64,
    pub smtp_rejected: u64,
    pub smtp_catchall: u64,
    pub smtp_unknown: u64,
    pub smtp_enabled: bool,
    pub smtp_elapsed_ms: u64,
    pub cache_hits: u64,
    pub final_alive: u64,
    pub final_dead: u64,
    pub final_unknown: u64,
    pub smtp_attempted_emails: u64,
    pub smtp_cache_hits: u64,
    pub smtp_coverage_percent: f64,
    pub smtp_policy_blocked: u64,
    pub smtp_temp_failure: u64,
    pub smtp_mailbox_full: u64,
    pub smtp_mailbox_disabled: u64,
    pub smtp_bad_mailbox: u64,
    pub smtp_bad_domain: u64,
    pub smtp_network_error: u64,
    pub smtp_protocol_error: u64,
    pub smtp_timeout: u64,
    pub elapsed_ms: u128,
    pub output_dir: Option<String>,
    pub current_domain: Option<String>,
    pub current_email: Option<String>,
}

#[derive(Clone, Serialize, Debug)]
pub struct ErrorPayload {
    pub message_en: String,
    pub message_vi: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    pub(crate) invalid: u64,
    pub(crate) public: u64,
    pub(crate) edu: u64,
    pub(crate) targeted: u64,
    pub(crate) custom: u64,
    pub(crate) duplicates: u64,
    pub(crate) mx_dead: u64,
    pub(crate) mx_has_mx: u64,
    pub(crate) mx_a_fallback: u64,
    pub(crate) mx_inconclusive: u64,
    pub(crate) mx_parked: u64,
    pub(crate) mx_disposable: u64,
    pub(crate) mx_typo: u64,
    pub(crate) smtp_deliverable: u64,
    pub(crate) smtp_rejected: u64,
    pub(crate) smtp_catchall: u64,
    pub(crate) smtp_unknown: u64,
    pub(crate) cache_hits: u64,
    pub(crate) final_alive: u64,
    pub(crate) final_dead: u64,
    pub(crate) final_unknown: u64,
    pub(crate) smtp_attempted_emails: u64,
    pub(crate) smtp_cache_hits: u64,
    pub(crate) smtp_policy_blocked: u64,
    pub(crate) smtp_temp_failure: u64,
    pub(crate) smtp_mailbox_full: u64,
    pub(crate) smtp_mailbox_disabled: u64,
    pub(crate) smtp_bad_mailbox: u64,
    pub(crate) smtp_bad_domain: u64,
    pub(crate) smtp_network_error: u64,
    pub(crate) smtp_protocol_error: u64,
    pub(crate) smtp_timeout: u64,
}

pub(crate) struct CollectedDomains {
    pub(crate) unique_domains: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedEmail {
    pub(crate) normalized_email: String,
    pub(crate) canonical_email_key: String,
    pub(crate) raw_local_part: String,
    pub(crate) normalized_domain: String,
}

pub(crate) struct SecondPassResult {
    pub(crate) processed_lines: u64,
    pub(crate) smtp_spool_path: Option<PathBuf>,
}

#[derive(Copy, Clone)]
pub(crate) enum EmailGroup {
    Public,
    Edu,
    Targeted,
    Custom,
    MxDead,
    MxHasMx,
    MxARecordFallback,
    MxInconclusive,
    MxParked,
    MxDisposable,
    MxTypo,
}

#[derive(Copy, Clone)]
pub(crate) enum ProcessingMode {
    BasicFilter,
    VerifyDns,
}
