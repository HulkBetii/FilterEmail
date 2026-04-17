import re

with open("src/App.tsx", "r") as f:
    content = f.read()

# Imports
content = content.replace(
    'import { open as openDialog } from "@tauri-apps/plugin-dialog";',
    'import { open as openDialog } from "@tauri-apps/plugin-dialog";\nimport { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";'
)

content = content.replace(
    'import {\n  AlertCircle,',
    'import {\n  AlertCircle,\n  Copy,',
)

# Processing payload types
content = content.replace(
    'pub custom: number;\n  elapsed_ms: number;',
    'custom: number;\n  duplicates: number;\n  elapsed_ms: number;'
)

content = content.replace(
    '  custom: 0,\n  elapsed_ms: 0,',
    '  custom: 0,\n  duplicates: 0,\n  elapsed_ms: 0,'
)

# Stat cards
content = content.replace(
    'key: "custom" as const,',
    'key: "duplicates" as const,\n    icon: Copy,\n    chip: "bg-gray-50 text-gray-700 ring-gray-200",\n  },\n  {\n    key: "custom" as const,'
)

# State definitions
content = content.replace(
    'const [selectedFilePath, setSelectedFilePath] = useState("");',
    '''const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [checkMx, setCheckMx] = useState(false);'''
)

# Config persistence
persistence_code = '''
  useEffect(() => {
    const savedDomains = localStorage.getItem("targetDomains");
    if (savedDomains) setTargetDomains(savedDomains);
    const savedMx = localStorage.getItem("checkMx");
    if (savedMx === "true") setCheckMx(true);
    const savedOut = localStorage.getItem("lastOutputDir");
    if (savedOut) {
        setOutputDir(savedOut);
        setLastOutputDir(savedOut);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("targetDomains", targetDomains);
  }, [targetDomains]);

  useEffect(() => {
    localStorage.setItem("checkMx", checkMx ? "true" : "false");
  }, [checkMx]);

  useEffect(() => {
    if (outputDir) localStorage.setItem("lastOutputDir", outputDir);
  }, [outputDir]);
'''
content = content.replace(
    'const t = translations[language];\n  const [banner, setBanner] = useState<BannerState>({\n    tone: "idle",\n    message: translations.en.idleBanner,\n  });',
    'const t = translations[language];\n  const [banner, setBanner] = useState<BannerState>({\n    tone: "idle",\n    message: translations.en.idleBanner,\n  });\n' + persistence_code
)

# Banners and dependencies
content = content.replace(
    'if (selectedFilePath) {',
    'if (selectedFiles.length > 0) {'
)
content = content.replace(
    'message: t.selectedFileBanner(basename(selectedFilePath)),',
    'message: selectedFiles.length === 1 ? t.selectedFileBanner(basename(selectedFiles[0])) : `Đã chọn ${selectedFiles.length} tệp.`,',
)
content = content.replace(
    'selectedFilePath,',
    'selectedFiles.length,'
)

# Listeners completeness notification
notification_code = r'''
          setBanner({
            tone: "success",
            message: translations[language].completeBanner,
          });
          isPermissionGranted().then((granted) => {
             if (!granted) {
                 requestPermission().then((g) => {
                     if (g === 'granted') sendNotification({ title: 'Hoàn tất', body: 'Quá trình lọc email đã xong!' });
                 });
             } else {
                 sendNotification({ title: 'Hoàn tất', body: 'Quá trình lọc email đã xong!' });
             }
          });
'''
content = content.replace(
    '''setBanner({
            tone: "success",
            message: translations[language].completeBanner,
          });''',
    notification_code
)

# Drag events
content = content.replace(
    'const firstPath = event.payload.paths[0];',
    'const paths = event.payload.paths;'
)
content = content.replace(
    '''if (firstPath) {
              setSelectedFilePath(firstPath);
              setBanner({
                tone: "idle",
                message: translations[language].selectedFileBanner(basename(firstPath)),
              });
            }''',
    '''if (paths && paths.length > 0) {
              setSelectedFiles(paths);
              setBanner({
                tone: "idle",
                message: paths.length === 1 ? translations[language].selectedFileBanner(basename(paths[0])) : `Đã chọn ${paths.length} tệp.`,
              });
            }'''
)

# total classified
content = content.replace(
    'stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom,',
    'stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom + stats.duplicates,'
)

# Pick input file
content = content.replace(
    'multiple: false,\n      directory: false,',
    'multiple: true,\n      directory: false,'
)
content = content.replace(
    '''if (typeof selected === "string") {
      setSelectedFilePath(selected);
      setBanner({
        tone: "idle",
        message: t.selectedFileBanner(basename(selected)),
      });
    }''',
    '''if (typeof selected === "string") {
      setSelectedFiles([selected]);
      setBanner({
        tone: "idle",
        message: t.selectedFileBanner(basename(selected)),
      });
    } else if (Array.isArray(selected)) {
      setSelectedFiles(selected);
      setBanner({
        tone: "idle",
        message: `Đã chọn ${selected.length} tệp.`,
      });
    }'''
)

# Invoke
content = content.replace(
    'disabled={!selectedFilePath || !outputDir || isProcessing}',
    'disabled={selectedFiles.length === 0 || !outputDir || isProcessing}'
)
content = content.replace(
    '''await invoke("process_file", {
        file_path: selectedFilePath,
        output_dir: outputDir,
        target_domains: targetDomains,
      });''',
    '''await invoke("process_file", {
        file_paths: selectedFiles,
        output_dir: outputDir,
        target_domains: targetDomains,
        check_mx: checkMx,
      });'''
)
content = content.replace(
    'if (!selectedFilePath || !outputDir || isProcessing) {',
    'if (selectedFiles.length === 0 || !outputDir || isProcessing) {'
)

# File display
content = content.replace(
    '{selectedFilePath || t.labels.noFile}',
    '{selectedFiles.length > 0 ? (selectedFiles.length === 1 ? basename(selectedFiles[0]) : `${selectedFiles.length} tệp`) : t.labels.noFile}'
)

# Add MX Check toggle UI right after targeted
checkmx_html = r'''
              <div className="rounded-2xl border border-white/80 bg-white/88 p-3 shadow-sm shadow-slate-200/60">
                <label className="flex items-center gap-3 cursor-pointer">
                  <div className="relative flex items-center">
                    <input
                      type="checkbox"
                      checked={checkMx}
                      onChange={(e) => setCheckMx(e.target.checked)}
                      className="sr-only"
                    />
                    <div className={`block h-6 w-10 rounded-full transition-colors ${checkMx ? 'bg-sky-500' : 'bg-slate-300'}`}></div>
                    <div className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-white transition-transform ${checkMx ? 'translate-x-4' : 'translate-x-0'}`}></div>
                  </div>
                  <span className="text-sm font-medium text-slate-700">{t.labels.mxCheckLabel}</span>
                </label>
              </div>'''
content = content.replace(
    'className="mt-2 w-full rounded-xl bg-slate-100 px-4 py-2 text-slate-900 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-500/50 transition-colors border border-transparent"\n                />\n              </div>',
    'className="mt-2 w-full rounded-xl bg-slate-100 px-4 py-2 text-slate-900 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-500/50 transition-colors border border-transparent"\n                />\n              </div>\n\n' + checkmx_html
)


with open("src/App.tsx", "w") as f:
    f.write(content)

