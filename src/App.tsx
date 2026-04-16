import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  AlertCircle,
  CheckCircle,
  ChevronDown,
  ChevronUp,
  Clock,
  Copy,
  FileSpreadsheet,
  FolderOpen,
  History,
  Info,
  LoaderCircle,
  Mail,
  SearchCheck,
  ShieldCheck,
  Target,
  Trash2,
  CloudUpload,
  Users,
  XCircle,
} from "lucide-react";
import appLogo from "./assets/logo.png";
import { FinalSummary } from "./components/final-summary";
import { HistoryModal } from "./components/history-modal";
import { TopDashboard } from "./components/top-dashboard";
import {
  VerifyHeroCard,
  type VerifyBucketKey,
} from "./components/verify-ui";
import {
  formatBackendError,
  getSavedLanguage,
  persistLanguage,
  translations,
  type ErrorPayload,
  type Language,
} from "./i18n";

type ProcessingPayload = {
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
  elapsed_ms: number;
  output_dir?: string;
  current_domain?: string | null;
};

type BannerState =
  | { tone: "idle"; message: string }
  | { tone: "success"; message: string }
  | { tone: "error"; message: string };

type HistoryEntry = {
  id: string;
  timestamp: number;
  fileNames: string[];
  mode: "filter" | "verify";
  stats: ProcessingPayload;
};

const initialStats: ProcessingPayload = {
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
  elapsed_ms: 0,
  current_domain: null,
};

const DEFAULT_TIMEOUT_MS = 1500;
const DEFAULT_MAX_CONCURRENT = 40;

const statCards = [
  {
    key: "invalid" as const,
    icon: XCircle,
    chip: "bg-red-50 text-red-700 ring-red-100",
  },
  {
    key: "public" as const,
    icon: Users,
    chip: "bg-blue-50 text-blue-700 ring-blue-100",
  },
  {
    key: "edu" as const,
    icon: ShieldCheck,
    chip: "bg-emerald-50 text-emerald-700 ring-emerald-100",
  },
  {
    key: "targeted" as const,
    icon: Target,
    chip: "bg-fuchsia-50 text-fuchsia-700 ring-fuchsia-100",
  },
  {
    key: "custom" as const,
    icon: Mail,
    chip: "bg-amber-50 text-amber-700 ring-amber-100",
  },
  {
    key: "duplicates" as const,
    icon: Copy,
    chip: "bg-slate-50 text-slate-700 ring-slate-200",
  },
  {
    key: "mx_disposable" as const,
    icon: Trash2,
    chip: "bg-orange-50 text-orange-700 ring-orange-100",
  },
  {
    key: "mx_has_mx" as const,
    icon: CheckCircle,
    chip: "bg-emerald-50 text-emerald-700 ring-emerald-100",
  },
  {
    key: "mx_a_fallback" as const,
    icon: FolderOpen,
    chip: "bg-cyan-50 text-cyan-700 ring-cyan-100",
  },
  {
    key: "mx_typo" as const,
    icon: SearchCheck,
    chip: "bg-violet-50 text-violet-700 ring-violet-100",
  },
  {
    key: "mx_parked" as const,
    icon: AlertCircle,
    chip: "bg-amber-50 text-amber-700 ring-amber-100",
  },
];

function basename(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function normalizeStats(value: Partial<ProcessingPayload> | null | undefined): ProcessingPayload {
  return {
    ...initialStats,
    ...value,
    output_dir: value?.output_dir,
    current_domain: value?.current_domain ?? null,
  };
}

function formatLocaleNumber(value: number, language: Language) {
  return value.toLocaleString(language === "vi" ? "vi-VN" : "en-US");
}

function isVerifyStats(stats: ProcessingPayload) {
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
    stats.cache_hits > 0
  );
}

export default function App() {
  const [language, setLanguage] = useState<Language>(getSavedLanguage);
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [timeoutMs, setTimeoutMs] = useState(DEFAULT_TIMEOUT_MS);
  const [maxConcurrent, setMaxConcurrent] = useState(DEFAULT_MAX_CONCURRENT);
  const [usePersistentCache, setUsePersistentCache] = useState(false);
  const [showAdvancedDns, setShowAdvancedDns] = useState(false);
  const [smtpEnabled, setSmtpEnabled] = useState(false);
  const [vpsApiUrl, setVpsApiUrl] = useState("");
  const [vpsApiKey, setVpsApiKey] = useState("");

  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);
  const [outputDir, setOutputDir] = useState("");
  const [targetDomains, setTargetDomains] = useState("");
  const [lastOutputDir, setLastOutputDir] = useState("");
  const [dragActive, setDragActive] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [stats, setStats] = useState<ProcessingPayload>(initialStats);
  const t = translations[language];
  const [banner, setBanner] = useState<BannerState>({
    tone: "idle",
    message: translations.en.idleBanner,
  });
  const [activeTab, setActiveTab] = useState<"filter" | "verify">("filter");
  const verifyMode = activeTab === "verify";

  useEffect(() => {
    const saved = localStorage.getItem("filteremail-history");
    if (saved) {
      try {
        const parsed = JSON.parse(saved) as Array<Partial<HistoryEntry>>;
        setHistory(
          parsed.map((entry) => ({
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
          })),
        );
      } catch {
        setHistory([]);
      }
    }
  }, []);

  useEffect(() => {
    const savedDomains = localStorage.getItem("targetDomains");
    if (savedDomains) setTargetDomains(savedDomains);
    const savedMx = localStorage.getItem("checkMx");
    if (savedMx) { /* legacy: tab state now drives check_mx */ }
    const savedOut = localStorage.getItem("lastOutputDir");
    if (savedOut) {
      setOutputDir(savedOut);
      setLastOutputDir(savedOut);
    }
    const savedTimeout = Number(localStorage.getItem("deepDnsTimeoutMs") ?? DEFAULT_TIMEOUT_MS);
    if (Number.isFinite(savedTimeout) && savedTimeout > 0) {
      setTimeoutMs(savedTimeout);
    }
    const savedConcurrent = Number(
      localStorage.getItem("deepDnsMaxConcurrent") ?? DEFAULT_MAX_CONCURRENT,
    );
    if (Number.isFinite(savedConcurrent) && savedConcurrent > 0) {
      setMaxConcurrent(savedConcurrent);
    }
    const savedPersistentCache = localStorage.getItem("deepDnsPersistentCache");
    if (savedPersistentCache === "true") {
      setUsePersistentCache(true);
    }
    const savedSmtpEnabled = localStorage.getItem("smtpVerifyEnabled");
    if (savedSmtpEnabled === "true") {
      setSmtpEnabled(true);
    }
    const savedVpsUrl = localStorage.getItem("smtpVerifyVpsApiUrl");
    if (savedVpsUrl) {
      setVpsApiUrl(savedVpsUrl);
    }
    const savedVpsKey = localStorage.getItem("smtpVerifyVpsApiKey");
    if (savedVpsKey) {
      setVpsApiKey(savedVpsKey);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("targetDomains", targetDomains);
  }, [targetDomains]);

  useEffect(() => {
    localStorage.setItem("deepDnsTimeoutMs", String(timeoutMs));
  }, [timeoutMs]);

  useEffect(() => {
    localStorage.setItem("deepDnsMaxConcurrent", String(maxConcurrent));
  }, [maxConcurrent]);

  useEffect(() => {
    localStorage.setItem("deepDnsPersistentCache", usePersistentCache ? "true" : "false");
  }, [usePersistentCache]);

  useEffect(() => {
    localStorage.setItem("smtpVerifyEnabled", smtpEnabled ? "true" : "false");
  }, [smtpEnabled]);

  useEffect(() => {
    localStorage.setItem("smtpVerifyVpsApiUrl", vpsApiUrl);
  }, [vpsApiUrl]);

  useEffect(() => {
    localStorage.setItem("smtpVerifyVpsApiKey", vpsApiKey);
  }, [vpsApiKey]);

  useEffect(() => {
    if (outputDir) localStorage.setItem("lastOutputDir", outputDir);
  }, [outputDir]);

  useEffect(() => {
    persistLanguage(language);
  }, [language]);

  useEffect(() => {
    if (isProcessing && stats.processed_lines > 0) {
      setBanner({
        tone: "idle",
        message: t.progressBanner(stats.processed_lines, stats.current_domain),
      });
      return;
    }

    if (banner.tone === "success") {
      setBanner({
        tone: "success",
        message: t.completeBanner,
      });
      return;
    }

    if (banner.tone === "idle") {
      if (selectedFiles.length > 0) {
        setBanner({
          tone: "idle",
          message:
            selectedFiles.length === 1
              ? t.selectedFileBanner(basename(selectedFiles[0]))
              : `Đã chọn ${selectedFiles.length} tệp.`,
        });
      } else if (outputDir) {
        setBanner({
          tone: "idle",
          message: t.selectedOutputBanner,
        });
      } else {
        setBanner({
          tone: "idle",
          message: t.idleBanner,
        });
      }
    }
  }, [
    banner.tone,
    isProcessing,
    language,
    outputDir,
    selectedFiles,
    stats.current_domain,
    stats.processed_lines,
    t,
  ]);

  useEffect(() => {
    let mounted = true;

    const setupListeners = async () => {
      const unlistenProgress = await listen<ProcessingPayload>("processing-progress", ({ payload }) => {
        if (!mounted) return;
        const normalized = normalizeStats(payload);
        setStats(normalized);
        setIsProcessing(true);
        setBanner({
          tone: "idle",
          message: translations[language].progressBanner(
            normalized.processed_lines,
            normalized.current_domain,
          ),
        });
      });

      const unlistenComplete = await listen<ProcessingPayload>("processing-complete", ({ payload }) => {
        if (!mounted) return;
        const normalized = normalizeStats(payload);
        setStats(normalized);
        setIsProcessing(false);
        setLastOutputDir(normalized.output_dir ?? "");
        setBanner({
          tone: "success",
          message: translations[language].completeBanner,
        });

        const newEntry: HistoryEntry = {
          id: crypto.randomUUID(),
          timestamp: Date.now(),
          fileNames: selectedFiles.map((file) => basename(file)),
          mode: verifyMode ? "verify" : "filter",
          stats: normalized,
        };

        setHistory((prev) => {
          const next = [newEntry, ...prev].slice(0, 20);
          localStorage.setItem("filteremail-history", JSON.stringify(next));
          return next;
        });

        isPermissionGranted().then((granted) => {
          if (!granted) {
            requestPermission().then((result) => {
              if (result === "granted") {
                sendNotification({ title: "Hoàn tất", body: "Quá trình lọc email đã xong!" });
              }
            });
          } else {
            sendNotification({ title: "Hoàn tất", body: "Quá trình lọc email đã xong!" });
          }
        });
      });

      const unlistenError = await listen<ErrorPayload>("processing-error", ({ payload }) => {
        if (!mounted) return;
        setIsProcessing(false);
        setBanner({
          tone: "error",
          message: formatBackendError(payload, language),
        });
      });

      const unlistenDragDrop = await getCurrentWindow().onDragDropEvent((event) => {
        if (!mounted) return;

        switch (event.payload.type) {
          case "enter":
          case "over":
            setDragActive(true);
            break;
          case "leave":
            setDragActive(false);
            break;
          case "drop": {
            setDragActive(false);
            const paths = event.payload.paths;
            if (paths && paths.length > 0) {
              setSelectedFiles(paths);
              setBanner({
                tone: "idle",
                message:
                  paths.length === 1
                    ? translations[language].selectedFileBanner(basename(paths[0]))
                    : `Đã chọn ${paths.length} tệp.`,
              });
            }
            break;
          }
        }
      });

      return () => {
        unlistenProgress();
        unlistenComplete();
        unlistenError();
        unlistenDragDrop();
      };
    };

    const cleanupPromise = setupListeners();

    return () => {
      mounted = false;
      void cleanupPromise.then((cleanup) => cleanup());
    };
  }, [language, selectedFiles]);

  const canOpenFolder = Boolean(lastOutputDir || outputDir);
  const resolvedOutputDir = lastOutputDir || outputDir;

  const totalClassified = useMemo(
    () =>
      stats.invalid +
      stats.public +
      stats.edu +
      stats.targeted +
      stats.custom +
      stats.duplicates +
      stats.mx_dead +
      stats.mx_has_mx +
      stats.mx_a_fallback +
      stats.mx_inconclusive +
      stats.mx_parked +
      stats.mx_disposable +
      stats.mx_typo,
    [stats],
  );

  const invalidRate = totalClassified === 0 ? 0 : (stats.invalid / totalClassified) * 100;
  const publicRate = totalClassified === 0 ? 0 : (stats.public / totalClassified) * 100;
  const eduRate = totalClassified === 0 ? 0 : (stats.edu / totalClassified) * 100;
  const targetedRate = totalClassified === 0 ? 0 : (stats.targeted / totalClassified) * 100;
  const customRate = totalClassified === 0 ? 0 : (stats.custom / totalClassified) * 100;
  const verifyDeliverableCount = stats.mx_has_mx + stats.mx_a_fallback;
  const verifyDomainCount =
    stats.mx_dead +
    stats.mx_has_mx +
    stats.mx_a_fallback +
    stats.mx_inconclusive +
    stats.mx_parked +
    stats.mx_disposable +
    stats.mx_typo;
  const verifyReviewCount =
    stats.mx_inconclusive + stats.mx_parked + stats.mx_disposable + stats.mx_typo;
  const verifyDeliverableRate =
    totalClassified === 0 ? 0 : (verifyDeliverableCount / totalClassified) * 100;
  const verifyDeadRate = totalClassified === 0 ? 0 : (stats.mx_dead / totalClassified) * 100;
  const verifyReviewRate =
    totalClassified === 0 ? 0 : (verifyReviewCount / totalClassified) * 100;
  const verifyFallbackRate =
    totalClassified === 0 ? 0 : (stats.mx_a_fallback / totalClassified) * 100;
  const verifyParkedRate =
    totalClassified === 0 ? 0 : (stats.mx_parked / totalClassified) * 100;
  const verifyDisposableRate =
    totalClassified === 0 ? 0 : (stats.mx_disposable / totalClassified) * 100;
  const verifyTypoRate = totalClassified === 0 ? 0 : (stats.mx_typo / totalClassified) * 100;
  const smtpCheckedCount =
    stats.smtp_deliverable + stats.smtp_rejected + stats.smtp_catchall + stats.smtp_unknown;
  const smtpDeliverableRate =
    smtpCheckedCount === 0 ? 0 : (stats.smtp_deliverable / smtpCheckedCount) * 100;
  const smtpRejectedRate =
    smtpCheckedCount === 0 ? 0 : (stats.smtp_rejected / smtpCheckedCount) * 100;
  const smtpCatchallRate =
    smtpCheckedCount === 0 ? 0 : (stats.smtp_catchall / smtpCheckedCount) * 100;
  const smtpUnknownRate =
    smtpCheckedCount === 0 ? 0 : (stats.smtp_unknown / smtpCheckedCount) * 100;
  const validCount =
    activeTab === "verify"
      ? verifyDeliverableCount
      : stats.public + stats.edu + stats.targeted + stats.custom;

  const pickInputFile = async () => {
    const selected = await openDialog({
      multiple: true,
      directory: false,
      filters: [{ name: "Email Lists", extensions: ["txt", "csv"] }],
    });

    if (typeof selected === "string") {
      setSelectedFiles([selected]);
      setBanner({
        tone: "idle",
        message: t.selectedFileBanner(basename(selected)),
      });
    } else if (Array.isArray(selected)) {
      setSelectedFiles(selected);
      setBanner({
        tone: "idle",
        message: `Đã chọn ${selected.length} tệp.`,
      });
    }
  };

  const pickOutputDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
    });

    if (typeof selected === "string") {
      setOutputDir(selected);
      setLastOutputDir(selected);
      setBanner({
        tone: "idle",
        message: t.selectedOutputBanner,
      });
    }
  };

  const handleProcess = async () => {
    if (selectedFiles.length === 0 || !outputDir || isProcessing) {
      return;
    }

    setIsProcessing(true);
    setLastOutputDir(outputDir);
    setStats(initialStats);
    setBanner({
      tone: "idle",
      message: t.preparingBanner,
    });

    try {
      await invoke("process_file", {
        file_paths: selectedFiles,
        output_dir: outputDir,
        target_domains: activeTab === "filter" ? targetDomains : "",
        check_mx: verifyMode,
        timeout_ms: timeoutMs,
        max_concurrent: maxConcurrent,
        use_persistent_cache: usePersistentCache,
        smtp_enabled: verifyMode ? smtpEnabled : false,
        vps_api_url: verifyMode ? vpsApiUrl : "",
        vps_api_key: verifyMode ? vpsApiKey : "",
      });
    } catch (error) {
      console.error("Invoke Error:", error);
      setIsProcessing(false);
      setBanner({
        tone: "error",
        message:
          typeof error === "string"
            ? error
            : error instanceof Error
              ? error.message
              : t.labels.genericBackendError,
      });
    }
  };

  const openResultFolder = async () => {
    if (!resolvedOutputDir) return;
    try {
      await revealItemInDir(resolvedOutputDir);
    } catch (error) {
      console.error(error);
      await openPath(resolvedOutputDir).catch(console.error);
    }
  };

  return (
    <main className="min-h-screen bg-slate-50 font-sans text-slate-900">
      <div className="mx-auto w-full max-w-7xl space-y-5 p-4 sm:p-6 lg:p-8">
        <header className="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-white px-5 py-3 shadow-sm ring-1 ring-slate-900/5">
          <div className="flex min-w-0 items-center gap-3">
            <img
              src={appLogo}
              alt="FilterEmail logo"
              className="h-12 w-12 shrink-0 rounded-2xl object-cover shadow-md shadow-sky-500/20 ring-1 ring-sky-100"
            />
            <div className="min-w-0">
              <p className="truncate text-base font-bold leading-tight text-slate-800">FilterEmail Desktop</p>
              <p className="truncate text-[11px] font-medium text-slate-400">{t.labels.heroBadge}</p>
            </div>
          </div>

          {/* ── Tab navigation (centre) ── */}
          <div className="flex space-x-1 rounded-[1.25rem] bg-slate-100 p-1">
            <button
              onClick={() => setActiveTab("filter")}
              className={`flex items-center gap-1.5 rounded-xl px-5 py-2 text-sm font-semibold transition-all duration-200 ${
                activeTab === "filter"
                  ? "bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/8"
                  : "text-slate-500 hover:text-slate-800"
              }`}
            >
              <Mail className="h-3.5 w-3.5 shrink-0" />
              {t.labels.tabBasicFilter}
            </button>
            <button
              onClick={() => setActiveTab("verify")}
              className={`flex items-center gap-1.5 rounded-xl px-5 py-2 text-sm font-semibold transition-all duration-200 ${
                activeTab === "verify"
                  ? "bg-slate-900 text-white shadow-md shadow-slate-900/20"
                  : "text-slate-500 hover:text-slate-800"
              }`}
            >
              <ShieldCheck className="h-3.5 w-3.5 shrink-0" />
              {t.labels.tabDnsVerify}
            </button>
          </div>

          <div className="flex shrink-0 items-center gap-2">
            <button
              onClick={() => setIsHistoryOpen(true)}
              className="flex items-center gap-1.5 rounded-full bg-sky-50 px-3 py-1.5 text-xs font-semibold text-sky-700 ring-1 ring-sky-200 transition hover:bg-sky-100"
            >
              <History className="h-3.5 w-3.5 shrink-0" />
              <span>{t.labels.openHistory}</span>
            </button>
            <div className="flex rounded-full bg-slate-100 p-1">
              {(["en", "vi"] as const).map((lang) => (
                <button
                  key={lang}
                  onClick={() => setLanguage(lang)}
                  className={`rounded-full px-3 py-1 text-xs font-bold transition ${
                    language === lang
                      ? "bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/10"
                      : "text-slate-500 hover:text-slate-700"
                  }`}
                >
                  {lang === "en" ? t.labels.english : t.labels.vietnamese}
                </button>
              ))}
            </div>
          </div>
        </header>

        {banner.tone !== "idle" && (
          <div
            className={`flex min-w-0 items-start gap-3 rounded-2xl border p-4 text-sm font-medium ${
              banner.tone === "error"
                ? "border-red-200 bg-red-50 text-red-800"
                : "border-emerald-200 bg-emerald-50 text-emerald-800"
            }`}
          >
            {banner.tone === "error" ? (
              <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
            ) : (
              <CheckCircle className="mt-0.5 h-5 w-5 shrink-0" />
            )}
            <p className="min-w-0 break-words leading-relaxed">{banner.message}</p>
          </div>
        )}

        <TopDashboard
          activeTab={activeTab}
          language={language}
          dragActive={dragActive}
          totalClassified={totalClassified}
          progressPercent={stats.progress_percent}
          isProcessing={isProcessing}
          currentDomain={stats.current_domain ?? null}
          cacheHits={stats.cache_hits}
          labels={t.labels}
          canOpenFolder={canOpenFolder}
          onPickInputFile={pickInputFile}
          onOpenResultFolder={openResultFolder}
          onDragOver={(event) => {
            event.preventDefault();
            setDragActive(true);
          }}
          onDragLeave={() => setDragActive(false)}
          formatNumber={(value) => formatLocaleNumber(value, language)}
        />

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
          <div className="flex flex-col gap-4 lg:col-span-5">
            <div className="flex-1 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-slate-100">
              <div className="space-y-4">
                <div>
                  <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{t.labels.selectedFile}</label>
                  <div className="mt-1.5 flex min-w-0 items-center gap-2 rounded-2xl bg-slate-50 px-3 py-2.5 ring-1 ring-slate-100">
                    <FileSpreadsheet className="h-4 w-4 shrink-0 text-slate-400" />
                    <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-700">
                      {selectedFiles.length > 0
                        ? selectedFiles.length === 1
                          ? basename(selectedFiles[0])
                          : `${selectedFiles.length} ${language === "vi" ? "tệp" : "files"}`
                        : t.labels.noFile}
                    </span>
                  </div>
                </div>

                <div>
                  <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{t.labels.outputFolder}</label>
                  <div className="mt-1.5 flex min-w-0 items-center gap-2">
                    <div className="flex min-w-0 flex-1 items-center gap-2 rounded-2xl bg-slate-50 px-3 py-2.5 ring-1 ring-slate-100">
                      <FolderOpen className="h-4 w-4 shrink-0 text-slate-400" />
                      <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-600">
                        {outputDir || t.labels.noFolder}
                      </span>
                    </div>
                    <button
                      onClick={pickOutputDir}
                      className="shrink-0 rounded-2xl bg-slate-100 px-3 py-2.5 text-sm font-bold text-slate-700 transition hover:bg-slate-200"
                    >
                      {t.labels.selectFolder}
                    </button>
                  </div>
                </div>

                {activeTab === "filter" && (
                  <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                    <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                      {t.labels.targetedInputLabel}
                    </label>
                    <textarea
                      rows={3}
                      value={targetDomains}
                      onChange={(event) => setTargetDomains(event.target.value)}
                      placeholder={t.labels.targetedInputPlaceholder}
                      className="mt-1.5 w-full resize-none rounded-2xl bg-slate-50 px-4 py-3 text-sm text-slate-900 ring-1 ring-slate-100 transition placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-400/60"
                    />
                    <p className="mt-1 text-[11px] leading-relaxed text-slate-400">
                      {language === "vi" ? "Mỗi dòng một domain, hoặc phân cách bằng dấu phẩy." : "One domain per line, or separated by commas."}
                    </p>
                  </div>
                )}

                {activeTab === "verify" && (
                  <div className="animate-in fade-in slide-in-from-bottom-2 flex flex-col gap-4 duration-300">

                    {/* ── Section A: DNS Config ── */}
                    <div className="rounded-2xl border border-slate-200 bg-slate-50/50 p-4">
                      <div className="mb-3 flex items-center justify-between">
                        <p className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                          {language === "vi" ? "⚙️ Cấu hình DNS" : "⚙️ DNS Config"}
                        </p>
                        <button
                          onClick={() => setShowAdvancedDns(!showAdvancedDns)}
                          className="flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-bold text-slate-500 transition-colors hover:bg-slate-200 hover:text-slate-800"
                        >
                          {language === "vi" ? "Nâng cao" : "Advanced"}
                          {showAdvancedDns ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
                        </button>
                      </div>

                      {showAdvancedDns && (
                        <div className="mb-3 grid animate-in fade-in slide-in-from-top-2 grid-cols-1 gap-3 px-1 sm:grid-cols-2">
                          <div>
                            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                              {t.labels.timeoutLabel}
                            </label>
                            <input
                              type="number"
                              min={250}
                              max={5000}
                              step={50}
                              value={timeoutMs}
                              onChange={(event) =>
                                setTimeoutMs(
                                  Math.max(
                                    250,
                                    Math.min(5000, Number(event.target.value) || DEFAULT_TIMEOUT_MS),
                                  ),
                                )
                              }
                              className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-slate-200 transition focus:outline-none focus:ring-2 focus:ring-sky-400/60"
                            />
                            <p className="mt-1 text-[11px] leading-relaxed text-slate-400">{t.labels.timeoutHint}</p>
                          </div>
                          <div>
                            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                              {t.labels.concurrencyLabel}
                            </label>
                            <input
                              type="number"
                              min={1}
                              max={50}
                              step={1}
                              value={maxConcurrent}
                              onChange={(event) =>
                                setMaxConcurrent(
                                  Math.max(
                                    1,
                                    Math.min(50, Number(event.target.value) || DEFAULT_MAX_CONCURRENT),
                                  ),
                                )
                              }
                              className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-slate-200 transition focus:outline-none focus:ring-2 focus:ring-sky-400/60"
                            />
                            <p className="mt-1 text-[11px] leading-relaxed text-slate-400">{t.labels.concurrencyHint}</p>
                          </div>
                        </div>
                      )}

                      {/* Persistent Cache toggle */}
                      <label
                        className="mt-3 flex cursor-pointer items-center justify-between gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2.5 transition hover:bg-slate-50"
                        onClick={() => setUsePersistentCache((v) => !v)}
                      >
                        <div className="min-w-0">
                          <span className="text-sm font-semibold text-slate-700">{t.labels.persistentCacheLabel}</span>
                          <p className="mt-0.5 text-[11px] leading-relaxed text-slate-400">{t.labels.persistentCacheHint}</p>
                        </div>
                        <div className="relative shrink-0">
                          <div className={`h-5 w-9 rounded-full transition-colors ${usePersistentCache ? "bg-sky-500" : "bg-slate-300"}`} />
                          <div className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform ${usePersistentCache ? "translate-x-4" : "translate-x-0"}`} />
                        </div>
                      </label>

                      {/* Cache hit badge */}
                      {usePersistentCache && stats.cache_hits > 0 && (
                        <div className="mt-2 flex items-center gap-1.5 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs font-semibold text-emerald-800">
                          <CheckCircle className="h-3.5 w-3.5 shrink-0" />
                          {t.labels.cacheStatus(stats.cache_hits)}
                        </div>
                      )}
                    </div>

                    {/* ── Section B: SMTP Verify ── */}
                    <div className="rounded-2xl border border-violet-200 bg-gradient-to-br from-violet-50 to-indigo-50 p-4">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <p className="text-[10px] font-bold uppercase tracking-widest text-violet-600">
                              {language === "vi" ? "📧 Xác Minh SMTP (VPS)" : "📧 SMTP Verify (VPS)"}
                            </p>
                            <span className={`rounded-full px-2 py-0.5 text-[10px] font-bold ${
                              smtpEnabled
                                ? vpsApiUrl && vpsApiKey
                                  ? "bg-emerald-100 text-emerald-700"
                                  : "bg-amber-100 text-amber-700"
                                : "bg-slate-200 text-slate-500"
                            }`}>
                              {smtpEnabled
                                ? vpsApiUrl && vpsApiKey
                                  ? (language === "vi" ? "Đã cấu hình" : "Configured")
                                  : (language === "vi" ? "Cần cấu hình" : "Needs setup")
                                : "OFF"}
                            </span>
                          </div>
                          <p className="mt-1 text-[11px] leading-relaxed text-violet-700/70">{t.labels.smtpVerifyHint}</p>
                        </div>
                        <button
                          onClick={() => setSmtpEnabled(!smtpEnabled)}
                          className={`mt-1 flex h-6 w-11 shrink-0 items-center rounded-full transition-colors ${
                            smtpEnabled ? "bg-violet-600" : "bg-slate-300"
                          }`}
                        >
                          <span className={`mx-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
                            smtpEnabled ? "translate-x-5" : "translate-x-0"
                          }`} />
                        </button>
                      </div>

                      {smtpEnabled && (
                        <div className="mt-3 flex flex-col gap-3">
                          <div>
                            <label className="text-[10px] font-bold uppercase tracking-widest text-violet-600">{t.labels.vpsApiUrlLabel}</label>
                            <input
                              type="url"
                              value={vpsApiUrl}
                              onChange={(event) => setVpsApiUrl(event.target.value)}
                              placeholder={t.labels.vpsApiUrlPlaceholder}
                              className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-violet-200 transition placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-400/60"
                            />
                          </div>
                          <div>
                            <label className="text-[10px] font-bold uppercase tracking-widest text-violet-600">{t.labels.vpsApiKeyLabel}</label>
                            <input
                              type="password"
                              value={vpsApiKey}
                              onChange={(event) => setVpsApiKey(event.target.value)}
                              placeholder={t.labels.vpsApiKeyPlaceholder}
                              className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-violet-200 transition placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-400/60"
                            />
                          </div>
                          {stats.smtp_elapsed_ms > 0 && (
                            <div className="flex items-center gap-1.5 rounded-xl border border-violet-200 bg-white/60 px-3 py-2 text-xs font-semibold text-violet-700">
                              <Clock className="h-3.5 w-3.5 shrink-0" />
                              {t.labels.smtpElapsed}: {(stats.smtp_elapsed_ms / 1000).toFixed(1)}s
                            </div>
                          )}
                        </div>
                      )}
                    </div>


                  </div>
                )}
              </div>
            </div>

            <button
              onClick={handleProcess}
              disabled={selectedFiles.length === 0 || !outputDir || isProcessing}
              className={`group flex h-14 w-full items-center justify-center gap-2.5 rounded-2xl font-bold text-white shadow-lg transition active:scale-[.98] disabled:pointer-events-none disabled:bg-slate-200 disabled:text-slate-400 disabled:shadow-none ${
                verifyMode
                  ? "bg-violet-600 shadow-violet-600/30 hover:bg-violet-500"
                  : "bg-blue-600 shadow-blue-600/30 hover:bg-blue-500"
              }`}
            >
              {isProcessing ? (
                <>
                  <LoaderCircle className="h-5 w-5 animate-spin" />
                  <span>{t.labels.processing}</span>
                </>
              ) : (
                <>
                  {verifyMode ? <ShieldCheck className="h-5 w-5 transition-transform group-hover:scale-110" /> : <Mail className="h-5 w-5 transition-transform group-hover:-rotate-12" />}
                  <span>{verifyMode ? (language === "vi" ? "Bắt đầu Xác Minh DNS" : "Start DNS Verify") : (language === "vi" ? "Bắt đầu Lọc" : "Start Filtering")}</span>
                </>
              )}
            </button>

            {/* Real-time scanning indicator */}
            {isProcessing && verifyMode && stats.current_domain && (
              <div className="flex items-center gap-2 rounded-2xl border border-violet-200 bg-violet-50 px-4 py-2.5 text-xs font-semibold text-violet-700">
                <LoaderCircle className="h-3.5 w-3.5 shrink-0 animate-spin" />
                <span className="min-w-0 truncate">
                  {language === "vi" ? "Đang quét:" : "Scanning:"} <span className="font-bold">{stats.current_domain}</span>
                </span>
                  {stats.cache_hits > 0 && (
                    <span className="ml-auto shrink-0 rounded-full bg-violet-200 px-2 py-0.5 text-violet-800">
                      {stats.cache_hits} cached
                    </span>
                  )}
              </div>
            )}
          </div>

          <div className="space-y-3 lg:col-span-7">
            {verifyMode && (
              <>
                <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                  <VerifyHeroCard
                    bucket="mx_has_mx"
                    label={t.labels.mx_has_mx}
                    value={formatLocaleNumber(stats.mx_has_mx, language)}
                    fileName="10_T2_DNS_Valid_Has_MX.txt"
                  />
                  <VerifyHeroCard
                    bucket="mx_a_fallback"
                    label={t.labels.mx_a_fallback}
                    value={formatLocaleNumber(stats.mx_a_fallback, language)}
                    fileName="11_T2_DNS_Valid_ARecord.txt"
                  />
                  <VerifyHeroCard
                    bucket="mx_dead"
                    label={t.labels.mx_dead}
                    value={formatLocaleNumber(stats.mx_dead, language)}
                    fileName="12_T2_DNS_Error_Dead.txt"
                  />
                  <VerifyHeroCard
                    bucket="mx_inconclusive"
                    label={t.labels.mx_inconclusive}
                    value={formatLocaleNumber(stats.mx_inconclusive, language)}
                    fileName="16_T2_DNS_Inconclusive.txt"
                  />
                </div>

                {/* Review group: parked / disposable / typo */}
                {(stats.mx_parked > 0 || stats.mx_disposable > 0 || stats.mx_typo > 0) && (
                  <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4">
                    <p className="mb-3 text-[10px] font-bold uppercase tracking-widest text-amber-700">
                      {language === "vi" ? "⚠️ Cần Kiểm Tra" : "⚠️ Review Required"}
                    </p>
                    <div className="grid grid-cols-3 gap-2">
                      {[
                        { key: "mx_parked" as const, label: t.labels.mx_parked, value: stats.mx_parked, file: "13_T2_DNS_Risk_Parked.txt", color: "text-yellow-700" },
                        { key: "mx_disposable" as const, label: t.labels.mx_disposable, value: stats.mx_disposable, file: "14_T2_DNS_Risk_Disposable.txt", color: "text-orange-700" },
                        { key: "mx_typo" as const, label: t.labels.mx_typo, value: stats.mx_typo, file: "15_T2_DNS_Typo_Suggestion.txt", color: "text-violet-700" },
                      ].map(({ key, label, value, color }) => (
                        <div key={key} className="flex flex-col items-center rounded-xl border border-amber-200 bg-white px-3 py-3 text-center shadow-sm">
                          <p className={`text-xl font-extrabold leading-none ${color}`}>{formatLocaleNumber(value, language)}</p>
                          <p className="mt-1 text-[10px] font-bold uppercase tracking-wide text-slate-400">{label}</p>
                        </div>
                      ))}
                    </div>
                    <div className="mt-3 flex items-center gap-1.5 rounded-xl border border-amber-200 bg-amber-100/50 px-3 py-2 text-xs font-medium text-amber-800">
                      <Info className="h-4 w-4 shrink-0" />
                      {t.labels.reviewNote}
                    </div>
                  </div>
                )}

                {stats.smtp_enabled && (
                  <section className="rounded-3xl border border-violet-200 bg-white p-4 shadow-sm">
                    <div className="mb-3 flex flex-col gap-1">
                      <h3 className="text-sm font-bold text-slate-900">{t.labels.smtpSummaryTitle}</h3>
                      <p className="text-xs leading-relaxed text-slate-500">{t.labels.smtpSummaryBody}</p>
                    </div>
                    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                      <VerifyHeroCard bucket="smtp_deliverable" label={t.labels.smtp_deliverable} value={formatLocaleNumber(stats.smtp_deliverable, language)} fileName="20_T3_SMTP_Deliverable.txt" />
                      <VerifyHeroCard bucket="smtp_rejected" label={t.labels.smtp_rejected} value={formatLocaleNumber(stats.smtp_rejected, language)} fileName="22_T3_SMTP_Rejected.txt" />
                      <VerifyHeroCard bucket="smtp_catchall" label={t.labels.smtp_catchall} value={formatLocaleNumber(stats.smtp_catchall, language)} fileName="21_T3_SMTP_CatchAll.txt" />
                      <VerifyHeroCard bucket="smtp_unknown" label={t.labels.smtp_unknown} value={formatLocaleNumber(stats.smtp_unknown, language)} fileName="23_T3_SMTP_Unknown.txt" />
                    </div>
                  </section>
                )}
              </>
            )}

            {verifyMode ? (
              <div className="rounded-2xl border border-slate-200 bg-slate-50/50 p-4">
                <p className="mb-3 text-[10px] font-bold uppercase tracking-widest text-slate-400">
                  {language === "vi" ? "Tiền Xử Lý (Basic Filter)" : "Pre-processing (Basic Filter)"}
                </p>
                <div className="flex flex-wrap items-center gap-2">
                  {statCards
                    .filter((card) => ["invalid", "public", "edu", "targeted", "custom", "duplicates"].includes(card.key))
                    .map((card) => {
                      const Icon = card.icon;
                      const value = stats[card.key];
                      const pct = totalClassified > 0 ? ((value / totalClassified) * 100).toFixed(1) : "0.0";
                      return (
                        <div key={card.key} className="flex items-center gap-1.5 rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs shadow-sm transition-all hover:bg-slate-50 hover:shadow">
                          <Icon className="h-3.5 w-3.5 text-slate-400" />
                          <span className="font-medium text-slate-500">{t.labels[card.key]}:</span>
                          <span className="font-bold text-slate-800">{formatLocaleNumber(value, language)}</span>
                          <span className="text-[10px] font-semibold text-slate-400">({pct}%)</span>
                        </div>
                      );
                    })}
                </div>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                {statCards
                  .filter((card) => ["invalid", "public", "edu", "targeted", "custom", "duplicates"].includes(card.key))
                  .map((card) => {
                    const Icon = card.icon;
                    const value = stats[card.key];
                    const pct = totalClassified > 0 ? ((value / totalClassified) * 100).toFixed(1) : "0.0";
                    return (
                      <article
                        key={card.key}
                        className="group flex flex-col gap-3 overflow-hidden rounded-2xl border border-slate-100 bg-white p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md"
                      >
                        <div
                          className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl ring-1 transition-transform group-hover:scale-110 ${card.chip}`}
                        >
                          <Icon className="h-5 w-5" />
                        </div>
                        <div className="min-w-0">
                          <p className="text-2xl font-extrabold leading-none tracking-tight text-slate-800">
                            {formatLocaleNumber(value, language)}
                          </p>
                          <p className="mt-1 truncate text-[10px] font-bold uppercase tracking-wider text-slate-400">
                            {t.labels[card.key]}
                          </p>
                          <p className="text-[11px] font-semibold text-slate-300">{pct}%</p>
                        </div>
                      </article>
                    );
                  })}
              </div>
            )}
          </div>
        </div>

        <HistoryModal
          isOpen={isHistoryOpen}
          history={history}
          language={language}
          labels={t.labels}
          statCards={statCards}
          formatNumber={(value) => formatLocaleNumber(value, language)}
          onClose={() => setIsHistoryOpen(false)}
          onOpenFolder={(dir) => {
            revealItemInDir(dir).catch(console.error);
          }}
          onClearHistory={() => {
            setHistory([]);
            localStorage.removeItem("filteremail-history");
          }}
        />

        {totalClassified > 0 && (
          <FinalSummary
            verifyMode={verifyMode}
            labels={t.labels}
            totalClassified={totalClassified}
            stats={stats}
            resolvedOutputDir={resolvedOutputDir}
            canOpenFolder={canOpenFolder}
            verifyDeliverableCount={verifyDeliverableCount}
            verifyDeliverableRate={verifyDeliverableRate}
            verifyDeadRate={verifyDeadRate}
            verifyReviewCount={verifyReviewCount}
            verifyReviewRate={verifyReviewRate}
            verifyFallbackRate={verifyFallbackRate}
            verifyParkedRate={verifyParkedRate}
            verifyDisposableRate={verifyDisposableRate}
            verifyTypoRate={verifyTypoRate}
            verifyDomainCount={verifyDomainCount}
            smtpCheckedCount={smtpCheckedCount}
            smtpDeliverableRate={smtpDeliverableRate}
            smtpRejectedRate={smtpRejectedRate}
            smtpCatchallRate={smtpCatchallRate}
            smtpUnknownRate={smtpUnknownRate}
            invalidRate={invalidRate}
            publicRate={publicRate}
            eduRate={eduRate}
            targetedRate={targetedRate}
            customRate={customRate}
            validCount={validCount}
            formatNumber={(value) => formatLocaleNumber(value, language)}
            onOpenFolder={openResultFolder}
          />
        )}
      </div>
    </main>
  );
}
