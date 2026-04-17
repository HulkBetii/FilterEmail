# Compatibility Invariants

This refactor preserves the external contracts below. Future cleanup should treat these as stable unless a deliberate compatibility change is planned.

## Tauri commands

- `process_file(...)`
- `check_port_25()`

The command names, argument shape, and snake_case wire contract must remain unchanged.

## Tauri events

- `processing-progress`
- `processing-complete`
- `processing-error`

Frontend listeners depend on these exact event names and payload shapes.

## Frontend persistence keys

- `filteremail-history`
- `targetDomains`
- `checkMx`
- `lastOutputDir`
- `deepDnsTimeoutMs`
- `deepDnsMaxConcurrent`
- `deepDnsPersistentCache`
- `smtpVerifyEnabled`
- `smtpVerifyVpsApiUrl`
- `smtpVerifyVpsApiKey`

Existing local data should continue loading without migration.

## Output files

The output directory layout and filenames remain unchanged:

- `01_T1_Valid_Public.txt`
- `02_T1_Valid_EduGov.txt`
- `03_T1_Valid_Targeted.txt`
- `04_T1_Valid_Other.txt`
- `05_T1_Invalid_Syntax.txt`
- `10_T2_DNS_Valid_Has_MX.txt`
- `11_T2_DNS_Valid_ARecord.txt`
- `12_T2_DNS_Error_Dead.txt`
- `13_T2_DNS_Risk_Parked.txt`
- `14_T2_DNS_Risk_Disposable.txt`
- `15_T2_DNS_Typo_Suggestion.txt`
- `16_T2_DNS_Inconclusive.txt`
- `20_T3_SMTP_Deliverable.txt`
- `21_T3_SMTP_CatchAll.txt`
- `22_T3_SMTP_Rejected.txt`
- `23_T3_SMTP_Unknown.txt`
- `30_T4_FINAL_Alive.txt`
- `31_T4_FINAL_Dead.txt`
- `32_T4_FINAL_Unknown.txt`
- `33_T4_FINAL_Detail.csv`

## Detail CSV columns

`33_T4_FINAL_Detail.csv` keeps this exact column order:

1. `email`
2. `final_status`
3. `dns_status`
4. `smtp_outcome`
5. `smtp_basic_code`
6. `smtp_enhanced_code`
7. `smtp_reply_text`
8. `mx_host`
9. `catch_all`
10. `smtp_cached`
11. `tested_at`
