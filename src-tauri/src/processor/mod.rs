mod cache;
mod classify;
mod dns;
mod errors;
mod input;
mod output;
mod payload;
mod pipeline;
mod types;

pub use self::types::{ErrorPayload, MxStatus, ProcessingPayload};

use self::{
    cache::PersistentCache,
    classify::{build_edu_patterns, normalize_domain},
    dns::{collect_unique_domains, scan_domains},
    errors::{backend_error, map_regex_error_payload},
    input::total_bytes,
    output::{build_run_output_dir, build_writers, flush_writers},
    pipeline::{process_second_pass, process_smtp_spool},
    types::{ProcessingMode, Stats},
};
use crate::smtp_client::SmtpApiClient;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    time::Instant,
};

const BUFFER_CAPACITY: usize = 1024 * 1024;
const EMIT_EVERY: u64 = 50;
const DOMAIN_SCAN_BATCH_SIZE: usize = 1_000;
const SMTP_BATCH_SIZE: usize = 5;
const FIRST_PASS_PROGRESS_END: f64 = 35.0;
const DOMAIN_SCAN_PROGRESS_END: f64 = 65.0;
const CACHE_TTL_SECS: i64 = 6 * 3600;
const SMTP_CACHE_TTL_SECS: i64 = 6 * 3600;
const PUBLIC_DOMAINS: [&str; 19] = [
    "gmail.com",
    "yahoo.com",
    "aol.com",
    "outlook.com",
    "icloud.com",
    "hotmail.com",
    "mail.com",
    "ymail.com",
    "live.com",
    "msn.com",
    "gmx.es",
    "googlemail.com",
    "pm.me",
    "o2.pl",
    "inbox.lv",
    "yahoo.co.uk",
    "yahoo.ca",
    "yahoo.com.mx",
    "yahoo.com.ph",
];
const PARKING_MX_SUFFIXES: &[&str] = &[
    "registrar-servers.com",
    "sedoparking.com",
    "parkingcrew.net",
    "hugedomains.com",
    "above.com",
    "bodis.com",
    "afternic.com",
    "dan.com",
];
const PARKED_DOMAIN_SUFFIXES: &[&str] = &[
    "hugedomains.com",
    "afternic.com",
    "dan.com",
    "sedoparking.com",
    "parkingcrew.net",
    "bodis.com",
    "above.com",
];
const TYPO_MAP: &[(&str, &[&str])] = &[
    (
        "gmail.com",
        &[
            "gmial.com",
            "gmai.com",
            "gamil.com",
            "gmal.com",
            "gnail.com",
        ],
    ),
    (
        "yahoo.com",
        &["yahooo.com", "yaho.com", "yhoo.com", "yaoo.com"],
    ),
    (
        "outlook.com",
        &["outlok.com", "outloook.com", "outllook.com"],
    ),
    ("hotmail.com", &["hotmai.com", "hotmial.com", "hotmale.com"]),
    ("icloud.com", &["iclould.com", "icolud.com"]),
];
const DISPOSABLE_DOMAINS: &str = include_str!("../data/disposable_domains.txt");

#[allow(clippy::too_many_arguments)]
pub async fn process_file_core<F>(
    file_paths: Vec<String>,
    output_path: &Path,
    target_domains: Vec<String>,
    check_mx: bool,
    timeout_ms: u64,
    max_concurrent: usize,
    use_persistent_cache: bool,
    persistent_cache_path: Option<&Path>,
    smtp_enabled: bool,
    vps_api_url: &str,
    vps_api_key: &str,
    mut emit_progress_event: F,
) -> Result<ProcessingPayload, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let started_at = Instant::now();

    if file_paths.is_empty() {
        return Err(backend_error(
            "No input files provided.",
            "Không có tệp đầu vào nào được cung cấp.",
            None,
        ));
    }

    fs::create_dir_all(output_path).map_err(|error| {
        backend_error(
            "Failed to create output directory.",
            "Không thể tạo thư mục đầu ra.",
            Some(error.to_string()),
        )
    })?;

    let run_output_path = build_run_output_dir(output_path, &file_paths)?;
    fs::create_dir_all(&run_output_path).map_err(|error| {
        backend_error(
            "Failed to create the session output directory.",
            "Không thể tạo thư mục đầu ra cho phiên lọc.",
            Some(error.to_string()),
        )
    })?;

    let output_dir = run_output_path.to_string_lossy().to_string();
    let total_bytes = total_bytes(&file_paths);
    let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}")
        .map_err(map_regex_error_payload)?;
    let target_domains_set: HashSet<String> = target_domains
        .into_iter()
        .filter_map(|value| normalize_domain(&value).ok())
        .collect();
    let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
    let edu_patterns = build_edu_patterns()?;

    let smtp_client = if check_mx && smtp_enabled {
        SmtpApiClient::new(vps_api_url.to_string(), vps_api_key.to_string())
    } else {
        None
    };
    let smtp_phase_enabled = check_mx && smtp_enabled;
    let persistent_cache = if use_persistent_cache {
        persistent_cache_path.map(PersistentCache::new).transpose()?
    } else {
        None
    };

    let (cache_hits, domain_statuses) = if check_mx {
        let collected_domains = collect_unique_domains(
            &file_paths,
            &extractor_regex,
            total_bytes,
            &output_dir,
            started_at,
            smtp_phase_enabled,
            &mut emit_progress_event,
        )?;

        let mut cached_domain_statuses = if let Some(cache) = &persistent_cache {
            cache.load_many(&collected_domains.unique_domains)?
        } else {
            HashMap::new()
        };
        let cache_hits = cached_domain_statuses.len() as u64;

        let domains_to_scan: Vec<String> = collected_domains
            .unique_domains
            .into_iter()
            .filter(|domain| !cached_domain_statuses.contains_key(domain))
            .collect();

        let freshly_scanned = scan_domains(
            domains_to_scan,
            timeout_ms,
            max_concurrent,
            total_bytes,
            &output_dir,
            started_at,
            smtp_phase_enabled,
            &mut emit_progress_event,
        )
        .await?;

        if let Some(cache) = &persistent_cache {
            cache.store_many(&freshly_scanned)?;
        }

        cached_domain_statuses.extend(freshly_scanned);
        (cache_hits, cached_domain_statuses)
    } else {
        (0, HashMap::new())
    };

    let mut writers =
        build_writers(&run_output_path, check_mx, smtp_phase_enabled).map_err(|error| {
            errors::error_payload_from_io(
                "Failed to create one or more result files.",
                "Không thể tạo một hoặc nhiều tệp kết quả.",
                error,
            )
        })?;
    let mut stats = Stats {
        cache_hits,
        ..Default::default()
    };

    let second_pass = process_second_pass(
        &file_paths,
        total_bytes,
        &extractor_regex,
        &public_domains,
        &edu_patterns,
        &target_domains_set,
        if check_mx {
            ProcessingMode::VerifyDns
        } else {
            ProcessingMode::BasicFilter
        },
        &domain_statuses,
        smtp_phase_enabled,
        &output_dir,
        started_at,
        &mut writers,
        &mut stats,
        &mut emit_progress_event,
    )?;

    let smtp_elapsed_ms = if smtp_phase_enabled {
        let smtp_started_at = Instant::now();
        if let Some(spool_path) = second_pass.smtp_spool_path.as_ref() {
            process_smtp_spool(
                spool_path,
                persistent_cache.as_ref(),
                smtp_client.as_ref(),
                &mut writers,
                &mut stats,
                second_pass.processed_lines,
                &output_dir,
                started_at,
                &mut emit_progress_event,
            )
            .await?;

            let _ = fs::remove_file(spool_path);
        }
        smtp_started_at.elapsed().as_millis() as u64
    } else {
        0
    };

    flush_writers(&mut writers)?;

    Ok(payload::build_processing_payload(
        &output_dir,
        second_pass.processed_lines,
        if check_mx {
            100.0
        } else {
            payload::scale_second_pass_progress(total_bytes, total_bytes, false, false)
        },
        &stats,
        smtp_phase_enabled,
        smtp_elapsed_ms,
        started_at.elapsed().as_millis(),
        None,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp_status::{SmtpProbeRecord, SmtpStatus};
    use std::{fs, sync::Arc};
    use tokio::sync::Semaphore;

    #[test]
    fn normalize_domain_handles_idn_and_trailing_dot() {
        assert_eq!(
            classify::normalize_domain(" MÜNCHEN.DE. ").unwrap(),
            "xn--mnchen-3ya.de"
        );
    }

    #[test]
    fn typo_and_parked_checks_work() {
        assert_eq!(classify::check_typo("gmial.com"), Some("gmail.com".to_string()));
        assert!(classify::is_parked_mx("mx1.registrar-servers.com."));
        assert!(classify::is_parked_domain("hugedomains.com"));
        assert!(classify::is_parked_domain("shop.hugedomains.com"));
    }

    #[test]
    fn extract_email_falls_back_for_unicode_domains() {
        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let extracted = input::extract_email_candidate_from_line(
            "User.Name@münchen.de",
            &extractor_regex,
        )
        .unwrap();
        let parsed = input::parse_email_candidate(&extracted).unwrap();
        assert_eq!(parsed.raw_local_part, "User.Name");
        assert_eq!(parsed.normalized_domain, "xn--mnchen-3ya.de");
        assert_eq!(parsed.normalized_email, "User.Name@xn--mnchen-3ya.de");
    }

    #[test]
    fn disposable_domains_are_embedded() {
        assert!(classify::is_disposable_domain("mailinator.com"));
    }

    #[test]
    fn persistent_cache_round_trip_restores_statuses() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-cache-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let cache_path = base_dir.join("mx_cache.sqlite3");
        let cache = PersistentCache::new(&cache_path).unwrap();

        let mut values = HashMap::new();
        values.insert("gmail.com".to_string(), MxStatus::HasMx);
        values.insert("null.test".to_string(), MxStatus::NullMx);
        values.insert(
            "gmial.com".to_string(),
            MxStatus::TypoSuggestion("gmail.com".to_string()),
        );
        cache.store_many(&values).unwrap();

        let restored = cache
            .load_many(&[
                "gmail.com".to_string(),
                "null.test".to_string(),
                "gmial.com".to_string(),
                "unknown.com".to_string(),
            ])
            .unwrap();

        assert_eq!(restored.get("gmail.com"), Some(&MxStatus::HasMx));
        assert_eq!(restored.get("null.test"), Some(&MxStatus::NullMx));
        assert_eq!(
            restored.get("gmial.com"),
            Some(&MxStatus::TypoSuggestion("gmail.com".to_string()))
        );
        assert!(!restored.contains_key("unknown.com"));
    }

    #[test]
    fn smtp_cache_preserves_exact_local_part() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-smtp-cache-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let cache_path = base_dir.join("smtp_cache.sqlite3");
        let cache = PersistentCache::new(&cache_path).unwrap();

        cache
            .store_smtp_many(&[
                SmtpProbeRecord {
                    email: "User.Name@gmail.com".to_string(),
                    outcome: SmtpStatus::Accepted,
                    cached: false,
                    duration_ms: 10,
                    ..Default::default()
                },
                SmtpProbeRecord {
                    email: "other@gmail.com".to_string(),
                    outcome: SmtpStatus::BadMailbox,
                    cached: false,
                    duration_ms: 11,
                    ..Default::default()
                },
            ])
            .unwrap();

        let loaded = cache
            .load_smtp_many(&[
                input::parse_email_candidate("User.Name@gmail.com").unwrap(),
                input::parse_email_candidate("user.name@gmail.com").unwrap(),
                input::parse_email_candidate("other@gmail.com").unwrap(),
            ])
            .unwrap();

        assert_eq!(
            loaded.get("User.Name@gmail.com").map(|record| &record.outcome),
            Some(&SmtpStatus::Accepted)
        );
        assert!(!loaded.contains_key("user.name@gmail.com"));
        assert_eq!(
            loaded.get("other@gmail.com").map(|record| &record.outcome),
            Some(&SmtpStatus::BadMailbox)
        );
    }

    #[test]
    fn synthetic_smtp_fallback_records_are_not_persisted() {
        let fallback = SmtpProbeRecord {
            email: "person@gmail.com".to_string(),
            outcome: SmtpStatus::Inconclusive,
            ..Default::default()
        };
        let real_timeout = SmtpProbeRecord {
            email: "person@gmail.com".to_string(),
            outcome: SmtpStatus::Inconclusive,
            smtp_reply_text: Some("timed out".to_string()),
            mx_host: Some("gmail-smtp-in.l.google.com".to_string()),
            duration_ms: 1_234,
            ..Default::default()
        };

        assert!(!pipeline::should_persist_smtp_record(&fallback));
        assert!(pipeline::should_persist_smtp_record(&real_timeout));
    }

    #[tokio::test]
    async fn typo_is_prioritized_before_disposable_when_both_match() {
        let resolver = dns::build_resolver(1_500);
        let cache = Arc::new(cache::DomainCache::default());
        let semaphore = Arc::new(Semaphore::new(1));
        let status = dns::check_domain_mx_async(
            "gmial.com".to_string(),
            resolver,
            cache,
            semaphore,
        )
        .await;

        assert_eq!(status, MxStatus::TypoSuggestion("gmail.com".to_string()));
    }

    #[test]
    fn verify_second_pass_writes_t2_and_t4_outputs() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-verify-pass-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let input_path = base_dir.join("emails.txt");
        let output_dir = base_dir.join("output");
        fs::create_dir_all(&output_dir).unwrap();
        fs::write(
            &input_path,
            "alive@gmail.com\nmaybe@timeout.test\ndead@dead.test\ninvalid-line\nalive@gmail.com\n",
        )
        .unwrap();

        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
        let edu_patterns = build_edu_patterns().unwrap();
        let target_domains = HashSet::new();
        let mut writers = build_writers(&output_dir, true, false).unwrap();
        let mut stats = Stats::default();
        let mut domain_statuses = HashMap::new();
        domain_statuses.insert("gmail.com".to_string(), MxStatus::HasMx);
        domain_statuses.insert("timeout.test".to_string(), MxStatus::Inconclusive);
        domain_statuses.insert("dead.test".to_string(), MxStatus::Dead);

        let result = process_second_pass(
            &[input_path.to_string_lossy().to_string()],
            fs::metadata(&input_path).unwrap().len(),
            &extractor_regex,
            &public_domains,
            &edu_patterns,
            &target_domains,
            ProcessingMode::VerifyDns,
            &domain_statuses,
            false,
            &output_dir.to_string_lossy(),
            Instant::now(),
            &mut writers,
            &mut stats,
            &mut |_payload, _event| Ok(()),
        )
        .unwrap();
        flush_writers(&mut writers).unwrap();

        assert_eq!(result.processed_lines, 5);
        assert_eq!(stats.invalid, 1);
        assert_eq!(stats.mx_has_mx, 1);
        assert_eq!(stats.mx_inconclusive, 1);
        assert_eq!(stats.mx_dead, 1);
        assert_eq!(stats.duplicates, 1);
        assert_eq!(stats.final_dead, 2);
        assert_eq!(stats.final_unknown, 2);

        let has_mx_file =
            fs::read_to_string(output_dir.join("10_T2_DNS_Valid_Has_MX.txt")).unwrap();
        let inconclusive_file =
            fs::read_to_string(output_dir.join("16_T2_DNS_Inconclusive.txt")).unwrap();
        let dead_file =
            fs::read_to_string(output_dir.join("12_T2_DNS_Error_Dead.txt")).unwrap();
        let final_dead_file =
            fs::read_to_string(output_dir.join("31_T4_FINAL_Dead.txt")).unwrap();
        let final_unknown_file =
            fs::read_to_string(output_dir.join("32_T4_FINAL_Unknown.txt")).unwrap();
        let detail_csv =
            fs::read_to_string(output_dir.join("33_T4_FINAL_Detail.csv")).unwrap();

        assert!(has_mx_file.contains("alive@gmail.com"));
        assert!(inconclusive_file.contains("maybe@timeout.test"));
        assert!(dead_file.contains("dead@dead.test"));
        assert!(final_dead_file.contains("dead@dead.test"));
        assert!(final_dead_file.contains("invalid-line"));
        assert!(final_unknown_file.contains("alive@gmail.com"));
        assert!(final_unknown_file.contains("maybe@timeout.test"));
        assert!(detail_csv.contains("SyntaxInvalid"));
        assert!(detail_csv.contains("Inconclusive"));
    }

    #[test]
    fn collect_unique_domains_deduplicates_domains() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-domains-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let input_path = base_dir.join("emails.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(
            &input_path,
            "a@gmail.com\nb@gmail.com\nc@outlook.com\nd@OUTLOOK.COM.\n",
        )
        .unwrap();

        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let domains = collect_unique_domains(
            &[input_path.to_string_lossy().to_string()],
            &extractor_regex,
            fs::metadata(&input_path).unwrap().len(),
            &base_dir.to_string_lossy(),
            Instant::now(),
            false,
            &mut |_payload, _event| Ok(()),
        )
        .unwrap();

        let domain_set: HashSet<String> = domains.unique_domains.into_iter().collect();
        assert_eq!(domain_set.len(), 2);
    }
}
