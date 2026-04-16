use crate::smtp_client::SmtpApiClient;
use crate::smtp_status::SmtpStatus;
use crate::smtp_verify::{DomainVerifyResult, OutputBucket};
use chrono::Local;
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
const EMIT_EVERY: u64 = 500;
const DOMAIN_SCAN_BATCH_SIZE: usize = 1_000;
const FIRST_PASS_PROGRESS_END: f64 = 35.0;
const DOMAIN_SCAN_PROGRESS_END: f64 = 65.0;
const CACHE_TTL_SECS: i64 = 6 * 3600;
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
    pub elapsed_ms: u128,
    pub output_dir: Option<String>,
    pub current_domain: Option<String>,
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
}

struct CollectedDomains {
    unique_domains: Vec<String>,
    sample_emails: HashMap<String, String>,
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
            CREATE INDEX IF NOT EXISTS idx_mx_cache_cached_at ON mx_cache(cached_at);",
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
        "parked" => Some(MxStatus::Parked),
        "disposable" => Some(MxStatus::Disposable),
        "inconclusive" => Some(MxStatus::Inconclusive),
        _ => value
            .strip_prefix("typo:")
            .map(|suggestion| MxStatus::TypoSuggestion(suggestion.to_string())),
    }
}

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

    let (cache_hits, domain_results, smtp_elapsed_ms) = if check_mx {
        let collected_domains = collect_unique_domains(
            &file_paths,
            &extractor_regex,
            total_bytes,
            &output_dir,
            started_at,
            smtp_phase_enabled,
            &mut emit_progress_event,
        )?;

        let persistent_cache = if use_persistent_cache {
            persistent_cache_path
                .map(PersistentCache::new)
                .transpose()?
        } else {
            None
        };

        let mut domain_statuses = if let Some(cache) = &persistent_cache {
            cache.load_many(&collected_domains.unique_domains)?
        } else {
            HashMap::new()
        };
        let cache_hits = domain_statuses.len() as u64;

        let domains_to_scan: Vec<String> = collected_domains
            .unique_domains
            .into_iter()
            .filter(|domain| !domain_statuses.contains_key(domain))
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

        domain_statuses.extend(freshly_scanned);
        let smtp_started_at = Instant::now();
        let domain_results = build_domain_verify_results(
            domain_statuses,
            &collected_domains.sample_emails,
            smtp_phase_enabled,
            smtp_client.as_ref(),
        )
        .await;
        let smtp_elapsed_ms = if smtp_phase_enabled {
            smtp_started_at.elapsed().as_millis() as u64
        } else {
            0
        };

        (cache_hits, domain_results, smtp_elapsed_ms)
    } else {
        (0, HashMap::new(), 0)
    };

    let mut writers = build_writers(&run_output_path, smtp_phase_enabled).map_err(|error| {
        error_payload_from_io(
            "Failed to create one or more result files.",
            "Không thể tạo một hoặc nhiều tệp kết quả.",
            error,
        )
    })?;

    let payload = process_files_with_domain_results(
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
        cache_hits,
        &domain_results,
        smtp_phase_enabled,
        smtp_elapsed_ms,
        &output_dir,
        started_at,
        &mut writers,
        &mut emit_progress_event,
    )?;

    flush_writers(&mut writers)?;

    Ok(payload)
}

fn total_bytes(file_paths: &[String]) -> u64 {
    file_paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|meta| meta.len())
        .sum()
}

fn extract_email_from_line(line: &str, extractor_regex: &Regex) -> Option<String> {
    if let Some(matched) = extractor_regex.find(line) {
        return Some(matched.as_str().trim().to_lowercase());
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
            Some(candidate.to_lowercase())
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
    let mut sample_emails = HashMap::new();

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

            if let Some(email_match) = extract_email_from_line(&line, extractor_regex) {
                if let Some((_, domain)) = email_match.rsplit_once('@') {
                    if let Ok(normalized_domain) = normalize_domain(domain) {
                        domains.insert(normalized_domain.clone());
                        sample_emails
                            .entry(normalized_domain)
                            .or_insert(email_match);
                    }
                }
            }

            if processed_lines % EMIT_EVERY == 0 {
                let payload = build_processing_payload(
                    output_dir,
                    processed_lines,
                    scale_progress(total_bytes, bytes_read, FIRST_PASS_PROGRESS_END),
                    &Stats::default(),
                    smtp_enabled,
                    0,
                    started_at.elapsed().as_millis(),
                    None,
                );
                emit_progress_event(payload, "processing-progress").ok();
            }
        }
    }

    Ok(CollectedDomains {
        unique_domains: domains.into_iter().collect(),
        sample_emails,
    })
}

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

            if processed_domains % EMIT_EVERY as usize == 0 || processed_domains == total_domains {
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
                );
                emit_progress_event(payload, "processing-progress").ok();
            }
        }
    }

    Ok(results)
}

async fn build_domain_verify_results(
    domain_statuses: HashMap<String, MxStatus>,
    sample_emails: &HashMap<String, String>,
    smtp_enabled: bool,
    smtp_client: Option<&SmtpApiClient>,
) -> HashMap<String, DomainVerifyResult> {
    if !smtp_enabled {
        return domain_statuses
            .into_iter()
            .map(|(domain, dns)| (domain, DomainVerifyResult { dns, smtp: None }))
            .collect();
    }

    let smtp_targets: Vec<(String, String)> = domain_statuses
        .iter()
        .filter_map(|(domain, dns)| match dns {
            MxStatus::HasMx => Some((
                domain.clone(),
                sample_emails
                    .get(domain)
                    .cloned()
                    .unwrap_or_else(|| format!("postmaster@{domain}")),
            )),
            _ => None,
        })
        .collect();

    let smtp_statuses = match smtp_client {
        Some(client) if !smtp_targets.is_empty() => client.verify_batch(&smtp_targets).await,
        _ => smtp_targets
            .iter()
            .map(|(domain, _)| (domain.clone(), SmtpStatus::Inconclusive))
            .collect(),
    };

    domain_statuses
        .into_iter()
        .map(|(domain, dns)| {
            let smtp = if matches!(dns, MxStatus::HasMx) {
                Some(
                    smtp_statuses
                        .get(&domain)
                        .cloned()
                        .unwrap_or(SmtpStatus::Inconclusive),
                )
            } else {
                None
            };
            (domain, DomainVerifyResult { dns, smtp })
        })
        .collect()
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
                if lookup.iter().next().is_none() {
                    final_status = check_a_record_fallback(&resolver, &domain).await;
                } else {
                    let all_parked = lookup
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

fn process_files_with_domain_results<F>(
    file_paths: &[String],
    total_bytes: u64,
    extractor_regex: &Regex,
    public_domains: &HashSet<&'static str>,
    edu_patterns: &[Regex],
    target_domains: &HashSet<String>,
    processing_mode: ProcessingMode,
    cache_hits: u64,
    domain_results: &HashMap<String, DomainVerifyResult>,
    smtp_enabled: bool,
    smtp_elapsed_ms: u64,
    output_dir: &str,
    started_at: Instant,
    writers: &mut Writers,
    emit_progress_event: &mut F,
) -> Result<ProcessingPayload, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let mut line = String::with_capacity(1024);
    let mut bytes_read = 0u64;
    let mut processed_lines = 0u64;
    let mut stats = Stats::default();
    stats.cache_hits = cache_hits;
    let mut last_emitted_pct: i64 = -1;
    let mut seen_emails: HashSet<String> = HashSet::with_capacity(100_000);

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

            let extracted_email = match extract_email_from_line(&line, extractor_regex) {
                Some(matched) => matched,
                None => {
                    stats.invalid += 1;
                    write_line(&mut writers.invalid, line.trim(), &writers.invalid_name)?;
                    continue;
                }
            };

            if !seen_emails.insert(extracted_email.clone()) {
                stats.duplicates += 1;
                continue;
            }

            let (_, raw_domain) = extracted_email.rsplit_once('@').unwrap_or(("", ""));
            let normalized_domain = match normalize_domain(raw_domain) {
                Ok(value) => value,
                Err(_) => {
                    stats.invalid += 1;
                    write_line(
                        &mut writers.invalid,
                        &extracted_email,
                        &writers.invalid_name,
                    )?;
                    continue;
                }
            };

            let domain_result = if matches!(processing_mode, ProcessingMode::VerifyDns) {
                domain_results
                    .get(&normalized_domain)
                    .cloned()
                    .unwrap_or(DomainVerifyResult {
                        dns: MxStatus::Inconclusive,
                        smtp: None,
                    })
            } else {
                DomainVerifyResult {
                    dns: MxStatus::HasMx,
                    smtp: None,
                }
            };

            let group = group_for_email(
                &domain_result.dns,
                &normalized_domain,
                public_domains,
                edu_patterns,
                target_domains,
                processing_mode,
            );

            match group {
                EmailGroup::Public => {
                    stats.public += 1;
                    write_line(&mut writers.public, &extracted_email, &writers.public_name)?;
                }
                EmailGroup::Edu => {
                    stats.edu += 1;
                    write_line(&mut writers.edu, &extracted_email, &writers.edu_name)?;
                }
                EmailGroup::Targeted => {
                    stats.targeted += 1;
                    write_line(
                        &mut writers.targeted,
                        &extracted_email,
                        &writers.targeted_name,
                    )?;
                }
                EmailGroup::Custom => {
                    stats.custom += 1;
                    write_line(&mut writers.custom, &extracted_email, &writers.custom_name)?;
                }
                EmailGroup::MxDead => {
                    stats.mx_dead += 1;
                    write_line(
                        &mut writers.mx_dead,
                        &extracted_email,
                        &writers.mx_dead_name,
                    )?;
                }
                EmailGroup::MxHasMx => {
                    stats.mx_has_mx += 1;
                    write_line(
                        &mut writers.mx_has_mx,
                        &extracted_email,
                        &writers.mx_has_mx_name,
                    )?;
                }
                EmailGroup::MxARecordFallback => {
                    stats.mx_a_fallback += 1;
                    write_line(
                        &mut writers.mx_a_fallback,
                        &extracted_email,
                        &writers.mx_a_fallback_name,
                    )?;
                }
                EmailGroup::MxInconclusive => {
                    stats.mx_inconclusive += 1;
                    write_line(
                        &mut writers.mx_inconclusive,
                        &extracted_email,
                        &writers.mx_inconclusive_name,
                    )?;
                }
                EmailGroup::MxParked => {
                    stats.mx_parked += 1;
                    write_line(
                        &mut writers.mx_parked,
                        &extracted_email,
                        &writers.mx_parked_name,
                    )?;
                }
                EmailGroup::MxDisposable => {
                    stats.mx_disposable += 1;
                    write_line(
                        &mut writers.mx_disposable,
                        &extracted_email,
                        &writers.mx_disposable_name,
                    )?;
                }
                EmailGroup::MxTypo => {
                    stats.mx_typo += 1;
                    let typo_value = match &domain_result.dns {
                        MxStatus::TypoSuggestion(suggestion) => {
                            format!("{extracted_email} -> {suggestion}")
                        }
                        _ => extracted_email.clone(),
                    };
                    write_line(&mut writers.mx_typo, &typo_value, &writers.mx_typo_name)?;
                }
            }

            if matches!(processing_mode, ProcessingMode::VerifyDns)
                && smtp_enabled
                && matches!(domain_result.dns, MxStatus::HasMx)
            {
                match domain_result.output_bucket() {
                    OutputBucket::SmtpDeliverable => {
                        stats.smtp_deliverable += 1;
                        write_optional_line(
                            writers.smtp_deliverable.as_mut(),
                            &extracted_email,
                            &writers.smtp_deliverable_name,
                        )?;
                    }
                    OutputBucket::SmtpRejected => {
                        stats.smtp_rejected += 1;
                        write_optional_line(
                            writers.smtp_rejected.as_mut(),
                            &extracted_email,
                            &writers.smtp_rejected_name,
                        )?;
                    }
                    OutputBucket::SmtpCatchAll => {
                        stats.smtp_catchall += 1;
                        write_optional_line(
                            writers.smtp_catchall.as_mut(),
                            &extracted_email,
                            &writers.smtp_catchall_name,
                        )?;
                    }
                    OutputBucket::HasMxSmtpUnknown => {
                        stats.smtp_unknown += 1;
                        write_optional_line(
                            writers.smtp_unknown.as_mut(),
                            &extracted_email,
                            &writers.smtp_unknown_name,
                        )?;
                    }
                    _ => {}
                }
            }

            if processed_lines % EMIT_EVERY == 0 {
                let verify_dns = matches!(processing_mode, ProcessingMode::VerifyDns);
                let current_pct =
                    scale_second_pass_progress(total_bytes, bytes_read, verify_dns) as i64;
                if current_pct != last_emitted_pct {
                    last_emitted_pct = current_pct;
                    let payload = build_processing_payload(
                        output_dir,
                        processed_lines,
                        scale_second_pass_progress(total_bytes, bytes_read, verify_dns),
                        &stats,
                        smtp_enabled,
                        smtp_elapsed_ms,
                        started_at.elapsed().as_millis(),
                        Some(normalized_domain.clone()),
                    );
                    emit_progress_event(payload, "processing-progress").ok();
                }
            }
        }
    }

    Ok(build_processing_payload(
        output_dir,
        processed_lines,
        if matches!(processing_mode, ProcessingMode::VerifyDns) {
            100.0
        } else {
            scale_second_pass_progress(total_bytes, bytes_read, false)
        },
        &stats,
        smtp_enabled,
        smtp_elapsed_ms,
        started_at.elapsed().as_millis(),
        None,
    ))
}

fn scale_progress(total_bytes: u64, bytes_read: u64, max_progress: f64) -> f64 {
    if total_bytes == 0 {
        max_progress
    } else {
        ((bytes_read as f64 / total_bytes as f64) * max_progress).clamp(0.0, max_progress)
    }
}

fn scale_second_pass_progress(total_bytes: u64, bytes_read: u64, check_mx: bool) -> f64 {
    if total_bytes == 0 {
        100.0
    } else if check_mx {
        let remaining = 100.0 - DOMAIN_SCAN_PROGRESS_END;
        (DOMAIN_SCAN_PROGRESS_END + ((bytes_read as f64 / total_bytes as f64) * remaining))
            .clamp(DOMAIN_SCAN_PROGRESS_END, 100.0)
    } else {
        ((bytes_read as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
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
        MxStatus::Dead => EmailGroup::MxDead,
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

fn build_writers(output_path: &Path, smtp_enabled: bool) -> Result<Writers, std::io::Error> {
    let invalid_name = "01_email_khong_hop_le__invalid.txt".to_string();
    let public_name = "02_email_cong_cong__public.txt".to_string();
    let edu_name = "03_email_edu_gov.txt".to_string();
    let targeted_name = "04_email_muc_tieu__targeted.txt".to_string();
    let custom_name = "05_email_khac__other.txt".to_string();
    let mx_dead_name = "10_dns_domain_chet__dead.txt".to_string();
    let mx_has_mx_name = "11_dns_mx_hop_le__has_mx.txt".to_string();
    let mx_a_fallback_name = "12_dns_fallback_a_record.txt".to_string();
    let mx_inconclusive_name = "13_dns_can_xem_them__inconclusive.txt".to_string();
    let mx_parked_name = "14_dns_domain_parked.txt".to_string();
    let mx_disposable_name = "15_dns_disposable.txt".to_string();
    let mx_typo_name = "16_dns_goi_y_sua_loi__typo.txt".to_string();
    let smtp_deliverable_name = "20_smtp_gui_duoc__deliverable.txt".to_string();
    let smtp_rejected_name = "21_smtp_tu_choi__rejected.txt".to_string();
    let smtp_catchall_name = "22_smtp_catch_all.txt".to_string();
    let smtp_unknown_name = "23_smtp_chua_ro__unknown.txt".to_string();

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

fn build_processing_payload(
    output_dir: &str,
    processed_lines: u64,
    progress_percent: f64,
    stats: &Stats,
    smtp_enabled: bool,
    smtp_elapsed_ms: u64,
    elapsed_ms: u128,
    current_domain: Option<String>,
) -> ProcessingPayload {
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
        elapsed_ms,
        output_dir: Some(output_dir.to_string()),
        current_domain,
    }
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
        let extracted = extract_email_from_line("unicode@münchen.de", &extractor_regex);
        assert_eq!(extracted, Some("unicode@münchen.de".to_string()));
    }

    #[test]
    fn disposable_domains_are_embedded() {
        assert!(is_disposable_domain("mailinator.com"));
    }

    #[test]
    fn inconclusive_and_dead_go_to_separate_outputs() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-test-{}",
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let input_path = base_dir.join("emails.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(
            &input_path,
            "alive@gmail.com\nmaybe@timeout.test\ndead@dead.test\nalive@gmail.com\n",
        )
        .unwrap();

        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
        let edu_patterns = build_edu_patterns().unwrap();
        let target_domains = HashSet::new();
        let output_dir = base_dir.join("output");
        fs::create_dir_all(&output_dir).unwrap();
        let mut writers = build_writers(&output_dir, false).unwrap();
        let run_output = output_dir.to_string_lossy().to_string();

        let mut domain_results = HashMap::new();
        domain_results.insert(
            "gmail.com".to_string(),
            DomainVerifyResult {
                dns: MxStatus::HasMx,
                smtp: None,
            },
        );
        domain_results.insert(
            "timeout.test".to_string(),
            DomainVerifyResult {
                dns: MxStatus::Inconclusive,
                smtp: None,
            },
        );
        domain_results.insert(
            "dead.test".to_string(),
            DomainVerifyResult {
                dns: MxStatus::Dead,
                smtp: None,
            },
        );

        let payload = process_files_with_domain_results(
            &[input_path.to_string_lossy().to_string()],
            fs::metadata(&input_path).unwrap().len(),
            &extractor_regex,
            &public_domains,
            &edu_patterns,
            &target_domains,
            ProcessingMode::VerifyDns,
            0,
            &domain_results,
            false,
            0,
            &run_output,
            Instant::now(),
            &mut writers,
            &mut |_payload, _event| Ok(()),
        )
        .unwrap();
        flush_writers(&mut writers).unwrap();

        assert_eq!(payload.mx_has_mx, 1);
        assert_eq!(payload.mx_inconclusive, 1);
        assert_eq!(payload.mx_dead, 1);
        assert_eq!(payload.duplicates, 1);

        let dead_file = fs::read_to_string(output_dir.join("10_dns_domain_chet__dead.txt")).unwrap();
        let inconclusive_file =
            fs::read_to_string(output_dir.join("13_dns_can_xem_them__inconclusive.txt")).unwrap();
        assert!(dead_file.contains("dead@dead.test"));
        assert!(inconclusive_file.contains("maybe@timeout.test"));
    }

    #[test]
    fn verify_mode_keeps_successful_dns_results_in_explicit_buckets() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-verify-success-{}",
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let input_path = base_dir.join("emails.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(&input_path, "mx@gmail.com\nfallback@example.com\n").unwrap();

        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
        let edu_patterns = build_edu_patterns().unwrap();
        let target_domains = HashSet::new();
        let output_dir = base_dir.join("output");
        fs::create_dir_all(&output_dir).unwrap();
        let mut writers = build_writers(&output_dir, false).unwrap();
        let run_output = output_dir.to_string_lossy().to_string();

        let mut domain_results = HashMap::new();
        domain_results.insert(
            "gmail.com".to_string(),
            DomainVerifyResult {
                dns: MxStatus::HasMx,
                smtp: None,
            },
        );
        domain_results.insert(
            "example.com".to_string(),
            DomainVerifyResult {
                dns: MxStatus::ARecordFallback,
                smtp: None,
            },
        );

        let payload = process_files_with_domain_results(
            &[input_path.to_string_lossy().to_string()],
            fs::metadata(&input_path).unwrap().len(),
            &extractor_regex,
            &public_domains,
            &edu_patterns,
            &target_domains,
            ProcessingMode::VerifyDns,
            0,
            &domain_results,
            false,
            0,
            &run_output,
            Instant::now(),
            &mut writers,
            &mut |_payload, _event| Ok(()),
        )
        .unwrap();
        flush_writers(&mut writers).unwrap();

        assert_eq!(payload.mx_has_mx, 1);
        assert_eq!(payload.mx_a_fallback, 1);
        assert_eq!(payload.public, 0);
        assert_eq!(payload.custom, 0);

        let mx_file = fs::read_to_string(output_dir.join("11_dns_mx_hop_le__has_mx.txt")).unwrap();
        let fallback_file =
            fs::read_to_string(output_dir.join("12_dns_fallback_a_record.txt")).unwrap();
        assert!(mx_file.contains("mx@gmail.com"));
        assert!(fallback_file.contains("fallback@example.com"));
    }

    #[test]
    fn smtp_results_write_additive_output_files_for_has_mx_domains() {
        let base_dir = std::env::temp_dir().join(format!(
            "filteremail-smtp-files-{}",
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let input_path = base_dir.join("emails.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(
            &input_path,
            "ok@gmail.com\nreject@proton.me\ncatch@catchall.test\nfallback@example.com\n",
        )
        .unwrap();

        let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
        let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
        let edu_patterns = build_edu_patterns().unwrap();
        let target_domains = HashSet::new();
        let output_dir = base_dir.join("output");
        fs::create_dir_all(&output_dir).unwrap();
        let mut writers = build_writers(&output_dir, true).unwrap();
        let run_output = output_dir.to_string_lossy().to_string();

        let mut domain_results = HashMap::new();
        domain_results.insert(
            "gmail.com".to_string(),
            DomainVerifyResult {
                dns: MxStatus::HasMx,
                smtp: Some(SmtpStatus::Deliverable),
            },
        );
        domain_results.insert(
            "proton.me".to_string(),
            DomainVerifyResult {
                dns: MxStatus::HasMx,
                smtp: Some(SmtpStatus::Rejected),
            },
        );
        domain_results.insert(
            "catchall.test".to_string(),
            DomainVerifyResult {
                dns: MxStatus::HasMx,
                smtp: Some(SmtpStatus::CatchAll),
            },
        );
        domain_results.insert(
            "example.com".to_string(),
            DomainVerifyResult {
                dns: MxStatus::ARecordFallback,
                smtp: None,
            },
        );

        let payload = process_files_with_domain_results(
            &[input_path.to_string_lossy().to_string()],
            fs::metadata(&input_path).unwrap().len(),
            &extractor_regex,
            &public_domains,
            &edu_patterns,
            &target_domains,
            ProcessingMode::VerifyDns,
            0,
            &domain_results,
            true,
            123,
            &run_output,
            Instant::now(),
            &mut writers,
            &mut |_payload, _event| Ok(()),
        )
        .unwrap();
        flush_writers(&mut writers).unwrap();

        assert_eq!(payload.mx_has_mx, 3);
        assert_eq!(payload.mx_a_fallback, 1);
        assert_eq!(payload.smtp_deliverable, 1);
        assert_eq!(payload.smtp_rejected, 1);
        assert_eq!(payload.smtp_catchall, 1);
        assert_eq!(payload.smtp_unknown, 0);
        assert!(payload.smtp_enabled);
        assert_eq!(payload.smtp_elapsed_ms, 123);

        let has_mx_file = fs::read_to_string(output_dir.join("11_dns_mx_hop_le__has_mx.txt")).unwrap();
        let deliverable_file =
            fs::read_to_string(output_dir.join("20_smtp_gui_duoc__deliverable.txt")).unwrap();
        let rejected_file =
            fs::read_to_string(output_dir.join("21_smtp_tu_choi__rejected.txt")).unwrap();
        let catchall_file =
            fs::read_to_string(output_dir.join("22_smtp_catch_all.txt")).unwrap();
        let fallback_file =
            fs::read_to_string(output_dir.join("12_dns_fallback_a_record.txt")).unwrap();

        assert!(has_mx_file.contains("ok@gmail.com"));
        assert!(has_mx_file.contains("reject@proton.me"));
        assert!(has_mx_file.contains("catch@catchall.test"));
        assert!(deliverable_file.contains("ok@gmail.com"));
        assert!(rejected_file.contains("reject@proton.me"));
        assert!(catchall_file.contains("catch@catchall.test"));
        assert!(fallback_file.contains("fallback@example.com"));
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
        values.insert(
            "gmial.com".to_string(),
            MxStatus::TypoSuggestion("gmail.com".to_string()),
        );
        cache.store_many(&values).unwrap();

        let restored = cache
            .load_many(&[
                "gmail.com".to_string(),
                "gmial.com".to_string(),
                "unknown.com".to_string(),
            ])
            .unwrap();

        assert_eq!(restored.get("gmail.com"), Some(&MxStatus::HasMx));
        assert_eq!(
            restored.get("gmial.com"),
            Some(&MxStatus::TypoSuggestion("gmail.com".to_string()))
        );
        assert!(!restored.contains_key("unknown.com"));
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
