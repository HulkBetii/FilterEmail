use chrono::Local;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};

const BUFFER_CAPACITY: usize = 1024 * 1024;
const EMIT_EVERY: u64 = 10_000;
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

#[derive(Clone, Serialize, Debug)]
pub struct ProcessingPayload {
    pub processed_lines: u64,
    pub progress_percent: f64,
    pub invalid: u64,
    pub public: u64,
    pub edu: u64,
    pub custom: u64,
    pub elapsed_ms: u128,
    pub output_dir: Option<String>,
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
    custom: BufWriter<File>,
    invalid_name: String,
    public_name: String,
    edu_name: String,
    custom_name: String,
}

#[derive(Copy, Clone)]
enum EmailGroup {
    Invalid,
    Public,
    Edu,
    Custom,
}

pub fn process_file_core<F>(
    input_path: &Path,
    output_path: &Path,
    mut emit_progress_event: F,
) -> Result<ProcessingPayload, ErrorPayload>
where
    F: FnMut(ProcessingPayload, &str) -> Result<(), String>,
{
    let started_at = Instant::now();

    if !input_path.exists() {
        return Err(backend_error(
            "Input file does not exist.",
            "Tệp đầu vào không tồn tại.",
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

    let input_file = File::open(input_path).map_err(|error| {
        backend_error(
            "Failed to open input file.",
            "Không thể mở tệp đầu vào.",
            Some(error.to_string()),
        )
    })?;

    let run_output_path = build_run_output_dir(output_path, input_path)?;
    fs::create_dir_all(&run_output_path).map_err(|error| {
        backend_error(
            "Failed to create the session output directory.",
            "Không thể tạo thư mục đầu ra cho phiên lọc.",
            Some(error.to_string()),
        )
    })?;

    let output_dir = run_output_path.to_string_lossy().to_string();
    let total_bytes = input_file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, input_file);
    let mut writers = build_writers(&run_output_path).map_err(|error| {
        error_payload_from_io(
            "Failed to create one or more result files.",
            "Không thể tạo một hoặc nhiều tệp kết quả.",
            error,
        )
    })?;

    let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
    let edu_patterns = build_edu_patterns()?;

    let mut line = String::with_capacity(1024);
    let mut bytes_read: u64 = 0;
    let mut processed_lines: u64 = 0;
    let mut invalid: u64 = 0;
    let mut public: u64 = 0;
    let mut edu: u64 = 0;
    let mut custom: u64 = 0;

    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|error| {
            backend_error(
                "Failed while reading the input file.",
                "Đã xảy ra lỗi khi đọc tệp đầu vào.",
                Some(error.to_string()),
            )
        })?;

        if read == 0 {
            break;
        }

        bytes_read += read as u64;
        processed_lines += 1;

        let normalized = line.trim().to_lowercase();
        let group = classify_email(&normalized, &public_domains, &edu_patterns);

        match group {
            EmailGroup::Invalid => {
                invalid += 1;
                write_line(&mut writers.invalid, &normalized, &writers.invalid_name)?;
            }
            EmailGroup::Public => {
                public += 1;
                write_line(&mut writers.public, &normalized, &writers.public_name)?;
            }
            EmailGroup::Edu => {
                edu += 1;
                write_line(&mut writers.edu, &normalized, &writers.edu_name)?;
            }
            EmailGroup::Custom => {
                custom += 1;
                write_line(&mut writers.custom, &normalized, &writers.custom_name)?;
            }
        }

        if processed_lines % EMIT_EVERY == 0 {
            let payload = build_processing_payload(
                &output_dir,
                processed_lines,
                bytes_read,
                total_bytes,
                invalid,
                public,
                edu,
                custom,
                started_at.elapsed().as_millis(),
            );

            emit_progress_event(payload, "processing-progress").map_err(|error| {
                backend_error(
                    "Failed to emit progress event.",
                    "Không thể phát sự kiện tiến độ.",
                    Some(error),
                )
            })?;
        }
    }

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
        "Failed to flush edu or gov email results to disk.",
        "Không thể ghi hoàn tất kết quả email giáo dục hoặc chính phủ xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.custom,
        "Failed to flush custom email results to disk.",
        "Không thể ghi hoàn tất kết quả email doanh nghiệp xuống đĩa.",
    )?;

    Ok(build_processing_payload(
        &output_dir,
        processed_lines,
        bytes_read,
        total_bytes,
        invalid,
        public,
        edu,
        custom,
        started_at.elapsed().as_millis(),
    ))
}

fn build_writers(output_path: &Path) -> Result<Writers, std::io::Error> {
    let invalid_name = "invalid_emails.txt".to_string();
    let public_name = "public_emails.txt".to_string();
    let edu_name = "edu_gov_emails.txt".to_string();
    let custom_name = "custom_webmail_emails.txt".to_string();

    let invalid = File::create(output_path.join(&invalid_name))?;
    let public = File::create(output_path.join(&public_name))?;
    let edu = File::create(output_path.join(&edu_name))?;
    let custom = File::create(output_path.join(&custom_name))?;

    Ok(Writers {
        invalid: BufWriter::with_capacity(BUFFER_CAPACITY, invalid),
        public: BufWriter::with_capacity(BUFFER_CAPACITY, public),
        edu: BufWriter::with_capacity(BUFFER_CAPACITY, edu),
        custom: BufWriter::with_capacity(BUFFER_CAPACITY, custom),
        invalid_name,
        public_name,
        edu_name,
        custom_name,
    })
}

fn build_run_output_dir(base_output_path: &Path, input_path: &Path) -> Result<std::path::PathBuf, ErrorPayload> {
    let source_stem = input_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_path_segment)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "emails".to_string());

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
    email: &str,
    public_domains: &HashSet<&'static str>,
    edu_patterns: &[Regex],
) -> EmailGroup {
    if !is_valid_email(email) {
        return EmailGroup::Invalid;
    }

    let (_, domain) = email.rsplit_once('@').unwrap_or(("", ""));

    if public_domains.contains(domain) {
        return EmailGroup::Public;
    }

    if edu_patterns.iter().any(|regex| regex.is_match(domain)) {
        return EmailGroup::Edu;
    }

    EmailGroup::Custom
}

fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || email.contains(' ') {
        return false;
    }

    let Some((local_part, domain)) = email.rsplit_once('@') else {
        return false;
    };

    if local_part.is_empty() || domain.is_empty() {
        return false;
    }

    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }

    domain.contains('.')
}

fn write_line(writer: &mut BufWriter<File>, value: &str, file_name: &str) -> Result<(), ErrorPayload> {
    writer.write_all(value.as_bytes()).map_err(|error| {
        backend_error(
            "Failed to write a classified email to the result file.",
            "Không thể ghi một email đã phân loại vào tệp kết quả.",
            Some(format!("{file_name}: {error}")),
        )
    })?;
    writer.write_all(b"\n").map_err(|error| {
        backend_error(
            "Failed to finish writing a classified email line.",
            "Không thể hoàn tất việc ghi một dòng email đã phân loại.",
            Some(format!("{file_name}: {error}")),
        )
    })
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

fn build_processing_payload(
    output_dir: &str,
    processed_lines: u64,
    bytes_read: u64,
    total_bytes: u64,
    invalid: u64,
    public: u64,
    edu: u64,
    custom: u64,
    elapsed_ms: u128,
) -> ProcessingPayload {
    let progress_percent = if total_bytes == 0 {
        100.0
    } else {
        ((bytes_read as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
    };

    ProcessingPayload {
        processed_lines,
        progress_percent,
        invalid,
        public,
        edu,
        custom,
        elapsed_ms,
        output_dir: Some(output_dir.to_string()),
    }
}

fn backend_error(message_en: &str, message_vi: &str, detail: Option<String>) -> ErrorPayload {
    ErrorPayload {
        message_en: attach_detail(message_en, detail.clone()),
        message_vi: attach_detail_vi(message_vi, detail),
    }
}

fn error_payload_from_io(message_en: &str, message_vi: &str, error: std::io::Error) -> ErrorPayload {
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
        "Failed to initialize email classification patterns.",
        "Không thể khởi tạo các mẫu phân loại email.",
        Some(error.to_string()),
    )
}
