import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  formatBackendError,
  getSavedLanguage,
  persistLanguage,
  translations,
  type ErrorPayload,
  type Language,
} from "./i18n";
import {
  AlertCircle,
  Copy,
  CheckCircle2,
  Languages,
  FileSpreadsheet,
  FolderOpen,
  LoaderCircle,
  Mail,
  ShieldCheck,
  UploadCloud,
  Users,
  XCircle,
  Target,
  CheckCircle,
  History,
  X,
  Trash2,
  Wifi,
  WifiOff,
} from "lucide-react";

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
  elapsed_ms: number;
  output_dir?: string;
};

type BannerState =
  | { tone: "idle"; message: string }
  | { tone: "success"; message: string }
  | { tone: "error"; message: string };

type HistoryEntry = {
  id: string;
  timestamp: number;
  fileNames: string[];
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
  elapsed_ms: 0,
};

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
    key: "duplicates" as const,
    icon: Copy,
    chip: "bg-gray-50 text-gray-700 ring-gray-200",
  },
  {
    key: "custom" as const,
    icon: Mail,
    chip: "bg-amber-50 text-amber-700 ring-amber-100",
  },
];

function formatDuration(milliseconds: number) {
  const totalSeconds = Math.floor(milliseconds / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${minutes.toString().padStart(2, "0")}:${seconds
    .toString()
    .padStart(2, "0")}`;
}

function basename(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function formatPercent(value: number) {
  return `${value.toFixed(1)}%`;
}

export default function App() {
  const [language, setLanguage] = useState<Language>(getSavedLanguage);
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [checkMx, setCheckMx] = useState(false);
  const [port25Status, setPort25Status] = useState<"idle" | "checking" | "open" | "closed">("idle");
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);

  useEffect(() => {
    const saved = localStorage.getItem("filteremail-history");
    if (saved) {
      try {
        setHistory(JSON.parse(saved));
      } catch (e) {}
    }
  }, []);

  // Check port 25 whenever MX scan is toggled on
  useEffect(() => {
    if (!checkMx) {
      setPort25Status("idle");
      return;
    }
    setPort25Status("checking");
    invoke<boolean>("check_port_25")
      .then((open) => setPort25Status(open ? "open" : "closed"))
      .catch(() => setPort25Status("closed"));
  }, [checkMx]);
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

  useEffect(() => {
    const savedDomains = localStorage.getItem("targetDomains");
    if (savedDomains) setTargetDomains(savedDomains);
    const savedMx = localStorage.getItem("checkMx");
    if (savedMx === "true") setCheckMx(true);
    const savedOut = localStorage.getItem("lastOutputDir");
    if (savedOut) {
        setOutputDir(savedOut);
        setLastOutputDir(savedOut);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("targetDomains", targetDomains);
  }, [targetDomains]);

  useEffect(() => {
    localStorage.setItem("checkMx", checkMx ? "true" : "false");
  }, [checkMx]);

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
        message: t.progressBanner(stats.processed_lines),
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
          message: selectedFiles.length === 1 ? t.selectedFileBanner(basename(selectedFiles[0])) : `Đã chọn ${selectedFiles.length} tệp.`,
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
    selectedFiles.length,
    stats.processed_lines,
    t,
  ]);

  useEffect(() => {
    let mounted = true;

    const setupListeners = async () => {
      const unlistenProgress = await listen<ProcessingPayload>(
        "processing-progress",
        ({ payload }) => {
          if (!mounted) return;
          setStats(payload);
          setIsProcessing(true);
          setBanner({
            tone: "idle",
            message: translations[language].progressBanner(payload.processed_lines),
          });
        },
      );

      const unlistenComplete = await listen<ProcessingPayload>(
        "processing-complete",
        ({ payload }) => {
          if (!mounted) return;
          setStats(payload);
          setIsProcessing(false);
          setLastOutputDir(payload.output_dir ?? "");
          
          setBanner({
            tone: "success",
            message: translations[language].completeBanner,
          });

          // push history
          const newEntry: HistoryEntry = {
            id: crypto.randomUUID(),
            timestamp: Date.now(),
            fileNames: selectedFiles.map(f => basename(f)),
            stats: payload
          };
          setHistory(prev => {
            const next = [newEntry, ...prev].slice(0, 20);
            localStorage.setItem("filteremail-history", JSON.stringify(next));
            return next;
          });
          isPermissionGranted().then((granted) => {
             if (!granted) {
                 requestPermission().then((g) => {
                     if (g === 'granted') sendNotification({ title: 'Hoàn tất', body: 'Quá trình lọc email đã xong!' });
                 });
             } else {
                 sendNotification({ title: 'Hoàn tất', body: 'Quá trình lọc email đã xong!' });
             }
          });

        },
      );

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
                message: paths.length === 1 ? translations[language].selectedFileBanner(basename(paths[0])) : `Đã chọn ${paths.length} tệp.`,
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
  }, [language]);

  const progressWidth = `${Math.max(0, Math.min(stats.progress_percent, 100)).toFixed(1)}%`;
  const canOpenFolder = Boolean(lastOutputDir || outputDir);
  const resolvedOutputDir = lastOutputDir || outputDir;

  const totalClassified = useMemo(
    () => stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom + stats.duplicates + stats.mx_dead,
    [stats],
  );
  const showSummary = banner.tone === "success" && totalClassified > 0;
  const invalidRate = totalClassified === 0 ? 0 : (stats.invalid / totalClassified) * 100;
  const publicRate = totalClassified === 0 ? 0 : (stats.public / totalClassified) * 100;
  const eduRate = totalClassified === 0 ? 0 : (stats.edu / totalClassified) * 100;
  const targetedRate = totalClassified === 0 ? 0 : (stats.targeted / totalClassified) * 100;
  const customRate = totalClassified === 0 ? 0 : (stats.custom / totalClassified) * 100;

  const pickInputFile = async () => {
    const selected = await openDialog({
      multiple: true,
      directory: false,
      filters: [
        {
          name: "Email Lists",
          extensions: ["txt", "csv"],
        },
      ],
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

  // alias used in JSX
  const selectOutputDir = pickOutputDir;

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
        target_domains: targetDomains,
        check_mx: checkMx,
      });
    } catch (error) {
      console.error("Invoke Error:", error);
      setIsProcessing(false);
      setBanner({
        tone: "error",
        message: typeof error === "string" ? error : (error instanceof Error ? error.message : t.labels.genericBackendError),
      });
    }
  };

  const openResultFolder = async () => {
    if (!resolvedOutputDir) return;
    try {
      await revealItemInDir(resolvedOutputDir);
    } catch (e) {
      console.error(e);
      await openPath(resolvedOutputDir).catch(console.error);
    }
  };

  const bannerStyles =
    banner.tone === "success"
      ? "border-emerald-300 bg-emerald-50/95 text-emerald-800"
      : banner.tone === "error"
        ? "border-red-300 bg-red-50/95 text-red-800"
        : "border-slate-200/80 bg-white/85 text-slate-700";

  return (
    <main className="min-h-screen bg-slate-50 font-sans text-slate-900">
      <div className="mx-auto w-full max-w-7xl space-y-5 p-4 sm:p-6 lg:p-8">

        {/* ── HEADER ── */}
        <header className="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-white px-5 py-3 shadow-sm ring-1 ring-slate-900/5">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-sky-500 shadow-md shadow-sky-500/30">
              <CheckCircle2 className="h-5 w-5 text-white" />
            </div>
            <div className="min-w-0">
              <p className="truncate text-base font-bold text-slate-800 leading-tight">FilterEmail Desktop</p>
              <p className="truncate text-[11px] font-medium text-slate-400">{t.labels.heroBadge}</p>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <button
              onClick={() => setIsHistoryOpen(true)}
              className="flex items-center gap-1.5 rounded-full bg-sky-50 px-3 py-1.5 text-xs font-semibold text-sky-700 ring-1 ring-sky-200 transition hover:bg-sky-100 whitespace-nowrap"
            >
              <History className="h-3.5 w-3.5 shrink-0" />
              <span>{t.labels.openHistory}</span>
            </button>
            <div className="flex rounded-full bg-slate-100 p-1">
              {(["en", "vi"] as const).map((lang) => (
                <button
                  key={lang}
                  onClick={() => setLanguage(lang)}
                  className={`rounded-full px-3 py-1 text-xs font-bold transition whitespace-nowrap ${
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

        {/* ── STATUS BANNER ── */}
        {banner.tone !== "idle" && (
          <div className={`flex min-w-0 items-start gap-3 rounded-2xl border p-4 text-sm font-medium ${
            banner.tone === "error"
              ? "border-red-200 bg-red-50 text-red-800"
              : "border-emerald-200 bg-emerald-50 text-emerald-800"
          }`}>
            {banner.tone === "error"
              ? <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
              : <CheckCircle className="mt-0.5 h-5 w-5 shrink-0" />}
            <p className="min-w-0 break-words leading-relaxed">{banner.message}</p>
          </div>
        )}

        {/* ── TOP ROW: Dropzone | Black Stats Card ── */}
        <div className="grid grid-cols-1 gap-5 lg:grid-cols-12 lg:items-stretch">
          {/* Dropzone */}
          <div
            onDragOver={(e) => { e.preventDefault(); setDragActive(true); }}
            onDragLeave={() => setDragActive(false)}
            className={`flex flex-col items-center justify-center rounded-3xl border-2 border-dashed p-6 text-center transition-all lg:col-span-5 ${
              dragActive
                ? "scale-[1.01] border-sky-400 bg-sky-50 shadow-lg"
                : "border-slate-200 bg-white shadow-sm"
            }`}
          >
            <div className={`mb-4 flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl shadow-sm transition-colors ${
              dragActive ? "bg-sky-500 text-white" : "bg-slate-900 text-white"
            }`}>
              <UploadCloud className="h-6 w-6" />
            </div>
            <h3 className="text-base font-bold text-slate-900">{t.labels.dragTitle}</h3>
            <p className="mt-1.5 max-w-xs text-sm leading-relaxed text-slate-500">{t.labels.dragBody}</p>
            <button
              onClick={pickInputFile}
              className="mt-5 flex items-center gap-2 rounded-full bg-slate-900 px-5 py-2 text-sm font-semibold text-white shadow transition hover:bg-slate-700 active:scale-95"
            >
              <UploadCloud className="h-4 w-4" />
              {t.labels.chooseFile}
            </button>
          </div>

          {/* Big Stats Card */}
          <article className="relative flex flex-col overflow-hidden rounded-3xl bg-slate-900 p-6 shadow-xl lg:col-span-7 sm:flex-row sm:items-center sm:gap-6">
            <div className="pointer-events-none absolute -right-16 -top-16 h-56 w-56 rounded-full bg-sky-500/20 blur-3xl" />
            <div className="pointer-events-none absolute -bottom-16 -left-16 h-56 w-56 rounded-full bg-indigo-500/20 blur-3xl" />

            {/* Left: number */}
            <div className="relative z-10 min-w-0 flex-1">
              <p className="text-[11px] font-bold uppercase tracking-widest text-slate-400">{t.labels.classified}</p>
              <p className="mt-2 text-5xl font-extrabold leading-none tracking-tight text-white lg:text-6xl">
                {totalClassified.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
              </p>
              <p className="mt-3 text-sm leading-relaxed text-slate-400 line-clamp-2">{t.labels.classifiedBody}</p>
            </div>

            {/* Right: progress box */}
            <div className="relative z-10 mt-4 w-full shrink-0 rounded-2xl bg-white/10 p-4 ring-1 ring-white/20 backdrop-blur-md sm:mt-0 sm:w-56">
              <div className="flex items-center justify-between text-xs font-bold text-sky-200">
                <span className="uppercase tracking-wide">{t.labels.progress}</span>
                <span className="text-lg text-white">{stats.progress_percent.toFixed(1)}%</span>
              </div>
              <div className="mt-2 h-2.5 overflow-hidden rounded-full bg-black/40">
                <div
                  className="h-full rounded-full bg-gradient-to-r from-sky-400 to-indigo-500 transition-all duration-300"
                  style={{ width: `${stats.progress_percent}%` }}
                />
              </div>
              <button
                onClick={() => revealItemInDir(lastOutputDir || outputDir)}
                disabled={!lastOutputDir}
                className="mt-4 flex w-full items-center justify-center gap-2 rounded-xl bg-white py-2.5 text-sm font-bold text-slate-900 shadow transition hover:bg-slate-100 disabled:pointer-events-none disabled:opacity-30"
              >
                <FolderOpen className="h-4 w-4 shrink-0" />
                <span className="truncate">{t.labels.openFolder}</span>
              </button>
            </div>
          </article>
        </div>

        {/* ── BOTTOM ROW: Config | Metric Cards ── */}
        <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">

          {/* Config + Button */}
          <div className="flex flex-col gap-4 lg:col-span-5">
            <div className="flex-1 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-slate-100">
              <div className="space-y-4">

                {/* Selected file */}
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

                {/* Output folder */}
                <div>
                  <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{t.labels.outputFolder}</label>
                  <div className="mt-1.5 flex min-w-0 items-center gap-2">
                    <div className="flex min-w-0 flex-1 items-center gap-2 rounded-2xl bg-slate-50 px-3 py-2.5 ring-1 ring-slate-100">
                      <FolderOpen className="h-4 w-4 shrink-0 text-slate-400" />
                      <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-600">{outputDir || t.labels.noFolder}</span>
                    </div>
                    <button
                      onClick={selectOutputDir}
                      className="shrink-0 rounded-2xl bg-slate-100 px-3 py-2.5 text-sm font-bold text-slate-700 transition hover:bg-slate-200 whitespace-nowrap"
                    >
                      {t.labels.selectFolder}
                    </button>
                  </div>
                </div>

                {/* Target domains */}
                <div>
                  <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{t.labels.targetedInputLabel}</label>
                  <input
                    type="text"
                    value={targetDomains}
                    onChange={(e) => setTargetDomains(e.target.value)}
                    placeholder={t.labels.targetedInputPlaceholder}
                    className="mt-1.5 w-full rounded-2xl bg-slate-50 px-3 py-2.5 text-sm text-slate-900 ring-1 ring-slate-100 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-400/60 transition"
                  />
                </div>

                {/* MX toggle */}
                <label className="flex cursor-pointer items-center justify-between rounded-2xl bg-slate-50 px-4 py-3 ring-1 ring-slate-100 transition hover:bg-slate-100">
                  <span className="min-w-0 flex-1 pr-3 text-sm font-semibold text-slate-700 leading-snug">{t.labels.mxCheckLabel}</span>
                  <div className="relative shrink-0">
                    <input type="checkbox" checked={checkMx} onChange={(e) => setCheckMx(e.target.checked)} className="sr-only" />
                    <div className={`h-6 w-11 rounded-full transition-colors ${checkMx ? "bg-sky-500" : "bg-slate-300"}`} />
                    <div className={`absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${checkMx ? "translate-x-5" : "translate-x-0"}`} />
                  </div>
                </label>

                {/* Port 25 status badge */}
                {port25Status !== "idle" && (
                  <div className={`flex items-center gap-2 rounded-xl px-3 py-2 text-xs font-semibold ring-1 transition-all ${
                    port25Status === "checking"
                      ? "bg-amber-50 text-amber-700 ring-amber-200"
                      : port25Status === "open"
                        ? "bg-emerald-50 text-emerald-700 ring-emerald-200"
                        : "bg-red-50 text-red-700 ring-red-200"
                  }`}>
                    {port25Status === "checking" ? (
                      <><LoaderCircle className="h-3.5 w-3.5 animate-spin shrink-0" /><span>Port 25: {language === "vi" ? "Đang kiểm tra..." : "Checking..."}</span></>
                    ) : port25Status === "open" ? (
                      <><Wifi className="h-3.5 w-3.5 shrink-0" /><span>Port 25: {language === "vi" ? "Đã mở ✓" : "Open ✓"}</span></>
                    ) : (
                      <><WifiOff className="h-3.5 w-3.5 shrink-0" /><span>Port 25: {language === "vi" ? "Bị chặn ✗ (MX vẫn hoạt động)" : "Blocked ✗ (MX still works)"}</span></>
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* Action button */}
            <button
              onClick={handleProcess}
              disabled={selectedFiles.length === 0 || !outputDir || isProcessing}
              className="group flex h-14 w-full items-center justify-center gap-2.5 rounded-2xl bg-blue-600 font-bold text-white shadow-lg shadow-blue-600/30 transition hover:bg-blue-500 active:scale-[.98] disabled:pointer-events-none disabled:bg-slate-200 disabled:text-slate-400 disabled:shadow-none"
            >
              {isProcessing ? (
                <><LoaderCircle className="h-5 w-5 animate-spin" /><span>{t.labels.processing}</span></>
              ) : (
                <><Mail className="h-5 w-5 transition-transform group-hover:-rotate-12" /><span>{t.labels.start}</span></>
              )}
            </button>
          </div>

          {/* Metric Cards */}
          <div className="lg:col-span-7">
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              {/* MX Dead alert row */}
              {stats.mx_dead > 0 && (
                <div className="col-span-2 sm:col-span-3 flex items-center gap-4 rounded-2xl border border-red-200 bg-red-50 p-4">
                  <div className="shrink-0 rounded-xl bg-red-600 p-2.5 text-white shadow-lg shadow-red-600/30">
                    <AlertCircle className="h-6 w-6" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="text-[10px] font-bold uppercase tracking-widest text-red-600">{t.labels.mx_dead}</p>
                    <p className="text-2xl font-extrabold text-red-900 leading-tight">
                      {stats.mx_dead.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                    </p>
                    <p className="truncate text-[11px] text-red-500 font-medium">dead_emails.txt</p>
                  </div>
                </div>
              )}

              {/* Stat cards */}
              {statCards.map((card) => {
                const Icon = card.icon;
                const value = stats[card.key as keyof ProcessingPayload] as number;
                const pct = totalClassified > 0 ? ((value / totalClassified) * 100).toFixed(1) : "0.0";
                return (
                  <article
                    key={card.key}
                    className="group flex flex-col gap-3 overflow-hidden rounded-2xl border border-slate-100 bg-white p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md"
                  >
                    <div className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl ring-1 transition-transform group-hover:scale-110 ${card.chip}`}>
                      <Icon className="h-5 w-5" />
                    </div>
                    <div className="min-w-0">
                      <p className="text-2xl font-extrabold tracking-tight text-slate-800 leading-none">
                        {value.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                      </p>
                      <p className="mt-1 truncate text-[10px] font-bold uppercase tracking-wider text-slate-400">
                        {t.labels[card.key as keyof typeof t.labels]}
                      </p>
                      <p className="text-[11px] font-semibold text-slate-300">{pct}%</p>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>

        </div>
      </div>

      {/* History Modal */}
      {isHistoryOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-sm transition-opacity">
          <div className="relative w-full max-w-2xl max-h-[85vh] flex flex-col rounded-3xl bg-white shadow-2xl ring-1 ring-slate-900/5">
            {/* Header */}
            <div className="flex items-center justify-between border-b border-slate-100 px-6 py-4">
              <h2 className="text-xl font-bold text-slate-800 flex items-center gap-2">
                <History className="h-5 w-5 text-sky-500" />
                {t.labels.historyTitle}
              </h2>
              <button onClick={() => setIsHistoryOpen(false)} className="rounded-full p-2 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors">
                <X className="h-5 w-5" />
              </button>
            </div>
            
            {/* Content */}
            <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
              {history.length === 0 ? (
                <div className="py-12 text-center text-sm text-slate-500">{t.labels.emptyHistory}</div>
              ) : (
                history.map((entry) => {
                  const total = entry.stats.invalid + entry.stats.public + entry.stats.edu + entry.stats.targeted + entry.stats.custom + entry.stats.duplicates + (entry.stats.mx_dead || 0);
                  const valid = entry.stats.public + entry.stats.edu + entry.stats.targeted + entry.stats.custom;
                  return (
                    <div key={entry.id} className="rounded-2xl border border-slate-200 bg-slate-50 p-4 transition-all hover:border-sky-300">
                      <div className="flex flex-wrap items-start justify-between gap-2 mb-2">
                        <div>
                          <div className="text-xs font-semibold text-slate-400 mb-1">
                            {new Date(entry.timestamp).toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                          </div>
                          <div className="text-sm font-medium text-slate-700 break-all leading-tight">
                            {entry.fileNames.join(", ")}
                          </div>
                        </div>
                        <button
                          onClick={() => revealItemInDir(entry.stats.output_dir || "")}
                          className="flex shrink-0 items-center gap-1.5 rounded-lg bg-sky-100 px-3 py-1.5 text-xs font-semibold text-sky-700 hover:bg-sky-200 transition-colors shadow-sm"
                        >
                          <FolderOpen className="h-3.5 w-3.5" />
                          <span className="hidden sm:inline">{t.labels.openFolder}</span>
                        </button>
                      </div>
                      
                      <div className="mt-3 flex flex-col gap-2">
                        {/* Summary primary */}
                        <div className="grid grid-cols-3 sm:grid-cols-5 gap-2">
                          <div className="rounded-lg bg-white p-2 border border-slate-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Total</div>
                            <div className="text-sm font-bold text-slate-800">{total.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-emerald-50 p-2 border border-emerald-100 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-emerald-600">Valid</div>
                            <div className="text-sm font-bold text-emerald-700">{valid.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Dups</div>
                            <div className="text-sm font-bold text-slate-600">{entry.stats.duplicates.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-red-50 p-2 border border-red-100 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-red-500">Invalid</div>
                            <div className="text-sm font-bold text-red-600">{entry.stats.invalid.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-red-100 p-2 border border-red-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-red-700">MX Dead</div>
                            <div className="text-sm font-bold text-red-800">{(entry.stats.mx_dead || 0).toLocaleString()}</div>
                          </div>
                        </div>

                        {/* Breakdown */}
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.public}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.public.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.edu}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.edu.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.targeted}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.targeted.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.custom}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.custom.toLocaleString()}</div>
                          </div>
                        </div>
                        
                        <div className="text-right text-[10px] text-slate-400 font-medium mt-1">
                          {t.labels.elapsed}: {(entry.stats.elapsed_ms / 1000).toFixed(2)}s
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>

            {/* Footer */}
            {history.length > 0 && (
              <div className="border-t border-slate-100 px-6 py-3 flex justify-end">
                <button
                  onClick={() => {
                    setHistory([]);
                    localStorage.removeItem("filteremail-history");
                  }}
                  className="flex items-center gap-2 rounded-lg text-red-600 hover:bg-red-50 px-3 py-2 text-sm font-semibold transition-colors"
                >
                  <Trash2 className="h-4 w-4" />
                  {t.labels.clearHistory}
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </main>
  );
}
