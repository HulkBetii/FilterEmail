import { AlertCircle, CheckCircle } from "lucide-react";
import { AppHeader } from "./components/app-header";
import { HistoryModal } from "./components/history-modal";
import { ResultsPanel } from "./components/results-panel";
import { SettingsPanel } from "./components/settings-panel";
import { TopDashboard } from "./components/top-dashboard";
import { useProcessingController } from "./hooks/use-processing-controller";
import { STAT_CARDS } from "./lib/stat-cards";

export default function App() {
  const controller = useProcessingController();

  return (
    <main className="min-h-screen bg-slate-50 font-sans text-slate-900">
      <div className="mx-auto w-full max-w-7xl space-y-5 p-4 sm:p-6 lg:p-8">
        <AppHeader
          activeTab={controller.activeTab}
          language={controller.language}
          labels={controller.t.labels}
          onChangeLanguage={controller.setLanguage}
          onChangeTab={controller.setActiveTab}
          onOpenHistory={() => controller.setIsHistoryOpen(true)}
        />

        {controller.banner.tone !== "idle" && (
          <div
            className={`flex min-w-0 items-start gap-3 rounded-2xl border p-4 text-sm font-medium ${
              controller.banner.tone === "error"
                ? "border-red-200 bg-red-50 text-red-800"
                : "border-emerald-200 bg-emerald-50 text-emerald-800"
            }`}
          >
            {controller.banner.tone === "error" ? (
              <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
            ) : (
              <CheckCircle className="mt-0.5 h-5 w-5 shrink-0" />
            )}
            <p className="min-w-0 break-words leading-relaxed">
              {controller.banner.message}
            </p>
          </div>
        )}

        <TopDashboard
          activeTab={controller.activeTab}
          language={controller.language}
          dragActive={controller.dragActive}
          totalClassified={
            controller.verifyMode
              ? controller.finalTotal
              : controller.totalClassified
          }
          progressPercent={controller.stats.progress_percent}
          isProcessing={controller.isProcessing}
          currentDomain={controller.stats.current_domain ?? null}
          currentEmail={controller.stats.current_email ?? null}
          cacheHits={controller.stats.cache_hits}
          labels={controller.t.labels}
          canOpenFolder={controller.canOpenFolder}
          onPickInputFile={controller.pickInputFile}
          onOpenResultFolder={controller.openResultFolder}
          onDragOver={(event) => {
            event.preventDefault();
            controller.setDragActive(true);
          }}
          onDragLeave={() => controller.setDragActive(false)}
          formatNumber={controller.formatNumber}
        />

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
          <SettingsPanel
            activeTab={controller.activeTab}
            isProcessing={controller.isProcessing}
            labels={controller.t.labels}
            language={controller.language}
            outputDir={controller.outputDir}
            selectedFiles={controller.selectedFiles}
            showAdvancedDns={controller.showAdvancedDns}
            smtpEnabled={controller.smtpEnabled}
            stats={controller.stats}
            targetDomains={controller.targetDomains}
            timeoutMs={controller.timeoutMs}
            maxConcurrent={controller.maxConcurrent}
            usePersistentCache={controller.usePersistentCache}
            vpsApiKey={controller.vpsApiKey}
            vpsApiUrl={controller.vpsApiUrl}
            onChangeMaxConcurrent={controller.setMaxConcurrent}
            onChangeTargetDomains={controller.setTargetDomains}
            onChangeTimeoutMs={controller.setTimeoutMs}
            onChangeVpsApiKey={controller.setVpsApiKey}
            onChangeVpsApiUrl={controller.setVpsApiUrl}
            onPickOutputDir={controller.pickOutputDir}
            onStartProcessing={controller.handleProcess}
            onToggleAdvancedDns={() =>
              controller.setShowAdvancedDns(!controller.showAdvancedDns)
            }
            onTogglePersistentCache={() =>
              controller.setUsePersistentCache(!controller.usePersistentCache)
            }
            onToggleSmtpEnabled={() =>
              controller.setSmtpEnabled(!controller.smtpEnabled)
            }
          />

          <ResultsPanel
            formatNumber={controller.formatNumber}
            labels={controller.t.labels}
            language={controller.language}
            showDnsDiag={controller.showDnsDiag}
            showSmtpDiag={controller.showSmtpDiag}
            stats={controller.stats}
            totalClassified={controller.totalClassified}
            verifyMode={controller.verifyMode}
            onToggleDnsDiag={() =>
              controller.setShowDnsDiag(!controller.showDnsDiag)
            }
            onToggleSmtpDiag={() =>
              controller.setShowSmtpDiag(!controller.showSmtpDiag)
            }
          />
        </div>

        <HistoryModal
          isOpen={controller.isHistoryOpen}
          history={controller.history}
          language={controller.language}
          labels={controller.t.labels}
          statCards={STAT_CARDS}
          formatNumber={controller.formatNumber}
          onClose={() => controller.setIsHistoryOpen(false)}
          onOpenFolder={controller.openHistoryFolder}
          onClearHistory={controller.clearHistory}
        />

        <footer className="pb-2 pt-1 text-center text-xs font-medium text-slate-400">
          © 2026 HulkBetii. All rights reserved.
        </footer>
      </div>
    </main>
  );
}
