# UI Review & Đề xuất Cải tiến Frontend

## Phân tích Backend đã có gì MỚI (mà FE chưa hiển thị tốt)

### Tầng 1: Basic Filter (đã có, ổn)
- Phân loại: `invalid`, `public`, `edu`, `targeted`, `custom`, `duplicates` ✅

### Tầng 2: Deep DNS Scan (`check_mx = true`)
Các bucket MỚI backend đã xử lý nhưng FE chưa hiển thị rõ ràng:
- `mx_has_mx` — Domain có MX record hợp lệ ✅
- `mx_a_fallback` — Domain chỉ có A record (không có MX) ✅
- `mx_dead` — Domain chết (không resolve được) ✅
- `mx_inconclusive` — Không kết luận được ✅
- `mx_parked` — Domain đang parking (bán tên miền) ⚠️ Chỉ hiện trong stat card nhỏ
- `mx_disposable` — Email tạm thời (throw-away) ⚠️ Chỉ hiện trong stat card nhỏ
- `mx_typo` — Domain bị gõ sai chính tả ⚠️ Chỉ hiện trong stat card nhỏ
- `cache_hits` — Số domain lấy từ cache (SQLite) ⚠️ Gần như ẩn
- `current_domain` — Domain đang quét real-time ⚠️ Không hiển thị trong quá trình chạy

### Tầng 3: SMTP Verify (`smtp_enabled = true`, cần VPS API)
**Hoàn toàn mới** — FE chưa có UI cấu hình đẹp:
- `smtp_deliverable` — Email gửi được thực sự ✅ Có card nhưng không nổi bật
- `smtp_rejected` — Email bị từ chối SMTP ✅
- `smtp_catchall` — Server nhận tất cả (không verify được) ✅
- `smtp_unknown` — Không xác định được ✅
- `smtp_elapsed_ms` — Thời gian xử lý SMTP ❌ Không hiển thị

---

## Vấn đề UX hiện tại

| Vấn đề | Mức độ |
|--------|--------|
| Tab navigation bar (max-w-sm) bị nhỏ, tách biệt khỏi Header → cảm giác "nút rời" | 🔴 Cao |
| SMTP section: 2 ô text input VPS URL/Key nằm trần trong settings card → không chuyên nghiệp | 🔴 Cao |
| Tab 2 Verify: khi chạy xong, `mx_parked / mx_disposable / mx_typo` hiển thị ở stat cards nhỏ bên dưới, cách xa 4 card hero phía trên → giao diện bị vỡ nhịp | 🟡 Trung bình |
| Không có chỉ báo real-time domain đang scan (current_domain rất quan trọng với user) | 🟡 Trung bình |
| `cache_hits` và `smtp_elapsed_ms` bị bỏ hoàn toàn → user không biết cache có hoạt động không | 🟡 Trung bình |
| Nút Start không hiển thị cấu hình đang active (SMTP on/off) | 🟠 Thấp |

---

## Đề xuất Phương án Cải tiến

### 1. Tab Bar — Tích hợp vào Header
Đưa 2 tab vào cùng hàng Header (bên phải cạnh Language Switcher) → tạo cảm giác navigation liền mạch như một app desktop thực sự.

### 2. Tab 1 — Lọc Thông Thường (tidak ada change lớn)
UI đã ổn, chỉ thêm:
- Cho phép nhập nhiều dòng Target Domains (thay textarea cho input một dòng)

### 3. Tab 2 — Verify Email: 3 Sub-sections rõ ràng
Chia settings panel thành 3 khối với border/label riêng:

**Khối A — ⚙️ DNS Config** (Timeout, Concurrency, Cache):
- Giữ giống hiện tại nhưng có tiêu đề phân cách

**Khối B — 📧 SMTP Verify** (card riêng biệt, không nhúng vào cùng khối DNS):
- Card SMTP có nền gradient riêng để distinguishes rõ ràng
- Toggle On/Off rõ ràng hơn (dùng switch visual thay checkbox nhỏ)
- URL/API Key input chỉ hiện khi bật SMTP
- Badge hiển thị trạng thái: `SMTP: OFF` / `SMTP: Đã cấu hình` / `SMTP: Chờ VPS`

**Khối C — 🌐 Port 25 Status** (giữ nguyên)

### 4. Real-time Scanning Feedback — `current_domain`
Khi đang chạy Verify, hiển thị dòng nhỏ bên dưới progress bar:
```
🔍 Đang quét: gmail.com  |  Cache hits: 234  |  SMTP: 00:14s
```

### 5. Gộp các stat nhỏ vào nhóm Verify thành layout 3-column
Thay vì để `mx_parked`, `mx_disposable`, `mx_typo` nằm la liệt như stat cards độc lập, gộp chúng vào một khối "Review Required" (màu amber) tương tự các hero card phía trên.

---

## Chi tiết file sẽ thay đổi (FE ONLY)

| File | Thay đổi |
|------|----------|
| `App.tsx` | Tích hợp Tab vào Header; thêm current_domain indicator; restructure settings |
| `top-dashboard.tsx` | Thêm current_domain prop; hiển thị real-time scanning text |
| `verify-ui.tsx` | Thêm `VerifyReviewGroup` component gộp 3 stat nhỏ; thêm SMTP stats badge |
| `i18n.ts` | Thêm label mới: `smtpConfigured`, `smtpOff`, `cacheHits`, `scanningDomain` |

> [!IMPORTANT]
> **Bạn có muốn tôi thực hiện toàn bộ phương án này không?**
> Phần cốt lõi nhất (thứ tự ưu tiên):
> 1. ✅ Tích hợp Tab vào Header (thay layout hiện tại)
> 2. ✅ Chia 3 khối settings cho Tab Verify + SMTP card chuyên nghiệp
> 3. ✅ Hiển thị `current_domain` real-time
> 4. ✅ Gộp stat nhỏ thành "Review Group" 
>
> Nếu đồng ý, tôi sẽ thực hiện theo thứ tự ưu tiên trên.
