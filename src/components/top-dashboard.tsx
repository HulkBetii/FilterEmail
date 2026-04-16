import { CloudUpload, FolderOpen, LoaderCircle } from "lucide-react";

type Labels = {
  dragBody: string;
  chooseFile: string;
  classified: string;
  classifiedBody: string;
  progress: string;
  openFolder: string;
};

export function TopDashboard({
  activeTab,
  language,
  dragActive,
  totalClassified,
  progressPercent,
  isProcessing,
  currentDomain,
  cacheHits,
  labels,
  canOpenFolder,
  onPickInputFile,
  onOpenResultFolder,
  onDragOver,
  onDragLeave,
  formatNumber,
}: {
  activeTab: "filter" | "verify";
  language: "en" | "vi";
  dragActive: boolean;
  totalClassified: number;
  progressPercent: number;
  isProcessing: boolean;
  currentDomain: string | null;
  cacheHits: number;
  labels: Labels;
  canOpenFolder: boolean;
  onPickInputFile: () => void;
  onOpenResultFolder: () => void;
  onDragOver: (event: React.DragEvent<HTMLDivElement>) => void;
  onDragLeave: () => void;
  formatNumber: (value: number) => string;
}) {
  return (
    <div className="grid grid-cols-1 gap-5 lg:grid-cols-12 lg:items-stretch">
      <div
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        className={`flex min-h-[12rem] flex-col items-center justify-center rounded-[2.5rem] border-2 border-dashed px-6 py-5 text-center transition-all lg:col-span-5 ${
          dragActive
            ? "scale-[1.01] border-sky-400 bg-sky-50 shadow-lg"
            : "border-slate-200/80 bg-white"
        }`}
      >
        <div
          className={`mb-3 flex h-12 w-12 shrink-0 items-center justify-center rounded-[1rem] transition-colors ${
            dragActive ? "bg-sky-500 text-white" : "bg-[#111827] text-white"
          }`}
        >
          <CloudUpload className="h-5 w-5" strokeWidth={2.5} />
        </div>
        <h3 className="text-lg font-bold tracking-tight text-slate-900">
          {activeTab === "filter"
            ? language === "vi"
              ? "Công Cụ Phân Loại Báo Cáo (Lọc Thường)"
              : "Basic Reporting Tool (Filter)"
            : language === "vi"
              ? "Công Cụ Xác Minh Domain (Deep DNS)"
              : "Domain Verification Tool (Deep)"}
        </h3>
        <p className="mt-1 max-w-[18rem] text-[0.85rem] leading-relaxed text-slate-500">
          {labels.dragBody}
        </p>
        <button
          onClick={onPickInputFile}
          className="mt-4 flex items-center gap-2 rounded-full bg-[#111827] px-6 py-2.5 text-[0.9rem] font-semibold text-white shadow-sm transition hover:bg-slate-800 active:scale-95"
        >
          <CloudUpload className="h-4 w-4" />
          {labels.chooseFile}
        </button>
      </div>

      <article className="relative min-h-[12rem] overflow-hidden rounded-[2.5rem] bg-[#0b1221] px-6 py-6 shadow-xl lg:col-span-7 lg:p-8">
        <div className="relative z-10 flex h-full flex-col justify-center gap-6 lg:flex-row lg:items-center lg:justify-between">
          <div className="max-w-lg min-w-0 flex-1">
            <p className="text-[10px] font-bold uppercase tracking-[0.2em] text-slate-400">
              {labels.classified}
            </p>
            <p className="mt-1 text-[3.25rem] font-bold leading-[0.9] tracking-tight text-white lg:text-[4rem]">
              {formatNumber(totalClassified)}
            </p>
            <p className="mt-2 text-[0.85rem] leading-relaxed text-slate-400">
              {labels.classifiedBody}
            </p>
          </div>

          <div className="w-full shrink-0 self-center rounded-[1.25rem] border border-white/5 bg-white/[0.03] p-5 backdrop-blur-md lg:w-[18rem]">
            <div className="flex items-center justify-between gap-3 text-xs font-bold text-slate-300">
              <span className="uppercase tracking-widest">{labels.progress}</span>
              <span className="text-sm text-white">{progressPercent.toFixed(1)}%</span>
            </div>
            <div className="mt-3 h-2 overflow-hidden rounded-full bg-slate-800/80">
              <div
                className="h-full rounded-full bg-gradient-to-r from-sky-400 to-indigo-500 shadow-[0_0_12px_rgba(56,189,248,0.5)] transition-all duration-300"
                style={{ width: `${progressPercent}%` }}
              />
            </div>
            {/* Real-time scanning indicator inside the dark card */}
            {isProcessing && activeTab === "verify" && currentDomain && (
              <div className="mt-3 flex items-center gap-2 overflow-hidden rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-[11px] font-semibold text-slate-300 backdrop-blur-sm">
                <LoaderCircle className="h-3 w-3 shrink-0 animate-spin text-sky-400" />
                <span className="min-w-0 truncate">
                  <span className="text-slate-400">{language === "vi" ? "Đang quét:" : "Scanning:"} </span>
                  <span className="text-white">{currentDomain}</span>
                </span>
                {cacheHits > 0 && (
                  <span className="ml-auto shrink-0 rounded-full bg-sky-500/20 px-2 py-0.5 text-sky-400">
                    {cacheHits} cached
                  </span>
                )}
              </div>
            )}
            <button
              onClick={onOpenResultFolder}
              disabled={!canOpenFolder}
              className="mt-4 flex w-full items-center justify-center gap-2 rounded-[0.85rem] bg-white py-2.5 text-[0.85rem] font-bold text-slate-900 shadow transition hover:bg-slate-100 active:scale-95 disabled:pointer-events-none disabled:opacity-30"
            >
              <FolderOpen className="h-4 w-4 shrink-0" />
              <span className="truncate">{labels.openFolder}</span>
            </button>
          </div>
        </div>
      </article>
    </div>
  );
}
