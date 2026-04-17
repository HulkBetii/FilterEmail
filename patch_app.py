with open("src/App.tsx", "r") as f:
    content = f.read()

# 1. Icons import
content = content.replace(
    "  XCircle,\n} from \"lucide-react\";",
    "  XCircle,\n  Target,\n} from \"lucide-react\";"
)

# 2. ProcessingPayload
content = content.replace(
    "  edu: number;\n  custom: number;",
    "  edu: number;\n  targeted: number;\n  custom: number;"
)

# 3. initial stats
content = content.replace(
    "  edu: 0,\n  custom: 0,",
    "  edu: 0,\n  targeted: 0,\n  custom: 0,"
)

# 4. statCards
old_stat_cards = """  {
    key: "edu" as const,
    icon: ShieldCheck,
    chip: "bg-emerald-50 text-emerald-700 ring-emerald-100",
  },
  {
    key: "custom" as const,
    icon: Mail,"""
new_stat_cards = """  {
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
    key: "custom" as const,
    icon: Mail,"""
content = content.replace(old_stat_cards, new_stat_cards)

# 5. state hook
content = content.replace(
    "  const [outputDir, setOutputDir] = useState(\"\");",
    "  const [outputDir, setOutputDir] = useState(\"\");\n  const [targetDomains, setTargetDomains] = useState(\"\");"
)

# 6. totalClassified
content = content.replace(
    "() => stats.invalid + stats.public + stats.edu + stats.custom,",
    "() => stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom,"
)

# 7. rates
old_rates = """  const publicRate = totalClassified === 0 ? 0 : (stats.public / totalClassified) * 100;
  const eduRate = totalClassified === 0 ? 0 : (stats.edu / totalClassified) * 100;
  const customRate = totalClassified === 0 ? 0 : (stats.custom / totalClassified) * 100;"""
new_rates = """  const publicRate = totalClassified === 0 ? 0 : (stats.public / totalClassified) * 100;
  const eduRate = totalClassified === 0 ? 0 : (stats.edu / totalClassified) * 100;
  const targetedRate = totalClassified === 0 ? 0 : (stats.targeted / totalClassified) * 100;
  const customRate = totalClassified === 0 ? 0 : (stats.custom / totalClassified) * 100;"""
content = content.replace(old_rates, new_rates)

# 8. payload in invoke
content = content.replace(
    "        file_path: selectedFilePath,\n        output_dir: outputDir,",
    "        file_path: selectedFilePath,\n        output_dir: outputDir,\n        target_domains: targetDomains,"
)

# 9. UI Input - add target domains input block before bannerStyles
old_ui = """              <div className="rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60">
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
              </div>"""

new_ui = """              <div className="rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60">
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

              <div className="rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60">
                <p className="text-sm font-medium text-slate-600">{t.labels.targetedInputLabel}</p>
                <input
                  type="text"
                  value={targetDomains}
                  onChange={(e) => setTargetDomains(e.target.value)}
                  placeholder={t.labels.targetedInputPlaceholder}
                  className="mt-2 w-full rounded-xl border border-slate-300 bg-white px-4 py-2.5 text-slate-900 placeholder-slate-400 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500 transition-colors"
                />
              </div>"""
content = content.replace(old_ui, new_ui)

# 10. Summary Rates Section
old_summary_rates = """                  <div className="mt-4 grid gap-3 sm:grid-cols-2 2xl:grid-cols-4">
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
                  </div>"""

new_summary_rates = """                  <div className="mt-4 grid gap-3 sm:grid-cols-2 2xl:grid-cols-5">
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
                    <div className="rounded-2xl bg-white/92 p-4 ring-1 ring-fuchsia-200">
                      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">
                        {t.labels.summaryTargetedRate}
                      </p>
                      <p className="mt-2 text-xl font-semibold text-slate-900">
                        {formatPercent(targetedRate)}
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
                  </div>"""
content = content.replace(old_summary_rates, new_summary_rates)


with open("src/App.tsx", "w") as f:
    f.write(content)

