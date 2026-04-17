import re

with open("src-tauri/src/processor.rs", "r") as f:
    content = f.read()

# 1. ProcessingPayload
content = content.replace(
    "pub edu: u64,\n    pub custom: u64,",
    "pub edu: u64,\n    pub targeted: u64,\n    pub custom: u64,"
)

# 2. Writers
content = content.replace(
    "    edu: BufWriter<File>,\n    custom: BufWriter<File>,",
    "    edu: BufWriter<File>,\n    targeted: BufWriter<File>,\n    custom: BufWriter<File>,"
)
content = content.replace(
    "    edu_name: String,\n    custom_name: String,",
    "    edu_name: String,\n    targeted_name: String,\n    custom_name: String,"
)

# 3. EmailGroup
content = content.replace(
    "    Edu,\n    Custom",
    "    Edu,\n    Targeted,\n    Custom"
)

# 4. process_file_core signature
content = content.replace(
    "pub fn process_file_core<F>(\n    input_path: &Path,\n    output_path: &Path,\n    mut emit_progress_event: F,\n) -> Result<ProcessingPayload, ErrorPayload>",
    "pub fn process_file_core<F>(\n    input_path: &Path,\n    output_path: &Path,\n    target_domains: Vec<String>,\n    mut emit_progress_event: F,\n) -> Result<ProcessingPayload, ErrorPayload>"
)

# 5. HashSet for target domains
content = content.replace(
    "let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();",
    "let target_domains_set: HashSet<String> = target_domains.into_iter().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();\n    let public_domains: HashSet<&'static str> = PUBLIC_DOMAINS.iter().copied().collect();"
)

# 6. variables setup
content = content.replace(
    "    let mut edu: u64 = 0;\n    let mut custom: u64 = 0;",
    "    let mut edu: u64 = 0;\n    let mut targeted: u64 = 0;\n    let mut custom: u64 = 0;"
)

# 7. classify match
old_match = """        let group = classify_email(&normalized, &public_domains, &edu_patterns);

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
        }"""
new_match = """        let group = classify_email(&normalized, &public_domains, &edu_patterns, &target_domains_set);

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
            EmailGroup::Targeted => {
                targeted += 1;
                write_line(&mut writers.targeted, &normalized, &writers.targeted_name)?;
            }
            EmailGroup::Custom => {
                custom += 1;
                write_line(&mut writers.custom, &normalized, &writers.custom_name)?;
            }
        }"""
content = content.replace(old_match, new_match)

# 8. build payload
content = content.replace(
    "invalid,\n                public,\n                edu,\n                custom,\n",
    "invalid,\n                public,\n                edu,\n                targeted,\n                custom,\n"
)
# At the end payload building
content = content.replace(
    "        invalid,\n        public,\n        edu,\n        custom,\n        started_at.elapsed().as_millis(),",
    "        invalid,\n        public,\n        edu,\n        targeted,\n        custom,\n        started_at.elapsed().as_millis(),"
)


# 9. flush writers
old_flush = """    flush_writer(
        &mut writers.edu,
        "Failed to flush edu or gov email results to disk.",
        "Không thể ghi hoàn tất kết quả email giáo dục hoặc chính phủ xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.custom,"""
new_flush = """    flush_writer(
        &mut writers.edu,
        "Failed to flush edu or gov email results to disk.",
        "Không thể ghi hoàn tất kết quả email giáo dục hoặc chính phủ xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.targeted,
        "Failed to flush targeted email results to disk.",
        "Không thể ghi hoàn tất kết quả email tùy chọn xuống đĩa.",
    )?;
    flush_writer(
        &mut writers.custom,"""
content = content.replace(old_flush, new_flush)

# 10. build writers function
old_build_writers = """fn build_writers(output_path: &Path) -> Result<Writers, std::io::Error> {
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
}"""
new_build_writers = """fn build_writers(output_path: &Path) -> Result<Writers, std::io::Error> {
    let invalid_name = "invalid_emails.txt".to_string();
    let public_name = "public_emails.txt".to_string();
    let edu_name = "edu_gov_emails.txt".to_string();
    let targeted_name = "targeted_emails.txt".to_string();
    let custom_name = "other_emails.txt".to_string();

    let invalid = File::create(output_path.join(&invalid_name))?;
    let public = File::create(output_path.join(&public_name))?;
    let edu = File::create(output_path.join(&edu_name))?;
    let targeted = File::create(output_path.join(&targeted_name))?;
    let custom = File::create(output_path.join(&custom_name))?;

    Ok(Writers {
        invalid: BufWriter::with_capacity(BUFFER_CAPACITY, invalid),
        public: BufWriter::with_capacity(BUFFER_CAPACITY, public),
        edu: BufWriter::with_capacity(BUFFER_CAPACITY, edu),
        targeted: BufWriter::with_capacity(BUFFER_CAPACITY, targeted),
        custom: BufWriter::with_capacity(BUFFER_CAPACITY, custom),
        invalid_name,
        public_name,
        edu_name,
        targeted_name,
        custom_name,
    })
}"""
content = content.replace(old_build_writers, new_build_writers)

# 11. classify_email args
content = content.replace(
    "fn classify_email(\n    email: &str,\n    public_domains: &HashSet<&'static str>,\n    edu_patterns: &[Regex],\n) -> EmailGroup {",
    "fn classify_email(\n    email: &str,\n    public_domains: &HashSet<&'static str>,\n    edu_patterns: &[Regex],\n    target_domains: &HashSet<String>,\n) -> EmailGroup {"
)

# 12. classify_email logic
old_classify_logic = """    if public_domains.contains(domain) {
        return EmailGroup::Public;
    }

    if edu_patterns.iter().any(|regex| regex.is_match(domain)) {
        return EmailGroup::Edu;
    }

    EmailGroup::Custom"""
new_classify_logic = """    if target_domains.contains(domain) {
        return EmailGroup::Targeted;
    }

    if public_domains.contains(domain) {
        return EmailGroup::Public;
    }

    if edu_patterns.iter().any(|regex| regex.is_match(domain)) {
        return EmailGroup::Edu;
    }

    EmailGroup::Custom"""
content = content.replace(old_classify_logic, new_classify_logic)

# 13. build_processing_payload args
content = content.replace(
    "    edu: u64,\n    custom: u64,\n    elapsed_ms: u128,",
    "    edu: u64,\n    targeted: u64,\n    custom: u64,\n    elapsed_ms: u128,"
)

content = content.replace(
    "        edu,\n        custom,\n        elapsed_ms,",
    "        edu,\n        targeted,\n        custom,\n        elapsed_ms,"
)


with open("src-tauri/src/processor.rs", "w") as f:
    f.write(content)

