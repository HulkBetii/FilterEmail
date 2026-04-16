import { VerifySummaryCard } from "./verify-ui";
import { translations, type Language } from "../i18n";

type ProcessingPayload = {
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
};

type Labels = (typeof translations.en)["labels"];

export function FinalSummary({
  verifyMode,
  labels,
  totalClassified,
  stats,
  resolvedOutputDir,
  canOpenFolder,
  verifyDeliverableCount,
  verifyDeliverableRate,
  verifyDeadRate,
  verifyReviewCount,
  verifyReviewRate,
  verifyFallbackRate,
  verifyParkedRate,
  verifyDisposableRate,
  verifyTypoRate,
  verifyDomainCount,
  smtpCheckedCount,
  smtpDeliverableRate,
  smtpRejectedRate,
  smtpCatchallRate,
  smtpUnknownRate,
  invalidRate,
  publicRate,
  eduRate,
  targetedRate,
  customRate,
  validCount,
  formatNumber,
  onOpenFolder,
}: {
  verifyMode: boolean;
  labels: Labels;
  totalClassified: number;
  stats: ProcessingPayload;
  resolvedOutputDir: string;
  canOpenFolder: boolean;
  verifyDeliverableCount: number;
  verifyDeliverableRate: number;
  verifyDeadRate: number;
  verifyReviewCount: number;
  verifyReviewRate: number;
  verifyFallbackRate: number;
  verifyParkedRate: number;
  verifyDisposableRate: number;
  verifyTypoRate: number;
  verifyDomainCount: number;
  smtpCheckedCount: number;
  smtpDeliverableRate: number;
  smtpRejectedRate: number;
  smtpCatchallRate: number;
  smtpUnknownRate: number;
  invalidRate: number;
  publicRate: number;
  eduRate: number;
  targetedRate: number;
  customRate: number;
  validCount: number;
  formatNumber: (value: number) => string;
  onOpenFolder: () => void;
}) {
  return (
    <section className="rounded-3xl bg-white p-5 shadow-sm ring-1 ring-slate-100">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h3 className="text-lg font-bold text-slate-900">
            {verifyMode ? labels.verifySummaryTitle : labels.summaryTitle}
          </h3>
          <p className="mt-1 text-sm leading-relaxed text-slate-500">
            {verifyMode ? labels.verifySummaryBody : labels.summaryBody}
          </p>
        </div>
        <button
          onClick={onOpenFolder}
          disabled={!canOpenFolder}
          className="rounded-2xl bg-slate-900 px-4 py-2 text-sm font-semibold text-white transition hover:bg-slate-700 disabled:pointer-events-none disabled:opacity-40"
        >
          {labels.openFolder}
        </button>
      </div>

      {verifyMode ? (
        <>
          <div className="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.summaryTotal}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">
                {formatNumber(totalClassified)}
              </div>
            </div>
            <div className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-emerald-700">
                {labels.summaryVerified}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-emerald-900">
                {formatNumber(verifyDeliverableCount)}
              </div>
              <div className="mt-1 text-xs font-semibold text-emerald-700">
                {verifyDeliverableRate.toFixed(1)}%
              </div>
            </div>
            <div className="rounded-2xl border border-red-200 bg-red-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-red-600">
                {labels.mx_dead}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-red-900">
                {formatNumber(stats.mx_dead)}
              </div>
              <div className="mt-1 text-xs font-semibold text-red-600">{verifyDeadRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-amber-700">
                {labels.mx_inconclusive}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-amber-900">
                {formatNumber(verifyReviewCount)}
              </div>
              <div className="mt-1 text-xs font-semibold text-amber-700">
                {verifyReviewRate.toFixed(1)}%
              </div>
            </div>
          </div>

          <div className="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <VerifySummaryCard bucket="mx_has_mx" label={labels.mx_has_mx} value={formatNumber(stats.mx_has_mx)} />
            <VerifySummaryCard bucket="mx_a_fallback" label={labels.summaryFallbackRate} value={`${verifyFallbackRate.toFixed(1)}%`} />
            <VerifySummaryCard bucket="mx_dead" label={labels.summaryDeadRate} value={`${verifyDeadRate.toFixed(1)}%`} />
            <VerifySummaryCard bucket="mx_inconclusive" label={labels.summaryReviewRate} value={`${verifyReviewRate.toFixed(1)}%`} />
            <VerifySummaryCard bucket="mx_parked" label={labels.summaryParkedRate} value={`${verifyParkedRate.toFixed(1)}%`} />
            <VerifySummaryCard bucket="mx_disposable" label={labels.summaryDisposableRate} value={`${verifyDisposableRate.toFixed(1)}%`} />
            <VerifySummaryCard bucket="mx_typo" label={labels.summaryTypoRate} value={`${verifyTypoRate.toFixed(1)}%`} />
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.summaryCacheHits}
              </div>
              <div className="mt-2 text-xl font-extrabold text-slate-900">
                {formatNumber(stats.cache_hits)}
              </div>
              <div className="mt-1 text-xs font-semibold text-slate-500">
                {labels.cacheCoverage(stats.cache_hits, verifyDomainCount)}
              </div>
            </div>
          </div>

          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.summaryFolder}
              </div>
              <div className="mt-2 truncate text-sm font-semibold text-slate-700">
                {resolvedOutputDir || "-"}
              </div>
            </div>
            <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm font-medium text-amber-800">
              {stats.mx_inconclusive > 0
                ? `${labels.mx_inconclusive}: ${formatNumber(stats.mx_inconclusive)}`
                : `${labels.summaryVerified}: ${formatNumber(verifyDeliverableCount)}`}
            </div>
          </div>

          {stats.smtp_enabled && (
            <>
              <div className="mt-5">
                <h4 className="text-sm font-bold text-slate-900">{labels.smtpSummaryTitle}</h4>
                <p className="mt-1 text-sm leading-relaxed text-slate-500">{labels.smtpSummaryBody}</p>
              </div>

              <div className="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
                <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
                  <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                    {labels.smtpChecked}
                  </div>
                  <div className="mt-2 text-2xl font-extrabold text-slate-900">
                    {formatNumber(smtpCheckedCount)}
                  </div>
                </div>
                <VerifySummaryCard
                  bucket="smtp_deliverable"
                  label={labels.smtp_deliverable}
                  value={`${formatNumber(stats.smtp_deliverable)} • ${smtpDeliverableRate.toFixed(1)}%`}
                />
                <VerifySummaryCard
                  bucket="smtp_rejected"
                  label={labels.smtp_rejected}
                  value={`${formatNumber(stats.smtp_rejected)} • ${smtpRejectedRate.toFixed(1)}%`}
                />
                <VerifySummaryCard
                  bucket="smtp_unknown"
                  label={labels.smtp_unknown}
                  value={`${formatNumber(stats.smtp_unknown)} • ${smtpUnknownRate.toFixed(1)}%`}
                />
              </div>

              <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
                <VerifySummaryCard
                  bucket="smtp_catchall"
                  label={labels.smtp_catchall}
                  value={`${formatNumber(stats.smtp_catchall)} • ${smtpCatchallRate.toFixed(1)}%`}
                />
                <div className="rounded-2xl bg-slate-50 p-4">
                  <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                    {labels.smtpElapsed}
                  </div>
                  <div className="mt-2 text-xl font-extrabold text-slate-900">
                    {(stats.smtp_elapsed_ms / 1000).toFixed(2)}s
                  </div>
                </div>
              </div>
            </>
          )}
        </>
      ) : (
        <>
          <div className="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-5">
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryTotal}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{formatNumber(totalClassified)}</div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryInvalidRate}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{invalidRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryPublicRate}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{publicRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryEduRate}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{eduRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryTargetedRate}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{targetedRate.toFixed(1)}%</div>
            </div>
          </div>

          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryCustomRate}</div>
              <div className="mt-2 text-2xl font-extrabold text-slate-900">{customRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">{labels.summaryFolder}</div>
              <div className="mt-2 truncate text-sm font-semibold text-slate-700">{resolvedOutputDir || "-"}</div>
            </div>
          </div>

          <div className="mt-3 rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm font-medium text-amber-800">
            {`${labels.valid}: ${formatNumber(validCount)}`}
          </div>
        </>
      )}
    </section>
  );
}
