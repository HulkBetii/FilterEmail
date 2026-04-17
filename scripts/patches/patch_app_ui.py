import re

with open("src/App.tsx", "r") as f:
    content = f.read()

# 1. Main container paddings (make it compact)
content = content.replace(
    'className="min-h-screen bg-aura px-3 py-4 text-ink sm:px-5 sm:py-6 lg:px-8 lg:py-8"',
    'className="min-h-screen bg-aura px-3 py-3 text-ink sm:px-4 sm:py-4 lg:px-6 lg:py-6"'
)
content = content.replace(
    'className="mx-auto flex w-full max-w-7xl flex-col gap-4 lg:gap-6"',
    'className="mx-auto flex w-full max-w-6xl flex-col gap-4"'
)

# 2. Main Hero Card
content = content.replace(
    'className="rounded-[2rem] border border-white/70 bg-glass p-5 shadow-float backdrop-blur-2xl sm:p-6 lg:p-8"',
    'className="rounded-3xl border border-white/70 bg-glass p-4 shadow-float backdrop-blur-2xl sm:p-5 lg:p-6"'
)

# 3. Hero Text
content = content.replace(
    'text-3xl font-semibold leading-[1.02] tracking-tight text-slate-900 sm:text-4xl lg:text-5xl xl:text-[4.35rem]',
    'text-2xl font-semibold leading-tight tracking-tight text-slate-900 sm:text-3xl lg:text-4xl'
)

# 4. Status panel inside Hero
content = content.replace(
    'rounded-[1.75rem] border border-white/80 bg-white/78 p-4 shadow-glass backdrop-blur-xl sm:p-5',
    'rounded-3xl border border-white/80 bg-white/78 p-4 shadow-sm backdrop-blur-xl'
)

# 5. Language Segmented Control (iOS style)
content = content.replace(
    'rounded-2xl bg-white/95 px-3 py-3 ring-1 ring-slate-200/70',
    'rounded-2xl bg-white/60 px-3 py-2 ring-1 ring-slate-200/50'
)
content = content.replace(
    'bg-slate-200/80 p-1',
    'bg-slate-200/60 p-1 shadow-inner'
)
content = content.replace(
    'bg-slate-900 text-white shadow',
    'bg-white text-slate-900 shadow-sm ring-1 ring-slate-900/5'
)
content = content.replace(
    'text-slate-700 hover:bg-white hover:text-slate-900',
    'text-slate-500 hover:text-slate-700'
)

# 6. Lower layout (Drag & Drop, Controls, Results) - gap shrinking
content = content.replace(
    'className="grid gap-4 xl:grid-cols-[minmax(0,1.28fr)_minmax(320px,0.92fr)] xl:gap-6"',
    'className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(300px,0.85fr)]"'
)

# 7. Left panel layout
content = content.replace(
    'className="space-y-4 rounded-[2rem] border border-white/80 bg-white/72 p-5 shadow-glass backdrop-blur-2xl sm:p-6 lg:p-8"',
    'className="space-y-4 rounded-3xl border border-white/80 bg-white/72 p-4 shadow-sm backdrop-blur-2xl sm:p-5"'
)

# 8. Main Drag zone
content = content.replace(
    'rounded-[1.75rem] border border-dashed p-6 transition-all duration-300 sm:p-8',
    'rounded-[1.5rem] border border-dashed p-5 transition-all duration-300 sm:p-6'
)

# 9. Control panels (rounded-[1.5rem] -> rounded-2xl)
content = content.replace(
    'rounded-[1.5rem] border border-white/80 bg-white/88 p-4 shadow-sm shadow-slate-200/60',
    'rounded-2xl border border-white/80 bg-white/88 p-3 shadow-sm shadow-slate-200/60'
)

# 10. iOS style input
content = content.replace(
    'mt-2 w-full rounded-xl border border-slate-300 bg-white px-4 py-2.5 text-slate-900 placeholder-slate-400 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500 transition-colors',
    'mt-2 w-full rounded-xl bg-slate-100 px-4 py-2 text-slate-900 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-sky-500/50 transition-colors border border-transparent'
)

# 11. Buttons (make them slightly smaller and standard iOS height)
content = content.replace(
    'px-4 py-2.5 text-sm',
    'px-4 py-2 text-sm'
)
content = content.replace(
    'px-5 py-3 text-sm font-medium',
    'px-4 py-2.5 text-sm font-medium'
)

# 12. Summary panel
content = content.replace(
    'rounded-[1.75rem] border border-emerald-300 bg-gradient-to-br from-emerald-50 via-white to-sky-50 p-5 shadow-glass',
    'rounded-3xl border border-emerald-300 bg-gradient-to-br from-emerald-50 via-white to-sky-50 p-4 shadow-sm'
)

# 13. Stat Cards right side
content = content.replace(
    'rounded-[1.75rem] border border-white/80 bg-white/80 p-4 shadow-glass backdrop-blur-2xl sm:p-5',
    'rounded-3xl border border-white/80 bg-white/80 p-4 shadow-sm backdrop-blur-xl sm:p-4'
)

content = content.replace(
    'rounded-[1.75rem] border border-white/60 bg-slate-900 p-5 text-white shadow-float',
    'rounded-3xl border border-white/60 bg-slate-900 p-4 text-white shadow-md'
)

# iOS specific font size tweaks for stat values
content = content.replace(
    'text-3xl font-semibold tracking-tight',
    'text-2xl font-semibold tracking-tight'
)

with open("src/App.tsx", "w") as f:
    f.write(content)

