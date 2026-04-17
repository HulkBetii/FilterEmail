import re

# 1. Update i18n.ts to ensure mx_dead exists
with open("src/i18n.ts", "r") as f:
    i18n = f.read()

if "mx_dead" not in i18n:
    i18n = i18n.replace(
        'duplicates: string;\n    custom: string;',
        'duplicates: string;\n    mx_dead: string;\n    custom: string;'
    )
    i18n = i18n.replace(
        'custom: "Other",',
        'mx_dead: "Dead Domains (MX)",\n      custom: "Other",'
    )
    i18n = i18n.replace(
        'custom: "Khác",',
        'mx_dead: "Tên miền sống ảo (Die)",\n      custom: "Khác",'
    )
    with open("src/i18n.ts", "w") as f:
        f.write(i18n)

# 2. Update processor.rs to write dead_emails.txt
with open("src-tauri/src/processor.rs", "r") as f:
    proc = f.read()

# Add mx_dead to Writers struct
proc = proc.replace(
    '''custom: BufWriter<File>,\n    invalid_name: String,''',
    '''custom: BufWriter<File>,\n    mx_dead: BufWriter<File>,\n    invalid_name: String,'''
)
proc = proc.replace(
    '''custom_name: String,\n}''',
    '''custom_name: String,\n    mx_dead_name: String,\n}'''
)

# Add EmailGroup::MxDead
proc = proc.replace(
    '''Targeted,\n    Custom,\n}''',
    '''Targeted,\n    Custom,\n    MxDead,\n}'''
)

# Match EmailGroup::MxDead
proc = proc.replace(
    '''mx_dead += 1;\n                EmailGroup::Invalid''',
    '''EmailGroup::MxDead'''
)

match_block = '''EmailGroup::Custom => {
                    custom += 1;
                    write_line(&mut writers.custom, &extracted_email, &writers.custom_name)?;
                }'''
new_match_block = '''EmailGroup::Custom => {
                    custom += 1;
                    write_line(&mut writers.custom, &extracted_email, &writers.custom_name)?;
                }
                EmailGroup::MxDead => {
                    mx_dead += 1;
                    write_line(&mut writers.mx_dead, &extracted_email, &writers.mx_dead_name)?;
                }'''
proc = proc.replace(match_block, new_match_block)

# Flush mx_dead
proc = proc.replace(
    '''flush_writer(&mut writers.custom, "Failed to flush custom email results to disk.", "Không thể ghi hoàn tất kết quả email doanh nghiệp.")?;''',
    '''flush_writer(&mut writers.custom, "Failed to flush custom email results to disk.", "Không thể ghi hoàn tất kết quả email doanh nghiệp.")?;\n    flush_writer(&mut writers.mx_dead, "Failed to flush dead email results to disk.", "Không thể ghi tệp mail chết.")?;'''
)

# Create file in build_writers
proc = proc.replace(
    '''let custom_name = "other_emails.txt".to_string();''',
    '''let custom_name = "other_emails.txt".to_string();\n    let mx_dead_name = "dead_emails.txt".to_string();'''
)
proc = proc.replace(
    '''let custom = File::create(output_path.join(&custom_name))?;''',
    '''let custom = File::create(output_path.join(&custom_name))?;\n    let mx_dead = File::create(output_path.join(&mx_dead_name))?;'''
)
proc = proc.replace(
    '''custom: BufWriter::with_capacity(BUFFER_CAPACITY, custom),''',
    '''custom: BufWriter::with_capacity(BUFFER_CAPACITY, custom),\n        mx_dead: BufWriter::with_capacity(BUFFER_CAPACITY, mx_dead),'''
)
proc = proc.replace(
    '''custom_name,\n    })''',
    '''custom_name,\n        mx_dead_name,\n    })'''
)

with open("src-tauri/src/processor.rs", "w") as f:
    f.write(proc)

# 3. Update App.tsx UI detail description
with open("src/App.tsx", "r") as f:
    app = f.read()

app = app.replace(
    '{stats.mx_dead.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}',
    '{stats.mx_dead.toLocaleString(language === "vi" ? "vi-VN" : "en-US")} <span className="text-sm font-medium text-red-700 ml-2">({language === "vi" ? "Xuất vào dead_emails.txt" : "Saved to dead_emails.txt"})</span>'
)

with open("src/App.tsx", "w") as f:
    f.write(app)

