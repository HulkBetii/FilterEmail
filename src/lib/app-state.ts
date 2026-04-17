import type { Language } from "../i18n";

export type ActiveTab = "filter" | "verify";

export type ProcessingPayload = {
  processed_lines: number;
  progress_percent: number;
  invalid: number;
  public: number;
  edu: number;
  targeted: number;
  custom: number;
  duplicates: number;
  mx_dead: number;
  mx_has_mx: number;
  mx_a_fallback: number;
  mx_inconclusive: number;
  mx_parked: number;
  mx_disposable: number;
  mx_typo: number;
  smtp_deliverable: number;
  smtp_rejected: number;
  smtp_catchall: number;
  smtp_unknown: number;
  smtp_enabled: boolean;
  smtp_elapsed_ms: number;
  cache_hits: number;
  final_alive: number;
  final_dead: number;
  final_unknown: number;
  smtp_attempted_emails: number;
  smtp_cache_hits: number;
  smtp_coverage_percent: number;
  smtp_policy_blocked: number;
  smtp_temp_failure: number;
  smtp_mailbox_full: number;
  smtp_mailbox_disabled: number;
  smtp_bad_mailbox: number;
  smtp_bad_domain: number;
  smtp_network_error: number;
  smtp_protocol_error: number;
  smtp_timeout: number;
  elapsed_ms: number;
  output_dir?: string;
  current_domain?: string | null;
  current_email?: string | null;
};

export type BannerState =
  | { tone: "idle"; message: string }
  | { tone: "success"; message: string }
  | { tone: "error"; message: string };

export type HistoryEntry = {
  id: string;
  timestamp: number;
  fileNames: string[];
  mode: ActiveTab;
  stats: ProcessingPayload;
};

export const DEFAULT_TIMEOUT_MS = 1500;
export const DEFAULT_MAX_CONCURRENT = 40;
export const MAX_HISTORY_ENTRIES = 20;

export const initialStats: ProcessingPayload = {
  processed_lines: 0,
  progress_percent: 0,
  invalid: 0,
  public: 0,
  edu: 0,
  targeted: 0,
  custom: 0,
  duplicates: 0,
  mx_dead: 0,
  mx_has_mx: 0,
  mx_a_fallback: 0,
  mx_inconclusive: 0,
  mx_parked: 0,
  mx_disposable: 0,
  mx_typo: 0,
  smtp_deliverable: 0,
  smtp_rejected: 0,
  smtp_catchall: 0,
  smtp_unknown: 0,
  smtp_enabled: false,
  smtp_elapsed_ms: 0,
  cache_hits: 0,
  final_alive: 0,
  final_dead: 0,
  final_unknown: 0,
  smtp_attempted_emails: 0,
  smtp_cache_hits: 0,
  smtp_coverage_percent: 0,
  smtp_policy_blocked: 0,
  smtp_temp_failure: 0,
  smtp_mailbox_full: 0,
  smtp_mailbox_disabled: 0,
  smtp_bad_mailbox: 0,
  smtp_bad_domain: 0,
  smtp_network_error: 0,
  smtp_protocol_error: 0,
  smtp_timeout: 0,
  elapsed_ms: 0,
  current_domain: null,
  current_email: null,
};

export function basename(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

export function normalizeStats(
  value: Partial<ProcessingPayload> | null | undefined,
): ProcessingPayload {
  return {
    ...initialStats,
    ...value,
    output_dir: value?.output_dir,
    current_domain: value?.current_domain ?? null,
    current_email: value?.current_email ?? null,
  };
}

export function formatLocaleNumber(value: number, language: Language) {
  return value.toLocaleString(language === "vi" ? "vi-VN" : "en-US");
}

export function isVerifyStats(stats: ProcessingPayload) {
  return (
    stats.mx_dead > 0 ||
    stats.mx_has_mx > 0 ||
    stats.mx_a_fallback > 0 ||
    stats.mx_inconclusive > 0 ||
    stats.mx_parked > 0 ||
    stats.mx_disposable > 0 ||
    stats.mx_typo > 0 ||
    stats.smtp_enabled ||
    stats.smtp_deliverable > 0 ||
    stats.smtp_rejected > 0 ||
    stats.smtp_catchall > 0 ||
    stats.smtp_unknown > 0 ||
    stats.cache_hits > 0 ||
    stats.final_alive > 0 ||
    stats.final_dead > 0 ||
    stats.final_unknown > 0
  );
}
