with open("src/App.tsx", "r") as f:
    app = f.read()

import re

old_grid = """<div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-3">
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
                      </div>"""

new_grid = """<div className="mt-3 flex flex-col gap-2">
                        {/* Summary primary */}
                        <div className="grid grid-cols-3 sm:grid-cols-5 gap-2">
                          <div className="rounded-lg bg-white p-2 border border-slate-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Total</div>
                            <div className="text-sm font-bold text-slate-800">{total.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-emerald-50 p-2 border border-emerald-100 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-emerald-600">Valid</div>
                            <div className="text-sm font-bold text-emerald-700">{valid.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-slate-400">Dups</div>
                            <div className="text-sm font-bold text-slate-600">{entry.stats.duplicates.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-red-50 p-2 border border-red-100 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-red-500">Invalid</div>
                            <div className="text-sm font-bold text-red-600">{entry.stats.invalid.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-red-100 p-2 border border-red-200 text-center shadow-sm">
                            <div className="text-[10px] uppercase font-bold text-red-700">MX Dead</div>
                            <div className="text-sm font-bold text-red-800">{(entry.stats.mx_dead || 0).toLocaleString()}</div>
                          </div>
                        </div>

                        {/* Breakdown */}
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.public}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.public.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.edu}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.edu.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.targeted}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.targeted.toLocaleString()}</div>
                          </div>
                          <div className="rounded-lg bg-white p-2 border border-slate-100 flex justify-between items-center px-3">
                            <div className="text-xs font-medium text-slate-500">{t.labels.custom}</div>
                            <div className="text-sm font-bold text-slate-700">{entry.stats.custom.toLocaleString()}</div>
                          </div>
                        </div>
                        
                        <div className="text-right text-[10px] text-slate-400 font-medium mt-1">
                          {t.labels.elapsed}: {(entry.stats.elapsed_ms / 1000).toFixed(2)}s
                        </div>
                      </div>"""

app = app.replace(old_grid, new_grid)

with open("src/App.tsx", "w") as f:
    f.write(app)

