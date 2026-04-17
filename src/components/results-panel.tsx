import { STAT_CARDS } from "../lib/stat-cards";
import type { ProcessingPayload } from "../lib/app-state";
import type { Language, TranslationLabels } from "../i18n";
import { VerifyHeroCard, VerifySummaryCard } from "./verify-ui";

export function ResultsPanel({
  formatNumber,
  labels,
  language,
  showDnsDiag,
  showSmtpDiag,
  stats,
  totalClassified,
  verifyMode,
  onToggleDnsDiag,
  onToggleSmtpDiag,
}: {
  formatNumber: (value: number) => string;
  labels: TranslationLabels;
  language: Language;
  showDnsDiag: boolean;
  showSmtpDiag: boolean;
  stats: ProcessingPayload;
  totalClassified: number;
  verifyMode: boolean;
  onToggleDnsDiag: () => void;
  onToggleSmtpDiag: () => void;
}) {
  return (
    <div className="space-y-3 lg:col-span-7">
      {verifyMode && (
        <>
          {stats.processed_lines > 0 && (
            <div className="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
              <VerifyHeroCard
                bucket="final_alive"
                label={labels.final_alive}
                value={formatNumber(stats.final_alive)}
                fileName="30_T4_FINAL_Alive.txt"
              />
              <VerifyHeroCard
                bucket="final_dead"
                label={labels.final_dead}
                value={formatNumber(stats.final_dead)}
                fileName="31_T4_FINAL_Dead.txt"
              />
              <VerifyHeroCard
                bucket="final_unknown"
                label={labels.final_unknown}
                value={formatNumber(stats.final_unknown)}
                fileName="32_T4_FINAL_Unknown.txt"
              />
            </div>
          )}

          {stats.smtp_enabled && stats.processed_lines > 0 && (
            <div className="mb-4 rounded-2xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm font-medium text-sky-900">
              {labels.smtp_alive_note}
            </div>
          )}

          {stats.smtp_enabled && stats.processed_lines > 0 && (
            <div className="mb-4 rounded-3xl border border-violet-200 bg-white shadow-sm">
              <button
                onClick={onToggleSmtpDiag}
                className="flex w-full cursor-pointer items-center justify-between p-5 text-left"
              >
                <div className="flex flex-col gap-1.5">
                  <h3 className="text-base font-extrabold text-slate-900">
                    {language === "vi"
                      ? "T3: Phân Tích Lỗi SMTP"
                      : "T3: SMTP Diagnostics"}
                  </h3>
                  <p className="text-xs font-medium text-slate-500">
                    {formatNumber(stats.smtp_attempted_emails)} attempted •{" "}
                    {stats.smtp_coverage_percent.toFixed(1)}% coverage •{" "}
                    {formatNumber(stats.smtp_cache_hits)} cached
                  </p>
                </div>
                <div
                  className={`flex items-center justify-center rounded-full bg-violet-100 p-2 text-violet-600 transition-transform duration-300 ${
                    showSmtpDiag ? "rotate-180" : "rotate-0"
                  }`}
                >
                  <svg
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="m6 9 6 6 6-6" />
                  </svg>
                </div>
              </button>
              <div
                className={`overflow-hidden transition-all duration-300 ease-in-out ${
                  showSmtpDiag ? "max-h-[600px] opacity-100" : "max-h-0 opacity-0"
                }`}
              >
                <div className="border-t border-violet-100 p-5">
                  <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                    <VerifySummaryCard
                      bucket="smtp_deliverable"
                      label={labels.smtp_deliverable}
                      value={formatNumber(stats.smtp_deliverable)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_catchall"
                      label={labels.smtp_catchall}
                      value={formatNumber(stats.smtp_catchall)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_policy_blocked"
                      label={labels.smtp_policy_blocked}
                      value={formatNumber(stats.smtp_policy_blocked)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_bad_mailbox"
                      label={labels.smtp_bad_mailbox}
                      value={formatNumber(stats.smtp_bad_mailbox)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_bad_domain"
                      label={labels.smtp_bad_domain}
                      value={formatNumber(stats.smtp_bad_domain)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_mailbox_full"
                      label={labels.smtp_mailbox_full}
                      value={formatNumber(stats.smtp_mailbox_full)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_mailbox_disabled"
                      label={labels.smtp_mailbox_disabled}
                      value={formatNumber(stats.smtp_mailbox_disabled)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_temp_failure"
                      label={labels.smtp_temp_failure}
                      value={formatNumber(stats.smtp_temp_failure)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_network_error"
                      label={labels.smtp_network_error}
                      value={formatNumber(stats.smtp_network_error)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_protocol_error"
                      label={labels.smtp_protocol_error}
                      value={formatNumber(stats.smtp_protocol_error)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_timeout"
                      label={labels.smtp_timeout}
                      value={formatNumber(stats.smtp_timeout)}
                    />
                    <VerifySummaryCard
                      bucket="smtp_unknown"
                      label={labels.smtp_unknown}
                      value={formatNumber(stats.smtp_unknown)}
                    />
                  </div>
                </div>
              </div>
            </div>
          )}

          {stats.processed_lines > 0 && (
            <div className="mb-4 rounded-3xl border border-slate-200 bg-white shadow-sm">
              <button
                onClick={onToggleDnsDiag}
                className="flex w-full cursor-pointer items-center justify-between p-5 text-left"
              >
                <div className="flex flex-col gap-1.5">
                  <h3 className="text-base font-extrabold text-slate-900">
                    {language === "vi"
                      ? "T2: Báo Cáo Sức Khỏe DNS"
                      : "T2: DNS Health"}
                  </h3>
                  <p className="text-xs font-medium text-slate-500">
                    {language === "vi"
                      ? `${formatNumber(stats.mx_has_mx)} domain có MX hợp lệ`
                      : `${formatNumber(stats.mx_has_mx)} domains with valid MX`}
                  </p>
                </div>
                <div
                  className={`flex items-center justify-center rounded-full bg-slate-100 p-2 text-slate-600 transition-transform duration-300 ${
                    showDnsDiag ? "rotate-180" : "rotate-0"
                  }`}
                >
                  <svg
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="m6 9 6 6 6-6" />
                  </svg>
                </div>
              </button>
              <div
                className={`overflow-hidden transition-all duration-300 ease-in-out ${
                  showDnsDiag ? "max-h-[500px] opacity-100" : "max-h-0 opacity-0"
                }`}
              >
                <div className="space-y-4 border-t border-slate-100 p-5">
                  <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                    <VerifySummaryCard
                      bucket="mx_has_mx"
                      label={labels.mx_has_mx}
                      value={formatNumber(stats.mx_has_mx)}
                    />
                    <VerifySummaryCard
                      bucket="mx_a_fallback"
                      label={labels.mx_a_fallback}
                      value={formatNumber(stats.mx_a_fallback)}
                    />
                    <VerifySummaryCard
                      bucket="mx_dead"
                      label={labels.mx_dead}
                      value={formatNumber(stats.mx_dead)}
                    />
                    <VerifySummaryCard
                      bucket="mx_inconclusive"
                      label={labels.mx_inconclusive}
                      value={formatNumber(stats.mx_inconclusive)}
                    />
                  </div>
                  {(stats.mx_parked > 0 ||
                    stats.mx_disposable > 0 ||
                    stats.mx_typo > 0) && (
                    <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4">
                      <p className="mb-3 text-[10px] font-bold uppercase tracking-widest text-amber-700">
                        {language === "vi"
                          ? "⚠️ Cần Kiểm Tra (Rủi Ro)"
                          : "⚠️ Review Required"}
                      </p>
                      <div className="grid grid-cols-3 gap-2">
                        {[
                          {
                            key: "mx_parked",
                            label: labels.mx_parked,
                            value: stats.mx_parked,
                            color: "text-amber-700",
                          },
                          {
                            key: "mx_disposable",
                            label: labels.mx_disposable,
                            value: stats.mx_disposable,
                            color: "text-orange-700",
                          },
                          {
                            key: "mx_typo",
                            label: labels.mx_typo,
                            value: stats.mx_typo,
                            color: "text-violet-700",
                          },
                        ].map(({ key, label, value, color }) => (
                          <div
                            key={key}
                            className="flex flex-col items-center rounded-xl border border-amber-200 bg-white px-3 py-3 text-center shadow-sm"
                          >
                            <p
                              className={`text-xl font-extrabold leading-none ${color}`}
                            >
                              {formatNumber(value)}
                            </p>
                            <p className="mt-1 text-[10px] font-bold uppercase tracking-wide text-slate-500">
                              {label}
                            </p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </>
      )}

      {verifyMode ? (
        stats.processed_lines > 0 && (
          <div className="rounded-2xl border border-slate-200 bg-slate-50/50 p-4">
            <p className="mb-3 text-[10px] font-bold uppercase tracking-widest text-slate-400">
              {language === "vi"
                ? "Tiền Xử Lý (Basic Filter)"
                : "Pre-processing (Basic Filter)"}
            </p>
            <div className="flex flex-wrap items-center gap-2">
              {STAT_CARDS.filter((card) =>
                [
                  "invalid",
                  "public",
                  "edu",
                  "targeted",
                  "custom",
                  "duplicates",
                ].includes(card.key),
              ).map((card) => {
                const Icon = card.icon;
                const value = stats[card.key];
                const percentage =
                  totalClassified > 0
                    ? ((value / totalClassified) * 100).toFixed(1)
                    : "0.0";

                return (
                  <div
                    key={card.key}
                    className="flex items-center gap-1.5 rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs shadow-sm transition-all hover:bg-slate-50 hover:shadow"
                  >
                    <Icon className="h-3.5 w-3.5 text-slate-400" />
                    <span className="font-medium text-slate-500">
                      {labels[card.key]}:
                    </span>
                    <span className="font-bold text-slate-800">
                      {formatNumber(value)}
                    </span>
                    <span className="text-[10px] font-semibold text-slate-400">
                      ({percentage}%)
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )
      ) : (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {STAT_CARDS.filter((card) =>
            [
              "invalid",
              "public",
              "edu",
              "targeted",
              "custom",
              "duplicates",
            ].includes(card.key),
          ).map((card) => {
            const Icon = card.icon;
            const value = stats[card.key];
            const percentage =
              totalClassified > 0
                ? ((value / totalClassified) * 100).toFixed(1)
                : "0.0";

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
                    {formatNumber(value)}
                  </p>
                  <p className="mt-1 truncate text-[10px] font-bold uppercase tracking-wider text-slate-400">
                    {labels[card.key]}
                  </p>
                  <p className="text-[11px] font-semibold text-slate-300">
                    {percentage}%
                  </p>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}
