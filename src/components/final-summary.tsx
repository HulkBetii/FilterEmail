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
  final_alive: number;
  final_dead: number;
  final_unknown: number;
  smtp_attempted_emails: number;
  smtp_cache_hits: number;
  smtp_coverage_percent: number;
  smtp_policy_blocked: number;
  smtp_temp_failure: number;
  smtp_mailbox_full: number;
  smtp_mailbox_disabled: number;
  smtp_bad_mailbox: number;
  smtp_bad_domain: number;
  smtp_network_error: number;
  smtp_protocol_error: number;
  smtp_timeout: number;
};

type Labels = (typeof translations.en)["labels"];

export function FinalSummary({
  verifyMode,
  labels,
  totalClassified,
  finalTotal,
  stats,
  resolvedOutputDir,
  canOpenFolder,
  verifyDeliverableCount,
  verifyDeliverableRate,
  verifyDeadRate,
  verifyUnknownRate,
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
  smtpCoveragePercent,
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
  finalTotal: number;
  stats: ProcessingPayload;
  resolvedOutputDir: string;
  canOpenFolder: boolean;
  verifyDeliverableCount: number;
  verifyDeliverableRate: number;
  verifyDeadRate: number;
  verifyUnknownRate: number;
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
  smtpCoveragePercent: number;
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
                {formatNumber(finalTotal)}
              </div>
            </div>
            <div className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-emerald-700">
                {labels.final_alive}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-emerald-900">
                {formatNumber(stats.final_alive)}
              </div>
              <div className="mt-1 text-xs font-semibold text-emerald-700">
                {verifyDeliverableRate.toFixed(1)}%
              </div>
            </div>
            <div className="rounded-2xl border border-red-200 bg-red-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-red-600">
                {labels.final_dead}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-red-900">
                {formatNumber(stats.final_dead)}
              </div>
              <div className="mt-1 text-xs font-semibold text-red-600">{verifyDeadRate.toFixed(1)}%</div>
            </div>
            <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-amber-700">
                {labels.final_unknown}
              </div>
              <div className="mt-2 text-2xl font-extrabold text-amber-900">
                {formatNumber(stats.final_unknown)}
              </div>
              <div className="mt-1 text-xs font-semibold text-amber-700">
                {verifyUnknownRate.toFixed(1)}%
              </div>
            </div>
          </div>

          <div className="mt-3 rounded-2xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm font-medium text-sky-900">
            {labels.smtp_alive_note}
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

          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.summaryFolder}
              </div>
              <div className="mt-2 truncate text-sm font-semibold text-slate-700">
                {resolvedOutputDir || "-"}
              </div>
            </div>
            <div className="rounded-2xl bg-slate-50 p-4">
              <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                {labels.smtp_coverage_percent}
              </div>
              <div className="mt-2 text-xl font-extrabold text-slate-900">
                {smtpCoveragePercent.toFixed(1)}%
              </div>
              <div className="mt-1 text-xs font-semibold text-slate-500">
                {formatNumber(stats.smtp_attempted_emails)} / {formatNumber(stats.mx_has_mx)}
              </div>
            </div>
            <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm font-medium text-amber-800">
              {labels.reviewNote}
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
                    {labels.smtp_attempted_emails}
                  </div>
                  <div className="mt-2 text-2xl font-extrabold text-slate-900">
                    {formatNumber(stats.smtp_attempted_emails)}
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
                    {labels.smtp_cache_hits}
                  </div>
                  <div className="mt-2 text-xl font-extrabold text-slate-900">
                    {formatNumber(stats.smtp_cache_hits)}
                  </div>
                  <div className="mt-1 text-xs font-semibold text-slate-500">
                    {labels.cacheCoverage(stats.smtp_cache_hits, stats.smtp_attempted_emails)}
                  </div>
                </div>
              </div>

              <div className="mt-3 rounded-2xl border border-slate-200 bg-slate-50 p-4">
                <div className="text-[10px] font-bold uppercase tracking-widest text-slate-400">
                  {labels.smtp_unknown_breakdown}
                </div>
                <div className="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
                  <VerifySummaryCard
                    bucket="smtp_policy_blocked"
                    label={labels.smtp_policy_blocked}
                    value={formatNumber(stats.smtp_policy_blocked)}
                  />
                  <VerifySummaryCard
                    bucket="smtp_temp_failure"
                    label={labels.smtp_temp_failure}
                    value={formatNumber(stats.smtp_temp_failure)}
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
                    bucket="mx_inconclusive"
                    label={labels.mx_inconclusive}
                    value={formatNumber(stats.mx_inconclusive)}
                  />
                  <VerifySummaryCard
                    bucket="mx_typo"
                    label={labels.mx_typo}
                    value={formatNumber(stats.mx_typo)}
                  />
                  <VerifySummaryCard
                    bucket="mx_disposable"
                    label={labels.mx_disposable}
                    value={formatNumber(stats.mx_disposable)}
                  />
                  <VerifySummaryCard
                    bucket="mx_parked"
                    label={labels.mx_parked}
                    value={formatNumber(stats.mx_parked)}
                  />
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
