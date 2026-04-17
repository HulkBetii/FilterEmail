# Plan: Upgrade Verify DNS to DNS + SMTP Ping via VPS

## Summary
Upgrade the current `Verify DNS` flow from DNS-only verification to a two-layer model: keep the existing Tauri/Rust DNS pipeline unchanged in its core behavior, then add an SMTP verification phase that runs only for `MxStatus::HasMx` domains by calling a separate Rust/Axum VPS service over HTTPS. The implementation must preserve all current DNS contracts, output files, payload fields, dedup behavior, and `Inconclusive` handling, while adding SMTP-specific status, files, payload counts, settings, and UI sections.

## Key Changes

### 1. VPS service (`verify-vps/`) for SMTP probing
- Create a separate Rust/Axum project at `verify-vps/` with `main.rs`, `smtp.rs`, `catch_all.rs`, `rate_limiter.rs`, and `cache.rs`.
- Expose `POST /verify/smtp` with Bearer auth and batch request/response exactly as defined in the prompt.
- Implement SMTP sequence as `EHLO -> MAIL FROM -> RCPT TO -> QUIT` with 8s connect/probe timeout and no message sending.
- Implement catch-all detection first using a random UUID mailbox on the same domain, then only probe the real email if catch-all is false.
- Map SMTP responses to:
  - `Deliverable`: `250`, `251`
  - `Rejected`: `550`, `551`, `553`, `554`
  - `Inconclusive`: `4xx`, timeout, connect fail, refused
  - `CatchAll`: random mailbox accepted
- Add per-MX-host token-bucket rate limiting with provider-specific defaults from the prompt.
- Add VPS-side in-memory TTL cache:
  - key by full email for normal SMTP status
  - key by domain for catch-all
  - TTL 2h
- Keep scale assumptions aligned with 100K/day and a single-IP VPS.

### 2. Tauri backend: add SMTP client and composite verification result
- Add new backend modules:
  - `src-tauri/src/smtp_status.rs`
  - `src-tauri/src/smtp_verify.rs` or equivalent composite-result module
  - `src-tauri/src/smtp_client.rs`
- Keep `MxStatus` exactly unchanged.
- Add `SmtpStatus` enum exactly as specified.
- Add `DomainVerifyResult { dns: MxStatus, smtp: Option<SmtpStatus> }` plus `output_bucket()` helper to decide final file routing.
- Extend Tauri command input contract to accept SMTP settings without breaking existing callers:
  - `smtp_enabled: bool`
  - `vps_api_url: String`
  - `vps_api_key: String`
- In `smtp_client.rs`, implement `SmtpApiClient::verify_batch()` using `reqwest::Client`.
- If VPS is unreachable, times out, or misconfigured, return `SmtpStatus::Inconclusive` for all requested HasMx domains instead of failing the job.
- Keep DNS persistent cache contract unchanged:
  - same SQLite DB
  - same 6h TTL
  - same cache key/value for DNS layer only
- Do not store SMTP results in the existing DNS SQLite cache.

### 3. Integrate SMTP into the existing processor pipeline
- Keep current `processor.rs` DNS pipeline order unchanged:
  - cache
  - typo
  - disposable
  - MX
  - A fallback
  - parked
- Preserve domain normalization and domain deduplication exactly as-is.
- After DNS scan completes, collect only domains whose DNS result is `MxStatus::HasMx`.
- Build a domain-to-sample-email map from the current stream results so SMTP can probe one real email per HasMx domain.
- Batch-call `SmtpApiClient::verify_batch()` only when:
  - `check_mx = true`
  - `smtp_enabled = true`
  - there is at least one `HasMx` domain
- Merge DNS + SMTP into `DomainVerifyResult` and route outputs as:
  - keep current files exactly as before
  - continue writing all `HasMx` entries to `has_mx_emails.txt`
  - add new SMTP files:
    - `smtp_deliverable_emails.txt`
    - `smtp_rejected_emails.txt`
    - `smtp_catchall_emails.txt`
    - `smtp_unknown_emails.txt`
- Add payload fields without removing any existing fields:
  - `smtp_deliverable`
  - `smtp_rejected`
  - `smtp_catchall`
  - `smtp_unknown`
  - `smtp_enabled`
  - `smtp_elapsed_ms`
- Update stats aggregation and emitted progress/final payloads so the frontend can render SMTP results independently from DNS counts.

### 4. Frontend: verify settings and SMTP result sections
- Extend the current verify-mode UI to include SMTP settings in the existing verify settings area:
  - toggle `SMTP Verify (VPS)`
  - input `VPS API URL`
  - input `API Key`
- Persist these values in `localStorage` alongside current verify settings.
- Pass the new SMTP config to the Tauri `process_file` invoke call.
- Add SMTP result display blocks in the already-modularized verify UI:
  - top dashboard / metric area
  - final summary
  - history modal
- Show the new SMTP section only when `payload.smtp_enabled = true`.
- Keep all existing DNS UI intact; this is additive, not a replacement.
- Update `i18n.ts` with the new SMTP-related keys only; do not rename or remove existing keys.

## Public API / Interface Changes
- Tauri command `process_file(...)` adds:
  - `smtp_enabled: bool`
  - `vps_api_url: String`
  - `vps_api_key: String`
- `ProcessingPayload` adds:
  - `smtp_deliverable: u64`
  - `smtp_rejected: u64`
  - `smtp_catchall: u64`
  - `smtp_unknown: u64`
  - `smtp_enabled: bool`
  - `smtp_elapsed_ms: u64`
- New backend enums/types:
  - `SmtpStatus`
  - `DomainVerifyResult`
- New VPS API:
  - `POST /verify/smtp` with the batch request/response contract from the prompt

## Test Plan
- Backend unit tests:
  - `MxStatus` unchanged and still serializes/deserializes as before
  - `SmtpStatus` mapping for accepted, rejected, temp fail, timeout
  - `DomainVerifyResult::output_bucket()` covers all DNS/SMTP combinations
  - SMTP client returns `Inconclusive` on VPS failure instead of crashing the job
- Processor integration tests:
  - `HasMx` domains still go to `has_mx_emails.txt` regardless of SMTP result
  - `ARecordFallback`, `Dead`, `Parked`, `Disposable`, `TypoSuggestion`, `Inconclusive` skip SMTP entirely
  - new `smtp_*.txt` files are written correctly when SMTP is enabled
  - DNS-only mode still works with no SMTP config
- VPS tests:
  - catch-all probe runs before real RCPT probe
  - per-MX rate limiting enforces provider caps
  - email cache vs domain catch-all cache behaves as specified
- Frontend checks:
  - SMTP settings persist across reload
  - verify UI renders SMTP sections only when enabled
  - history and final summary render new SMTP counts correctly
- Acceptance smoke tests with real VPS:
  - `valid@gmail.com` => `HasMx + Deliverable`
  - `definitelynotreal99999@gmail.com` => `HasMx + Rejected`
  - catch-all domain => `HasMx + CatchAll`
  - dead domain => `Dead` and no SMTP call
  - VPS down => `HasMx + smtp_unknown` / `Inconclusive`, no app crash
  - `cargo test` pass
  - `npm run build` pass

## Assumptions and Defaults
- The VPS service lives inside the same repository as a sibling project under `verify-vps/`, not a separate repo.
- SMTP config is user-configurable from the desktop app and stored in localStorage; no secure credential vault is introduced in this phase.
- `smtp_unknown` is the frontend/output bucket for both `SmtpStatus::Inconclusive` and “SMTP not executed after HasMx because VPS was unavailable”.
- DNS persistent cache remains DNS-only; SMTP caching exists only on the VPS in-memory layer for this phase.
- Rollout target is additive backward compatibility: existing DNS users can continue running verify mode with SMTP disabled and see the current behavior unchanged.
