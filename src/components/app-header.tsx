import { History, Mail, ShieldCheck } from "lucide-react";
import appLogo from "../assets/logo.png";
import type { ActiveTab } from "../lib/app-state";
import type { Language, TranslationLabels } from "../i18n";

export function AppHeader({
  activeTab,
  language,
  labels,
  onChangeLanguage,
  onChangeTab,
  onOpenHistory,
}: {
  activeTab: ActiveTab;
  language: Language;
  labels: TranslationLabels;
  onChangeLanguage: (language: Language) => void;
  onChangeTab: (tab: ActiveTab) => void;
  onOpenHistory: () => void;
}) {
  return (
    <header className="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-white px-5 py-3 shadow-sm ring-1 ring-slate-900/5">
      <div className="flex min-w-0 items-center gap-3">
        <img
          src={appLogo}
          alt="FilterEmail logo"
          className="h-12 w-12 shrink-0 rounded-2xl object-cover shadow-md shadow-sky-500/20 ring-1 ring-sky-100"
        />
        <div className="min-w-0">
          <p className="truncate text-base font-bold leading-tight text-slate-800">
            FilterEmail Desktop
          </p>
          <p className="truncate text-[11px] font-medium text-slate-400">
            {labels.heroBadge}
          </p>
        </div>
      </div>

      <div className="flex space-x-1 rounded-[1.25rem] bg-slate-100 p-1">
        <button
          onClick={() => onChangeTab("filter")}
          className={`flex items-center gap-1.5 rounded-xl px-5 py-2 text-sm font-semibold transition-all duration-200 ${
            activeTab === "filter"
              ? "bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/8"
              : "text-slate-500 hover:text-slate-800"
          }`}
        >
          <Mail className="h-3.5 w-3.5 shrink-0" />
          {labels.tabBasicFilter}
        </button>
        <button
          onClick={() => onChangeTab("verify")}
          className={`flex items-center gap-1.5 rounded-xl px-5 py-2 text-sm font-semibold transition-all duration-200 ${
            activeTab === "verify"
              ? "bg-slate-900 text-white shadow-md shadow-slate-900/20"
              : "text-slate-500 hover:text-slate-800"
          }`}
        >
          <ShieldCheck className="h-3.5 w-3.5 shrink-0" />
          {labels.tabDnsVerify}
        </button>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <button
          onClick={onOpenHistory}
          className="flex items-center gap-1.5 rounded-full bg-sky-50 px-3 py-1.5 text-xs font-semibold text-sky-700 ring-1 ring-sky-200 transition hover:bg-sky-100"
        >
          <History className="h-3.5 w-3.5 shrink-0" />
          <span>{labels.openHistory}</span>
        </button>
        <div className="flex rounded-full bg-slate-100 p-1">
          {(["en", "vi"] as const).map((lang) => (
            <button
              key={lang}
              onClick={() => onChangeLanguage(lang)}
              className={`rounded-full px-3 py-1 text-xs font-bold transition ${
                language === lang
                  ? "bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/10"
                  : "text-slate-500 hover:text-slate-700"
              }`}
            >
              {lang === "en" ? labels.english : labels.vietnamese}
            </button>
          ))}
        </div>
      </div>
    </header>
  );
}
