import {
  CheckCircle,
  ChevronDown,
  ChevronUp,
  Clock,
  FileSpreadsheet,
  FolderOpen,
  LoaderCircle,
  Mail,
  ShieldCheck,
} from "lucide-react";
import type {
  ActiveTab,
  ProcessingPayload,
} from "../lib/app-state";
import {
  DEFAULT_MAX_CONCURRENT,
  DEFAULT_TIMEOUT_MS,
  basename,
} from "../lib/app-state";
import type { Language, TranslationLabels } from "../i18n";

export function SettingsPanel({
  activeTab,
  isProcessing,
  labels,
  language,
  outputDir,
  selectedFiles,
  showAdvancedDns,
  smtpEnabled,
  stats,
  targetDomains,
  timeoutMs,
  maxConcurrent,
  usePersistentCache,
  vpsApiKey,
  vpsApiUrl,
  onChangeMaxConcurrent,
  onChangeTargetDomains,
  onChangeTimeoutMs,
  onChangeVpsApiKey,
  onChangeVpsApiUrl,
  onPickOutputDir,
  onStartProcessing,
  onToggleAdvancedDns,
  onTogglePersistentCache,
  onToggleSmtpEnabled,
}: {
  activeTab: ActiveTab;
  isProcessing: boolean;
  labels: TranslationLabels;
  language: Language;
  outputDir: string;
  selectedFiles: string[];
  showAdvancedDns: boolean;
  smtpEnabled: boolean;
  stats: ProcessingPayload;
  targetDomains: string;
  timeoutMs: number;
  maxConcurrent: number;
  usePersistentCache: boolean;
  vpsApiKey: string;
  vpsApiUrl: string;
  onChangeMaxConcurrent: (value: number) => void;
  onChangeTargetDomains: (value: string) => void;
  onChangeTimeoutMs: (value: number) => void;
  onChangeVpsApiKey: (value: string) => void;
  onChangeVpsApiUrl: (value: string) => void;
  onPickOutputDir: () => void;
  onStartProcessing: () => void;
  onToggleAdvancedDns: () => void;
  onTogglePersistentCache: () => void;
  onToggleSmtpEnabled: () => void;
}) {
  const verifyMode = activeTab === "verify";

  return (
    <div className="flex flex-col gap-4 lg:col-span-5">
      <div className="flex-1 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-slate-100">
        <div className="space-y-4">
          <div>
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
              {labels.selectedFile}
            </label>
            <div className="mt-1.5 flex min-w-0 items-center gap-2 rounded-2xl bg-slate-50 px-3 py-2.5 ring-1 ring-slate-100">
              <FileSpreadsheet className="h-4 w-4 shrink-0 text-slate-400" />
              <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-700">
                {selectedFiles.length > 0
                  ? selectedFiles.length === 1
                    ? basename(selectedFiles[0])
                    : `${selectedFiles.length} ${
                        language === "vi" ? "tệp" : "files"
                      }`
                  : labels.noFile}
              </span>
            </div>
          </div>

          <div>
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
              {labels.outputFolder}
            </label>
            <div className="mt-1.5 flex min-w-0 items-center gap-2">
              <div className="flex min-w-0 flex-1 items-center gap-2 rounded-2xl bg-slate-50 px-3 py-2.5 ring-1 ring-slate-100">
                <FolderOpen className="h-4 w-4 shrink-0 text-slate-400" />
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-600">
                  {outputDir || labels.noFolder}
                </span>
              </div>
              <button
                onClick={onPickOutputDir}
                className="shrink-0 rounded-2xl bg-slate-100 px-3 py-2.5 text-sm font-bold text-slate-700 transition hover:bg-slate-200"
              >
                {labels.selectFolder}
              </button>
            </div>
          </div>

          {activeTab === "filter" && (
            <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
              <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.targetedInputLabel}
              </label>
              <textarea
                rows={3}
                value={targetDomains}
                onChange={(event) => onChangeTargetDomains(event.target.value)}
                placeholder={labels.targetedInputPlaceholder}
                className="mt-1.5 w-full resize-none rounded-2xl bg-slate-50 px-4 py-3 text-sm text-slate-900 ring-1 ring-slate-100 transition placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-400/60"
              />
              <p className="mt-1 text-[11px] leading-relaxed text-slate-400">
                {language === "vi"
                  ? "Mỗi dòng một domain, hoặc phân cách bằng dấu phẩy."
                  : "One domain per line, or separated by commas."}
              </p>
            </div>
          )}

          {verifyMode && (
            <div className="animate-in fade-in slide-in-from-bottom-2 flex flex-col gap-4 duration-300">
              <div className="rounded-2xl border border-slate-200 bg-slate-50/50 p-4">
                <div className="mb-3 flex items-center justify-between">
                  <p className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                    {language === "vi" ? "⚙️ Cấu hình DNS" : "⚙️ DNS Config"}
                  </p>
                  <button
                    onClick={onToggleAdvancedDns}
                    className="flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-bold text-slate-500 transition-colors hover:bg-slate-200 hover:text-slate-800"
                  >
                    {language === "vi" ? "Nâng cao" : "Advanced"}
                    {showAdvancedDns ? (
                      <ChevronUp className="h-3 w-3" />
                    ) : (
                      <ChevronDown className="h-3 w-3" />
                    )}
                  </button>
                </div>

                {showAdvancedDns && (
                  <div className="mb-3 grid animate-in fade-in slide-in-from-top-2 grid-cols-1 gap-3 px-1 sm:grid-cols-2">
                    <div>
                      <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                        {labels.timeoutLabel}
                      </label>
                      <input
                        type="number"
                        min={250}
                        max={5000}
                        step={50}
                        value={timeoutMs}
                        onChange={(event) =>
                          onChangeTimeoutMs(
                            Math.max(
                              250,
                              Math.min(
                                5000,
                                Number(event.target.value) ||
                                  DEFAULT_TIMEOUT_MS,
                              ),
                            ),
                          )
                        }
                        className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-slate-200 transition focus:outline-none focus:ring-2 focus:ring-sky-400/60"
                      />
                      <p className="mt-1 text-[11px] leading-relaxed text-slate-400">
                        {labels.timeoutHint}
                      </p>
                    </div>
                    <div>
                      <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                        {labels.concurrencyLabel}
                      </label>
                      <input
                        type="number"
                        min={1}
                        max={50}
                        step={1}
                        value={maxConcurrent}
                        onChange={(event) =>
                          onChangeMaxConcurrent(
                            Math.max(
                              1,
                              Math.min(
                                50,
                                Number(event.target.value) ||
                                  DEFAULT_MAX_CONCURRENT,
                              ),
                            ),
                          )
                        }
                        className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-slate-200 transition focus:outline-none focus:ring-2 focus:ring-sky-400/60"
                      />
                      <p className="mt-1 text-[11px] leading-relaxed text-slate-400">
                        {labels.concurrencyHint}
                      </p>
                    </div>
                  </div>
                )}

                <label
                  className="mt-3 flex cursor-pointer items-center justify-between gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2.5 transition hover:bg-slate-50"
                  onClick={onTogglePersistentCache}
                >
                  <div className="min-w-0">
                    <span className="text-sm font-semibold text-slate-700">
                      {labels.persistentCacheLabel}
                    </span>
                    <p className="mt-0.5 text-[11px] leading-relaxed text-slate-400">
                      {labels.persistentCacheHint}
                    </p>
                  </div>
                  <div className="relative shrink-0">
                    <div
                      className={`h-5 w-9 rounded-full transition-colors ${
                        usePersistentCache ? "bg-sky-500" : "bg-slate-300"
                      }`}
                    />
                    <div
                      className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform ${
                        usePersistentCache
                          ? "translate-x-4"
                          : "translate-x-0"
                      }`}
                    />
                  </div>
                </label>

                {usePersistentCache && stats.cache_hits > 0 && (
                  <div className="mt-2 flex items-center gap-1.5 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs font-semibold text-emerald-800">
                    <CheckCircle className="h-3.5 w-3.5 shrink-0" />
                    {labels.cacheStatus(stats.cache_hits)}
                  </div>
                )}
              </div>

              <div className="rounded-2xl border border-violet-200 bg-gradient-to-br from-violet-50 to-indigo-50 p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-[10px] font-bold uppercase tracking-widest text-violet-600">
                        {language === "vi"
                          ? "📧 Xác Minh SMTP (VPS)"
                          : "📧 SMTP Verify (VPS)"}
                      </p>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-bold ${
                          smtpEnabled
                            ? vpsApiUrl && vpsApiKey
                              ? "bg-emerald-100 text-emerald-700"
                              : "bg-amber-100 text-amber-700"
                            : "bg-slate-200 text-slate-500"
                        }`}
                      >
                        {smtpEnabled
                          ? vpsApiUrl && vpsApiKey
                            ? language === "vi"
                              ? "Đã cấu hình"
                              : "Configured"
                            : language === "vi"
                              ? "Cần cấu hình"
                              : "Needs setup"
                          : "OFF"}
                      </span>
                    </div>
                    <p className="mt-1 text-[11px] leading-relaxed text-violet-700/70">
                      {labels.smtpVerifyHint}
                    </p>
                  </div>
                  <button
                    onClick={onToggleSmtpEnabled}
                    className={`mt-1 flex h-6 w-11 shrink-0 items-center rounded-full transition-colors ${
                      smtpEnabled ? "bg-violet-600" : "bg-slate-300"
                    }`}
                  >
                    <span
                      className={`mx-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
                        smtpEnabled ? "translate-x-5" : "translate-x-0"
                      }`}
                    />
                  </button>
                </div>

                {smtpEnabled && (
                  <div className="mt-3 flex flex-col gap-3">
                    <div>
                      <label className="text-[10px] font-bold uppercase tracking-widest text-violet-600">
                        {labels.vpsApiUrlLabel}
                      </label>
                      <input
                        type="url"
                        value={vpsApiUrl}
                        onChange={(event) =>
                          onChangeVpsApiUrl(event.target.value)
                        }
                        placeholder={labels.vpsApiUrlPlaceholder}
                        className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-violet-200 transition placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-400/60"
                      />
                    </div>
                    <div>
                      <label className="text-[10px] font-bold uppercase tracking-widest text-violet-600">
                        {labels.vpsApiKeyLabel}
                      </label>
                      <input
                        type="password"
                        value={vpsApiKey}
                        onChange={(event) =>
                          onChangeVpsApiKey(event.target.value)
                        }
                        placeholder={labels.vpsApiKeyPlaceholder}
                        className="mt-1.5 w-full rounded-2xl bg-white px-3 py-2.5 text-sm text-slate-900 ring-1 ring-violet-200 transition placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-400/60"
                      />
                    </div>
                    {stats.smtp_elapsed_ms > 0 && (
                      <div className="flex items-center gap-1.5 rounded-xl border border-violet-200 bg-white/60 px-3 py-2 text-xs font-semibold text-violet-700">
                        <Clock className="h-3.5 w-3.5 shrink-0" />
                        {labels.smtpElapsed}:{" "}
                        {(stats.smtp_elapsed_ms / 1000).toFixed(1)}s
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
        onClick={onStartProcessing}
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
            <span>{labels.processing}</span>
          </>
        ) : (
          <>
            {verifyMode ? (
              <ShieldCheck className="h-5 w-5 transition-transform group-hover:scale-110" />
            ) : (
              <Mail className="h-5 w-5 transition-transform group-hover:-rotate-12" />
            )}
            <span>
              {verifyMode
                ? language === "vi"
                  ? "Bắt đầu Xác Minh DNS"
                  : "Start DNS Verify"
                : language === "vi"
                  ? "Bắt đầu Lọc"
                  : "Start Filtering"}
            </span>
          </>
        )}
      </button>
    </div>
  );
}
