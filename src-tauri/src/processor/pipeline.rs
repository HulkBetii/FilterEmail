use super::cache::PersistentCache;
use super::classify::group_for_email;
use super::errors::error_payload_from_io;
use super::input::{extract_email_candidate_from_line, parse_email_candidate};
use super::output::{
    Writers, write_detail_row, write_final_result, write_line,
    write_smtp_legacy_output,
};
use super::payload::{
    build_processing_payload, scale_second_pass_progress, smtp_phase_progress,
};
use super::types::{
    EmailGroup, ErrorPayload, MxStatus, ParsedEmail, ProcessingMode,
    SecondPassResult, Stats,
};
use crate::smtp_client::{SmtpApiClient, SmtpVerifyTarget};
use crate::smtp_status::{FinalTriage, SmtpProbeRecord, SmtpStatus};
use crate::smtp_verify::{dns_status_name, final_triage_for};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::Path,
    time::Instant,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_second_pass<F>(
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
    F: FnMut(super::ProcessingPayload, &str) -> Result<(), String>,
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
            super::BUFFER_CAPACITY,
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

        let mut reader = BufReader::with_capacity(super::BUFFER_CAPACITY, input_file);

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

            if processed_lines.is_multiple_of(super::EMIT_EVERY) {
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
        flush_spool_writer(
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
pub(crate) async fn process_smtp_spool<F>(
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
    F: FnMut(super::ProcessingPayload, &str) -> Result<(), String>,
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
    let mut reader = BufReader::with_capacity(super::BUFFER_CAPACITY, input);
    let mut line = String::with_capacity(256);
    let mut batch = Vec::with_capacity(super::SMTP_BATCH_SIZE);
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

        if batch.len() >= super::SMTP_BATCH_SIZE {
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

pub(crate) fn should_persist_smtp_record(record: &SmtpProbeRecord) -> bool {
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
    F: FnMut(super::ProcessingPayload, &str) -> Result<(), String>,
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
        if (*processed_targets).is_multiple_of(super::EMIT_EVERY as usize)
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
    let reader = BufReader::with_capacity(super::BUFFER_CAPACITY, input);
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

fn flush_spool_writer(
    writer: &mut BufWriter<File>,
    message_en: &str,
    message_vi: &str,
) -> Result<(), ErrorPayload> {
    std::io::Write::flush(writer)
        .map_err(|error| error_payload_from_io(message_en, message_vi, error))
}
