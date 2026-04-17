# Codex Prompt: Nâng cấp Verify DNS → DNS + SMTP Ping (90%+ accuracy)

---

## Nhiệm vụ tổng quát

Nâng cấp chức năng **Verify DNS** hiện có trong app FilterEmail (Tauri + Rust backend + React frontend) lên mức **DNS + SMTP ping**, mục tiêu accuracy ≥ 90%.

Scale: ~100K email/ngày. Budget tối thiểu. Tauri vẫn là UI chính, SMTP verification chạy trên **VPS backend riêng** (Rust/Axum), Tauri gọi lên qua HTTPS API.

**Đọc toàn bộ phần này trước khi chạm vào bất kỳ file nào.**

---

## Hiểu đúng cơ chế hiện tại — KHÔNG được thay đổi những phần này

### Pipeline hiện tại (processor.rs)

```
File stream (không load toàn bộ vào RAM)
    │
    ▼
Parse email từng dòng
    │
    ▼
Normalize domain:
    - lowercase + trim + strip trailing dot
    - IDN → punycode (idna crate)
    │
    ▼
[check_mx = false] → Legacy filter pipeline
[check_mx = true]  → Verify DNS pipeline (mục tiêu nâng cấp)
    │
    ▼ (verify mode)
Deduplicate domain — mỗi domain chỉ scan MỘT lần
    │
    ▼
Domain scan pipeline (theo đúng thứ tự này):
    1. Cache hit? (in-memory HashMap + SQLite nếu persistent cache bật)
    2. Typo detection → TypoSuggestion(String)   [PHẢI trước Disposable]
    3. Disposable detection → Disposable
    4. MX lookup async (hickory TokioAsyncResolver)
       ├─ NXDOMAIN → Dead
       ├─ SERVFAIL/timeout → retry 2 lần → Inconclusive
       ├─ Empty answer (no MX) → A-record fallback → ARecordFallback hoặc Dead
       └─ MX found → parked check → Parked hoặc HasMx
    │
    ▼
Map domain status → từng email
    │
    ▼
Ghi output files riêng biệt
```

### MxStatus enum hiện tại — KHÔNG thêm/xóa/đổi tên variant

```rust
pub enum MxStatus {
    HasMx,
    ARecordFallback,
    Dead,
    Parked,
    Disposable,
    TypoSuggestion(String),
    Inconclusive,
}
```

### Output files hiện tại — KHÔNG xóa, KHÔNG đổi tên

```
has_mx_emails.txt
a_record_fallback_emails.txt
dead_emails.txt
inconclusive_emails.txt
parked_emails.txt
disposable_emails.txt
typo_suggestions.txt
```

### ProcessingPayload fields hiện tại — KHÔNG xóa field nào

```
processed_lines, progress_percent,
invalid, public, edu, targeted, custom, duplicates,
mx_dead, mx_has_mx, mx_a_fallback, mx_inconclusive,
mx_parked, mx_disposable, mx_typo,
cache_hits, elapsed_ms, output_dir, current_domain
```

### Persistent cache — KHÔNG thay đổi contract

- SQLite, TTL 6h
- Preload → scan miss → ghi lại
- Cache key = normalized domain, value = MxStatus serialized

### Invariant tuyệt đối — vi phạm là bug

1. `Inconclusive` KHÔNG BAO GIỜ được gộp vào `Dead`
2. `HasMx` / `ARecordFallback` KHÔNG BAO GIỜ collapse về legacy buckets
3. Typo detection LUÔN chạy TRƯỚC Disposable detection
4. Domain LUÔN được normalize trước mọi xử lý
5. Mỗi domain chỉ bị scan DNS/SMTP đúng 1 lần (dedup trước)

---

## Phần cần thêm mới — SMTP Ping Layer

### Mục tiêu

Sau khi DNS verify trả `HasMx`, thực hiện thêm **SMTP RCPT TO probe** để phân loại thêm:

```
HasMx (DNS confirmed)
    │
    ▼
SMTP Ping (gọi VPS API)
    ├─ SmtpDeliverable   → mailbox confirmed exist
    ├─ SmtpRejected      → mailbox confirmed not exist (550/551/553)
    ├─ SmtpCatchAll      → server accept all (catch-all detected)
    └─ SmtpInconclusive  → timeout / greylist / 4xx temp / connection refused
```

`ARecordFallback`, `Dead`, `Parked`, `Disposable`, `TypoSuggestion`, `Inconclusive` đều **bỏ qua SMTP ping** — không có MX thì không SMTP.

### Thay đổi 1 — Thêm SmtpStatus enum (backend mới, KHÔNG trong processor.rs hiện tại)

Tạo file mới `src-tauri/src/smtp_verify.rs` (hoặc module riêng), KHÔNG sửa processor.rs trực tiếp ở bước này:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SmtpStatus {
    Deliverable,    // RCPT TO → 250/251
    Rejected,       // RCPT TO → 550/551/553/554 (user unknown)
    CatchAll,       // probe address ngẫu nhiên cũng 250 → catch-all
    Inconclusive,   // 4xx / timeout / connection refused / port blocked
}
```

### Thay đổi 2 — Kiến trúc VPS client

Tauri backend KHÔNG tự kết nối SMTP trực tiếp (port 25 bị ISP block trên desktop).
Thay vào đó: Tauri gọi **HTTPS API lên VPS** để thực hiện SMTP check.

#### 2a. VPS service (Rust/Axum) — tạo project riêng `verify-vps/`

```
verify-vps/
├── src/
│   ├── main.rs          ← Axum server, POST /verify/smtp
│   ├── smtp.rs          ← SMTP probe logic
│   ├── catch_all.rs     ← Catch-all detection
│   ├── rate_limiter.rs  ← Per-domain rate limiting
│   └── cache.rs         ← In-memory TTL cache (mini, không cần Redis ở scale nhỏ)
└── Cargo.toml
```

**API contract VPS:**

```
POST /verify/smtp
Content-Type: application/json
Authorization: Bearer <SMTP_API_KEY>

Request:
{
  "domains": ["gmail.com", "company.com"],   // batch, deduplicated
  "emails": {                                 // email per domain để probe
    "gmail.com": "probe-target@gmail.com",
    "company.com": "probe-target@company.com"
  }
}

Response:
{
  "results": {
    "gmail.com": { "status": "Deliverable", "mx_host": "gmail-smtp-in.l.google.com" },
    "company.com": { "status": "CatchAll" },
    "notexist.xyz": { "status": "Rejected" }
  },
  "elapsed_ms": 1240
}
```

#### 2b. SMTP probe logic (smtp.rs trên VPS)

```rust
// Thứ tự bắt buộc:
// 1. Catch-all detection TRƯỚC — tránh false positive hàng loạt
// 2. Nếu không phải catch-all → RCPT TO với email thật

// Catch-all probe: gửi RCPT TO địa chỉ random chắc chắn không tồn tại
async fn detect_catch_all(mx_host: &str, domain: &str) -> bool {
    let probe = format!("zz-noexist-{}@{}", &uuid::Uuid::new_v4().to_string()[..8], domain);
    matches!(smtp_rcpt_check(mx_host, &probe, PROBE_FROM).await, SmtpRcptResult::Accepted)
}

// SMTP sequence: không gửi mail thật
// EHLO → MAIL FROM → RCPT TO → QUIT
// Timeout: 8s per connection
// MAIL FROM dùng domain VPS (phải có SPF + PTR record đúng)

// Parse RCPT TO response:
// 250, 251       → Accepted
// 550,551,553,554 → Rejected
// 421,450,451,452 → TempFail → SmtpStatus::Inconclusive
// Timeout        → SmtpStatus::Inconclusive
// Connect fail   → SmtpStatus::Inconclusive (port blocked)
```

**PTR record bắt buộc:** VPS phải có reverse DNS (PTR) trỏ về domain verify của bạn.
Nếu thiếu PTR, Gmail/Microsoft reject ở bước EHLO → toàn bộ kết quả thành Inconclusive.

#### 2c. Rate limiting trên VPS (rate_limiter.rs)

```rust
// Giới hạn theo MX host, không phải domain
// Gmail: max 20 connections/phút
// Microsoft: max 15 connections/phút
// Yahoo: max 10 connections/phút
// Default: max 30 connections/phút

// Scale nhỏ (~100K email/ngày = ~70 email/phút) → 1 IP đủ
// Không cần IP rotation ở scale này
// Implement dạng token bucket per mx_host
```

#### 2d. Cache trên VPS (cache.rs)

```rust
// In-memory HashMap<String, (SmtpStatus, Instant)>
// TTL: 2h cho SmtpStatus (ngắn hơn DNS cache vì mailbox thay đổi thường hơn)
// Không cần Redis ở scale 100K/ngày
// Key: email address (không phải domain — vì cùng domain có mailbox live/dead khác nhau)
// Ngoại lệ: CatchAll cache theo domain vì toàn bộ domain là catch-all
```

### Thay đổi 3 — Tauri backend tích hợp VPS client

Thêm vào `src-tauri/src/smtp_client.rs` (file MỚI):

```rust
use reqwest::Client;

pub struct SmtpApiClient {
    client: Client,
    base_url: String,   // từ config, ví dụ: https://verify.yourdomain.com
    api_key: String,    // từ env hoặc Tauri secure store
}

impl SmtpApiClient {
    // Gọi VPS theo batch — gộp nhiều domain vào 1 request
    pub async fn verify_batch(
        &self,
        // chỉ gửi domain có MxStatus::HasMx
        mx_domains: &[(String, String)], // (domain, email)
    ) -> HashMap<String, SmtpStatus> {
        // Nếu VPS không available → trả Inconclusive cho tất cả
        // KHÔNG fail cả job khi VPS down
        // Timeout request: 30s (batch có thể lớn)
    }
}
```

**Tích hợp vào processor.rs:**

```rust
// Sau bước MX lookup, bổ sung SMTP ping cho HasMx domains
// Không thay đổi thứ tự pipeline hiện tại
// Chỉ thêm 1 bước sau cùng trong verify mode

// Pseudo-code tích hợp:
let mx_domains: Vec<(String, String)> = domain_results
    .iter()
    .filter(|(_, status)| matches!(status, MxStatus::HasMx))
    .map(|(domain, _)| {
        let email = email_for_domain.get(domain).cloned().unwrap_or_default();
        (domain.clone(), email)
    })
    .collect();

let smtp_results = if smtp_client.is_some() && !mx_domains.is_empty() {
    smtp_client.as_ref().unwrap().verify_batch(&mx_domains).await
} else {
    HashMap::new() // VPS không config → bỏ qua SMTP, giữ HasMx
};
```

### Thay đổi 4 — MxStatus KHÔNG thay đổi, thêm composite result

KHÔNG sửa MxStatus enum. Thêm struct mới để hold cả hai tầng:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DomainVerifyResult {
    pub dns: MxStatus,
    pub smtp: Option<SmtpStatus>, // None nếu SMTP không chạy hoặc DNS không phải HasMx
}

// Helper để quyết định output file
impl DomainVerifyResult {
    pub fn output_bucket(&self) -> OutputBucket {
        match (&self.dns, &self.smtp) {
            (MxStatus::HasMx, Some(SmtpStatus::Deliverable))   => OutputBucket::SmtpDeliverable,
            (MxStatus::HasMx, Some(SmtpStatus::Rejected))      => OutputBucket::SmtpRejected,
            (MxStatus::HasMx, Some(SmtpStatus::CatchAll))      => OutputBucket::SmtpCatchAll,
            (MxStatus::HasMx, Some(SmtpStatus::Inconclusive))  => OutputBucket::HasMxSmtpUnknown,
            (MxStatus::HasMx, None)                            => OutputBucket::HasMxSmtpUnknown,
            (MxStatus::ARecordFallback, _)                     => OutputBucket::ARecordFallback,
            (MxStatus::Dead, _)                                => OutputBucket::Dead,
            (MxStatus::Parked, _)                              => OutputBucket::Parked,
            (MxStatus::Disposable, _)                          => OutputBucket::Disposable,
            (MxStatus::TypoSuggestion(_), _)                   => OutputBucket::Typo,
            (MxStatus::Inconclusive, _)                        => OutputBucket::Inconclusive,
        }
    }
}
```

### Thay đổi 5 — Output files mới (THÊM VÀO, không xóa file cũ)

```
// File CŨ — giữ nguyên, vẫn ghi như cũ (backward compat)
has_mx_emails.txt          ← giữ: tất cả HasMx (bao gồm cả chưa/đã SMTP)
a_record_fallback_emails.txt
dead_emails.txt
inconclusive_emails.txt
parked_emails.txt
disposable_emails.txt
typo_suggestions.txt

// File MỚI — thêm khi smtp_enabled = true
smtp_deliverable_emails.txt   ← HasMx + SmtpStatus::Deliverable
smtp_rejected_emails.txt      ← HasMx + SmtpStatus::Rejected
smtp_catchall_emails.txt      ← HasMx + SmtpStatus::CatchAll
smtp_unknown_emails.txt       ← HasMx + SmtpStatus::Inconclusive hoặc SMTP không chạy
```

Lý do giữ `has_mx_emails.txt`: người dùng có thể không config VPS → file cũ vẫn có giá trị.

### Thay đổi 6 — ProcessingPayload thêm fields mới (KHÔNG xóa field cũ)

```rust
// Thêm vào ProcessingPayload struct:
pub smtp_deliverable: u64,   // count SmtpDeliverable
pub smtp_rejected: u64,      // count SmtpRejected
pub smtp_catchall: u64,      // count SmtpCatchAll
pub smtp_unknown: u64,       // count HasMx nhưng SMTP inconclusive/not run
pub smtp_enabled: bool,      // VPS có được config không
pub smtp_elapsed_ms: u64,    // thời gian SMTP phase riêng
```

### Thay đổi 7 — Frontend (verify-ui.tsx, final-summary.tsx, top-dashboard.tsx)

**Chỉ thêm hiển thị mới, KHÔNG xóa UI cũ:**

```tsx
// Thêm section SMTP results (chỉ hiện khi smtp_enabled = true)
{payload.smtp_enabled && (
  <SmtpResultSection>
    <BucketCard
      label="Deliverable"
      count={payload.smtp_deliverable}
      color="green"
      file="smtp_deliverable_emails.txt"
    />
    <BucketCard
      label="Rejected"
      count={payload.smtp_rejected}
      color="red"
      file="smtp_rejected_emails.txt"
    />
    <BucketCard
      label="Catch-all"
      count={payload.smtp_catchall}
      color="amber"
      file="smtp_catchall_emails.txt"
    />
    <BucketCard
      label="SMTP Unknown"
      count={payload.smtp_unknown}
      color="gray"
      file="smtp_unknown_emails.txt"
    />
  </SmtpResultSection>
)}
```

**Thêm settings trong verify-ui.tsx:**

```tsx
// Trong verify settings panel
<SettingRow label="SMTP Verify (VPS)">
  <Toggle
    checked={smtpEnabled}
    onChange={setSmtpEnabled}
  />
</SettingRow>

{smtpEnabled && (
  <>
    <SettingRow label="VPS API URL">
      <Input
        value={vpsUrl}
        onChange={setVpsUrl}
        placeholder="https://verify.yourdomain.com"
      />
    </SettingRow>
    <SettingRow label="API Key">
      <Input
        type="password"
        value={vpsApiKey}
        onChange={setVpsApiKey}
      />
    </SettingRow>
  </>
)}
```

Settings persist vào localStorage như các settings verify khác hiện có.

### Thay đổi 8 — i18n.ts

Thêm key mới, KHÔNG đổi key cũ:

```ts
// Thêm vào i18n.ts
smtp_deliverable: { en: "Deliverable", vi: "Có thể gửi được" },
smtp_rejected: { en: "Mailbox rejected", vi: "Mailbox không tồn tại" },
smtp_catchall: { en: "Catch-all domain", vi: "Domain nhận tất cả" },
smtp_unknown: { en: "SMTP unknown", vi: "Không xác định được SMTP" },
smtp_verify_label: { en: "SMTP Verify (VPS)", vi: "Xác minh SMTP (VPS)" },
vps_api_url: { en: "VPS API URL", vi: "Địa chỉ VPS API" },
vps_api_key: { en: "API Key", vi: "Khóa API" },
```

---

## Cấu hình VPS tối thiểu cho scale 100K email/ngày

```bash
# Provider: Hetzner CX22 (~$6/tháng) hoặc Vultr Regular
# KHÔNG dùng AWS/GCP/Azure — port 25 bị block mặc định
# OS: Ubuntu 24.04

# Bắt buộc TRƯỚC KHI deploy:
# 1. Set PTR record (reverse DNS) trong Hetzner console
#    IP VPS → mail.yourdomain.com
# 2. Tạo DNS records:
#    A:   mail.yourdomain.com → IP_VPS
#    TXT: yourdomain.com → "v=spf1 ip4:IP_VPS ~all"
# 3. Mở port: 443 (HTTPS API), 25 outbound (SMTP probe)
# 4. KHÔNG mở port 25 inbound

# Deploy VPS service:
cargo build --release
scp target/x86_64-unknown-linux-gnu/release/verify-vps user@VPS:/opt/verify/

# Env vars VPS:
BIND_ADDR=0.0.0.0:443
TLS_CERT_PATH=/etc/letsencrypt/live/yourdomain.com/fullchain.pem
TLS_KEY_PATH=/etc/letsencrypt/live/yourdomain.com/privkey.pem
SMTP_FROM_DOMAIN=yourdomain.com
API_KEY=<random 32 char>
SMTP_TIMEOUT_SECS=8
MAX_CONCURRENT_SMTP=30

# Env vars Tauri (lưu trong app settings hoặc .env):
VPS_API_URL=https://mail.yourdomain.com
VPS_API_KEY=<same key>
```

---

## Thứ tự implement

Thực hiện theo đúng thứ tự này — KHÔNG skip bước:

**Bước 1 — VPS service (project riêng, không đụng Tauri)**
- Tạo `verify-vps/` Axum project
- Implement `smtp.rs`: EHLO → MAIL FROM → RCPT TO → QUIT sequence
- Implement `catch_all.rs`: probe với địa chỉ UUID random
- Implement `rate_limiter.rs`: token bucket per MX host
- Implement `cache.rs`: in-memory HashMap với TTL 2h
- Expose `POST /verify/smtp` với auth header
- Test thực: `curl` thủ công với gmail.com, một domain dead, một catch-all domain

**Bước 2 — Tauri SMTP client (file mới, chưa tích hợp vào pipeline)**
- Tạo `src-tauri/src/smtp_client.rs`
- Implement `SmtpApiClient::verify_batch()`
- Graceful fallback khi VPS down: trả `SmtpStatus::Inconclusive` cho tất cả
- Unit test mock VPS response

**Bước 3 — Thêm SmtpStatus và DomainVerifyResult**
- Tạo `src-tauri/src/smtp_status.rs`
- Định nghĩa `SmtpStatus` enum
- Định nghĩa `DomainVerifyResult` struct với `output_bucket()` helper
- KHÔNG sửa MxStatus

**Bước 4 — Tích hợp vào processor.rs**
- Sau bước MX lookup, collect tất cả `HasMx` domains
- Batch gọi `SmtpApiClient::verify_batch()` nếu SMTP enabled
- Merge kết quả vào `DomainVerifyResult`
- Ghi output files mới (smtp_*.txt) bổ sung
- Giữ nguyên logic ghi has_mx_emails.txt (tất cả HasMx, dù SMTP status là gì)
- Update ProcessingPayload với smtp_* fields mới

**Bước 5 — Frontend**
- Thêm smtp settings vào verify-ui.tsx
- Thêm smtp bucket display vào top-dashboard.tsx
- Thêm smtp section vào final-summary.tsx
- Cập nhật i18n.ts
- Persist smtp settings vào localStorage

**Bước 6 — Test end-to-end**
- `cargo test` phải pass
- `npm run build` phải pass
- Smoke test với VPS thật:
  - `valid@gmail.com` → HasMx + SmtpDeliverable
  - `definitelynotreal99999@gmail.com` → HasMx + SmtpRejected
  - `anything@catchalldomain.com` → HasMx + SmtpCatchAll
  - `bad@nonexistentdomain12345.com` → Dead (không qua SMTP)
  - VPS down → HasMx + SmtpInconclusive (không crash app)

---

## Điều TUYỆT ĐỐI không được làm

```
✗ Sửa MxStatus enum (thêm/xóa/đổi tên variant)
✗ Xóa bất kỳ output file nào hiện có
✗ Xóa bất kỳ field nào trong ProcessingPayload
✗ Gộp Inconclusive vào Dead
✗ Gộp HasMx/ARecordFallback về legacy buckets
✗ Chạy SMTP verify với domain không phải HasMx
✗ Để SMTP failure crash toàn bộ job
✗ Đảo thứ tự Typo detection vs Disposable detection
✗ Bỏ domain deduplication trước khi scan
✗ Load toàn bộ file vào RAM thay vì stream
✗ Kết nối SMTP trực tiếp từ Tauri (port 25 bị block ở desktop)
✗ Dùng AWS/GCP/Azure cho VPS (port 25 blocked)
✗ Cache SMTP result theo domain thay vì email address
   (ngoại lệ: CatchAll được cache theo domain là đúng)
```

---

## Checklist trước khi PR

- [ ] `MxStatus` enum không thay đổi
- [ ] Tất cả output files cũ vẫn được ghi bình thường
- [ ] `has_mx_emails.txt` vẫn chứa TẤT CẢ HasMx (kể cả chưa SMTP verify)
- [ ] `ProcessingPayload` chỉ có thêm field, không mất field nào
- [ ] VPS down không crash Tauri app — graceful Inconclusive
- [ ] `Inconclusive` (DNS) và `SmtpInconclusive` nằm trong file riêng, không lẫn vào Dead
- [ ] Catch-all detection chạy trước RCPT TO của email thật
- [ ] PTR record VPS đã được set trước khi test SMTP
- [ ] `cargo test` pass
- [ ] `npm run build` pass
- [ ] Smoke test 6 case ở Bước 6 đều pass
