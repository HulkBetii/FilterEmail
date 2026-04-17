# 1. Update processor.rs
with open("src-tauri/src/processor.rs", "r") as f:
    text = f.read()

# adding mx_dead
text = text.replace(
    'pub duplicates: u64,\n    pub elapsed_ms: u128,',
    'pub duplicates: u64,\n    pub mx_dead: u64,\n    pub elapsed_ms: u128,'
)
text = text.replace(
    'let mut duplicates: u64 = 0;',
    'let mut duplicates: u64 = 0;\n    let mut mx_dead: u64 = 0;'
)
text = text.replace(
    '''let group = if !is_alive {
                EmailGroup::Invalid
            } else {''',
    '''let group = if !is_alive {
                mx_dead += 1;
                EmailGroup::Invalid
            } else {'''
)
text = text.replace(
    '''duplicates: u64,
    elapsed_ms: u128,''',
    '''duplicates: u64,
    mx_dead: u64,
    elapsed_ms: u128,'''
)
text = text.replace(
    '''duplicates,
        elapsed_ms,''',
    '''duplicates,
        mx_dead,
        elapsed_ms,'''
)

with open("src-tauri/src/processor.rs", "w") as f:
    f.write(text)


# 2. Update i18n
with open("src/i18n.ts", "r") as f:
    i18n = f.read()

i18n = i18n.replace(
    'duplicates: "Duplicates",\n    custom: "Other",',
    'duplicates: "Duplicates",\n    mx_dead: "Dead Domains (MX)",\n    custom: "Other",'
)
i18n = i18n.replace(
    'duplicates: "Bị Trùng",\n    custom: "Khác",',
    'duplicates: "Bị Trùng",\n    mx_dead: "Tên miền đã chết",\n    custom: "Khác",'
)
with open("src/i18n.ts", "w") as f:
    f.write(i18n)

# 3. Update App.tsx
with open("src/App.tsx", "r") as f:
    app = f.read()

app = app.replace(
    'duplicates: number;\n  elapsed_ms: number;',
    'duplicates: number;\n  mx_dead: number;\n  elapsed_ms: number;'
)
app = app.replace(
    'duplicates: 0,\n  elapsed_ms: 0,',
    'duplicates: 0,\n  mx_dead: 0,\n  elapsed_ms: 0,'
)
app = app.replace(
    'stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom + stats.duplicates,',
    'stats.invalid + stats.public + stats.edu + stats.targeted + stats.custom + stats.duplicates + stats.mx_dead,'
)

# We want to conditionally display the mx_dead card
# Let's insert a conditional block in the rendering
app_stat_render = '''            {statCards.map((card) => {
              const Icon = card.icon;
              const value = stats[card.key];'''
app_stat_new = '''            {statCards.map((card) => {
              const Icon = card.icon;
              const value = stats[card.key as keyof ProcessingPayload] as number;'''
app = app.replace(app_stat_render, app_stat_new)

# Wait, we need to add mx_dead to statCards dynamically or conditionally rendering it
# Alternatively we inject mx_dead card directly if checkMx is true or stats.mx_dead > 0
# Let's add the card block manually.
mx_card_block = '''            {stats.mx_dead > 0 && (
              <article className="rounded-3xl border border-red-200 bg-red-50 p-4 shadow-sm backdrop-blur-xl sm:p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <span className="inline-flex max-w-full items-center rounded-full px-3 py-1 text-xs font-medium ring-1 bg-red-100 text-red-800 ring-red-200">
                      {t.labels.mx_dead}
                    </span>
                    <p className="mt-4 text-2xl font-semibold tracking-tight text-red-900 flex items-baseline gap-2">
                       {stats.mx_dead.toLocaleString(language === "vi" ? "vi-VN" : "en-US")}
                    </p>
                  </div>
                  <div className="rounded-2xl bg-red-600 p-3 text-white shadow-sm shadow-red-900/10">
                    <AlertCircle className="h-5 w-5" />
                  </div>
                </div>
              </article>
            )}'''

app = app.replace(
    '<article className="rounded-3xl border border-white/60 bg-slate-900 p-4 text-white shadow-md">',
    mx_card_block + '\n\n            <article className="rounded-3xl border border-white/60 bg-slate-900 p-4 text-white shadow-md">'
)

with open("src/App.tsx", "w") as f:
    f.write(app)

