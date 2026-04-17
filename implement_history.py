import re

# 1. Update i18n.ts
with open("src/i18n.ts", "r") as f:
    text = f.read()

text = text.replace(
    'summaryCustomRate: string;',
    'summaryCustomRate: string;\n    openHistory: string;\n    historyTitle: string;\n    clearHistory: string;\n    emptyHistory: string;\n    close: string;'
)

text = text.replace(
    'summaryCustomRate: "Other Rate",',
    'summaryCustomRate: "Other Rate",\n      openHistory: "History",\n      historyTitle: "Processing History",\n      clearHistory: "Clear History",\n      emptyHistory: "No history records yet.",\n      close: "Close",'
)

text = text.replace(
    'summaryCustomRate: "Tỷ Lệ Khác",',
    'summaryCustomRate: "Tỷ Lệ Khác",\n      openHistory: "Lịch sử",\n      historyTitle: "Lịch Sử Phiên Lọc",\n      clearHistory: "Xóa Lịch Sử",\n      emptyHistory: "Chưa có lưu trữ nào.",\n      close: "Đóng",'
)

with open("src/i18n.ts", "w") as f:
    f.write(text)

# 2. Update App.tsx
with open("src/App.tsx", "r") as f:
    app = f.read()

# Add Lucide icons
app = app.replace(
    '''import {
  AlertCircle,
  Copy,
  CheckCircle,''',
    '''import {
  AlertCircle,
  Copy,
  CheckCircle,
  History,
  X,
  Trash2,'''
)

# Add type
types = '''type HistoryEntry = {
  id: string;
  timestamp: number;
  fileNames: string[];
  stats: ProcessingPayload;
};

type BannerTone = "idle" | "success" | "error";'''
app = app.replace('type BannerTone = "idle" | "success" | "error";', types)

# Add state
states = '''const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);

  useEffect(() => {
    const saved = localStorage.getItem("filteremail-history");
    if (saved) {
      try {
        setHistory(JSON.parse(saved));
      } catch (e) {}
    }
  }, []);'''
app = app.replace(
    'const [checkMx, setCheckMx] = useState(false);',
    'const [checkMx, setCheckMx] = useState(false);\n  ' + states
)

# Find processing_complete unlisten block
replace_complete = '''setBanner({
            tone: "success",
            message: translations[language].completeBanner,
          });

          // push history
          const newEntry: HistoryEntry = {
            id: crypto.randomUUID(),
            timestamp: Date.now(),
            fileNames: selectedFiles.map(f => basename(f)),
            stats: payload
          };
          setHistory(prev => {
            const next = [newEntry, ...prev].slice(0, 20);
            localStorage.setItem("filteremail-history", JSON.stringify(next));
            return next;
          });'''
app = app.replace(
    '''setBanner({
            tone: "success",
            message: translations[language].completeBanner,
          });''',
    replace_complete
)

# Header UI (Top-Right controls)
header_ui = '''<div className="absolute right-4 top-4 z-10 flex gap-2">
        <button
          onClick={() => setIsHistoryOpen(true)}
          className="flex transform items-center justify-center gap-2 rounded-xl bg-white/20 px-3 py-2 text-xs font-semibold uppercase tracking-wider text-slate-700 shadow-sm backdrop-blur-md transition-all hover:scale-105 hover:bg-white/30"
          title={t.labels.openHistory}
        >
          <History className="h-4 w-4" />
          <span className="hidden sm:inline">{t.labels.openHistory}</span>
        </button>
        <button
          onClick={() => {
            const newLang = language === "en" ? "vi" : "en";
            setLanguage(newLang);
            persistLanguage(newLang);
          }}
          className="flex transform items-center justify-center rounded-xl bg-white/20 px-3 py-2 text-xs font-semibold uppercase tracking-wider text-slate-700 shadow-sm backdrop-blur-md transition-all hover:scale-105 hover:bg-white/30"
          title={t.labels.language}
        >
          {language}
        </button>
      </div>'''

app = app.replace(
    '''<button
        onClick={() => {
          const newLang = language === "en" ? "vi" : "en";
          setLanguage(newLang);
          persistLanguage(newLang);
        }}
        className="absolute right-4 top-4 z-10 flex transform items-center justify-center rounded-xl bg-white/20 px-3 py-2 text-xs font-semibold uppercase tracking-wider text-slate-700 shadow-sm backdrop-blur-md transition-all hover:scale-105 hover:bg-white/30"
        title={t.labels.language}
      >
        {language}
      </button>''',
    header_ui
)

# Render Modal UI
modal_ui = '''{/* History Modal */}
      {isHistoryOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-sm transition-opacity">
          <div className="relative w-full max-w-2xl max-h-[85vh] flex flex-col rounded-3xl bg-white shadow-2xl ring-1 ring-slate-900/5">
            {/* Header */}
            <div className="flex items-center justify-between border-b border-slate-100 px-6 py-4">
              <h2 className="text-xl font-bold text-slate-800 flex items-center gap-2">
                <History className="h-5 w-5 text-sky-500" />
                {t.labels.historyTitle}
              </h2>
              <button onClick={() => setIsHistoryOpen(false)} className="rounded-full p-2 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors">
                <X className="h-5 w-5" />
              </button>
            </div>
            
            {/* Content */}
            <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
              {history.length === 0 ? (
                <div className="py-12 text-center text-sm text-slate-500">{t.labels.emptyHistory}</div>
              ) : (
                history.map((entry) => {
                  const total = entry.stats.invalid + entry.stats.public + entry.stats.edu + entry.stats.targeted + entry.stats.custom + entry.stats.duplicates + (entry.stats.mx_dead || 0);
                  const valid = entry.stats.public + entry.stats.edu + entry.stats.targeted + entry.stats.custom;
                  return (
                    <div key={entry.id} className="rounded-2xl border border-slate-200 bg-slate-50 p-4 transition-all hover:border-sky-300">
                      <div className="flex flex-wrap items-start justify-between gap-2 mb-2">
                        <div>
                          <div className="text-xs font-semibold text-slate-400 mb-1">
                            {new Date(entry.timestamp).toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                          </div>
                          <div className="text-sm font-medium text-slate-700 break-all leading-tight">
                            {entry.fileNames.join(", ")}
                          </div>
                        </div>
                        <button
                          onClick={() => revealItemInDir(entry.stats.output_dir || "")}
                          className="flex shrink-0 items-center gap-1.5 rounded-lg bg-sky-100 px-3 py-1.5 text-xs font-semibold text-sky-700 hover:bg-sky-200 transition-colors shadow-sm"
                        >
                          <FolderOpen className="h-3.5 w-3.5" />
                          <span className="hidden sm:inline">{t.labels.openFolder}</span>
                        </button>
                      </div>
                      
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-3">
                         <div className="rounded-xl bg-white p-2 border border-slate-100 text-center">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Total</div>
                            <div className="text-sm font-bold text-slate-800">{total.toLocaleString()}</div>
                         </div>
                         <div className="rounded-xl bg-white p-2 border border-slate-100 text-center">
                            <div className="text-[10px] uppercase font-bold text-emerald-500">Valid</div>
                            <div className="text-sm font-bold text-emerald-600">{valid.toLocaleString()}</div>
                         </div>
                         <div className="rounded-xl bg-white p-2 border border-slate-100 text-center">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Dups</div>
                            <div className="text-sm font-bold text-slate-600">{entry.stats.duplicates.toLocaleString()}</div>
                         </div>
                         <div className="rounded-xl bg-white p-2 border border-slate-100 text-center">
                            <div className="text-[10px] uppercase font-bold text-red-400">Invalid</div>
                            <div className="text-sm font-bold text-red-600">{(entry.stats.invalid + (entry.stats.mx_dead || 0)).toLocaleString()}</div>
                         </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>

            {/* Footer */}
            {history.length > 0 && (
              <div className="border-t border-slate-100 px-6 py-3 flex justify-end">
                <button
                  onClick={() => {
                    setHistory([]);
                    localStorage.removeItem("filteremail-history");
                  }}
                  className="flex items-center gap-2 rounded-lg text-red-600 hover:bg-red-50 px-3 py-2 text-sm font-semibold transition-colors"
                >
                  <Trash2 className="h-4 w-4" />
                  {t.labels.clearHistory}
                </button>
              </div>
            )}
          </div>
        </div>
      )}'''

# Inject Modal UI just before the closing </div> of the main return
app = app.replace('</main>\n    </div>', '</main>\n      ' + modal_ui + '\n    </div>')

with open("src/App.tsx", "w") as f:
    f.write(app)

