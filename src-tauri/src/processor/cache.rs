use super::errors::backend_error;
use super::types::{ErrorPayload, MxStatus, ParsedEmail};
use crate::smtp_status::{SmtpProbeRecord, SmtpStatus};
use rusqlite::{Connection, params};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

#[derive(Default)]
pub(crate) struct DomainCache {
    inner: RwLock<HashMap<String, MxStatus>>,
}

impl DomainCache {
    pub(crate) async fn get(&self, domain: &str) -> Option<MxStatus> {
        self.inner.read().await.get(domain).cloned()
    }

    pub(crate) async fn set(&self, domain: String, status: MxStatus) {
        self.inner.write().await.insert(domain, status);
    }
}

pub(crate) struct PersistentCache {
    path: PathBuf,
}

impl PersistentCache {
    pub(crate) fn new(path: &Path) -> Result<Self, ErrorPayload> {
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

    pub(crate) fn load_many(
        &self,
        domains: &[String],
    ) -> Result<HashMap<String, MxStatus>, ErrorPayload> {
        let conn = Connection::open(&self.path).map_err(|error| {
            backend_error(
                "Failed to open persistent cache database.",
                "Không thể mở cơ sở dữ liệu persistent cache.",
                Some(error.to_string()),
            )
        })?;
        let cutoff = unix_now_secs() - super::CACHE_TTL_SECS;
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

    pub(crate) fn store_many(
        &self,
        domain_statuses: &HashMap<String, MxStatus>,
    ) -> Result<(), ErrorPayload> {
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

    pub(crate) fn load_smtp_many(
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
        let cutoff = unix_now_secs() - super::SMTP_CACHE_TTL_SECS;
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

    pub(crate) fn store_smtp_many(
        &self,
        records: &[SmtpProbeRecord],
    ) -> Result<(), ErrorPayload> {
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
