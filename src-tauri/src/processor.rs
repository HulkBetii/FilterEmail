use crate::smtp_client::{SmtpApiClient, SmtpVerifyTarget};
use crate::smtp_status::{FinalTriage, SmtpProbeRecord, SmtpStatus};
use crate::smtp_verify::{dns_status_name, final_triage_for, output_bucket_for, OutputBucket};
use chrono::{Local, Utc};
use hickory_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    proto::op::ResponseCode,
};
use idna::Config;
use rand::Rng;
use regex::Regex;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{RwLock, Semaphore},
    task::JoinSet,
    time::sleep,
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
const DISPOSABLE_DOMAINS: &str = include_str!("data/disposable_domains.txt");

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

struct Writers {
    invalid: BufWriter<File>,
    public: BufWriter<File>,
    edu: BufWriter<File>,
    targeted: BufWriter<File>,
    custom: BufWriter<File>,
    mx_dead: BufWriter<File>,
    mx_has_mx: BufWriter<File>,
    mx_a_fallback: BufWriter<File>,
    mx_inconclusive: BufWriter<File>,
    mx_parked: BufWriter<File>,
    mx_disposable: BufWriter<File>,
    mx_typo: BufWriter<File>,
    smtp_deliverable: Option<BufWriter<File>>,
    smtp_rejected: Option<BufWriter<File>>,
    smtp_catchall: Option<BufWriter<File>>,
    smtp_unknown: Option<BufWriter<File>>,
    final_alive: Option<BufWriter<File>>,
    final_dead: Option<BufWriter<File>>,
    final_unknown: Option<BufWriter<File>>,
    detail_csv: Option<BufWriter<File>>,
    invalid_name: String,
    public_name: String,
    edu_name: String,
    targeted_name: String,
    custom_name: String,
    mx_dead_name: String,
    mx_has_mx_name: String,
    mx_a_fallback_name: String,
    mx_inconclusive_name: String,
    mx_parked_name: String,
    mx_disposable_name: String,
    mx_typo_name: String,
    smtp_deliverable_name: String,
    smtp_rejected_name: String,
    smtp_catchall_name: String,
    smtp_unknown_name: String,
    final_alive_name: String,
    final_dead_name: String,
    final_unknown_name: String,
    detail_csv_name: String,
}

#[derive(Clone, Debug, Default)]
struct Stats {
    invalid: u64,
    public: u64,
    edu: u64,
    targeted: u64,
    custom: u64,
    duplicates: u64,
    mx_dead: u64,
    mx_has_mx: u64,
    mx_a_fallback: u64,
    mx_inconclusive: u64,
    mx_parked: u64,
    mx_disposable: u64,
    mx_typo: u64,
    smtp_deliverable: u64,
    smtp_rejected: u64,
    smtp_catchall: u64,
    smtp_unknown: u64,
    cache_hits: u64,
    final_alive: u64,
    final_dead: u64,
    final_unknown: u64,
    smtp_attempted_emails: u64,
    smtp_cache_hits: u64,
    smtp_policy_blocked: u64,
    smtp_temp_failure: u64,
    smtp_mailbox_full: u64,
    smtp_mailbox_disabled: u64,
    smtp_bad_mailbox: u64,
    smtp_bad_domain: u64,
    smtp_network_error: u64,
    smtp_protocol_error: u64,
    smtp_timeout: u64,
}

struct CollectedDomains {
    unique_domains: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedEmail {
    normalized_email: String,
    canonical_email_key: String,
    raw_local_part: String,
    normalized_domain: String,
}

struct SecondPassResult {
    processed_lines: u64,
    smtp_spool_path: Option<PathBuf>,
}

#[derive(Copy, Clone)]
enum EmailGroup {
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

#[derive(Default)]
pub struct DomainCache {
    inner: RwLock<HashMap<String, MxStatus>>,
}

impl DomainCache {
    pub async fn get(&self, domain: &str) -> Option<MxStatus> {
        self.inner.read().await.get(domain).cloned()
    }

    pub async fn set(&self, domain: String, status: MxStatus) {
        self.inner.write().await.insert(domain, status);
    }
}

struct PersistentCache {
    path: PathBuf,
}

impl PersistentCache {
    fn new(path: &Path) -> Result<Self, ErrorPayload> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                backend_error(
                    "Failed to create persistent cache directory.",
                    "Không thể tạo thư mục persistent cache.",
                    Some(error.to_string()),
                )
            })?;
        }

        let cache = Self {
            path: path.to_path_buf(),
        };
        cache.init()?;
        Ok(cache)
    }

    fn init(&self) -> Result<(), ErrorPayload> {
        let conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mx_cache (
                domain TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                cached_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_mx_cache_cached_at ON mx_cache(cached_at);
            CREATE TABLE IF NOT EXISTS smtp_cache (
                local_part TEXT NOT NULL COLLATE BINARY,
                domain TEXT NOT NULL,
                outcome TEXT NOT NULL,
                smtp_basic_code INTEGER,
                smtp_enhanced_code TEXT,
                smtp_reply_text TEXT,
                mx_host TEXT,
                catch_all INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY(local_part, domain)
            );
            CREATE INDEX IF NOT EXISTS idx_smtp_cache_cached_at ON smtp_cache(cached_at);
            CREATE TABLE IF NOT EXISTS smtp_catch_all_cache (
                domain TEXT PRIMARY KEY,
                catch_all INTEGER NOT NULL,
                mx_host TEXT,
                cached_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_smtp_catch_all_cache_cached_at ON smtp_catch_all_cache(cached_at);",
        )
        .map_err(|error| {
            backend_error(
                "Failed to initialize persistent cache database.",
                "Không thể khởi tạo cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        Ok(())
    }

    fn load_many(&self, domains: &[String]) -> Result<HashMap<String, MxStatus>, ErrorPayload> {
        let conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let cutoff = unix_now_secs() - CACHE_TTL_SECS;
        let mut results = HashMap::new();

        for domain in domains {
            let cached = conn
                .query_row(
                    "SELECT status FROM mx_cache WHERE domain = ?1 AND cached_at > ?2",
                    params![domain, cutoff],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .and_then(|value| parse_cached_status(&value));

            if let Some(status) = cached {
                results.insert(domain.clone(), status);
            }
        }

        Ok(results)
    }

    fn store_many(&self, domain_statuses: &HashMap<String, MxStatus>) -> Result<(), ErrorPayload> {
        if domain_statuses.is_empty() {
            return Ok(());
        }

        let mut conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let tx = conn.transaction().map_err(|error| {
            backend_error(
                "Failed to start persistent cache transaction.",
                "Không thể bắt đầu transaction cho persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let now = unix_now_secs();

        for (domain, status) in domain_statuses {
            tx.execute(
                "INSERT INTO mx_cache (domain, status, cached_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(domain) DO UPDATE SET status = excluded.status, cached_at = excluded.cached_at",
                params![domain, cached_status_value(status), now],
            )
            .map_err(|error| {
                backend_error(
                    "Failed to write persistent cache entry.",
                    "Không thể ghi mục persistent cache.",
                    Some(error.to_string()),
                )
            })?;
        }

        tx.commit().map_err(|error| {
            backend_error(
                "Failed to commit persistent cache transaction.",
                "Không thể lưu transaction persistent cache.",
                Some(error.to_string()),
            )
        })?;
        Ok(())
    }

    fn load_smtp_many(
        &self,
        emails: &[ParsedEmail],
    ) -> Result<HashMap<String, SmtpProbeRecord>, ErrorPayload> {
        let conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let cutoff = unix_now_secs() - SMTP_CACHE_TTL_SECS;
        let mut results = HashMap::new();

        for email in emails {
            let catch_all = conn
                .query_row(
                    "SELECT catch_all, mx_host FROM smtp_catch_all_cache WHERE domain = ?1 AND cached_at > ?2",
                    params![email.normalized_domain, cutoff],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .ok();

            if let Some((1, mx_host)) = catch_all {
                results.insert(
                    email.normalized_email.clone(),
                    SmtpProbeRecord {
                        email: email.normalized_email.clone(),
                        outcome: SmtpStatus::CatchAll,
                        mx_host,
                        catch_all: true,
                        cached: true,
                        ..Default::default()
                    },
                );
                continue;
            }

            let cached = conn
                .query_row(
                    "SELECT outcome, smtp_basic_code, smtp_enhanced_code, smtp_reply_text, mx_host, catch_all, duration_ms
                     FROM smtp_cache
                     WHERE local_part = ?1 AND domain = ?2 AND cached_at > ?3",
                    params![email.raw_local_part, email.normalized_domain, cutoff],
                    |row| {
                        Ok(SmtpProbeRecord {
                            email: email.normalized_email.clone(),
                            outcome: parse_cached_smtp_status(&row.get::<_, String>(0)?)?,
                            smtp_basic_code: row.get::<_, Option<u16>>(1)?,
                            smtp_enhanced_code: row.get::<_, Option<String>>(2)?,
                            smtp_reply_text: row.get::<_, Option<String>>(3)?,
                            mx_host: row.get::<_, Option<String>>(4)?,
                            catch_all: row.get::<_, i64>(5)? != 0,
                            cached: true,
                            duration_ms: row.get::<_, i64>(6)? as u64,
                        })
                    },
                )
                .ok();

            if let Some(record) = cached {
                results.insert(email.normalized_email.clone(), record);
            }
        }

        Ok(results)
    }

    fn store_smtp_many(&self, records: &[SmtpProbeRecord]) -> Result<(), ErrorPayload> {
        if records.is_empty() {
            return Ok(());
        }

        let mut conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let tx = conn.transaction().map_err(|error| {
            backend_error(
                "Failed to start persistent cache transaction.",
                "Không thể bắt đầu transaction cho SMTP cache.",
                Some(error.to_string()),
            )
        })?;
        let now = unix_now_secs();

        for record in records {
            let Some((local_part, domain)) = record.email.rsplit_once('@') else {
                continue;
            };

            tx.execute(
                "INSERT INTO smtp_cache (
                    local_part, domain, outcome, smtp_basic_code, smtp_enhanced_code, smtp_reply_text, mx_host, catch_all, duration_ms, cached_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(local_part, domain) DO UPDATE SET
                    outcome = excluded.outcome,
                    smtp_basic_code = excluded.smtp_basic_code,
                    smtp_enhanced_code = excluded.smtp_enhanced_code,
                    smtp_reply_text = excluded.smtp_reply_text,
                    mx_host = excluded.mx_host,
                    catch_all = excluded.catch_all,
                    duration_ms = excluded.duration_ms,
                    cached_at = excluded.cached_at",
                params![
                    local_part,
                    domain,
                    cached_smtp_status_value(&record.outcome),
                    record.smtp_basic_code,
                    record.smtp_enhanced_code,
                    record.smtp_reply_text,
                    record.mx_host,
                    if record.catch_all { 1 } else { 0 },
                    record.duration_ms as i64,
                    now,
                ],
            )
            .map_err(|error| {
                backend_error(
                    "Failed to write SMTP cache entry.",
                    "Không thể ghi mục SMTP cache.",
                    Some(error.to_string()),
                )
            })?;

            tx.execute(
                "INSERT INTO smtp_catch_all_cache (domain, catch_all, mx_host, cached_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(domain) DO UPDATE SET
                    catch_all = excluded.catch_all,
                    mx_host = excluded.mx_host,
                    cached_at = excluded.cached_at",
                params![
                    domain,
                    if record.catch_all { 1 } else { 0 },
                    record.mx_host,
                    now
                ],
            )
            .map_err(|error| {
                backend_error(
                    "Failed to write SMTP catch-all cache entry.",
                    "Không thể ghi mục SMTP catch-all cache.",
                    Some(error.to_string()),
                )
            })?;
        }

        tx.commit().map_err(|error| {
            backend_error(
                "Failed to commit SMTP cache transaction.",
                "Không thể lưu transaction SMTP cache.",
                Some(error.to_string()),
            )
        })?;
        Ok(())
    }
}

fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn cached_status_value(status: &MxStatus) -> String {
    match status {
        MxStatus::HasMx => "has_mx".to_string(),
        MxStatus::ARecordFallback => "a_record_fallback".to_string(),
        MxStatus::Dead => "dead".to_string(),
        MxStatus::NullMx => "null_mx".to_string(),
        MxStatus::Parked => "parked".to_string(),
        MxStatus::Disposable => "disposable".to_string(),
        MxStatus::TypoSuggestion(suggestion) => format!("typo:{suggestion}"),
        MxStatus::Inconclusive => "inconclusive".to_string(),
    }
}

fn parse_cached_status(value: &str) -> Option<MxStatus> {
    match value {
        "has_mx" => Some(MxStatus::HasMx),
        "a_record_fallback" => Some(MxStatus::ARecordFallback),
        "dead" => Some(MxStatus::Dead),
        "null_mx" => Some(MxStatus::NullMx),
        "parked" => Some(MxStatus::Parked),
        "disposable" => Some(MxStatus::Disposable),
        "inconclusive" => Some(MxStatus::Inconclusive),
        _ => value
            .strip_prefix("typo:")
            .map(|suggestion| MxStatus::TypoSuggestion(suggestion.to_string())),
    }
}

fn cached_smtp_status_value(status: &SmtpStatus) -> &'static str {
    status.as_str()
}

fn parse_cached_smtp_status(value: &str) -> Result<SmtpStatus, rusqlite::Error> {
    Ok(match value {
        "Accepted" => SmtpStatus::Accepted,
        "AcceptedForwarded" => SmtpStatus::AcceptedForwarded,
        "CatchAll" => SmtpStatus::CatchAll,
        "BadMailbox" => SmtpStatus::BadMailbox,
        "BadDomain" => SmtpStatus::BadDomain,
        "PolicyBlocked" => SmtpStatus::PolicyBlocked,
        "MailboxFull" => SmtpStatus::MailboxFull,
        "MailboxDisabled" => SmtpStatus::MailboxDisabled,
        "TempFailure" => SmtpStatus::TempFailure,
        "NetworkError" => SmtpStatus::NetworkError,
        "ProtocolError" => SmtpStatus::ProtocolError,
        "Timeout" => SmtpStatus::Timeout,
        _ => SmtpStatus::Inconclusive,
    })
}

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
        error_payload_from_io(
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

    Ok(build_processing_payload(
        &output_dir,
        second_pass.processed_lines,
        if check_mx { 100.0 } else { scale_second_pass_progress(total_bytes, total_bytes, false, false) },
        &stats,
        smtp_phase_enabled,
        smtp_elapsed_ms,
        started_at.elapsed().as_millis(),
        None,
        None,
    ))
}

fn total_bytes(file_paths: &[String]) -> u64 {
    file_paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|meta| meta.len())
        .sum()
}

fn extract_email_candidate_from_line(line: &str, extractor_regex: &Regex) -> Option<String> {
    if let Some(matched) = extractor_regex.find(line) {
        return Some(matched.as_str().trim().to_string());
    }

    line.split_whitespace()
        .map(|token| {
            token.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
                )
            })
        })
        .find_map(|token| {
            let candidate = token.trim();
            let (local, domain) = candidate.rsplit_once('@')?;
            if local.is_empty() || domain.is_empty() || !domain.contains('.') {
                return None;
            }
            normalize_domain(domain).ok()?;
            Some(candidate.to_string())
        })
}

fn parse_email_candidate(candidate: &str) -> Option<ParsedEmail> {
    let trimmed = candidate.trim();
    let (raw_local_part, raw_domain) = trimmed.rsplit_once('@')?;
    if raw_local_part.is_empty() || raw_domain.is_empty() || !raw_domain.contains('.') {
        return None;
    }

    let normalized_domain = normalize_domain(raw_domain).ok()?;
    let normalized_email = format!("{raw_local_part}@{normalized_domain}");
    let canonical_email_key = format!("{}@{}", raw_local_part.to_lowercase(), normalized_domain);

    Some(ParsedEmail {
        normalized_email,
        canonical_email_key,
        raw_local_part: raw_local_part.to_string(),
        normalized_domain,
    })
}

fn collect_unique_domains<F>(
    file_paths: &[String],
    extractor_regex: &Regex,
    total_bytes: u64,
    output_dir: &str,
    started_at: Instant,
    smtp_enabled: bool,
    emit_progress_event: &mut F,
) -> Result<CollectedDomains, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let mut bytes_read = 0u64;
    let mut processed_lines = 0u64;
    let mut domains = HashSet::new();
    for file_path in file_paths {
        let path = Path::new(file_path);
        if !path.exists() {
            continue;
        }

        let input_file = match File::open(path) {
            Ok(file) => file,
            Err(_) => continue,
        };

        let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, input_file);
        let mut line = String::with_capacity(1024);

        loop {
            line.clear();
            let read = match reader.read_line(&mut line) {
                Ok(bytes) => bytes,
                Err(_) => break,
            };

            if read == 0 {
                break;
            }

            processed_lines += 1;
            bytes_read += read as u64;

            if let Some(candidate) = extract_email_candidate_from_line(&line, extractor_regex)
                && let Some(parsed) = parse_email_candidate(&candidate)
            {
                domains.insert(parsed.normalized_domain);
            }

            if processed_lines.is_multiple_of(EMIT_EVERY) {
                let payload = build_processing_payload(
                    output_dir,
                    processed_lines,
                    scale_progress(total_bytes, bytes_read, FIRST_PASS_PROGRESS_END),
                    &Stats::default(),
                    smtp_enabled,
                    0,
                    started_at.elapsed().as_millis(),
                    None,
                    None,
                );
                emit_progress_event(payload, "processing-progress").ok();
            }
        }
    }

    Ok(CollectedDomains {
        unique_domains: domains.into_iter().collect(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn scan_domains<F>(
    unique_domains: Vec<String>,
    timeout_ms: u64,
    max_concurrent: usize,
    total_bytes: u64,
    output_dir: &str,
    started_at: Instant,
    smtp_enabled: bool,
    emit_progress_event: &mut F,
) -> Result<HashMap<String, MxStatus>, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    if unique_domains.is_empty() {
        return Ok(HashMap::new());
    }

    let resolver = build_resolver(timeout_ms);
    let cache = Arc::new(DomainCache::default());
    let semaphore = Arc::new(Semaphore::new(max_concurrent.clamp(1, 50)));
    let total_domains = unique_domains.len();
    let mut results = HashMap::with_capacity(total_domains);
    let mut processed_domains = 0usize;
    let total_lines_hint = if total_bytes == 0 {
        total_domains as u64
    } else {
        0
    };

    for batch in unique_domains.chunks(DOMAIN_SCAN_BATCH_SIZE) {
        let mut join_set = JoinSet::new();

        for domain in batch {
            let resolver = resolver.clone();
            let cache = Arc::clone(&cache);
            let semaphore = Arc::clone(&semaphore);
            let domain = domain.clone();

            join_set.spawn(async move {
                let status =
                    check_domain_mx_async(domain.clone(), resolver, cache, semaphore).await;
                (domain, status)
            });
        }

        while let Some(result) = join_set.join_next().await {
            let (domain, status) = result.map_err(|error| {
                backend_error(
                    "Deep DNS scan task failed.",
                    "Tác vụ quét DNS sâu thất bại.",
                    Some(error.to_string()),
                )
            })?;

            processed_domains += 1;
            results.insert(domain.clone(), status);

            if processed_domains.is_multiple_of(EMIT_EVERY as usize)
                || processed_domains == total_domains
            {
                let domain_progress = if total_domains == 0 {
                    DOMAIN_SCAN_PROGRESS_END
                } else {
                    FIRST_PASS_PROGRESS_END
                        + ((processed_domains as f64 / total_domains as f64)
                            * (DOMAIN_SCAN_PROGRESS_END - FIRST_PASS_PROGRESS_END))
                };
                let payload = build_processing_payload(
                    output_dir,
                    total_lines_hint,
                    domain_progress,
                    &Stats::default(),
                    smtp_enabled,
                    0,
                    started_at.elapsed().as_millis(),
                    Some(domain),
                    None,
                );
                emit_progress_event(payload, "processing-progress").ok();
            }
        }
    }

    Ok(results)
}

async fn check_domain_mx_async(
    domain: String,
    resolver: TokioAsyncResolver,
    cache: Arc<DomainCache>,
    semaphore: Arc<Semaphore>,
) -> MxStatus {
    if let Some(status) = cache.get(&domain).await {
        return status;
    }

    if let Some(suggestion) = check_typo(&domain) {
        let status = MxStatus::TypoSuggestion(suggestion);
        cache.set(domain, status.clone()).await;
        return status;
    }

    if is_disposable_domain(&domain) {
        let status = MxStatus::Disposable;
        cache.set(domain, status.clone()).await;
        return status;
    }

    let jitter_ms = rand::thread_rng().gen_range(0..50u64);
    sleep(Duration::from_millis(jitter_ms)).await;
    let permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(_) => {
            let status = MxStatus::Inconclusive;
            cache.set(domain, status.clone()).await;
            return status;
        }
    };

    let mut final_status = MxStatus::Inconclusive;

    for attempt in 0..=2u8 {
        match resolver.mx_lookup(domain.clone()).await {
            Ok(lookup) => {
                let mx_records: Vec<_> = lookup.iter().collect();
                let is_null_mx = !mx_records.is_empty()
                    && mx_records.iter().all(|mx| {
                        mx.preference() == 0 && mx.exchange().to_string().trim() == "."
                    });
                if is_null_mx {
                    final_status = MxStatus::NullMx;
                } else if mx_records.is_empty() {
                    final_status = check_a_record_fallback(&resolver, &domain).await;
                } else {
                    let all_parked = mx_records
                        .iter()
                        .all(|mx| is_parked_mx(&mx.exchange().to_string()));
                    final_status = if is_parked_domain(&domain) || all_parked {
                        MxStatus::Parked
                    } else {
                        MxStatus::HasMx
                    };
                }
                break;
            }
            Err(error) => match error.kind() {
                ResolveErrorKind::NoRecordsFound { response_code, .. }
                    if *response_code == ResponseCode::NXDomain =>
                {
                    final_status = MxStatus::Dead;
                    break;
                }
                ResolveErrorKind::NoRecordsFound { response_code, .. }
                    if *response_code == ResponseCode::NoError =>
                {
                    final_status = check_a_record_fallback(&resolver, &domain).await;
                    break;
                }
                _ if attempt < 2 => {
                    sleep(Duration::from_millis(80 * (attempt as u64 + 1))).await;
                    continue;
                }
                _ => {
                    final_status = MxStatus::Inconclusive;
                    break;
                }
            },
        }
    }

    drop(permit);
    cache.set(domain, final_status.clone()).await;
    final_status
}

async fn check_a_record_fallback(resolver: &TokioAsyncResolver, domain: &str) -> MxStatus {
    match resolver.lookup_ip(domain).await {
        Ok(lookup) if lookup.iter().next().is_some() => MxStatus::ARecordFallback,
        Ok(_) => MxStatus::Dead,
        Err(error) => match error.kind() {
            ResolveErrorKind::NoRecordsFound { response_code, .. }
                if *response_code == ResponseCode::NXDomain
                    || *response_code == ResponseCode::NoError =>
            {
                MxStatus::Dead
            }
            _ => MxStatus::Inconclusive,
        },
    }
}

#[derive(Copy, Clone)]
enum ProcessingMode {
    BasicFilter,
    VerifyDns,
}

fn build_resolver(timeout_ms: u64) -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_millis(timeout_ms.max(250));
    opts.attempts = 2;
    opts.validate = false;
    opts.cache_size = 1024;
    opts.preserve_intermediates = true;
    opts.rotate = true;
    TokioAsyncResolver::tokio(ResolverConfig::default(), opts)
}

#[allow(clippy::too_many_arguments)]
fn process_second_pass<F>(
    file_paths: &[String],
    total_bytes: u64,
    extractor_regex: &Regex,
    public_domains: &HashSet<&'static str>,
    edu_patterns: &[Regex],
    target_domains: &HashSet<String>,
    processing_mode: ProcessingMode,
    domain_statuses: &HashMap<String, MxStatus>,
    smtp_enabled: bool,
    output_dir: &str,
    started_at: Instant,
    writers: &mut Writers,
    stats: &mut Stats,
    emit_progress_event: &mut F,
) -> Result<SecondPassResult, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let mut line = String::with_capacity(1024);
    let mut bytes_read = 0u64;
    let mut processed_lines = 0u64;
    let mut last_emitted_pct: i64 = -1;
    let mut seen_emails: HashSet<String> = HashSet::with_capacity(100_000);
    let smtp_spool_path = if matches!(processing_mode, ProcessingMode::VerifyDns) && smtp_enabled {
        Some(Path::new(output_dir).join(".smtp_spool.txt"))
    } else {
        None
    };
    let mut smtp_spool_writer = if let Some(path) = smtp_spool_path.as_ref() {
        Some(BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(path).map_err(|error| {
                error_payload_from_io(
                    "Failed to create SMTP spool file.",
                    "Không thể tạo tệp tạm cho SMTP spool.",
                    error,
                )
            })?,
        ))
    } else {
        None
    };

    for file_path in file_paths {
        let path = Path::new(file_path);
        if !path.exists() {
            continue;
        }

        let input_file = match File::open(path) {
            Ok(file) => file,
            Err(_) => continue,
        };

        let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, input_file);

        loop {
            line.clear();
            let read = match reader.read_line(&mut line) {
                Ok(bytes) => bytes,
                Err(_) => break,
            };

            if read == 0 {
                break;
            }

            bytes_read += read as u64;
            processed_lines += 1;

            let parsed_email = match extract_email_candidate_from_line(&line, extractor_regex)
                .and_then(|candidate| parse_email_candidate(&candidate))
            {
                Some(parsed) => parsed,
                None => {
                    stats.invalid += 1;
                    write_line(&mut writers.invalid, line.trim(), &writers.invalid_name)?;
                    if matches!(processing_mode, ProcessingMode::VerifyDns) {
                        stats.final_dead += 1;
                        write_final_result(
                            writers,
                            FinalTriage::Dead,
                            line.trim(),
                            "SyntaxInvalid",
                            None,
                        )?;
                    }
                    write_detail_row(writers, line.trim(), FinalTriage::Dead, "SyntaxInvalid", None)?;
                    continue;
                }
            };

            if !seen_emails.insert(parsed_email.canonical_email_key.clone()) {
                stats.duplicates += 1;
                continue;
            }

            let dns_status = if matches!(processing_mode, ProcessingMode::VerifyDns) {
                domain_statuses
                    .get(&parsed_email.normalized_domain)
                    .cloned()
                    .unwrap_or(MxStatus::Inconclusive)
            } else {
                MxStatus::HasMx
            };

            let group = group_for_email(
                &dns_status,
                &parsed_email.normalized_domain,
                public_domains,
                edu_patterns,
                target_domains,
                processing_mode,
            );

            match group {
                EmailGroup::Public => {
                    stats.public += 1;
                    write_line(
                        &mut writers.public,
                        &parsed_email.normalized_email,
                        &writers.public_name,
                    )?;
                }
                EmailGroup::Edu => {
                    stats.edu += 1;
                    write_line(
                        &mut writers.edu,
                        &parsed_email.normalized_email,
                        &writers.edu_name,
                    )?;
                }
                EmailGroup::Targeted => {
                    stats.targeted += 1;
                    write_line(
                        &mut writers.targeted,
                        &parsed_email.normalized_email,
                        &writers.targeted_name,
                    )?;
                }
                EmailGroup::Custom => {
                    stats.custom += 1;
                    write_line(
                        &mut writers.custom,
                        &parsed_email.normalized_email,
                        &writers.custom_name,
                    )?;
                }
                EmailGroup::MxDead => {
                    stats.mx_dead += 1;
                    stats.final_dead += 1;
                    write_line(
                        &mut writers.mx_dead,
                        &parsed_email.normalized_email,
                        &writers.mx_dead_name,
                    )?;
                    write_final_result(
                        writers,
                        FinalTriage::Dead,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Dead,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
                EmailGroup::MxHasMx => {
                    stats.mx_has_mx += 1;
                    write_line(
                        &mut writers.mx_has_mx,
                        &parsed_email.normalized_email,
                        &writers.mx_has_mx_name,
                    )?;
                    if smtp_enabled {
                        append_smtp_spool_line(
                            smtp_spool_writer.as_mut(),
                            &parsed_email.normalized_email,
                        )?;
                    } else {
                        stats.final_unknown += 1;
                        write_final_result(
                            writers,
                            FinalTriage::Unknown,
                            &parsed_email.normalized_email,
                            &dns_status_name(&dns_status),
                            None,
                        )?;
                        write_detail_row(
                            writers,
                            &parsed_email.normalized_email,
                            FinalTriage::Unknown,
                            &dns_status_name(&dns_status),
                            None,
                        )?;
                    }
                }
                EmailGroup::MxARecordFallback => {
                    stats.mx_a_fallback += 1;
                    stats.final_unknown += 1;
                    write_line(
                        &mut writers.mx_a_fallback,
                        &parsed_email.normalized_email,
                        &writers.mx_a_fallback_name,
                    )?;
                    write_final_result(
                        writers,
                        FinalTriage::Unknown,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Unknown,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
                EmailGroup::MxInconclusive => {
                    stats.mx_inconclusive += 1;
                    stats.final_unknown += 1;
                    write_line(
                        &mut writers.mx_inconclusive,
                        &parsed_email.normalized_email,
                        &writers.mx_inconclusive_name,
                    )?;
                    write_final_result(
                        writers,
                        FinalTriage::Unknown,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Unknown,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
                EmailGroup::MxParked => {
                    stats.mx_parked += 1;
                    stats.final_unknown += 1;
                    write_line(
                        &mut writers.mx_parked,
                        &parsed_email.normalized_email,
                        &writers.mx_parked_name,
                    )?;
                    write_final_result(
                        writers,
                        FinalTriage::Unknown,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Unknown,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
                EmailGroup::MxDisposable => {
                    stats.mx_disposable += 1;
                    stats.final_unknown += 1;
                    write_line(
                        &mut writers.mx_disposable,
                        &parsed_email.normalized_email,
                        &writers.mx_disposable_name,
                    )?;
                    write_final_result(
                        writers,
                        FinalTriage::Unknown,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Unknown,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
                EmailGroup::MxTypo => {
                    stats.mx_typo += 1;
                    stats.final_unknown += 1;
                    let typo_value = match &dns_status {
                        MxStatus::TypoSuggestion(suggestion) => {
                            format!("{} -> {suggestion}", parsed_email.normalized_email)
                        }
                        _ => parsed_email.normalized_email.clone(),
                    };
                    write_line(&mut writers.mx_typo, &typo_value, &writers.mx_typo_name)?;
                    write_final_result(
                        writers,
                        FinalTriage::Unknown,
                        &parsed_email.normalized_email,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                    write_detail_row(
                        writers,
                        &parsed_email.normalized_email,
                        FinalTriage::Unknown,
                        &dns_status_name(&dns_status),
                        None,
                    )?;
                }
            }

            if processed_lines.is_multiple_of(EMIT_EVERY) {
                let verify_dns = matches!(processing_mode, ProcessingMode::VerifyDns);
                let current_pct = scale_second_pass_progress(
                    total_bytes,
                    bytes_read,
                    verify_dns,
                    smtp_enabled,
                ) as i64;
                if current_pct != last_emitted_pct {
                    last_emitted_pct = current_pct;
                    let payload = build_processing_payload(
                        output_dir,
                        processed_lines,
                        scale_second_pass_progress(
                            total_bytes,
                            bytes_read,
                            verify_dns,
                            smtp_enabled,
                        ),
                        stats,
                        smtp_enabled,
                        0,
                        started_at.elapsed().as_millis(),
                        None,
                        None,
                    );
                    emit_progress_event(payload, "processing-progress").ok();
                }
            }
        }
    }

    if let Some(spool_writer) = smtp_spool_writer.as_mut() {
        flush_writer(
            spool_writer,
            "Failed to flush SMTP spool file.",
            "Không thể lưu tệp SMTP spool.",
        )?;
    }

    Ok(SecondPassResult {
        processed_lines,
        smtp_spool_path,
    })
}

#[allow(clippy::too_many_arguments)]
async fn process_smtp_spool<F>(
    spool_path: &Path,
    persistent_cache: Option<&PersistentCache>,
    smtp_client: Option<&SmtpApiClient>,
    writers: &mut Writers,
    stats: &mut Stats,
    processed_lines: u64,
    output_dir: &str,
    started_at: Instant,
    emit_progress_event: &mut F,
) -> Result<(), ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    if !spool_path.exists() {
        return Ok(());
    }

    let total_targets = count_non_empty_lines(spool_path)?;
    if total_targets == 0 {
        return Ok(());
    }

    let input = File::open(spool_path).map_err(|error| {
        error_payload_from_io(
            "Failed to open SMTP spool file.",
            "Không thể mở tệp SMTP spool.",
            error,
        )
    })?;
    let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, input);
    let mut line = String::with_capacity(256);
    let mut batch = Vec::with_capacity(SMTP_BATCH_SIZE);
    let mut processed_targets = 0usize;

    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|error| {
            error_payload_from_io(
                "Failed to read SMTP spool file.",
                "Không thể đọc tệp SMTP spool.",
                error,
            )
        })?;
        if read == 0 {
            break;
        }

        let email = line.trim();
        if !email.is_empty() {
            batch.push(email.to_string());
        }

        if batch.len() >= SMTP_BATCH_SIZE {
            process_smtp_batch(
                &batch,
                persistent_cache,
                smtp_client,
                writers,
                stats,
                &mut processed_targets,
                total_targets,
                processed_lines,
                output_dir,
                started_at,
                emit_progress_event,
            )
            .await?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        process_smtp_batch(
            &batch,
            persistent_cache,
            smtp_client,
            writers,
            stats,
            &mut processed_targets,
            total_targets,
            processed_lines,
            output_dir,
            started_at,
            emit_progress_event,
        )
        .await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_smtp_batch<F>(
    emails: &[String],
    persistent_cache: Option<&PersistentCache>,
    smtp_client: Option<&SmtpApiClient>,
    writers: &mut Writers,
    stats: &mut Stats,
    processed_targets: &mut usize,
    total_targets: usize,
    processed_lines: u64,
    output_dir: &str,
    started_at: Instant,
    emit_progress_event: &mut F,
) -> Result<(), ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let parsed_emails: Vec<ParsedEmail> = emails
        .iter()
        .filter_map(|email| parse_email_candidate(email))
        .collect();
    if parsed_emails.is_empty() {
        return Ok(());
    }

    stats.smtp_attempted_emails += parsed_emails.len() as u64;

    let cached_records = if let Some(cache) = persistent_cache {
        cache.load_smtp_many(&parsed_emails)?
    } else {
        HashMap::new()
    };

    stats.smtp_cache_hits += cached_records.values().filter(|record| record.cached).count() as u64;

    let live_targets: Vec<SmtpVerifyTarget> = parsed_emails
        .iter()
        .filter(|parsed| !cached_records.contains_key(&parsed.normalized_email))
        .map(|parsed| SmtpVerifyTarget {
            email: parsed.normalized_email.clone(),
            normalized_domain: parsed.normalized_domain.clone(),
        })
        .collect();

    let live_records = if let Some(client) = smtp_client {
        client.verify_batch(&live_targets).await
    } else {
        HashMap::new()
    };

    if let Some(cache) = persistent_cache {
        let to_store: Vec<SmtpProbeRecord> = live_records
            .values()
            .filter(|record| should_persist_smtp_record(record))
            .cloned()
            .collect();
        cache.store_smtp_many(&to_store)?;
    }

    for parsed in parsed_emails {
        let local_cache_hit = cached_records.contains_key(&parsed.normalized_email);
        let mut record = cached_records
            .get(&parsed.normalized_email)
            .cloned()
            .or_else(|| live_records.get(&parsed.normalized_email).cloned())
            .unwrap_or_else(|| SmtpProbeRecord {
                email: parsed.normalized_email.clone(),
                outcome: SmtpStatus::Inconclusive,
                ..Default::default()
            });
        record.email = parsed.normalized_email.clone();
        if record.cached && !local_cache_hit {
            stats.smtp_cache_hits += 1;
        }

        apply_smtp_record(stats, &record);
        write_smtp_legacy_output(writers, &parsed.normalized_email, &record)?;

        let final_triage = final_triage_for(&MxStatus::HasMx, Some(&record));
        write_final_result(
            writers,
            final_triage,
            &parsed.normalized_email,
            &dns_status_name(&MxStatus::HasMx),
            Some(&record),
        )?;
        write_detail_row(
            writers,
            &parsed.normalized_email,
            final_triage,
            &dns_status_name(&MxStatus::HasMx),
            Some(&record),
        )?;

        *processed_targets += 1;
        if (*processed_targets).is_multiple_of(EMIT_EVERY as usize)
            || *processed_targets == total_targets
        {
            let payload = build_processing_payload(
                output_dir,
                processed_lines,
                smtp_phase_progress(*processed_targets, total_targets),
                stats,
                true,
                started_at.elapsed().as_millis() as u64,
                started_at.elapsed().as_millis(),
                None,
                Some(parsed.normalized_email.clone()),
            );
            emit_progress_event(payload, "processing-progress").ok();
        }
    }

    Ok(())
}

fn scale_progress(total_bytes: u64, bytes_read: u64, max_progress: f64) -> f64 {
    if total_bytes == 0 {
        max_progress
    } else {
        ((bytes_read as f64 / total_bytes as f64) * max_progress).clamp(0.0, max_progress)
    }
}

fn scale_second_pass_progress(
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

fn smtp_phase_progress(processed_targets: usize, total_targets: usize) -> f64 {
    if total_targets == 0 {
        100.0
    } else {
        (88.0 + ((processed_targets as f64 / total_targets as f64) * 12.0)).clamp(88.0, 100.0)
    }
}

fn group_for_email(
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

fn build_writers(
    output_path: &Path,
    verify_mode: bool,
    smtp_enabled: bool,
) -> Result<Writers, std::io::Error> {
    let invalid_name = "05_T1_Invalid_Syntax.txt".to_string();
    let public_name = "01_T1_Valid_Public.txt".to_string();
    let edu_name = "02_T1_Valid_EduGov.txt".to_string();
    let targeted_name = "03_T1_Valid_Targeted.txt".to_string();
    let custom_name = "04_T1_Valid_Other.txt".to_string();
    let mx_dead_name = "12_T2_DNS_Error_Dead.txt".to_string();
    let mx_has_mx_name = "10_T2_DNS_Valid_Has_MX.txt".to_string();
    let mx_a_fallback_name = "11_T2_DNS_Valid_ARecord.txt".to_string();
    let mx_inconclusive_name = "16_T2_DNS_Inconclusive.txt".to_string();
    let mx_parked_name = "13_T2_DNS_Risk_Parked.txt".to_string();
    let mx_disposable_name = "14_T2_DNS_Risk_Disposable.txt".to_string();
    let mx_typo_name = "15_T2_DNS_Typo_Suggestion.txt".to_string();
    let smtp_deliverable_name = "20_T3_SMTP_Deliverable.txt".to_string();
    let smtp_rejected_name = "22_T3_SMTP_Rejected.txt".to_string();
    let smtp_catchall_name = "21_T3_SMTP_CatchAll.txt".to_string();
    let smtp_unknown_name = "23_T3_SMTP_Unknown.txt".to_string();
    let final_alive_name = "30_T4_FINAL_Alive.txt".to_string();
    let final_dead_name = "31_T4_FINAL_Dead.txt".to_string();
    let final_unknown_name = "32_T4_FINAL_Unknown.txt".to_string();
    let detail_csv_name = "33_T4_FINAL_Detail.csv".to_string();
    let mut detail_csv = if verify_mode {
        Some(BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&detail_csv_name))?,
        ))
    } else {
        None
    };
    if let Some(writer) = detail_csv.as_mut() {
        writer.write_all(
            b"email,final_status,dns_status,smtp_outcome,smtp_basic_code,smtp_enhanced_code,smtp_reply_text,mx_host,catch_all,smtp_cached,tested_at\n",
        )?;
    }

    Ok(Writers {
        invalid: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&invalid_name))?,
        ),
        public: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&public_name))?,
        ),
        edu: BufWriter::with_capacity(BUFFER_CAPACITY, File::create(output_path.join(&edu_name))?),
        targeted: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&targeted_name))?,
        ),
        custom: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&custom_name))?,
        ),
        mx_dead: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_dead_name))?,
        ),
        mx_has_mx: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_has_mx_name))?,
        ),
        mx_a_fallback: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_a_fallback_name))?,
        ),
        mx_inconclusive: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_inconclusive_name))?,
        ),
        mx_parked: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_parked_name))?,
        ),
        mx_disposable: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_disposable_name))?,
        ),
        mx_typo: BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(output_path.join(&mx_typo_name))?,
        ),
        smtp_deliverable: if smtp_enabled {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_deliverable_name))?,
            ))
        } else {
            None
        },
        smtp_rejected: if smtp_enabled {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_rejected_name))?,
            ))
        } else {
            None
        },
        smtp_catchall: if smtp_enabled {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_catchall_name))?,
            ))
        } else {
            None
        },
        smtp_unknown: if smtp_enabled {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_unknown_name))?,
            ))
        } else {
            None
        },
        final_alive: if verify_mode {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&final_alive_name))?,
            ))
        } else {
            None
        },
        final_dead: if verify_mode {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&final_dead_name))?,
            ))
        } else {
            None
        },
        final_unknown: if verify_mode {
            Some(BufWriter::with_capacity(
                BUFFER_CAPACITY,
                File::create(output_path.join(&final_unknown_name))?,
            ))
        } else {
            None
        },
        detail_csv,
        invalid_name,
        public_name,
        edu_name,
        targeted_name,
        custom_name,
        mx_dead_name,
        mx_has_mx_name,
        mx_a_fallback_name,
        mx_inconclusive_name,
        mx_parked_name,
        mx_disposable_name,
        mx_typo_name,
        smtp_deliverable_name,
        smtp_rejected_name,
        smtp_catchall_name,
        smtp_unknown_name,
        final_alive_name,
        final_dead_name,
        final_unknown_name,
        detail_csv_name,
    })
}

fn flush_writers(writers: &mut Writers) -> Result<(), ErrorPayload> {
    flush_writer(
        &mut writers.invalid,
        "Failed to flush invalid email results to disk.",
        "Không thể ghi hoàn tất kết quả email không hợp lệ xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.public,
        "Failed to flush public email results to disk.",
        "Không thể ghi hoàn tất kết quả email công cộng xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.edu,
        "Failed to flush edu email results to disk.",
        "Không thể ghi hoàn tất kết quả email giáo dục xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.targeted,
        "Failed to flush targeted email results to disk.",
        "Không thể ghi hoàn tất kết quả email chọn lọc.",
    )?;
    flush_writer(
        &mut writers.custom,
        "Failed to flush custom email results to disk.",
        "Không thể ghi hoàn tất kết quả email doanh nghiệp.",
    )?;
    flush_writer(
        &mut writers.mx_dead,
        "Failed to flush dead email results to disk.",
        "Không thể ghi tệp mail chết.",
    )?;
    flush_writer(
        &mut writers.mx_has_mx,
        "Failed to flush valid MX email results to disk.",
        "Không thể ghi tệp mail có MX hợp lệ.",
    )?;
    flush_writer(
        &mut writers.mx_a_fallback,
        "Failed to flush A-record fallback email results to disk.",
        "Không thể ghi tệp mail dùng A record fallback.",
    )?;
    flush_writer(
        &mut writers.mx_inconclusive,
        "Failed to flush inconclusive email results to disk.",
        "Không thể ghi tệp mail cần kiểm tra thủ công.",
    )?;
    flush_writer(
        &mut writers.mx_parked,
        "Failed to flush parked email results to disk.",
        "Không thể ghi tệp mail trỏ tới parked domain.",
    )?;
    flush_writer(
        &mut writers.mx_disposable,
        "Failed to flush disposable email results to disk.",
        "Không thể ghi tệp mail disposable.",
    )?;
    flush_writer(
        &mut writers.mx_typo,
        "Failed to flush typo suggestion results to disk.",
        "Không thể ghi tệp gợi ý sửa lỗi chính tả domain.",
    )?;
    flush_optional_writer(
        writers.smtp_deliverable.as_mut(),
        "Failed to flush SMTP deliverable results to disk.",
        "Không thể ghi tệp SMTP deliverable.",
    )?;
    flush_optional_writer(
        writers.smtp_rejected.as_mut(),
        "Failed to flush SMTP rejected results to disk.",
        "Không thể ghi tệp SMTP rejected.",
    )?;
    flush_optional_writer(
        writers.smtp_catchall.as_mut(),
        "Failed to flush SMTP catch-all results to disk.",
        "Không thể ghi tệp SMTP catch-all.",
    )?;
    flush_optional_writer(
        writers.smtp_unknown.as_mut(),
        "Failed to flush SMTP unknown results to disk.",
        "Không thể ghi tệp SMTP unknown.",
    )?;
    flush_optional_writer(
        writers.final_alive.as_mut(),
        "Failed to flush final alive results to disk.",
        "Không thể ghi tệp kết quả Alive.",
    )?;
    flush_optional_writer(
        writers.final_dead.as_mut(),
        "Failed to flush final dead results to disk.",
        "Không thể ghi tệp kết quả Dead.",
    )?;
    flush_optional_writer(
        writers.final_unknown.as_mut(),
        "Failed to flush final unknown results to disk.",
        "Không thể ghi tệp kết quả Unknown.",
    )?;
    flush_optional_writer(
        writers.detail_csv.as_mut(),
        "Failed to flush detail CSV results to disk.",
        "Không thể ghi tệp CSV chi tiết.",
    )
}

fn build_run_output_dir(
    base_output_path: &Path,
    paths: &[String],
) -> Result<std::path::PathBuf, ErrorPayload> {
    let source_stem = if paths.len() == 1 {
        Path::new(&paths[0])
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(sanitize_path_segment)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "emails".to_string())
    } else {
        format!("batch_process_{}_files", paths.len())
    };

    let session_label = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    Ok(base_output_path.join(format!("{source_stem}__{session_label}")))
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn build_edu_patterns() -> Result<Vec<Regex>, ErrorPayload> {
    Ok(vec![
        Regex::new(r"\.edu$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.gov$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.k12\.[a-z]{2}\.us$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.edu\.[a-z]{2}$").map_err(map_regex_error_payload)?,
        Regex::new(r"\.org$").map_err(map_regex_error_payload)?,
    ])
}

fn classify_email(
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

#[allow(clippy::too_many_arguments)]
fn build_processing_payload(
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

fn append_smtp_spool_line(
    writer: Option<&mut BufWriter<File>>,
    email: &str,
) -> Result<(), ErrorPayload> {
    if let Some(writer) = writer {
        write_line(writer, email, ".smtp_spool.txt")?;
    }
    Ok(())
}

fn count_non_empty_lines(path: &Path) -> Result<usize, ErrorPayload> {
    let input = File::open(path).map_err(|error| {
        error_payload_from_io(
            "Failed to count SMTP spool lines.",
            "Không thể đếm số dòng trong SMTP spool.",
            error,
        )
    })?;
    let reader = BufReader::with_capacity(BUFFER_CAPACITY, input);
    let mut count = 0usize;
    for line in reader.lines() {
        let line = line.map_err(|error| {
            error_payload_from_io(
                "Failed to read SMTP spool lines.",
                "Không thể đọc các dòng trong SMTP spool.",
                error,
            )
        })?;
        if !line.trim().is_empty() {
            count += 1;
        }
    }
    Ok(count)
}

fn apply_smtp_record(stats: &mut Stats, record: &SmtpProbeRecord) {
    match record.outcome {
        SmtpStatus::Accepted | SmtpStatus::AcceptedForwarded => {
            stats.smtp_deliverable += 1;
            stats.final_alive += 1;
        }
        SmtpStatus::CatchAll => {
            stats.smtp_catchall += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::BadMailbox => {
            stats.smtp_rejected += 1;
            stats.smtp_bad_mailbox += 1;
            stats.final_dead += 1;
        }
        SmtpStatus::BadDomain => {
            stats.smtp_rejected += 1;
            stats.smtp_bad_domain += 1;
            stats.final_dead += 1;
        }
        SmtpStatus::PolicyBlocked => {
            stats.smtp_unknown += 1;
            stats.smtp_policy_blocked += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::MailboxFull => {
            stats.smtp_unknown += 1;
            stats.smtp_mailbox_full += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::MailboxDisabled => {
            stats.smtp_unknown += 1;
            stats.smtp_mailbox_disabled += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::TempFailure => {
            stats.smtp_unknown += 1;
            stats.smtp_temp_failure += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::NetworkError => {
            stats.smtp_unknown += 1;
            stats.smtp_network_error += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::ProtocolError => {
            stats.smtp_unknown += 1;
            stats.smtp_protocol_error += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::Timeout => {
            stats.smtp_unknown += 1;
            stats.smtp_timeout += 1;
            stats.final_unknown += 1;
        }
        SmtpStatus::Inconclusive => {
            stats.smtp_unknown += 1;
            stats.final_unknown += 1;
        }
    }
}

fn should_persist_smtp_record(record: &SmtpProbeRecord) -> bool {
    if record.outcome != SmtpStatus::Inconclusive {
        return true;
    }

    record.smtp_basic_code.is_some()
        || record.smtp_enhanced_code.is_some()
        || record.smtp_reply_text.is_some()
        || record.mx_host.is_some()
        || record.catch_all
        || record.duration_ms > 0
}

fn write_smtp_legacy_output(
    writers: &mut Writers,
    email: &str,
    record: &SmtpProbeRecord,
) -> Result<(), ErrorPayload> {
    match output_bucket_for(&MxStatus::HasMx, Some(record)) {
        OutputBucket::SmtpDeliverable => write_optional_line(
            writers.smtp_deliverable.as_mut(),
            email,
            &writers.smtp_deliverable_name,
        )?,
        OutputBucket::SmtpRejected => write_optional_line(
            writers.smtp_rejected.as_mut(),
            email,
            &writers.smtp_rejected_name,
        )?,
        OutputBucket::SmtpCatchAll => write_optional_line(
            writers.smtp_catchall.as_mut(),
            email,
            &writers.smtp_catchall_name,
        )?,
        OutputBucket::HasMxSmtpUnknown => write_optional_line(
            writers.smtp_unknown.as_mut(),
            email,
            &writers.smtp_unknown_name,
        )?,
        _ => {}
    }
    Ok(())
}

fn write_final_result(
    writers: &mut Writers,
    triage: FinalTriage,
    email: &str,
    _dns_status: &str,
    _record: Option<&SmtpProbeRecord>,
) -> Result<(), ErrorPayload> {
    match triage {
        FinalTriage::Alive => write_optional_line(
            writers.final_alive.as_mut(),
            email,
            &writers.final_alive_name,
        )?,
        FinalTriage::Dead => write_optional_line(
            writers.final_dead.as_mut(),
            email,
            &writers.final_dead_name,
        )?,
        FinalTriage::Unknown => write_optional_line(
            writers.final_unknown.as_mut(),
            email,
            &writers.final_unknown_name,
        )?,
    }
    Ok(())
}

fn write_detail_row(
    writers: &mut Writers,
    email: &str,
    triage: FinalTriage,
    dns_status: &str,
    record: Option<&SmtpProbeRecord>,
) -> Result<(), ErrorPayload> {
    let Some(writer) = writers.detail_csv.as_mut() else {
        return Ok(());
    };
    let row = [
        csv_escape(email),
        csv_escape(triage.as_str()),
        csv_escape(dns_status),
        csv_escape(record.map(|value| value.outcome.as_str()).unwrap_or("")),
        csv_escape(
            &record
                .and_then(|value| value.smtp_basic_code)
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        csv_escape(
            record
                .and_then(|value| value.smtp_enhanced_code.as_deref())
                .unwrap_or(""),
        ),
        csv_escape(record.and_then(|value| value.smtp_reply_text.as_deref()).unwrap_or("")),
        csv_escape(record.and_then(|value| value.mx_host.as_deref()).unwrap_or("")),
        csv_escape(
            &record
                .map(|value| value.catch_all.to_string())
                .unwrap_or_default(),
        ),
        csv_escape(
            &record
                .map(|value| value.cached.to_string())
                .unwrap_or_default(),
        ),
        csv_escape(&Utc::now().to_rfc3339()),
    ]
    .join(",");

    write_line(writer, &row, &writers.detail_csv_name)
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn write_line(
    writer: &mut BufWriter<File>,
    value: &str,
    file_name: &str,
) -> Result<(), ErrorPayload> {
    writer.write_all(value.as_bytes()).map_err(|error| {
        backend_error(
            "Failed to write to file.",
            "Lỗi ghi tệp.",
            Some(format!("{file_name}: {error}")),
        )
    })?;
    writer.write_all(b"\n").map_err(|error| {
        backend_error(
            "Failed to write newline.",
            "Lỗi ghi xuống dòng.",
            Some(format!("{file_name}: {error}")),
        )
    })
}

fn write_optional_line(
    writer: Option<&mut BufWriter<File>>,
    value: &str,
    file_name: &str,
) -> Result<(), ErrorPayload> {
    if let Some(writer) = writer {
        write_line(writer, value, file_name)?;
    }
    Ok(())
}

fn flush_writer(
    writer: &mut BufWriter<File>,
    message_en: &str,
    message_vi: &str,
) -> Result<(), ErrorPayload> {
    writer
        .flush()
        .map_err(|error| error_payload_from_io(message_en, message_vi, error))
}

fn flush_optional_writer(
    writer: Option<&mut BufWriter<File>>,
    message_en: &str,
    message_vi: &str,
) -> Result<(), ErrorPayload> {
    if let Some(writer) = writer {
        flush_writer(writer, message_en, message_vi)?;
    }
    Ok(())
}

fn normalize_domain(raw: &str) -> Result<String, String> {
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

fn is_parked_mx(mx_host: &str) -> bool {
    let host = mx_host.trim_end_matches('.').to_lowercase();
    PARKING_MX_SUFFIXES
        .iter()
        .any(|suffix| host.ends_with(suffix))
}

fn is_parked_domain(domain: &str) -> bool {
    let host = domain.trim_end_matches('.').to_lowercase();
    PARKED_DOMAIN_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

fn check_typo(domain: &str) -> Option<String> {
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

fn is_disposable_domain(domain: &str) -> bool {
    disposable_domains().contains(domain)
}

fn backend_error(message_en: &str, message_vi: &str, detail: Option<String>) -> ErrorPayload {
    ErrorPayload {
        message_en: attach_detail(message_en, detail.clone()),
        message_vi: attach_detail_vi(message_vi, detail),
    }
}

fn error_payload_from_io(
    message_en: &str,
    message_vi: &str,
    error: std::io::Error,
) -> ErrorPayload {
    backend_error(message_en, message_vi, Some(error.to_string()))
}

fn attach_detail(message: &str, detail: Option<String>) -> String {
    match detail {
        Some(detail) if !detail.is_empty() => format!("{message} Details: {detail}"),
        _ => message.to_string(),
    }
}

fn attach_detail_vi(message: &str, detail: Option<String>) -> String {
    match detail {
        Some(detail) if !detail.is_empty() => format!("{message} Chi tiết: {detail}"),
        _ => message.to_string(),
    }
}

fn map_regex_error_payload(error: regex::Error) -> ErrorPayload {
    backend_error(
        "Regex error.",
        "Lỗi biểu thức chính quy.",
        Some(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalize_domain_handles_idn_and_trailing_dot() {
        assert_eq!(
            normalize_domain(" MÜNCHEN.DE. ").unwrap(),
            "xn--mnchen-3ya.de"
        );
    }

    #[test]
    fn typo_and_parked_checks_work() {
        assert_eq!(check_typo("gmial.com"), Some("gmail.com".to_string()));
        assert!(is_parked_mx("mx1.registrar-servers.com."));
        assert!(is_parked_domain("hugedomains.com"));
        assert!(is_parked_domain("shop.hugedomains.com"));
    }

    #[test]
    fn extract_email_falls_back_for_unicode_domains() {
        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let extracted =
            extract_email_candidate_from_line("User.Name@münchen.de", &extractor_regex).unwrap();
        let parsed = parse_email_candidate(&extracted).unwrap();
        assert_eq!(parsed.raw_local_part, "User.Name");
        assert_eq!(parsed.normalized_domain, "xn--mnchen-3ya.de");
        assert_eq!(parsed.normalized_email, "User.Name@xn--mnchen-3ya.de");
    }

    #[test]
    fn disposable_domains_are_embedded() {
        assert!(is_disposable_domain("mailinator.com"));
    }

    #[test]
    fn persistent_cache_round_trip_restores_statuses() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-cache-{}",
            Local::now().timestamp_nanos_opt().unwrap_or_default()
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
            Local::now().timestamp_nanos_opt().unwrap_or_default()
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
                parse_email_candidate("User.Name@gmail.com").unwrap(),
                parse_email_candidate("user.name@gmail.com").unwrap(),
                parse_email_candidate("other@gmail.com").unwrap(),
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

        assert!(!should_persist_smtp_record(&fallback));
        assert!(should_persist_smtp_record(&real_timeout));
    }

    #[tokio::test]
    async fn typo_is_prioritized_before_disposable_when_both_match() {
        let resolver = build_resolver(1_500);
        let cache = Arc::new(DomainCache::default());
        let semaphore = Arc::new(Semaphore::new(1));
        let status =
            check_domain_mx_async("gmial.com".to_string(), resolver, cache, semaphore).await;

        assert_eq!(status, MxStatus::TypoSuggestion("gmail.com".to_string()));
    }

    #[test]
    fn verify_second_pass_writes_t2_and_t4_outputs() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-verify-pass-{}",
            Local::now().timestamp_nanos_opt().unwrap_or_default()
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
            Local::now().timestamp_nanos_opt().unwrap_or_default()
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
        assert!(domain_set.contains("gmail.com"));
        assert!(domain_set.contains("outlook.com"));
    }
}
