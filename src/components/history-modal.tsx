import { FolderOpen, History, Trash2, X } from "lucide-react";
import { VerifyHistoryGroup, type VerifyBucketKey } from "./verify-ui";
import type { Language } from "../i18n";
import { translations } from "../i18n";

type ProcessingPayload = {
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
};

type HistoryEntry = {
  id: string;
  timestamp: number;
  fileNames: string[];
  mode: "filter" | "verify";
  stats: ProcessingPayload;
};

type StatCard = {
  key:
    | "invalid"
    | "public"
    | "edu"
    | "targeted"
    | "custom"
    | "duplicates"
    | "mx_disposable"
    | "mx_has_mx"
    | "mx_a_fallback"
    | "mx_typo"
    | "mx_parked";
};

type Labels = (typeof translations.en)["labels"];

export function HistoryModal({
  isOpen,
  history,
  language,
  labels,
  statCards,
  formatNumber,
  onClose,
  onOpenFolder,
  onClearHistory,
}: {
  isOpen: boolean;
  history: HistoryEntry[];
  language: Language;
  labels: Labels;
  statCards: StatCard[];
  formatNumber: (value: number) => string;
  onClose: () => void;
  onOpenFolder: (dir: string) => void;
  onClearHistory: () => void;
}) {
  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-sm">
      <div className="relative flex max-h-[85vh] w-full max-w-2xl flex-col rounded-3xl bg-white shadow-2xl ring-1 ring-slate-900/5">
        <div className="flex items-center justify-between border-b border-slate-100 px-6 py-4">
          <h2 className="flex items-center gap-2 text-xl font-bold text-slate-800">
            <History className="h-5 w-5 text-sky-500" />
            {labels.historyTitle}
          </h2>
          <button
            onClick={onClose}
            className="rounded-full p-2 text-slate-400 transition-colors hover:bg-slate-100 hover:text-slate-600"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="flex-1 space-y-4 overflow-y-auto px-6 py-4">
          {history.length === 0 ? (
            <div className="py-12 text-center text-sm text-slate-500">{labels.emptyHistory}</div>
          ) : (
            history.map((entry) => {
              const entryIsVerify = entry.mode === "verify";
              const total =
                entry.stats.invalid +
                entry.stats.public +
                entry.stats.edu +
                entry.stats.targeted +
                entry.stats.custom +
                entry.stats.duplicates +
                entry.stats.mx_dead +
                entry.stats.mx_has_mx +
                entry.stats.mx_a_fallback +
                entry.stats.mx_inconclusive +
                entry.stats.mx_parked +
                entry.stats.mx_disposable +
                entry.stats.mx_typo;
              const valid = entryIsVerify
                ? entry.stats.mx_has_mx + entry.stats.mx_a_fallback
                : entry.stats.public +
                  entry.stats.edu +
                  entry.stats.targeted +
                  entry.stats.custom;
              const review =
                entry.stats.mx_inconclusive +
                entry.stats.mx_parked +
                entry.stats.mx_disposable +
                entry.stats.mx_typo;
              const verifyDomainCount =
                entry.stats.mx_dead +
                entry.stats.mx_has_mx +
                entry.stats.mx_a_fallback +
                entry.stats.mx_inconclusive +
                entry.stats.mx_parked +
                entry.stats.mx_disposable +
                entry.stats.mx_typo;
              const smtpChecked =
                entry.stats.smtp_deliverable +
                entry.stats.smtp_rejected +
                entry.stats.smtp_catchall +
                entry.stats.smtp_unknown;
              const historyCards = entryIsVerify
                ? []
                : statCards.filter((card) =>
                    ["invalid", "public", "edu", "targeted", "custom", "duplicates"].includes(card.key),
                  );
              const successHistoryCards: VerifyBucketKey[] = ["mx_has_mx", "mx_a_fallback"];
              const reviewHistoryCards: VerifyBucketKey[] = [
                "mx_inconclusive",
                "mx_parked",
                "mx_disposable",
                "mx_typo",
              ];
              const failureHistoryCards: VerifyBucketKey[] = ["mx_dead"];
              const smtpHistoryCards: VerifyBucketKey[] = [
                "smtp_deliverable",
                "smtp_rejected",
                "smtp_catchall",
                "smtp_unknown",
              ];

              return (
                <div
                  key={entry.id}
                  className="rounded-2xl border border-slate-200 bg-slate-50 p-4 transition-all hover:border-sky-300"
                >
                  <div className="mb-2 flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <div className="mb-1 flex flex-wrap items-center gap-2">
                        <div className="text-xs font-semibold text-slate-400">
                          {new Date(entry.timestamp).toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                        </div>
                        <span
                          className={`inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
                            entryIsVerify
                              ? "bg-cyan-100 text-cyan-700 ring-1 ring-cyan-200"
                              : "bg-slate-200 text-slate-700 ring-1 ring-slate-300"
                          }`}
                        >
                          {entryIsVerify ? labels.tabDnsVerify : labels.tabBasicFilter}
                        </span>
                      </div>
                      <div className="break-all text-sm font-medium leading-tight text-slate-700">
                        {entry.fileNames.join(", ")}
                      </div>
                    </div>
                    <button
                      onClick={() => entry.stats.output_dir && onOpenFolder(entry.stats.output_dir)}
                      disabled={!entry.stats.output_dir}
                      className="flex shrink-0 items-center gap-1.5 rounded-lg bg-sky-100 px-3 py-1.5 text-xs font-semibold text-sky-700 shadow-sm transition-colors hover:bg-sky-200 disabled:pointer-events-none disabled:opacity-40"
                    >
                      <FolderOpen className="h-3.5 w-3.5" />
                      <span className="hidden sm:inline">{labels.openFolder}</span>
                    </button>
                  </div>

                  <div className="mt-3 flex flex-col gap-2">
                    <div className="grid grid-cols-2 gap-2 sm:grid-cols-5">
                      <div className="rounded-lg border border-slate-200 bg-white p-2 text-center shadow-sm">
                        <div className="text-[10px] font-bold uppercase text-slate-400">{labels.total}</div>
                        <div className="text-sm font-bold text-slate-800">{formatNumber(total)}</div>
                      </div>
                      <div className="rounded-lg border border-emerald-100 bg-emerald-50 p-2 text-center shadow-sm">
                        <div className="text-[10px] font-bold uppercase text-emerald-600">
                          {entryIsVerify ? labels.summaryVerified : labels.valid}
                        </div>
                        <div className="text-sm font-bold text-emerald-700">{formatNumber(valid)}</div>
                      </div>
                      {entryIsVerify ? (
                        <div className="rounded-lg border border-slate-200 bg-white p-2 text-center shadow-sm">
                          <div className="text-[10px] font-bold uppercase text-slate-400">
                            {labels.summaryCacheHits}
                          </div>
                          <div className="text-sm font-bold text-slate-600">
                            {formatNumber(entry.stats.cache_hits)}
                          </div>
                          <div className="mt-1 text-[10px] font-medium text-slate-400">
                            {labels.cacheCoverage(entry.stats.cache_hits, verifyDomainCount)}
                          </div>
                        </div>
                      ) : (
                        <div className="rounded-lg border border-slate-200 bg-white p-2 text-center shadow-sm">
                          <div className="text-[10px] font-bold uppercase text-slate-400">{labels.duplicates}</div>
                          <div className="text-sm font-bold text-slate-600">
                            {formatNumber(entry.stats.duplicates)}
                          </div>
                        </div>
                      )}
                      <div className="rounded-lg border border-red-100 bg-red-50 p-2 text-center shadow-sm">
                        <div className="text-[10px] font-bold uppercase text-red-500">{labels.deadDomains}</div>
                        <div className="text-sm font-bold text-red-700">
                          {formatNumber(entry.stats.mx_dead)}
                        </div>
                      </div>
                      <div className="rounded-lg border border-amber-100 bg-amber-50 p-2 text-center shadow-sm">
                        <div className="text-[10px] font-bold uppercase text-amber-600">
                          {entryIsVerify ? labels.reviewDomains : labels.invalid}
                        </div>
                        <div className="text-sm font-bold text-amber-700">
                          {formatNumber(entryIsVerify ? review : entry.stats.invalid)}
                        </div>
                      </div>
                    </div>

                    {entryIsVerify ? (
                      <div className="grid grid-cols-1 gap-3">
                        <VerifyHistoryGroup
                          title={labels.historySuccessGroup}
                          titleClassName="text-emerald-700"
                          className="rounded-xl border border-emerald-100 bg-emerald-50/60 p-3"
                          buckets={successHistoryCards}
                          getValue={(bucket) => entry.stats[bucket] || 0}
                          getLabel={(bucket) => labels[bucket]}
                          formatValue={formatNumber}
                        />
                        <VerifyHistoryGroup
                          title={labels.historyReviewGroup}
                          titleClassName="text-amber-700"
                          className="rounded-xl border border-amber-100 bg-amber-50/70 p-3"
                          buckets={reviewHistoryCards}
                          getValue={(bucket) => entry.stats[bucket] || 0}
                          getLabel={(bucket) => labels[bucket]}
                          formatValue={formatNumber}
                        />
                        <VerifyHistoryGroup
                          title={labels.historyFailureGroup}
                          titleClassName="text-red-700"
                          className="rounded-xl border border-red-100 bg-red-50/70 p-3"
                          buckets={failureHistoryCards}
                          getValue={(bucket) => entry.stats[bucket] || 0}
                          getLabel={(bucket) => labels[bucket]}
                          formatValue={formatNumber}
                        />
                        {entry.stats.smtp_enabled && (
                          <VerifyHistoryGroup
                            title={`${labels.historySmtpGroup} • ${formatNumber(smtpChecked)}`}
                            titleClassName="text-slate-700"
                            className="rounded-xl border border-slate-200 bg-slate-50 p-3"
                            buckets={smtpHistoryCards}
                            getValue={(bucket) => entry.stats[bucket] || 0}
                            getLabel={(bucket) => labels[bucket]}
                            formatValue={formatNumber}
                          />
                        )}
                      </div>
                    ) : (
                      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {historyCards.map((card) => {
                          const value = entry.stats[card.key] || 0;
                          return (
                            <div
                              key={card.key}
                              className="flex items-center justify-between rounded-lg border border-slate-100 bg-white px-3 py-2 shadow-sm"
                            >
                              <div className="text-[11px] font-medium uppercase text-slate-500">
                                {labels[card.key]}
                              </div>
                              <div className="text-sm font-bold text-slate-700">
                                {formatNumber(value)}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}

                    <div className="text-right text-[10px] font-medium text-slate-400">
                      {labels.elapsed}: {(entry.stats.elapsed_ms / 1000).toFixed(2)}s
                      {entry.stats.smtp_enabled
                        ? ` • ${labels.smtpElapsed}: ${(entry.stats.smtp_elapsed_ms / 1000).toFixed(2)}s`
                        : ""}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {history.length > 0 && (
          <div className="flex justify-end border-t border-slate-100 px-6 py-3">
            <button
              onClick={onClearHistory}
              className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-semibold text-red-600 transition-colors hover:bg-red-50"
            >
              <Trash2 className="h-4 w-4" />
              {labels.clearHistory}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
