import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
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
} from "lucide-react";

type ProcessingPayload = {
  processed_lines: number;
  progress_percent: number;
  invalid: number;
  public: number;
  edu: number;
  custom: number;
  elapsed_ms: number;
  output_dir?: string;
};

type BannerState =
  | { tone: "idle"; message: string }
  | { tone: "success"; message: string }
  | { tone: "error"; message: string };

const initialStats: ProcessingPayload = {
  processed_lines: 0,
  progress_percent: 0,
  invalid: 0,
  public: 0,
  edu: 0,
  custom: 0,
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
  const [selectedFilePath, setSelectedFilePath] = useState("");
  const [outputDir, setOutputDir] = useState("");
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
      if (selectedFilePath) {
        setBanner({
          tone: "idle",
          message: t.selectedFileBanner(basename(selectedFilePath)),
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
    selectedFilePath,
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
            const firstPath = event.payload.paths[0];
            if (firstPath) {
              setSelectedFilePath(firstPath);
              setBanner({
                tone: "idle",
                message: translations[language].selectedFileBanner(basename(firstPath)),
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
    () => stats.invalid + stats.public + stats.edu + stats.custom,
    [stats],
  );
  const showSummary = banner.tone === "success" && totalClassified > 0;
  const invalidRate = totalClassified === 0 ? 0 : (stats.invalid / totalClassified) * 100;
  const publicRate = totalClassified === 0 ? 0 : (stats.public / totalClassified) * 100;
  const eduRate = totalClassified === 0 ? 0 : (stats.edu / totalClassified) * 100;
  const customRate = totalClassified === 0 ? 0 : (stats.custom / totalClassified) * 100;

  const pickInputFile = async () => {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "Email Lists",
          extensions: ["txt", "csv"],
        },
      ],
    });

    if (typeof selected === "string") {
      setSelectedFilePath(selected);
      setBanner({
        tone: "idle",
        message: t.selectedFileBanner(basename(selected)),
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
    if (!selectedFilePath || !outputDir || isProcessing) {
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
        file_path: selectedFilePath,
        output_dir: outputDir,
      });
    } catch (error) {
      setIsProcessing(false);
      setBanner({
        tone: "error",
        message: t.labels.genericBackendError,
      });
    }
  };

  const openResultFolder = async () => {
    if (!resolvedOutputDir) return;
    await openPath(resolvedOutputDir);
  };

  const bannerStyles =
    banner.tone === "success"
      ? "border-emerald-300 bg-emerald-50/95 text-emerald-800"
      : banner.tone === "error"
        ? "border-red-300 bg-red-50/95 text-red-800"
        : "border-slate-200/80 bg-white/85 text-slate-700";

  return (
    <main className="min-h-screen bg-aura px-3 py-4 text-ink sm:px-5 sm:py-6 lg:px-8 lg:py-8">
      <div className="mx-auto flex w-full max-w-7xl flex-col gap-4 lg:gap-6">
        <section className="rounded-[2rem] border border-white/70 bg-glass p-5 shadow-float backdrop-blur-2xl sm:p-6 lg:p-8">
          <div className="grid gap-5 xl:grid-cols-[minmax(0,1.45fr)_minmax(320px,0.85fr)] xl:items-start">
            <div className="min-w-0 space-y-4">
              <span className="inline-flex max-w-full items-center gap-2 rounded-full bg-white/95 px-4 py-2 text-sm font-medium text-slate-700 ring-1 ring-slate-200/70">
                <FileSpreadsheet className="h-4 w-4 text-primary" />
                {t.labels.heroBadge}
              </span>
              <div className="min-w-0 space-y-3">
                <h1 className="text-balance break-words text-3xl font-semibold leading-[1.02] tracking-tight text-slate-900 sm:text-4xl lg:text-5xl xl:text-[4.35rem]">
                  {t.labels.heroTitle}
                </h1>
                <p className="max-w-3xl text-pretty break-words text-base leading-7 text-slate-700 sm:text-lg">
                  {t.labels.heroBody}
                </p>
              </div>
            </div>

            <div className="grid min-w-0 gap-3 rounded-[1.75rem] border border-white/80 bg-white/78 p-4 shadow-glass backdrop-blur-xl sm:p-5">
              <div className="flex flex-col gap-3 rounded-2xl bg-white/95 px-3 py-3 ring-1 ring-slate-200/70 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex min-w-0 items-center gap-2 text-sm font-medium text-slate-600">
                  <Languages className="h-4 w-4 text-primary" />
                  {t.labels.language}
                </div>
                <div className="inline-flex w-fit max-w-full flex-wrap rounded-full bg-slate-200/80 p-1">
                  {(["en", "vi"] as const).map((lang) => {
                    const active = language === lang;
                    return (
                      <button
                        key={lang}
                        type="button"
                        onClick={() => setLanguage(lang)}
                        className={`rounded-full px-3 py-1.5 text-xs font-semibold transition ${
                          active
                            ? "bg-slate-900 text-white shadow"
                            : "text-slate-700 hover:bg-white hover:text-slate-900"
                        }`}
                      >
                        {lang === "en" ? t.labels.english : t.labels.vietnamese}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="flex items-center justify-between text-sm font-medium text-slate-600">
                <span>{t.labels.progress}</span>
                <span>{stats.progress_percent.toFixed(1)}%</span>
              </div>
              <div className="h-3 overflow-hidden rounded-full bg-slate-200">
                <div
                  className="h-full rounded-full bg-gradient-to-r from-sky-500 via-blue-600 to-cyan-500 transition-[width] duration-500 ease-out"
                  style={{ width: progressWidth }}
                />
              </div>
              <div className="grid gap-3 text-sm text-slate-600 sm:grid-cols-2">
                <div className="rounded-2xl bg-white/92 p-3 ring-1 ring-slate-200/70">
                  <p className="text-xs uppercase tracking-[0.2em] text-slate-500">
                    {t.labels.linesProcessed}
                  </p>
                  <p className="mt-1 text-xl font-semibold text-slate-900">
                    {stats.processed_lines.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                  </p>
                </div>
                <div className="rounded-2xl bg-white/92 p-3 ring-1 ring-slate-200/70">
                  <p className="text-xs uppercase tracking-[0.2em] text-slate-500">
                    {t.labels.elapsed}
                  </p>
                  <p className="mt-1 text-xl font-semibold text-slate-900">
                    {formatDuration(stats.elapsed_ms)}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.28fr)_minmax(320px,0.92fr)] xl:gap-6">
          <div className="space-y-4 rounded-[2rem] border border-white/80 bg-white/72 p-5 shadow-glass backdrop-blur-2xl sm:p-6 lg:p-8">
            <div
              className={`relative overflow-hidden rounded-[1.75rem] border border-dashed p-6 transition-all duration-300 sm:p-8 ${
                dragActive
                  ? "border-sky-500 bg-sky-50/95 shadow-lg"
                  : "border-slate-300 bg-white/88"
              }`}
            >
              <div className="absolute inset-0 bg-gradient-to-br from-white/40 via-transparent to-sky-100/30" />
              <div className="relative flex flex-col items-center gap-4 text-center">
                <div className="rounded-[1.5rem] bg-slate-900 px-5 py-4 text-white shadow-lg">
                  <UploadCloud className="h-8 w-8" />
                </div>
                <div className="max-w-2xl space-y-2">
                  <h2 className="text-balance text-2xl font-semibold text-slate-900">
                    {t.labels.dragTitle}
                  </h2>
                  <p className="text-pretty break-words text-sm leading-6 text-slate-600 sm:text-base">
                    {t.labels.dragBody}
                  </p>
                </div>

                <button
                  type="button"
                  onClick={pickInputFile}
                  className="inline-flex items-center gap-2 rounded-full bg-slate-900 px-5 py-3 text-sm font-medium text-white shadow-lg shadow-slate-900/10 transition hover:bg-slate-800"
                >
                  <UploadCloud className="h-4 w-4" />
                  {t.labels.chooseFile}
                </button>
              </div>
            </div>

            <div className="grid gap-4">
              <div className="rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-medium text-slate-600">{t.labels.selectedFile}</p>
                    <p className="mt-2 break-words text-base font-semibold leading-7 text-slate-900">
                      {selectedFilePath || t.labels.noFile}
                    </p>
                  </div>
                  <FileSpreadsheet className="h-5 w-5 shrink-0 text-slate-500" />
                </div>
              </div>

              <div className="rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60">
                <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-slate-600">{t.labels.outputFolder}</p>
                    <p className="mt-2 break-words text-base font-semibold leading-7 text-slate-900">
                      {outputDir || t.labels.noFolder}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={pickOutputDir}
                    className="inline-flex w-full shrink-0 items-center justify-center gap-2 rounded-full border border-slate-300 bg-white px-4 py-2.5 text-sm font-medium text-slate-800 transition hover:border-slate-400 hover:bg-slate-50 sm:w-fit"
                  >
                    <FolderOpen className="h-4 w-4" />
                    {t.labels.selectFolder}
                  </button>
                </div>
              </div>

              <div className={`rounded-[1.5rem] border px-4 py-3 text-sm ${bannerStyles}`}>
                <div className="flex items-start gap-3">
                  {banner.tone === "success" ? (
                    <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0" />
                  ) : banner.tone === "error" ? (
                    <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
                  ) : (
                    <LoaderCircle
                      className={`mt-0.5 h-5 w-5 shrink-0 ${isProcessing ? "animate-spin" : ""}`}
                    />
                  )}
                  <p className="leading-6">{banner.message}</p>
                </div>
              </div>

              {showSummary ? (
                <div className="rounded-[1.75rem] border border-emerald-300 bg-gradient-to-br from-emerald-50 via-white to-sky-50 p-5 shadow-glass">
                  <div className="flex items-start gap-3">
                    <div className="rounded-2xl bg-emerald-600 p-3 text-white shadow-lg shadow-emerald-600/25">
                      <CheckCircle2 className="h-5 w-5" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <h3 className="text-lg font-semibold text-slate-900">
                        {t.labels.summaryTitle}
                      </h3>
                      <p className="mt-1 text-sm leading-6 text-slate-700">
                        {t.labels.summaryBody}
                      </p>
                    </div>
                  </div>

                  <div className="mt-4 grid gap-3 xl:grid-cols-2">
                    <div className="rounded-2xl bg-white/95 p-4 ring-1 ring-emerald-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryTotal}
                      </p>
                      <p className="mt-2 text-2xl font-semibold text-slate-900">
                        {totalClassified.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                      </p>
                      <p className="mt-2 text-sm text-slate-600">
                        {t.labels.elapsed}: {formatDuration(stats.elapsed_ms)}
                      </p>
                    </div>

                    <div className="rounded-2xl bg-white/95 p-4 ring-1 ring-emerald-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryFolder}
                      </p>
                      <p className="mt-2 break-words text-sm font-medium leading-6 text-slate-900">
                        {resolvedOutputDir}
                      </p>
                    </div>
                  </div>

                  <div className="mt-4 grid gap-3 sm:grid-cols-2 2xl:grid-cols-4">
                    <div className="rounded-2xl bg-white/92 p-4 ring-1 ring-red-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryInvalidRate}
                      </p>
                      <p className="mt-2 text-xl font-semibold text-slate-900">
                        {formatPercent(invalidRate)}
                      </p>
                    </div>
                    <div className="rounded-2xl bg-white/92 p-4 ring-1 ring-blue-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryPublicRate}
                      </p>
                      <p className="mt-2 text-xl font-semibold text-slate-900">
                        {formatPercent(publicRate)}
                      </p>
                    </div>
                    <div className="rounded-2xl bg-white/92 p-4 ring-1 ring-emerald-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryEduRate}
                      </p>
                      <p className="mt-2 text-xl font-semibold text-slate-900">
                        {formatPercent(eduRate)}
                      </p>
                    </div>
                    <div className="rounded-2xl bg-white/92 p-4 ring-1 ring-amber-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryCustomRate}
                      </p>
                      <p className="mt-2 text-xl font-semibold text-slate-900">
                        {formatPercent(customRate)}
                      </p>
                    </div>
                  </div>
                </div>
              ) : null}

              <div className="flex flex-col gap-3 lg:flex-row">
                <button
                  type="button"
                  onClick={handleProcess}
                  disabled={!selectedFilePath || !outputDir || isProcessing}
                  className="inline-flex w-full items-center justify-center gap-2 rounded-full bg-primary px-5 py-3 text-sm font-medium text-white shadow-lg shadow-sky-600/25 transition hover:brightness-95 disabled:cursor-not-allowed disabled:bg-slate-300 lg:w-auto"
                >
                  {isProcessing ? (
                    <LoaderCircle className="h-4 w-4 animate-spin" />
                  ) : (
                    <Mail className="h-4 w-4" />
                  )}
                  {isProcessing ? t.labels.processing : t.labels.start}
                </button>

                <button
                  type="button"
                  onClick={openResultFolder}
                  disabled={!canOpenFolder}
                  className="inline-flex w-full items-center justify-center gap-2 rounded-full border border-slate-300 bg-white px-5 py-3 text-sm font-medium text-slate-800 transition hover:border-slate-400 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50 lg:w-auto"
                >
                  <FolderOpen className="h-4 w-4" />
                  {t.labels.openFolder}
                </button>
              </div>
            </div>
          </div>

          <div className="grid auto-rows-min gap-4">
            {statCards.map((card) => {
              const Icon = card.icon;
              const value = stats[card.key];

              return (
                <article
                  key={card.key}
                  className="rounded-[1.75rem] border border-white/80 bg-white/80 p-4 shadow-glass backdrop-blur-2xl sm:p-5"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <span
                        className={`inline-flex max-w-full items-center rounded-full px-3 py-1 text-xs font-medium ring-1 ${card.chip}`}
                      >
                        {t.labels[card.key]}
                      </span>
                      <p className="mt-4 text-3xl font-semibold tracking-tight text-slate-900">
                        {value.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                      </p>
                    </div>
                    <div className="rounded-2xl bg-slate-800 p-3 text-white shadow-sm shadow-slate-900/10">
                      <Icon className="h-5 w-5" />
                    </div>
                  </div>
                </article>
              );
            })}

            <article className="rounded-[1.75rem] border border-white/60 bg-slate-900 p-5 text-white shadow-float">
              <p className="text-sm uppercase tracking-[0.2em] text-slate-400">
                {t.labels.classified}
              </p>
              <p className="mt-3 text-4xl font-semibold tracking-tight">
                {totalClassified.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
              </p>
              <p className="mt-2 text-pretty break-words text-sm leading-6 text-slate-300">
                {t.labels.classifiedBody}
              </p>
            </article>
          </div>
        </section>
      </div>
    </main>
  );
}
