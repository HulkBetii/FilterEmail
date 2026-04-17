with open("src/App.tsx", "r") as f:
    content = f.read()

# 1. Remove showSummary block
# Find the start of showSummary ternary
start_idx = content.find("{showSummary ? (")
# Find the end of it (it ends with ') : null}')
end_idx = content.find(") : null}", start_idx)

if start_idx != -1 and end_idx != -1:
    content = content[:start_idx] + content[end_idx + 9:]

# 2. Add percentages to statCards
# Look for:
#             {statCards.map((card) => {
#               const Icon = card.icon;
#               const value = stats[card.key];
old_stat_map = """            {statCards.map((card) => {
              const Icon = card.icon;
              const value = stats[card.key];

              return ("""

new_stat_map = """            {statCards.map((card) => {
              const Icon = card.icon;
              const value = stats[card.key];
              const rate = totalClassified === 0 ? 0 : (value / totalClassified) * 100;
              const rateDisplay = value > 0 ? ` (${formatPercent(rate)})` : "";

              return ("""
content = content.replace(old_stat_map, new_stat_map)

# Update the display
old_value_display = """                      <p className="mt-4 text-2xl font-semibold tracking-tight text-slate-900">
                        {value.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                      </p>"""
new_value_display = """                      <p className="mt-4 text-2xl font-semibold tracking-tight text-slate-900 flex items-baseline gap-2">
                        {value.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                        <span className="text-sm font-medium text-slate-500">{rateDisplay}</span>
                      </p>"""
content = content.replace(old_value_display, new_value_display)

# 3. Fix the "Mở Thư Mục Kết Quả" button.
# Let's make it more prominent since the summary is gone, maybe full width, and explicitly show the output dir if available.
old_buttons = """              <div className="flex flex-col gap-3 lg:flex-row">
                <button
                  type="button"
                  onClick={handleProcess}
                  disabled={!selectedFilePath || !outputDir || isProcessing}
                  className="inline-flex w-full items-center justify-center gap-2 rounded-full bg-primary px-4 py-2.5 text-sm font-medium text-white shadow-lg shadow-sky-600/25 transition hover:brightness-95 disabled:cursor-not-allowed disabled:bg-slate-300 lg:w-auto"
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
                  className="inline-flex w-full items-center justify-center gap-2 rounded-full border border-slate-300 bg-white px-4 py-2.5 text-sm font-medium text-slate-800 transition hover:border-slate-400 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50 lg:w-auto"
                >
                  <FolderOpen className="h-4 w-4" />
                  {t.labels.openFolder}
                </button>
              </div>"""

new_buttons = """              <div className="grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  onClick={handleProcess}
                  disabled={!selectedFilePath || !outputDir || isProcessing}
                  className="inline-flex w-full items-center justify-center gap-2 rounded-full bg-primary px-4 py-3 text-sm font-semibold text-white shadow-md shadow-sky-600/20 transition hover:brightness-95 disabled:cursor-not-allowed disabled:bg-slate-300"
                >
                  {isProcessing ? (
                    <LoaderCircle className="h-4 w-4 animate-spin" />
                  ) : (
                    <Mail className="h-4 w-4" />
                  )}
                  {isProcessing ? t.labels.processing : t.labels.start}
                </button>

                {canOpenFolder && banner.tone === "success" && (
                  <button
                    type="button"
                    onClick={openResultFolder}
                    className="inline-flex w-full items-center justify-center gap-2 rounded-full bg-emerald-100 px-4 py-3 text-sm font-semibold text-emerald-800 shadow-sm transition hover:bg-emerald-200"
                  >
                    <FolderOpen className="h-4 w-4" />
                    {t.labels.openFolder}
                  </button>
                )}
              </div>"""
content = content.replace(old_buttons, new_buttons)


with open("src/App.tsx", "w") as f:
    f.write(content)

