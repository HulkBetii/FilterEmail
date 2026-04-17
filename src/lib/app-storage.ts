import {
  getSavedLanguage,
  persistLanguage,
  type Language,
} from "../i18n";
import {
  DEFAULT_MAX_CONCURRENT,
  DEFAULT_TIMEOUT_MS,
  type HistoryEntry,
  isVerifyStats,
  normalizeStats,
} from "./app-state";

const STORAGE_KEYS = {
  history: "filteremail-history",
  targetDomains: "targetDomains",
  checkMx: "checkMx",
  lastOutputDir: "lastOutputDir",
  timeoutMs: "deepDnsTimeoutMs",
  maxConcurrent: "deepDnsMaxConcurrent",
  persistentCache: "deepDnsPersistentCache",
  smtpEnabled: "smtpVerifyEnabled",
  vpsApiUrl: "smtpVerifyVpsApiUrl",
  vpsApiKey: "smtpVerifyVpsApiKey",
} as const;

export type PersistedSettings = {
  targetDomains: string;
  outputDir: string;
  lastOutputDir: string;
  timeoutMs: number;
  maxConcurrent: number;
  usePersistentCache: boolean;
  smtpEnabled: boolean;
  vpsApiUrl: string;
  vpsApiKey: string;
};

function parsePositiveNumber(
  value: string | null,
  fallback: number,
  minimum: number,
) {
  const parsed = Number(value ?? fallback);
  return Number.isFinite(parsed) && parsed > 0 ? Math.max(minimum, parsed) : fallback;
}

export function loadSavedHistory(): HistoryEntry[] {
  const saved = localStorage.getItem(STORAGE_KEYS.history);
  if (!saved) {
    return [];
  }

  try {
    const parsed = JSON.parse(saved) as Array<Partial<HistoryEntry>>;
    return parsed.map((entry) => ({
      id: entry.id ?? crypto.randomUUID(),
      timestamp: entry.timestamp ?? Date.now(),
      fileNames: entry.fileNames ?? [],
      mode:
        entry.mode === "filter" || entry.mode === "verify"
          ? entry.mode
          : isVerifyStats(normalizeStats(entry.stats))
            ? "verify"
            : "filter",
      stats: normalizeStats(entry.stats),
    }));
  } catch {
    return [];
  }
}

export function persistHistory(history: HistoryEntry[]) {
  localStorage.setItem(STORAGE_KEYS.history, JSON.stringify(history));
}

export function clearSavedHistory() {
  localStorage.removeItem(STORAGE_KEYS.history);
}

export function loadPersistedSettings(): PersistedSettings {
  void localStorage.getItem(STORAGE_KEYS.checkMx);

  const lastOutputDir = localStorage.getItem(STORAGE_KEYS.lastOutputDir) ?? "";

  return {
    targetDomains: localStorage.getItem(STORAGE_KEYS.targetDomains) ?? "",
    outputDir: lastOutputDir,
    lastOutputDir,
    timeoutMs: parsePositiveNumber(
      localStorage.getItem(STORAGE_KEYS.timeoutMs),
      DEFAULT_TIMEOUT_MS,
      250,
    ),
    maxConcurrent: parsePositiveNumber(
      localStorage.getItem(STORAGE_KEYS.maxConcurrent),
      DEFAULT_MAX_CONCURRENT,
      1,
    ),
    usePersistentCache:
      localStorage.getItem(STORAGE_KEYS.persistentCache) === "true",
    smtpEnabled: localStorage.getItem(STORAGE_KEYS.smtpEnabled) === "true",
    vpsApiUrl: localStorage.getItem(STORAGE_KEYS.vpsApiUrl) ?? "",
    vpsApiKey: localStorage.getItem(STORAGE_KEYS.vpsApiKey) ?? "",
  };
}

export function persistTargetDomains(value: string) {
  localStorage.setItem(STORAGE_KEYS.targetDomains, value);
}

export function persistTimeoutMs(value: number) {
  localStorage.setItem(STORAGE_KEYS.timeoutMs, String(value));
}

export function persistMaxConcurrent(value: number) {
  localStorage.setItem(STORAGE_KEYS.maxConcurrent, String(value));
}

export function persistPersistentCache(value: boolean) {
  localStorage.setItem(STORAGE_KEYS.persistentCache, value ? "true" : "false");
}

export function persistSmtpEnabled(value: boolean) {
  localStorage.setItem(STORAGE_KEYS.smtpEnabled, value ? "true" : "false");
}

export function persistVpsApiUrl(value: string) {
  localStorage.setItem(STORAGE_KEYS.vpsApiUrl, value);
}

export function persistVpsApiKey(value: string) {
  localStorage.setItem(STORAGE_KEYS.vpsApiKey, value);
}

export function persistLastOutputDir(value: string) {
  localStorage.setItem(STORAGE_KEYS.lastOutputDir, value);
}

export function loadSavedLanguagePreference(): Language {
  return getSavedLanguage();
}

export function persistLanguagePreference(language: Language) {
  persistLanguage(language);
}
