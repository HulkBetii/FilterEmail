# Prompt: Deep DNS Scan — Email Verification Feature (Tauri v2 + Rust)

---

## Bối cảnh dự án

Bạn đang xây dựng một **desktop email list filtering tool** bằng **Tauri v2** (Rust backend + frontend hiện đại).
App nhận file email list đầu vào (CSV/TXT, có thể lên tới hàng triệu dòng), phân loại từng email vào các nhóm,
rồi xuất kết quả.

Chức năng cần implement là **"Deep DNS Scan"** — một tính năng tùy chọn khi bật sẽ kiểm tra
domain của mỗi email có MX record hợp lệ hay không trước khi phân loại.

---

## Yêu cầu chức năng

### Input
- Danh sách email đã được parse từ file (Vec<String>)
- Flag `check_mx: bool` (user bật/tắt từ UI)
- Config từ user: `timeout_ms`, `max_concurrent`, `use_persistent_cache`

### Output
Mỗi email được gán một trong các trạng thái sau — **không được dùng bool đơn giản**:

```rust
pub enum MxStatus {
    HasMx,            // domain có MX record hợp lệ
    ARecordFallback,  // không có MX nhưng có A record (RFC 5321 §5.1)
    Dead,             // NXDOMAIN authoritative — domain không tồn tại
    Parked,           // có MX nhưng trỏ về parking service
    Disposable,       // domain thuộc danh sách disposable email
    TypoSuggestion(String), // domain sai chính tả, kèm gợi ý sửa
    Inconclusive,     // lỗi transient (SERVFAIL, timeout) — KHÔNG trash email này
}
```

> **Quy tắc tuyệt đối:** Email có status `Inconclusive` phải được đưa vào bucket riêng để user review.
> Tuyệt đối không xếp vào `Dead` hay loại bỏ silently.

---

## Kiến trúc kỹ thuật cần implement

### 1. Async DNS Resolver — dùng `TokioAsyncResolver`

**Không được dùng blocking `Resolver`** trong Tauri vì sẽ block Tokio runtime worker thread.

```rust
use hickory_resolver::{config::{ResolverConfig, ResolverOpts}, TokioAsyncResolver};
use std::time::Duration;

fn build_resolver() -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_millis(1_500); // KHÔNG để default 5s
    opts.attempts = 2;                            // 2 lần × 1.5s = 3s max/domain
    opts.validate = false;     // tắt DNSSEC — domain hợp lệ có DNSSEC broken sẽ false-negative
    opts.cache_size = 1024;    // default 32 là quá nhỏ cho bulk scan
    opts.preserve_intermediates = true; // cần cho CNAME chain của mail providers
    opts.rotate = true;        // dùng nameserver song song thay vì tuần tự
    TokioAsyncResolver::tokio(ResolverConfig::default(), opts)
}
```

### 2. Concurrency — Semaphore giới hạn 30–50 concurrent lookups

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let sem = Arc::new(Semaphore::new(50));

// Thêm jitter để tránh thundering herd khi burst request
use rand::Rng;
let jitter_ms = rand::thread_rng().gen_range(0..50u64);
tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
let _permit = sem.acquire().await.unwrap();
```

### 3. Logic phân loại domain — thứ tự quan trọng

Xử lý theo pipeline sau, **theo đúng thứ tự**:

```
Input domain
    │
    ▼
[Normalize] → lowercase + trim + bỏ trailing dot + IDN→punycode
    │
    ▼
[Cache hit?] → trả về kết quả cached ngay
    │
    ▼
[Disposable check] → so sánh với danh sách built-in
    │
    ▼
[Typo check] → gợi ý nếu khớp pattern lỗi phổ biến
    │
    ▼
[DNS MX lookup] với retry logic
    │
    ├─ NXDOMAIN → Dead (authoritative, không retry)
    ├─ SERVFAIL/timeout → retry tối đa 2 lần → Inconclusive nếu vẫn fail
    ├─ Empty answer (no MX) → thử A-record → ARecordFallback hoặc Dead
    └─ MX found → kiểm tra parked → Parked hoặc HasMx
```

### 4. Retry logic — phân biệt lỗi authoritative vs transient

```rust
use hickory_resolver::error::ResolveErrorKind;

for attempt in 0..=2u8 {
    match resolver.mx_lookup(&domain).await {
        Ok(lookup) => {
            if lookup.iter().next().is_none() {
                // Empty answer — no MX records
                return check_a_record_fallback(&resolver, &domain).await;
            }
            let all_parked = lookup.iter().all(|mx| is_parked_mx(&mx.exchange().to_string()));
            return if all_parked { MxStatus::Parked } else { MxStatus::HasMx };
        }
        Err(e) => match e.kind() {
            ResolveErrorKind::NoRecordsFound { response_code, .. }
                if *response_code == hickory_proto::op::ResponseCode::NXDomain =>
            {
                return MxStatus::Dead; // Authoritative — không retry
            }
            _ if attempt < 2 => {
                tokio::time::sleep(Duration::from_millis(80 * (attempt as u64 + 1))).await;
                continue;
            }
            _ => return MxStatus::Inconclusive,
        },
    }
}
MxStatus::Inconclusive
```

### 5. Parked domain detection

```rust
const PARKING_MX_SUFFIXES: &[&str] = &[
    "registrar-servers.com",   // Namecheap
    "sedoparking.com",         // Sedo
    "parkingcrew.net",
    "hugedomains.com",
    "above.com",
    "bodis.com",
    "afternic.com",
    "dan.com",
];

fn is_parked_mx(mx_host: &str) -> bool {
    let host = mx_host.trim_end_matches('.').to_lowercase();
    PARKING_MX_SUFFIXES.iter().any(|s| host.ends_with(s))
}
```

### 6. IDN / Punycode normalization — bắt buộc

```rust
// Cargo.toml: idna = "0.5"
use idna::Config;

fn normalize_domain(raw: &str) -> Result<String, String> {
    let domain = raw.trim().to_lowercase();
    let domain = domain.trim_end_matches('.');

    // Convert IDN sang ASCII punycode để DNS lookup
    let config = Config::default().use_std3_ascii_rules(true);
    let (ascii, result) = config.to_ascii(domain);
    result.map_err(|e| format!("IDN error: {:?}", e))?;
    Ok(ascii)
}
```

### 7. Disposable email domain check

Nhúng file danh sách vào binary (không cần network call):

```rust
// Đặt file tại: src/data/disposable_domains.txt
// Source: https://github.com/disposable-email-domains/disposable-email-domains
const DISPOSABLE_DOMAINS: &str = include_str!("../data/disposable_domains.txt");

fn build_disposable_set() -> std::collections::HashSet<&'static str> {
    DISPOSABLE_DOMAINS.lines().map(|l| l.trim()).collect()
}
// Khởi tạo 1 lần bằng lazy_static hoặc OnceLock
```

### 8. Typo detection cho domain phổ biến

```rust
const TYPO_MAP: &[(&str, &[&str])] = &[
    ("gmail.com",   &["gmial.com", "gmai.com", "gamil.com", "gmal.com", "gnail.com"]),
    ("yahoo.com",   &["yahooo.com", "yaho.com", "yhoo.com", "yaoo.com"]),
    ("outlook.com", &["outlok.com", "outloook.com", "outllook.com"]),
    ("hotmail.com", &["hotmai.com", "hotmial.com", "hotmale.com"]),
    ("icloud.com",  &["iclould.com", "icolud.com"]),
];

fn check_typo(domain: &str) -> Option<String> {
    for (correct, typos) in TYPO_MAP {
        if typos.contains(&domain) {
            return Some(correct.to_string());
        }
    }
    None
}
```

### 9. In-memory cache per session — bắt buộc

Domain deduplication và caching là yếu tố tăng tốc quan trọng nhất:

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct DomainCache {
    inner: RwLock<HashMap<String, MxStatus>>,
}

impl DomainCache {
    pub async fn get(&self, domain: &str) -> Option<MxStatus> {
        self.inner.read().await.get(domain).cloned()
    }
    pub async fn set(&self, domain: String, status: MxStatus) {
        self.inner.write().await.insert(domain, status);
    }
}
```

### 10. Persistent DNS cache (tùy chọn — SQLite)

Khi `use_persistent_cache = true`, lưu kết quả vào SQLite với TTL 6 giờ:

```rust
// Cargo.toml: rusqlite = { version = "0.31", features = ["bundled"] }
use rusqlite::{Connection, params};

const CACHE_TTL_SECS: u64 = 6 * 3600; // 6 giờ — an toàn cho MX record

fn init_cache_db(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS mx_cache (
            domain TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            cached_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_cached_at ON mx_cache(cached_at);
    ")?;
    Ok(conn)
}

fn get_cached(conn: &Connection, domain: &str) -> Option<String> {
    let cutoff = unix_now() - CACHE_TTL_SECS;
    conn.query_row(
        "SELECT status FROM mx_cache WHERE domain = ?1 AND cached_at > ?2",
        params![domain, cutoff],
        |r| r.get(0),
    ).ok()
}
```

### 11. Tauri command — async với progress events

```rust
use tauri::{AppHandle, Emitter};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ScanProgress {
    pub processed: usize,
    pub total: usize,
    pub dead_count: usize,
    pub inconclusive_count: usize,
    pub current_domain: String,
}

#[tauri::command]
pub async fn run_deep_scan(
    app: AppHandle,
    emails: Vec<String>,
    check_mx: bool,
    timeout_ms: u64,
    max_concurrent: usize,
) -> Result<ScanResult, String> {
    // 1. Build resolver và semaphore
    // 2. Dedup domains trước — chỉ spawn 1 task per domain
    // 3. Batch xử lý 1000 domain/batch
    // 4. Emit progress event mỗi 500 email processed
    // 5. Collect kết quả và map lại cho từng email
    
    let mut processed = 0usize;
    // ... main loop ...
    if processed % 500 == 0 {
        app.emit("scan:progress", ScanProgress {
            processed,
            total: emails.len(),
            dead_count,
            inconclusive_count,
            current_domain: domain.clone(),
        }).ok();
    }
    
    Ok(build_result(email_results))
}
```

---

## Cấu trúc file đề xuất

```
src-tauri/src/
├── commands/
│   └── deep_scan.rs        ← Tauri command entry point
├── dns/
│   ├── mod.rs
│   ├── resolver.rs         ← build_resolver(), ResolverOpts config
│   ├── mx_check.rs         ← check_domain_mx_async(), retry logic
│   ├── parked.rs           ← PARKING_MX_SUFFIXES, is_parked_mx()
│   └── cache.rs            ← DomainCache (in-memory + SQLite)
├── email/
│   ├── normalize.rs        ← normalize_domain(), IDN/punycode
│   ├── typo.rs             ← TYPO_MAP, check_typo()
│   └── disposable.rs       ← DISPOSABLE_DOMAINS, is_disposable()
└── data/
    └── disposable_domains.txt  ← file nhúng vào binary
```

---

## Cargo.toml dependencies cần thêm

```toml
[dependencies]
hickory-resolver = { version = "0.24", features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
idna = "0.5"
rusqlite = { version = "0.31", features = ["bundled"] }  # nếu dùng persistent cache
rand = "0.8"
serde = { version = "1", features = ["derive"] }
lazy_static = "1.4"
# hoặc thay lazy_static bằng std::sync::OnceLock (stable từ Rust 1.70)
```

---

## Các lưu ý quan trọng khi implement

### Không được làm
- ❌ Dùng blocking `Resolver::new()` bên trong async Tauri command
- ❌ Dùng `is_ok()` đơn giản — mất hết context lỗi
- ❌ Retry khi gặp NXDOMAIN (lãng phí, kết quả vẫn vậy)
- ❌ Mark `Inconclusive` thành `Dead` — user mất email hợp lệ
- ❌ Lookup domain chưa normalize — cache miss, kết quả sai
- ❌ Spawn 1 task per email — phải dedup domain trước
- ❌ Emit progress event mỗi 1 email — overhead quá lớn

### Phải làm
- ✅ Normalize domain (lowercase + punycode) TRƯỚC khi cache lookup
- ✅ Phân biệt NXDOMAIN (authoritative) vs SERVFAIL (transient)
- ✅ Giữ `Inconclusive` bucket riêng trong kết quả xuất ra
- ✅ Dedup domain list trước khi spawn DNS task
- ✅ Semaphore max 50 concurrent để tránh bị rate limit / IP block
- ✅ Jitter random 0–50ms trước mỗi lookup để tránh thundering herd
- ✅ Emit progress event mỗi ~500 email để UI không freeze
- ✅ Tắt DNSSEC validation (`opts.validate = false`) để tránh false negative

---

## Tiêu chí kiểm tra chức năng (acceptance criteria)

| Test case | Kết quả mong đợi |
|---|---|
| `gmail.com` | `HasMx` |
| `nonexistentdomain12345xyz.com` | `Dead` |
| `gmial.com` | `TypoSuggestion("gmail.com")` |
| `mailinator.com` | `Disposable` |
| Domain parked tại GoDaddy/Namecheap | `Parked` |
| DNS timeout sau 3s | `Inconclusive` (không phải Dead) |
| `münchen.de` (IDN) | Normalize sang `xn--mnchen-3ya.de` trước lookup |
| Domain không có MX nhưng có A record | `ARecordFallback` |
| Cùng domain xuất hiện 10,000 lần | Chỉ lookup 1 lần, dùng cache cho 9,999 lần còn lại |
| Scan 1M email, 50K unique domain | Không block UI, có progress bar cập nhật |

---

## Gợi ý thứ tự implement (từng bước)

1. **Bước 1 — Foundation:** Implement `MxStatus` enum + `normalize_domain()` + `build_resolver()` với đúng `ResolverOpts`
2. **Bước 2 — Core DNS logic:** `check_domain_mx_async()` với retry, phân biệt NXDOMAIN/SERVFAIL, A-record fallback
3. **Bước 3 — Enrichment:** Thêm parked detection + disposable check + typo detection
4. **Bước 4 — Performance:** In-memory `DomainCache` + domain dedup + Semaphore + jitter
5. **Bước 5 — Tauri integration:** `#[tauri::command]` async + progress event emit
6. **Bước 6 — Persistent cache (optional):** SQLite cache với TTL 6h, toggle từ settings UI
7. **Bước 7 — Testing:** Unit test từng case trong bảng acceptance criteria ở trên
