use super::classify::normalize_domain;
use super::types::ParsedEmail;
use regex::Regex;
use std::fs;

pub(crate) fn total_bytes(file_paths: &[String]) -> u64 {
    file_paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|meta| meta.len())
        .sum()
}

pub(crate) fn extract_email_candidate_from_line(
    line: &str,
    extractor_regex: &Regex,
) -> Option<String> {
    if let Some(matched) = extractor_regex.find(line) {
        return Some(matched.as_str().trim().to_string());
    }

    line.split_whitespace()
        .map(|token| {
            token.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\''
                        | '<'
                        | '>'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | ','
                        | ';'
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

pub(crate) fn parse_email_candidate(candidate: &str) -> Option<ParsedEmail> {
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
