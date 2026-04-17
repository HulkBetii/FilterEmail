use super::cache::DomainCache;
use super::classify::{
    check_typo, is_disposable_domain, is_parked_domain, is_parked_mx,
};
use super::errors::backend_error;
use super::input::{extract_email_candidate_from_line, parse_email_candidate};
use super::payload::{build_processing_payload, scale_progress};
use super::types::{CollectedDomains, ErrorPayload, MxStatus, Stats};
use hickory_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    proto::op::ResponseCode,
};
use rand::Rng;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::Semaphore,
    task::JoinSet,
    time::sleep,
};

pub(crate) fn collect_unique_domains<F>(
    file_paths: &[String],
    extractor_regex: &Regex,
    total_bytes: u64,
    output_dir: &str,
    started_at: Instant,
    smtp_enabled: bool,
    emit_progress_event: &mut F,
) -> Result<CollectedDomains, ErrorPayload>
where
    F: FnMut(super::ProcessingPayload, &str) -> Result<(), String>,
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

        let mut reader = BufReader::with_capacity(super::BUFFER_CAPACITY, input_file);
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

            if processed_lines.is_multiple_of(super::EMIT_EVERY) {
                let payload = build_processing_payload(
                    output_dir,
                    processed_lines,
                    scale_progress(total_bytes, bytes_read, super::FIRST_PASS_PROGRESS_END),
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
pub(crate) async fn scan_domains<F>(
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
    F: FnMut(super::ProcessingPayload, &str) -> Result<(), String>,
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

    for batch in unique_domains.chunks(super::DOMAIN_SCAN_BATCH_SIZE) {
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

            if processed_domains.is_multiple_of(super::EMIT_EVERY as usize)
                || processed_domains == total_domains
            {
                let domain_progress = if total_domains == 0 {
                    super::DOMAIN_SCAN_PROGRESS_END
                } else {
                    super::FIRST_PASS_PROGRESS_END
                        + ((processed_domains as f64 / total_domains as f64)
                            * (super::DOMAIN_SCAN_PROGRESS_END
                                - super::FIRST_PASS_PROGRESS_END))
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

pub(crate) async fn check_domain_mx_async(
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

pub(crate) async fn check_a_record_fallback(
    resolver: &TokioAsyncResolver,
    domain: &str,
) -> MxStatus {
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

pub(crate) fn build_resolver(timeout_ms: u64) -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_millis(timeout_ms.max(250));
    opts.attempts = 2;
    opts.validate = false;
    opts.cache_size = 1024;
    opts.preserve_intermediates = true;
    opts.rotate = true;
    TokioAsyncResolver::tokio(ResolverConfig::default(), opts)
}
