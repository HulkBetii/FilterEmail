use super::errors::{backend_error, error_payload_from_io};
use super::types::{ErrorPayload, MxStatus};
use chrono::{Local, Utc};
use crate::smtp_status::{FinalTriage, SmtpProbeRecord};
use crate::smtp_verify::{OutputBucket, output_bucket_for};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub(crate) struct Writers {
    pub(crate) invalid: BufWriter<File>,
    pub(crate) public: BufWriter<File>,
    pub(crate) edu: BufWriter<File>,
    pub(crate) targeted: BufWriter<File>,
    pub(crate) custom: BufWriter<File>,
    pub(crate) mx_dead: BufWriter<File>,
    pub(crate) mx_has_mx: BufWriter<File>,
    pub(crate) mx_a_fallback: BufWriter<File>,
    pub(crate) mx_inconclusive: BufWriter<File>,
    pub(crate) mx_parked: BufWriter<File>,
    pub(crate) mx_disposable: BufWriter<File>,
    pub(crate) mx_typo: BufWriter<File>,
    pub(crate) smtp_deliverable: Option<BufWriter<File>>,
    pub(crate) smtp_rejected: Option<BufWriter<File>>,
    pub(crate) smtp_catchall: Option<BufWriter<File>>,
    pub(crate) smtp_unknown: Option<BufWriter<File>>,
    pub(crate) final_alive: Option<BufWriter<File>>,
    pub(crate) final_dead: Option<BufWriter<File>>,
    pub(crate) final_unknown: Option<BufWriter<File>>,
    pub(crate) detail_csv: Option<BufWriter<File>>,
    pub(crate) invalid_name: String,
    pub(crate) public_name: String,
    pub(crate) edu_name: String,
    pub(crate) targeted_name: String,
    pub(crate) custom_name: String,
    pub(crate) mx_dead_name: String,
    pub(crate) mx_has_mx_name: String,
    pub(crate) mx_a_fallback_name: String,
    pub(crate) mx_inconclusive_name: String,
    pub(crate) mx_parked_name: String,
    pub(crate) mx_disposable_name: String,
    pub(crate) mx_typo_name: String,
    pub(crate) smtp_deliverable_name: String,
    pub(crate) smtp_rejected_name: String,
    pub(crate) smtp_catchall_name: String,
    pub(crate) smtp_unknown_name: String,
    pub(crate) final_alive_name: String,
    pub(crate) final_dead_name: String,
    pub(crate) final_unknown_name: String,
    pub(crate) detail_csv_name: String,
}

pub(crate) fn build_writers(
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
            super::BUFFER_CAPACITY,
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
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&invalid_name))?,
        ),
        public: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&public_name))?,
        ),
        edu: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&edu_name))?,
        ),
        targeted: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&targeted_name))?,
        ),
        custom: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&custom_name))?,
        ),
        mx_dead: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_dead_name))?,
        ),
        mx_has_mx: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_has_mx_name))?,
        ),
        mx_a_fallback: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_a_fallback_name))?,
        ),
        mx_inconclusive: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_inconclusive_name))?,
        ),
        mx_parked: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_parked_name))?,
        ),
        mx_disposable: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_disposable_name))?,
        ),
        mx_typo: BufWriter::with_capacity(
            super::BUFFER_CAPACITY,
            File::create(output_path.join(&mx_typo_name))?,
        ),
        smtp_deliverable: if smtp_enabled {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_deliverable_name))?,
            ))
        } else {
            None
        },
        smtp_rejected: if smtp_enabled {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_rejected_name))?,
            ))
        } else {
            None
        },
        smtp_catchall: if smtp_enabled {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_catchall_name))?,
            ))
        } else {
            None
        },
        smtp_unknown: if smtp_enabled {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&smtp_unknown_name))?,
            ))
        } else {
            None
        },
        final_alive: if verify_mode {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&final_alive_name))?,
            ))
        } else {
            None
        },
        final_dead: if verify_mode {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
                File::create(output_path.join(&final_dead_name))?,
            ))
        } else {
            None
        },
        final_unknown: if verify_mode {
            Some(BufWriter::with_capacity(
                super::BUFFER_CAPACITY,
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

pub(crate) fn flush_writers(writers: &mut Writers) -> Result<(), ErrorPayload> {
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

pub(crate) fn build_run_output_dir(
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

pub(crate) fn write_smtp_legacy_output(
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

pub(crate) fn write_final_result(
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

pub(crate) fn write_detail_row(
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

pub(crate) fn write_line(
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

pub(crate) fn write_optional_line(
    writer: Option<&mut BufWriter<File>>,
    value: &str,
    file_name: &str,
) -> Result<(), ErrorPayload> {
    if let Some(writer) = writer {
        write_line(writer, value, file_name)?;
    }
    Ok(())
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

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
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
