<div align="center">
  <img src="src/assets/logo.png" alt="FilterEmail Logo" width="120" />
  <h1>FilterEmail</h1>
  <p><strong>Desktop app for filtering large email lists and running conservative DNS + SMTP verification.</strong></p>
  <p>Built with Tauri v2, Rust, React, Tailwind CSS, and an optional Axum-based SMTP VPS service.</p>
</div>

---

## Overview

FilterEmail processes TXT and CSV email lists with a streaming Rust backend, so large files can be scanned without loading the whole dataset into memory.

The desktop app currently has two user-facing modes:

1. `Basic Filter`
   - Syntax and category filtering only.
   - Writes T1 output files such as invalid, public, edu/gov, targeted, and other.
2. `Verify DNS`
   - Keeps the filter pass, then adds DNS verification.
   - Optionally adds SMTP verification through a VPS service.
   - Produces legacy T1/T2/T3 outputs plus final T4 `Alive / Dead / Unknown` outputs.

## Current Verification Model

The verify pipeline is intentionally conservative.

### Layer 1: DNS

Each email is parsed, the domain is normalized, and domains are deduplicated before DNS work.

Normalized domain rules:
- lowercase
- trim whitespace
- strip trailing `.`
- convert IDN to punycode

Current DNS statuses:
- `HasMx`
- `ARecordFallback`
- `Dead`
- `NullMx`
- `Parked`
- `Disposable`
- `TypoSuggestion(...)`
- `Inconclusive`

### Layer 2: SMTP

SMTP verification is optional and runs only when:
- verify mode is enabled
- `SMTP Verify (VPS)` is enabled
- the DNS result is `HasMx`

SMTP is now **per-email**, not sampled per domain.

The desktop app calls:
- `POST /verify/smtp/v2`

through the separate `verify-vps` service and receives a per-email result with SMTP code, enhanced code, reply text, MX host, catch-all flag, cache flag, and duration.

Current SMTP outcomes:
- `Accepted`
- `AcceptedForwarded`
- `CatchAll`
- `BadMailbox`
- `BadDomain`
- `PolicyBlocked`
- `MailboxFull`
- `MailboxDisabled`
- `TempFailure`
- `NetworkError`
- `ProtocolError`
- `Timeout`
- `Inconclusive`

### Final Triage

The app adds a final T4 triage layer:

- `Alive`
  - syntax-valid
  - DNS `HasMx`
  - SMTP `Accepted` or `AcceptedForwarded`
  - not catch-all
- `Dead`
  - invalid syntax
  - DNS `Dead` or `NullMx`
  - SMTP `BadMailbox` or `BadDomain`
- `Unknown`
  - everything else, including:
  - `ARecordFallback`
  - `Parked`
  - `Disposable`
  - `TypoSuggestion`
  - `Inconclusive`
  - `CatchAll`
  - `PolicyBlocked`
  - `MailboxFull`
  - `MailboxDisabled`
  - `TempFailure`
  - `NetworkError`
  - `ProtocolError`
  - `Timeout`

Important: `Alive` means high-confidence SMTP acceptance for that exact email address. It is **not** a guarantee of inbox placement.

## Architecture

### Desktop app

- Frontend: React + TypeScript + Tailwind CSS
- Backend: Tauri v2 + Rust
- Key crates:
  - `tokio`
  - `hickory-resolver`
  - `rusqlite`
  - `reqwest`

Main backend files:
- `src-tauri/src/main.rs`
- `src-tauri/src/processor.rs`
- `src-tauri/src/smtp_status.rs`
- `src-tauri/src/smtp_verify.rs`
- `src-tauri/src/smtp_client.rs`

### SMTP VPS service

The optional SMTP service lives in:
- `verify-vps/`

It is a separate Rust service built with:
- `axum`
- `tokio`
- `hickory-resolver`

Main VPS files:
- `verify-vps/src/main.rs`
- `verify-vps/src/smtp.rs`
- `verify-vps/src/catch_all.rs`
- `verify-vps/src/cache.rs`
- `verify-vps/src/rate_limiter.rs`

## Key Features

- Streaming file processing with flat memory usage
- DNS dedupe by normalized domain
- Per-email SMTP verification for `HasMx` emails
- Persistent desktop SQLite cache
- VPS in-memory SMTP and catch-all cache
- Real-time progress payloads with DNS, SMTP, and final T4 counters
- English and Vietnamese UI
- Saved run history in the desktop app

## Output Files

Current output filenames are:

### T1: Filter

- `01_T1_Valid_Public.txt`
- `02_T1_Valid_EduGov.txt`
- `03_T1_Valid_Targeted.txt`
- `04_T1_Valid_Other.txt`
- `05_T1_Invalid_Syntax.txt`

### T2: DNS

- `10_T2_DNS_Valid_Has_MX.txt`
- `11_T2_DNS_Valid_ARecord.txt`
- `12_T2_DNS_Error_Dead.txt`
- `13_T2_DNS_Risk_Parked.txt`
- `14_T2_DNS_Risk_Disposable.txt`
- `15_T2_DNS_Typo_Suggestion.txt`
- `16_T2_DNS_Inconclusive.txt`

### T3: SMTP

- `20_T3_SMTP_Deliverable.txt`
- `21_T3_SMTP_CatchAll.txt`
- `22_T3_SMTP_Rejected.txt`
- `23_T3_SMTP_Unknown.txt`

### T4: Final

- `30_T4_FINAL_Alive.txt`
- `31_T4_FINAL_Dead.txt`
- `32_T4_FINAL_Unknown.txt`
- `33_T4_FINAL_Detail.csv`

`33_T4_FINAL_Detail.csv` currently contains:
- `email`
- `final_status`
- `dns_status`
- `smtp_outcome`
- `smtp_basic_code`
- `smtp_enhanced_code`
- `smtp_reply_text`
- `mx_host`
- `catch_all`
- `smtp_cached`
- `tested_at`

## Caching

When the UI toggle `Persistent DNS Cache` is enabled, the desktop app stores verification data in SQLite under the app local data directory.

Current desktop cache behavior:
- DNS cache TTL: 6 hours
- SMTP cache TTL: 6 hours
- catch-all cache TTL: 6 hours

The UI label still says `Persistent DNS Cache`, but the same SQLite database is also used for SMTP result caching and catch-all caching.

## Local Development

### Prerequisites

- Node.js 18+
- Rust stable
- Tauri v2 system dependencies for your OS

### Desktop app

```bash
git clone https://github.com/HulkBetii/FilterEmail.git
cd FilterEmail
npm install
npm run tauri dev
```

Build production desktop binaries:

```bash
npm run tauri build
```

### Optional SMTP VPS service

Run the SMTP service locally:

```bash
cd verify-vps
API_KEY=your-secret-key \
SMTP_FROM_DOMAIN=yourdomain.com \
cargo run
```

Important environment variables:
- `API_KEY`
- `SMTP_FROM_DOMAIN`
- `BIND_ADDR` default: `0.0.0.0:3000`
- `SMTP_TIMEOUT_SECS` default: `8`
- `MAX_CONCURRENT_SMTP` default: `30`
- `TLS_CERT_PATH` optional
- `TLS_KEY_PATH` optional

The desktop app expects:
- base URL of the SMTP service
- Bearer API key

## Verify Mode Settings

Current verify-mode settings in the UI:
- `DNS Timeout (ms)` default: `1500`
- `Max Concurrent Lookups` default: `40`
- `Persistent DNS Cache`
- `SMTP Verify (VPS)`
- `VPS API URL`
- `API Key`

The desktop backend also exposes a `check_port_25` command to test whether outbound TCP port 25 is reachable from the current environment.

## Evaluation Workflow

To measure verify quality without changing backend logic, use:

- [docs/verify-evaluation.md](docs/verify-evaluation.md)

The helper script:

```bash
python3 tools/verify_eval.py prepare /path/to/33_T4_FINAL_Detail.csv /path/to/verify_review.csv
python3 tools/verify_eval.py score /path/to/verify_review.csv
```

This workflow tracks:
- `Alive precision`
- `Dead precision`
- `Coverage`

## Known Limits

- This tool is not inbox-placement verification.
- It does not log into a mailbox.
- `ARecordFallback` is not treated as `Alive`.
- `CatchAll` is not treated as `Alive`.
- Large freemail providers may return `PolicyBlocked` or other anti-abuse responses depending on VPS IP reputation.
- `Unknown` is expected and intentional for ambiguous cases.

## License

© 2026 HulkBetii. All rights reserved.

This application is proprietary software. Unauthorized copying, modification, or distribution is prohibited.
