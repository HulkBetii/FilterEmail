import re

with open("src/App.tsx", "r") as f:
    lines = f.readlines()

start_index = -1
end_index = -1

for i, line in enumerate(lines):
    if line.startswith("  return ("):
        if start_index == -1:
            start_index = i
    if "{/* History Modal */}" in line:
        end_index = i
        break

if start_index == -1 or end_index == -1:
    print(f"Indices not found! start: {start_index}, end: {end_index}")
    exit(1)

new_jsx = """  return (
    <main className="min-h-screen bg-slate-50 text-slate-900 font-sans sm:p-6 lg:p-8 flex flex-col items-center justify-start">
      <div className="w-full max-w-7xl relative mx-auto space-y-6">
        
        {/* Header / Topbar */}
        <header className="flex flex-col sm:flex-row items-center justify-between gap-4 rounded-3xl bg-white/70 p-4 shadow-sm ring-1 ring-slate-900/5 backdrop-blur-md">
          <div className="flex items-center gap-3 px-2">
             <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-sky-500 shadow-lg shadow-sky-500/30">
                <CheckCircle2 className="h-6 w-6 text-white" />
             </div>
             <div>
                <h1 className="text-lg font-bold text-slate-800 tracking-tight">FilterEmail Desktop</h1>
                <p className="text-xs font-medium text-slate-500">{t.labels.heroBadge}</p>
             </div>
          </div>
          <div className="flex items-center gap-3">
             <button
                onClick={() => setIsHistoryOpen(true)}
                className="flex items-center gap-1.5 rounded-full bg-sky-100 px-4 py-2 text-sm font-semibold text-sky-700 shadow-sm transition-all hover:bg-sky-200 hover:scale-105 whitespace-nowrap"
                title={t.labels.openHistory}
              >
                <History className="h-4 w-4 shrink-0" />
                <span>{t.labels.openHistory}</span>
             </button>
             <div className="flex bg-slate-100 rounded-full p-1 shadow-inner">
                {(["en", "vi"] as const).map((lang) => {
                  const active = language === lang;
                  return (
                    <button
                      key={lang}
                      type="button"
                      onClick={() => setLanguage(lang)}
                      className={`rounded-full px-4 py-1.5 text-xs font-bold transition-all whitespace-nowrap ${
                        active
                          ? "bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/10"
                          : "text-slate-500 hover:text-slate-700"
                      }`}
                    >
                      {lang === "en" ? t.labels.english : t.labels.vietnamese}
                    </button>
                  );
                })}
             </div>
          </div>
        </header>

        {/* Status Banner */}
        {banner.tone !== "idle" && (
          <div
            className={`flex items-center gap-3 rounded-2xl p-4 text-sm font-medium shadow-sm transition-all
              ${banner.tone === "error" ? "bg-red-50 text-red-800 border-l-4 border-red-500" : "bg-emerald-50 text-emerald-800 border-l-4 border-emerald-500"}
            `}
          >
            {banner.tone === "error" ? <AlertCircle className="h-5 w-5 shrink-0" /> : <CheckCircle className="h-5 w-5 shrink-0" />}
            <p className="flex-1 text-pretty break-words leading-relaxed">{banner.message}</p>
          </div>
        )}

        {/* Main Grid Two Columns */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          
          {/* LEFT COLUMN: Input & Config */}
          <div className="lg:col-span-5 flex flex-col gap-6">
             {/* Dropzone */}
             <div
                onDragOver={(e) => {
                  e.preventDefault();
                  setIsDragging(true);
                }}
                onDragLeave={() => setIsDragging(false)}
                className={`relative flex flex-col items-center justify-center rounded-[2.5rem] p-10 text-center transition-all min-h-[220px] bg-white border-2 border-dashed
                  ${isDragging ? "scale-[1.02] border-sky-400 bg-sky-50 shadow-xl shadow-sky-900/10" : "border-slate-200 shadow-md"}
                `}
              >
                <div className={`mb-6 flex h-16 w-16 items-center justify-center rounded-2xl transition-all shadow-sm ${isDragging ? "bg-sky-500 text-white" : "bg-slate-900 text-slate-100"}`}>
                  <UploadCloud className="h-8 w-8" />
                </div>
                <h3 className="text-xl font-bold text-slate-900 tracking-tight">{t.labels.dragTitle}</h3>
                <p className="mt-3 text-sm leading-relaxed text-slate-500 max-w-[240px] text-pretty">
                  {t.labels.dragBody}
                </p>
                <div className="mt-8 flex justify-center">
                  <button
                    onClick={pickInputFile}
                    className="group flex items-center justify-center gap-2 rounded-full bg-slate-900 px-6 py-2.5 text-sm font-semibold text-white shadow-md transition-all hover:bg-slate-800 active:scale-95"
                  >
                    <UploadCloud className="h-4 w-4 transition-transform group-hover:-translate-y-0.5" />
                    {t.labels.chooseFile}
                  </button>
                </div>
              </div>

             {/* Config Card */}
             <div className="rounded-[2.5rem] bg-white p-6 shadow-md border border-slate-100">
                <div className="space-y-6">
                  {/* File status */}
                  <div>
                    <label className="text-[11px] font-bold uppercase tracking-wider text-slate-400">{t.labels.selectedFile}</label>
                    <div className="mt-2 flex items-center gap-3 rounded-2xl bg-slate-50 border border-slate-100 p-3 leading-tight font-medium text-slate-800">
                      <FileSpreadsheet className="h-5 w-5 text-slate-400 shrink-0" />
                      <span className="truncate flex-1">
                        {selectedFiles.length > 0 ? (selectedFiles.length === 1 ? basename(selectedFiles[0]) : `${selectedFiles.length} tệp`) : t.labels.noFile}
                      </span>
                    </div>
                  </div>
                  
                  {/* Output folder */}
                  <div>
                    <label className="text-[11px] font-bold uppercase tracking-wider text-slate-400">{t.labels.outputFolder}</label>
                    <div className="mt-2 flex items-center gap-2">
                       <div className="flex items-center gap-3 flex-1 overflow-hidden rounded-2xl bg-slate-50 border border-slate-100 p-3 leading-tight font-medium text-slate-800">
                          <FolderOpen className="h-5 w-5 text-slate-400 shrink-0" />
                          <span className="truncate flex-1 text-sm">{outputDir || t.labels.noFolder}</span>
                       </div>
                       <button
                          onClick={selectOutputDir}
                          className="shrink-0 flex items-center justify-center gap-1.5 rounded-2xl bg-slate-100 px-4 py-3 text-sm font-bold text-slate-700 hover:bg-slate-200 transition"
                        >
                          {t.labels.selectFolder}
                       </button>
                    </div>
                  </div>
                  
                  {/* Target Domains */}
                  <div>
                    <label className="text-[11px] font-bold uppercase tracking-wider text-slate-400">{t.labels.targetedInputLabel}</label>
                    <input
                      type="text"
                      value={targetDomains}
                      onChange={(e) => setTargetDomains(e.target.value)}
                      placeholder={t.labels.targetedInputPlaceholder}
                      className="mt-2 w-full rounded-2xl bg-slate-50 px-4 py-3 text-sm font-medium text-slate-900 border border-slate-100 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-500/50 transition-all shadow-inner"
                    />
                  </div>
                  
                  {/* Check MX DNS */}
                  <label className="flex items-center justify-between cursor-pointer rounded-2xl bg-slate-50 border border-slate-100 p-4 transition-all hover:bg-slate-100">
                    <span className="text-sm font-bold text-slate-700 pr-2">{t.labels.mxCheckLabel}</span>
                    <div className="relative flex items-center shrink-0">
                      <input
                        type="checkbox"
                        checked={checkMx}
                        onChange={(e) => setCheckMx(e.target.checked)}
                        className="sr-only"
                      />
                      <div className={`block h-7 w-12 rounded-full transition-colors ${checkMx ? "bg-sky-500 shadow-inner" : "bg-slate-300"}`}></div>
                      <div className={`absolute left-1 top-1 h-5 w-5 rounded-full bg-white shadow-sm transition-transform ${checkMx ? "translate-x-5" : "translate-x-0"}`}></div>
                    </div>
                  </label>
                </div>
             </div>
             
             {/* Main Action Button */}
             <button
                onClick={handleStart}
                disabled={selectedFiles.length === 0 || !outputDir || isProcessing}
                className="group relative flex h-16 w-full items-center justify-center gap-3 overflow-hidden rounded-3xl bg-blue-600 font-bold text-white shadow-lg shadow-blue-600/30 transition-all hover:bg-blue-500 disabled:pointer-events-none disabled:bg-slate-200 disabled:text-slate-400 disabled:shadow-none"
              >
                {isProcessing ? (
                  <>
                    <LoaderCircle className="h-6 w-6 animate-spin" />
                    <span className="text-lg">{t.labels.processing}</span>
                  </>
                ) : (
                  <>
                    <Mail className="h-6 w-6 transition-transform group-hover:-rotate-12 group-hover:scale-110" />
                    <span className="text-lg">{t.labels.start}</span>
                  </>
                )}
             </button>
             
          </div>

          {/* RIGHT COLUMN: Dashboard & Analytics */}
          <div className="lg:col-span-7 flex flex-col gap-6">
            
            {/* Realtime Big Banner (Total Records) */}
            <article className="relative flex flex-col sm:flex-row items-center justify-between gap-6 overflow-hidden rounded-[2.5rem] bg-slate-900 p-8 shadow-2xl">
              <div className="absolute -right-20 -top-20 h-64 w-64 rounded-full bg-sky-500/20 blur-3xl"></div>
              <div className="absolute -bottom-20 -left-20 h-64 w-64 rounded-full bg-indigo-500/20 blur-3xl"></div>
              
              <div className="relative z-10 flex-1 w-full text-center sm:text-left">
                <p className="text-xs font-bold uppercase tracking-[0.2em] text-slate-400 border-b border-white/10 pb-2 inline-block">
                  {t.labels.classified}
                </p>
                <div className="mt-4 flex flex-wrap items-end justify-center sm:justify-start gap-4">
                  <p className="text-5xl lg:text-7xl font-extrabold tracking-tight text-white drop-shadow-md leading-none">
                    {totalClassified.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                  </p>
                </div>
                <p className="mt-4 text-sm font-medium leading-relaxed text-slate-400 max-w-sm mx-auto sm:mx-0">
                  {t.labels.classifiedBody}
                </p>
              </div>

              {/* Progress Box Inside Giant Black Box */}
              <div className="relative z-10 w-full sm:w-64 shrink-0 rounded-3xl bg-white/10 p-5 backdrop-blur-md ring-1 ring-white/20">
                 <div className="flex items-center justify-between text-white font-bold mb-3">
                    <span className="uppercase text-[11px] tracking-wider text-sky-200">{t.labels.progress}</span>
                    <span className="text-xl drop-shadow-sm">{stats.progress_percent.toFixed(1)}%</span>
                 </div>
                 <div className="h-3 w-full overflow-hidden rounded-full bg-black/40 shadow-inner">
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-sky-400 to-indigo-500 shadow-xl transition-all duration-300"
                      style={{ width: `${stats.progress_percent}%` }}
                    />
                 </div>
                 
                 <button
                    onClick={() => revealItemInDir(lastOutputDir || outputDir)}
                    disabled={!lastOutputDir}
                    className="mt-6 flex w-full h-12 items-center justify-center gap-2 rounded-xl bg-white font-bold text-slate-900 shadow-md transition-all hover:bg-slate-100 disabled:opacity-30 disabled:pointer-events-none"
                  >
                    <FolderOpen className="h-5 w-5" />
                    {t.labels.openFolder}
                  </button>
              </div>
            </article>

            {/* Metric Grids */}
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-4">
              {/* Special Red MX Dead Card */}
              {stats.mx_dead > 0 && (
                <div className="col-span-2 sm:col-span-3 rounded-[2rem] border border-red-200 bg-red-50 p-5 shadow-sm shadow-red-900/5">
                  <div className="flex items-center justify-between gap-4">
                    <div className="flex-1 min-w-0">
                       <span className="inline-flex rounded-md bg-red-100 px-2 py-1 text-[10px] font-extrabold uppercase tracking-widest text-red-700 ring-1 ring-red-200">
                         {t.labels.mx_dead}
                       </span>
                       <div className="mt-2 flex items-baseline gap-2 flex-wrap">
                          <p className="text-3xl font-extrabold text-red-900 tracking-tight">
                            {stats.mx_dead.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                          </p>
                          <span className="text-xs font-bold text-red-600 block opacity-80">(dead_emails.txt)</span>
                       </div>
                    </div>
                    <div className="rounded-2xl bg-red-600 py-4 px-5 text-white shadow-xl shadow-red-600/30 shrink-0 transform transition hover:-rotate-6 hover:scale-110">
                      <AlertCircle className="h-8 w-8" />
                    </div>
                  </div>
                </div>
              )}

              {statCards.map((card) => {
                const Icon = card.icon;
                const value = stats[card.key as keyof ProcessingPayload] as number;
                return (
                  <article
                    key={card.key}
                    className="flex flex-col justify-between overflow-hidden rounded-3xl border border-slate-100 bg-white p-5 shadow-sm transition-all hover:-translate-y-1 hover:shadow-xl hover:border-slate-200 group"
                  >
                    <div className="flex items-center justify-between">
                      <div className={`flex shrink-0 items-center justify-center rounded-2xl p-3 ring-1 transition-transform group-hover:scale-110 ${card.chip}`}>
                        <Icon className="h-6 w-6" />
                      </div>
                    </div>
                    <div className="mt-5">
                       <p className="text-3xl font-extrabold tracking-tight text-slate-800">
                        {value.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                       </p>
                       <p className="mt-1 text-xs font-bold uppercase tracking-wider text-slate-400">
                        {t.labels[card.key as keyof typeof t.labels]}
                       </p>
                       <p className="mt-1 text-[11px] font-bold text-slate-300">
                        ({totalClassified > 0 ? ((value / totalClassified) * 100).toFixed(1) : "0.0"}%)
                       </p>
                    </div>
                  </article>
                );
              })}
            </div>
            
          </div>
        </div>
      </div>
"""

lines = lines[:start_index] + [new_jsx + "\n"] + lines[end_index:]

with open("src/App.tsx", "w") as f:
    f.writelines(lines)

