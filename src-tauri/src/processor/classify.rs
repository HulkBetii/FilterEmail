use super::errors::map_regex_error_payload;
use super::types::{EmailGroup, ErrorPayload, MxStatus, ProcessingMode};
use super::{
    DISPOSABLE_DOMAINS, PARKED_DOMAIN_SUFFIXES, PARKING_MX_SUFFIXES, TYPO_MAP,
};
use idna::Config;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

pub(crate) fn build_edu_patterns() -> Result<Vec<Regex>, ErrorPayload> {
    Ok(vec![
        Regex::new(r"\.edu$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.gov$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.k12\.[a-z]{2}\.us$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.edu\.[a-z]{2}$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.org$").map_err(map_regex_error_payload)?,
    ])
}

pub(crate) fn classify_email(
    domain: &str,
    public_domains: &HashSet<&'static str>,
    edu_patterns: &[Regex],
    target_domains: &HashSet<String>,
) -> EmailGroup {
    if target_domains.contains(domain) {
        return EmailGroup::Targeted;
    }
    if public_domains.contains(domain) {
        return EmailGroup::Public;
    }
    if edu_patterns.iter().any(|regex| regex.is_match(domain)) {
        return EmailGroup::Edu;
    }
    EmailGroup::Custom
}

pub(crate) fn group_for_email(
    mx_status: &MxStatus,
    domain: &str,
    public_domains: &HashSet<&'static str>,
    edu_patterns: &[Regex],
    target_domains: &HashSet<String>,
    processing_mode: ProcessingMode,
) -> EmailGroup {
    match mx_status {
        MxStatus::Dead | MxStatus::NullMx => EmailGroup::MxDead,
        MxStatus::HasMx if matches!(processing_mode, ProcessingMode::VerifyDns) => {
            EmailGroup::MxHasMx
        }
        MxStatus::ARecordFallback if matches!(processing_mode, ProcessingMode::VerifyDns) => {
            EmailGroup::MxARecordFallback
        }
        MxStatus::Parked => EmailGroup::MxParked,
        MxStatus::Disposable => EmailGroup::MxDisposable,
        MxStatus::TypoSuggestion(_) => EmailGroup::MxTypo,
        MxStatus::Inconclusive => EmailGroup::MxInconclusive,
        MxStatus::HasMx | MxStatus::ARecordFallback => {
            classify_email(domain, public_domains, edu_patterns, target_domains)
        }
    }
}

pub(crate) fn normalize_domain(raw: &str) -> Result<String, String> {
    let lowered = raw.trim().to_lowercase();
    let domain = lowered.trim_end_matches('.');
    if domain.is_empty() {
        return Err("Domain is empty".to_string());
    }

    let config = Config::default().use_std3_ascii_rules(true);
    let ascii = config
        .to_ascii(domain)
        .map_err(|error| format!("IDN error: {error:?}"))?;
    Ok(ascii)
}

pub(crate) fn is_parked_mx(mx_host: &str) -> bool {
    let host = mx_host.trim_end_matches('.').to_lowercase();
    PARKING_MX_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix))
}

pub(crate) fn is_parked_domain(domain: &str) -> bool {
    let host = domain.trim_end_matches('.').to_lowercase();
    PARKED_DOMAIN_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

pub(crate) fn check_typo(domain: &str) -> Option<String> {
    TYPO_MAP.iter().find_map(|(correct, typos)| {
        if typos.contains(&domain) {
            Some((*correct).to_string())
        } else {
            None
        }
    })
}

fn disposable_domains() -> &'static HashSet<&'static str> {
    static DISPOSABLE_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    DISPOSABLE_SET.get_or_init(|| {
        DISPOSABLE_DOMAINS
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    })
}

pub(crate) fn is_disposable_domain(domain: &str) -> bool {
    disposable_domains().contains(domain)
}
