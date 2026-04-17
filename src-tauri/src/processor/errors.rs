use super::types::ErrorPayload;

pub(crate) fn backend_error(
    message_en: &str,
    message_vi: &str,
    detail: Option<String>,
) -> ErrorPayload {
    ErrorPayload {
        message_en: attach_detail(message_en, detail.clone()),
        message_vi: attach_detail_vi(message_vi, detail),
    }
}

pub(crate) fn error_payload_from_io(
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

pub(crate) fn map_regex_error_payload(error: regex::Error) -> ErrorPayload {
    backend_error(
        "Regex error.",
        "Lỗi biểu thức chính quy.",
        Some(error.to_string()),
    )
}
