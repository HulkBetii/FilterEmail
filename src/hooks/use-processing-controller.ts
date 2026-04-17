import {
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
} from "react";
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
  formatBackendError,
  translations,
  type ErrorPayload,
  type Language,
} from "../i18n";
import {
  clearSavedHistory,
  loadPersistedSettings,
  loadSavedHistory,
  loadSavedLanguagePreference,
  persistHistory,
  persistLanguagePreference,
  persistLastOutputDir,
  persistMaxConcurrent,
  persistPersistentCache,
  persistSmtpEnabled,
  persistTargetDomains,
  persistTimeoutMs,
  persistVpsApiKey,
  persistVpsApiUrl,
} from "../lib/app-storage";
import {
  DEFAULT_MAX_CONCURRENT,
  DEFAULT_TIMEOUT_MS,
  MAX_HISTORY_ENTRIES,
  basename,
  formatLocaleNumber,
  initialStats,
  normalizeStats,
  type ActiveTab,
  type BannerState,
  type HistoryEntry,
  type ProcessingPayload,
} from "../lib/app-state";

function selectedFilesMessage(
  files: string[],
  t: (typeof translations.en),
) {
  if (files.length === 1) {
    return t.selectedFileBanner(basename(files[0]));
  }

  return `Đã chọn ${files.length} tệp.`;
}

async function sendCompletionNotification() {
  const granted = await isPermissionGranted();
  if (!granted) {
    const result = await requestPermission();
    if (result !== "granted") {
      return;
    }
  }

  sendNotification({ title: "Hoàn tất", body: "Quá trình lọc email đã xong!" });
}

export function useProcessingController() {
  const persistedSettingsRef = useRef(loadPersistedSettings());

  const [language, setLanguage] = useState<Language>(
    loadSavedLanguagePreference,
  );
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [timeoutMs, setTimeoutMs] = useState(
    persistedSettingsRef.current.timeoutMs,
  );
  const [maxConcurrent, setMaxConcurrent] = useState(
    persistedSettingsRef.current.maxConcurrent,
  );
  const [usePersistentCache, setUsePersistentCache] = useState(
    persistedSettingsRef.current.usePersistentCache,
  );
  const [showAdvancedDns, setShowAdvancedDns] = useState(false);
  const [smtpEnabled, setSmtpEnabled] = useState(
    persistedSettingsRef.current.smtpEnabled,
  );
  const [vpsApiUrl, setVpsApiUrl] = useState(
    persistedSettingsRef.current.vpsApiUrl,
  );
  const [vpsApiKey, setVpsApiKey] = useState(
    persistedSettingsRef.current.vpsApiKey,
  );
  const [showSmtpDiag, setShowSmtpDiag] = useState(false);
  const [showDnsDiag, setShowDnsDiag] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>(loadSavedHistory);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);
  const [outputDir, setOutputDir] = useState(
    persistedSettingsRef.current.outputDir,
  );
  const [targetDomains, setTargetDomains] = useState(
    persistedSettingsRef.current.targetDomains,
  );
  const [lastOutputDir, setLastOutputDir] = useState(
    persistedSettingsRef.current.lastOutputDir,
  );
  const [dragActive, setDragActive] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [stats, setStats] = useState<ProcessingPayload>(initialStats);
  const [banner, setBanner] = useState<BannerState>({
    tone: "idle",
    message: translations.en.idleBanner,
  });
  const [activeTab, setActiveTab] = useState<ActiveTab>("filter");

  const t = translations[language];
  const verifyMode = activeTab === "verify";
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
  const finalTotal =
    stats.final_alive + stats.final_dead + stats.final_unknown;
  const formatNumber = (value: number) =>
    formatLocaleNumber(value, language);
  const canStartProcessing =
    selectedFiles.length > 0 && Boolean(outputDir) && !isProcessing;

  useEffect(() => {
    persistTargetDomains(targetDomains);
  }, [targetDomains]);

  useEffect(() => {
    persistTimeoutMs(timeoutMs);
  }, [timeoutMs]);

  useEffect(() => {
    persistMaxConcurrent(maxConcurrent);
  }, [maxConcurrent]);

  useEffect(() => {
    persistPersistentCache(usePersistentCache);
  }, [usePersistentCache]);

  useEffect(() => {
    persistSmtpEnabled(smtpEnabled);
  }, [smtpEnabled]);

  useEffect(() => {
    persistVpsApiUrl(vpsApiUrl);
  }, [vpsApiUrl]);

  useEffect(() => {
    persistVpsApiKey(vpsApiKey);
  }, [vpsApiKey]);

  useEffect(() => {
    if (outputDir) {
      persistLastOutputDir(outputDir);
    }
  }, [outputDir]);

  useEffect(() => {
    persistLanguagePreference(language);
  }, [language]);

  useEffect(() => {
    if (isProcessing) {
      setShowDnsDiag(true);
      setShowSmtpDiag(true);
    }
  }, [isProcessing]);

  useEffect(() => {
    if (isProcessing && stats.processed_lines > 0) {
      setBanner({
        tone: "idle",
        message: t.progressBanner(
          stats.processed_lines,
          stats.current_domain ?? stats.current_email,
        ),
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
          message: selectedFilesMessage(selectedFiles, t),
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
    outputDir,
    selectedFiles,
    stats.current_domain,
    stats.current_email,
    stats.processed_lines,
    t,
  ]);

  const appendHistoryEntry = useEffectEvent((entry: HistoryEntry) => {
    setHistory((previous) => {
      const next = [entry, ...previous].slice(0, MAX_HISTORY_ENTRIES);
      persistHistory(next);
      return next;
    });
  });

  const handleProgressEvent = useEffectEvent((payload: ProcessingPayload) => {
    const normalized = normalizeStats(payload);
    setStats(normalized);
    setIsProcessing(true);
    setBanner({
      tone: "idle",
      message: translations[language].progressBanner(
        normalized.processed_lines,
        normalized.current_domain ?? normalized.current_email,
      ),
    });
  });

  const handleCompleteEvent = useEffectEvent((payload: ProcessingPayload) => {
    const normalized = normalizeStats(payload);
    setStats(normalized);
    setIsProcessing(false);
    setLastOutputDir(normalized.output_dir ?? "");
    setBanner({
      tone: "success",
      message: translations[language].completeBanner,
    });

    appendHistoryEntry({
      id: crypto.randomUUID(),
      timestamp: Date.now(),
      fileNames: selectedFiles.map((file) => basename(file)),
      mode: verifyMode ? "verify" : "filter",
      stats: normalized,
    });

    void sendCompletionNotification();
  });

  const handleErrorEvent = useEffectEvent((payload: ErrorPayload) => {
    setIsProcessing(false);
    setBanner({
      tone: "error",
      message: formatBackendError(payload, language),
    });
  });

  const handleDragDropEvent = useEffectEvent((paths: string[]) => {
    setDragActive(false);
    if (paths.length === 0) {
      return;
    }

    setSelectedFiles(paths);
    setBanner({
      tone: "idle",
      message: selectedFilesMessage(paths, translations[language]),
    });
  });

  useEffect(() => {
    let mounted = true;

    const setupListeners = async () => {
      const unlistenProgress = await listen<ProcessingPayload>(
        "processing-progress",
        ({ payload }) => {
          if (mounted) {
            handleProgressEvent(payload);
          }
        },
      );

      const unlistenComplete = await listen<ProcessingPayload>(
        "processing-complete",
        ({ payload }) => {
          if (mounted) {
            handleCompleteEvent(payload);
          }
        },
      );

      const unlistenError = await listen<ErrorPayload>(
        "processing-error",
        ({ payload }) => {
          if (mounted) {
            handleErrorEvent(payload);
          }
        },
      );

      const unlistenDragDrop = await getCurrentWindow().onDragDropEvent(
        (event) => {
          if (!mounted) {
            return;
          }

          switch (event.payload.type) {
            case "enter":
            case "over":
              setDragActive(true);
              break;
            case "leave":
              setDragActive(false);
              break;
            case "drop":
              handleDragDropEvent(event.payload.paths);
              break;
          }
        },
      );

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
  }, [
    handleCompleteEvent,
    handleDragDropEvent,
    handleErrorEvent,
    handleProgressEvent,
  ]);

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
    } else if (Array.isArray(selected) && selected.length > 0) {
      setSelectedFiles(selected);
      setBanner({
        tone: "idle",
        message: selectedFilesMessage(selected, t),
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
    if (!canStartProcessing) {
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
    if (!resolvedOutputDir) {
      return;
    }

    try {
      await revealItemInDir(resolvedOutputDir);
    } catch (error) {
      console.error(error);
      await openPath(resolvedOutputDir).catch(console.error);
    }
  };

  const openHistoryFolder = (directory: string) => {
    revealItemInDir(directory).catch(console.error);
  };

  const clearHistory = () => {
    setHistory([]);
    clearSavedHistory();
  };

  return {
    activeTab,
    banner,
    canOpenFolder,
    canStartProcessing,
    dragActive,
    finalTotal,
    formatNumber,
    handleProcess,
    history,
    isHistoryOpen,
    isProcessing,
    language,
    lastOutputDir,
    maxConcurrent,
    openHistoryFolder,
    openResultFolder,
    outputDir,
    pickInputFile,
    pickOutputDir,
    resolvedOutputDir,
    selectedFiles,
    setActiveTab,
    setDragActive,
    setIsHistoryOpen,
    setLanguage,
    setMaxConcurrent,
    setOutputDir,
    setShowAdvancedDns,
    setShowDnsDiag,
    setShowSmtpDiag,
    setSmtpEnabled,
    setTargetDomains,
    setTimeoutMs,
    setUsePersistentCache,
    setVpsApiKey,
    setVpsApiUrl,
    showAdvancedDns,
    showDnsDiag,
    showSmtpDiag,
    smtpEnabled,
    stats,
    t,
    targetDomains,
    timeoutMs,
    totalClassified,
    usePersistentCache,
    verifyMode,
    vpsApiKey,
    vpsApiUrl,
    clearHistory,
  };
}
