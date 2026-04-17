import re

domain_map = {
    "gmail.com", "yahoo.com", "aol.com", "outlook.com", "icloud.com",
    "hotmail.com", "mail.com", "ymail.com", "live.com", "msn.com",
    "gmx.es", "googlemail.com", "pm.me", "o2.pl", "inbox.lv",
    "yahoo.co.uk", "yahoo.ca", "yahoo.com.mx", "yahoo.com.ph"
}
edu_patterns = [r"\.edu$", r"\.gov$", r"\.k12\.[a-z]{2}\.us$", r"\.edu\.[a-z]{2}$", r"\.org$"]

stats = {"total_lines": 0, "processed": 0, "invalid": 0, "public": 0, "edu": 0, "targeted": 0, "custom": 0, "duplicates": 0}
seen = set()

regex = re.compile(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}")

targets = {"students.hcde.org", "gmx.es"}

with open("/Users/sangspm/Downloads/emails.txt", "r") as f:
    for line in f:
        line = line.strip()
        if not line: continue
        stats["total_lines"] += 1
        
        match = regex.search(line)
        if not match:
            stats["invalid"] += 1
            continue
            
        email = match.group(0).lower()
        if email in seen:
            stats["duplicates"] += 1
            continue
            
        seen.add(email)
        stats["processed"] += 1
        
        domain = email.split('@')[1]
        
        if domain in targets:
            stats["targeted"] += 1
        elif domain in domain_map:
            stats["public"] += 1
        elif any(re.search(p, domain) for p in edu_patterns):
            stats["edu"] += 1
        else:
            stats["custom"] += 1

print("--- BÁO CÁO PHÂN TÍCH (Giả lập Backend Rust) ---")
print(f"Tổng số dòng: {stats['total_lines']}")
print(f"Số email hợp lệ trích xuất: {stats['processed']}")
print(f"❌ Không hợp lệ / Rác: {stats['invalid']}")
print(f"🔄 Bị trùng lặp: {stats['duplicates']}")
print(f"🌐 Công cộng (Public): {stats['public']}")
print(f"🎓 Giáo dục / Chính phủ (Edu/Gov): {stats['edu']}")
print(f"🎯 Mục tiêu tùy chọn (Targeted): {stats['targeted']} (giả sử chọn 'students.hcde.org, gmx.es')")
print(f"🏢 Khác (Custom): {stats['custom']}")

