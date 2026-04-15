use chrono::Local;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};
use hickory_resolver::{config::{ResolverConfig, ResolverOpts}, Resolver};

const BUFFER_CAPACITY: usize = 1024 * 1024;
const EMIT_EVERY: u64 = 500;
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
    pub targeted: u64,
    pub custom: u64,
    pub duplicates: u64,
    pub mx_dead: u64,
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
    targeted: BufWriter<File>,
    custom: BufWriter<File>,
    mx_dead: BufWriter<File>,
    invalid_name: String,
    public_name: String,
    edu_name: String,
    targeted_name: String,
    custom_name: String,
    mx_dead_name: String,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
enum EmailGroup {
    Invalid,
    Public,
    Edu,
    Targeted,
    Custom,
    MxDead,
}

pub fn process_file_core<F>(
    file_paths: Vec<String>,
    output_path: &Path,
    target_domains: Vec<String>,
    check_mx: bool,
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
    
    let mut total_bytes: u64 = 0;
    for path in &file_paths {
        if let Ok(meta) = fs::metadata(path) {
            total_bytes += meta.len();
        }
    }

    let mut writers = build_writers(&run_output_path).map_err(|error| {
        error_payload_from_io(
            "Failed to create one or more result files.",
            "Không thể tạo một hoặc nhiều tệp kết quả.",
            error,
        )
    })?;

    let target_domains_set: HashSet<String> = target_domains.into_iter().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
    let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();
    let edu_patterns = build_edu_patterns()?;
    let extractor_regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").map_err(map_regex_error_payload)?;

    let resolver_opt = if check_mx {
        Resolver::new(ResolverConfig::default(), ResolverOpts::default()).ok()
    } else {
        None
    };

    let mut line = String::with_capacity(1024);
    let mut bytes_read: u64 = 0;
    let mut processed_lines: u64 = 0;
    let mut invalid: u64 = 0;
    let mut public: u64 = 0;
    let mut edu: u64 = 0;
    let mut targeted: u64 = 0;
    let mut custom: u64 = 0;
    let mut duplicates: u64 = 0;
    let mut mx_dead: u64 = 0;
    let mut last_emitted_pct: i64 = -1;

    let mut seen_emails: HashSet<String> = HashSet::with_capacity(100_000);
    let mut mx_cache: HashMap<String, bool> = HashMap::with_capacity(5_000);

    for file_path in file_paths {
        let path = Path::new(&file_path);
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
                Err(_) => break, // skip file on error
            };

            if read == 0 {
                break;
            }

            bytes_read += read as u64;
            processed_lines += 1;

            let extracted_email = match extractor_regex.find(&line) {
                Some(mat) => mat.as_str().trim().to_lowercase(),
                None => {
                    invalid += 1;
                    write_line(&mut writers.invalid, line.trim(), &writers.invalid_name)?;
                    continue;
                }
            };

            if !seen_emails.insert(extracted_email.clone()) {
                duplicates += 1;
                continue;
            }

            let (_, domain) = extracted_email.rsplit_once('@').unwrap_or(("", ""));
            
            // Check MX if enabled
            let is_alive = if let Some(resolver) = &resolver_opt {
                *mx_cache.entry(domain.to_string()).or_insert_with(|| {
                    resolver.mx_lookup(domain).is_ok()
                })
            } else {
                true
            };

            let group = if !is_alive {
                EmailGroup::MxDead
            } else {
                classify_email(domain, &public_domains, &edu_patterns, &target_domains_set)
            };

            match group {
                EmailGroup::Invalid => {
                    invalid += 1;
                    write_line(&mut writers.invalid, &extracted_email, &writers.invalid_name)?;
                }
                EmailGroup::Public => {
                    public += 1;
                    write_line(&mut writers.public, &extracted_email, &writers.public_name)?;
                }
                EmailGroup::Edu => {
                    edu += 1;
                    write_line(&mut writers.edu, &extracted_email, &writers.edu_name)?;
                }
                EmailGroup::Targeted => {
                    targeted += 1;
                    write_line(&mut writers.targeted, &extracted_email, &writers.targeted_name)?;
                }
                EmailGroup::Custom => {
                    custom += 1;
                    write_line(&mut writers.custom, &extracted_email, &writers.custom_name)?;
                }
                EmailGroup::MxDead => {
                    mx_dead += 1;
                    write_line(&mut writers.mx_dead, &extracted_email, &writers.mx_dead_name)?;
                }
            }

            if processed_lines % EMIT_EVERY == 0 {
                let current_pct = if total_bytes > 0 {
                    ((bytes_read as f64 / total_bytes as f64) * 100.0) as i64
                } else {
                    0
                };
                if current_pct != last_emitted_pct {
                    last_emitted_pct = current_pct;
                    let payload = build_processing_payload(
                        &output_dir,
                        processed_lines,
                        bytes_read,
                        total_bytes,
                        invalid,
                        public,
                        edu,
                        targeted,
                        custom,
                        duplicates,
                        mx_dead,
                        started_at.elapsed().as_millis(),
                    );
                    emit_progress_event(payload, "processing-progress").ok();
                }
            }
        }
    }

    flush_writer(&mut writers.invalid, "Failed to flush invalid email results to disk.", "Không thể ghi hoàn tất kết quả email không hợp lệ xuống đĩa.")?;
    flush_writer(&mut writers.public, "Failed to flush public email results to disk.", "Không thể ghi hoàn tất kết quả email công cộng xuống đĩa.")?;
    flush_writer(&mut writers.edu, "Failed to flush edu email results to disk.", "Không thể ghi hoàn tất kết quả email giáo dục xuống đĩa.")?;
    flush_writer(&mut writers.targeted, "Failed to flush targeted email results to disk.", "Không thể ghi hoàn tất kết quả email chọn lọc.")?;
    flush_writer(&mut writers.custom, "Failed to flush custom email results to disk.", "Không thể ghi hoàn tất kết quả email doanh nghiệp.")?;
    flush_writer(&mut writers.mx_dead, "Failed to flush dead email results to disk.", "Không thể ghi tệp mail chết.")?;

    Ok(build_processing_payload(
        &output_dir,
        processed_lines,
        bytes_read,
        total_bytes,
        invalid,
        public,
        edu,
        targeted,
        custom,
        duplicates,
        mx_dead,
        started_at.elapsed().as_millis(),
    ))
}

fn build_writers(output_path: &Path) -> Result<Writers, std::io::Error> {
    let invalid_name = "invalid_emails.txt".to_string();
    let public_name = "public_emails.txt".to_string();
    let edu_name = "edu_gov_emails.txt".to_string();
    let targeted_name = "targeted_emails.txt".to_string();
    let custom_name = "other_emails.txt".to_string();
    let mx_dead_name = "dead_emails.txt".to_string();

    let invalid = File::create(output_path.join(&invalid_name))?;
    let public = File::create(output_path.join(&public_name))?;
    let edu = File::create(output_path.join(&edu_name))?;
    let targeted = File::create(output_path.join(&targeted_name))?;
    let custom = File::create(output_path.join(&custom_name))?;
    let mx_dead = File::create(output_path.join(&mx_dead_name))?;

    Ok(Writers {
        invalid: BufWriter::with_capacity(BUFFER_CAPACITY, invalid),
        public: BufWriter::with_capacity(BUFFER_CAPACITY, public),
        edu: BufWriter::with_capacity(BUFFER_CAPACITY, edu),
        targeted: BufWriter::with_capacity(BUFFER_CAPACITY, targeted),
        custom: BufWriter::with_capacity(BUFFER_CAPACITY, custom),
        mx_dead: BufWriter::with_capacity(BUFFER_CAPACITY, mx_dead),
        invalid_name,
        public_name,
        edu_name,
        targeted_name,
        custom_name,
        mx_dead_name,
    })
}

fn build_run_output_dir(base_output_path: &Path, paths: &[String]) -> Result<std::path::PathBuf, ErrorPayload> {
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
    value.chars().map(|character| {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            character
        } else {
            '_'
        }
    }).collect::<String>().trim_matches('_').to_string()
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

fn write_line(writer: &mut BufWriter<File>, value: &str, file_name: &str) -> Result<(), ErrorPayload> {
    writer.write_all(value.as_bytes()).map_err(|error| {
        backend_error("Failed to write to file.", "Lỗi ghi tệp.", Some(format!("{file_name}: {error}")))
    })?;
    writer.write_all(b"\n").map_err(|error| {
        backend_error("Failed to write newline.", "Lỗi ghi xuống dòng.", Some(format!("{file_name}: {error}")))
    })
}

fn flush_writer(writer: &mut BufWriter<File>, message_en: &str, message_vi: &str) -> Result<(), ErrorPayload> {
    writer.flush().map_err(|error| error_payload_from_io(message_en, message_vi, error))
}

fn build_processing_payload(
    output_dir: &str,
    processed_lines: u64,
    bytes_read: u64,
    total_bytes: u64,
    invalid: u64,
    public: u64,
    edu: u64,
    targeted: u64,
    custom: u64,
    duplicates: u64,
    mx_dead: u64,
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
        targeted,
        custom,
        duplicates,
        mx_dead,
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
    backend_error("Regex error.", "Lỗi biểu thức chính quy.", Some(error.to_string()))
}
