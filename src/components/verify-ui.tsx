import {
  AlertCircle,
  CheckCircle,
  FolderOpen,
  SearchCheck,
  ShieldCheck,
  Trash2,
  XCircle,
  Clock,
  ServerCrash,
  WifiOff,
  UserX,
} from "lucide-react";

export const verifyBucketTone = {
  final_alive: {
    panel: "border-emerald-200 bg-emerald-50",
    label: "text-emerald-700",
    value: "text-emerald-900",
    item: "border-emerald-100 bg-white",
  },
  final_dead: {
    panel: "border-red-200 bg-red-50",
    label: "text-red-600",
    value: "text-red-900",
    item: "border-red-100 bg-white",
  },
  final_unknown: {
    panel: "border-amber-200 bg-amber-50",
    label: "text-amber-700",
    value: "text-amber-900",
    item: "border-amber-100 bg-white",
  },
  mx_has_mx: {
    panel: "border-emerald-200 bg-emerald-50",
    label: "text-emerald-700",
    value: "text-emerald-900",
    item: "border-emerald-100 bg-white",
  },
  mx_a_fallback: {
    panel: "border-cyan-200 bg-cyan-50",
    label: "text-cyan-700",
    value: "text-cyan-900",
    item: "border-cyan-100 bg-white",
  },
  mx_dead: {
    panel: "border-red-200 bg-red-50",
    label: "text-red-600",
    value: "text-red-900",
    item: "border-red-100 bg-white",
  },
  mx_inconclusive: {
    panel: "border-amber-200 bg-amber-50",
    label: "text-amber-700",
    value: "text-amber-900",
    item: "border-amber-100 bg-white",
  },
  mx_parked: {
    panel: "border-yellow-200 bg-yellow-50",
    label: "text-yellow-700",
    value: "text-yellow-900",
    item: "border-yellow-100 bg-white",
  },
  mx_disposable: {
    panel: "border-orange-200 bg-orange-50",
    label: "text-orange-700",
    value: "text-orange-900",
    item: "border-orange-100 bg-white",
  },
  mx_typo: {
    panel: "border-violet-200 bg-violet-50",
    label: "text-violet-700",
    value: "text-violet-900",
    item: "border-violet-100 bg-white",
  },
  smtp_deliverable: {
    panel: "border-emerald-200 bg-emerald-50",
    label: "text-emerald-700",
    value: "text-emerald-900",
    item: "border-emerald-100 bg-white",
  },
  smtp_rejected: {
    panel: "border-rose-200 bg-rose-50",
    label: "text-rose-700",
    value: "text-rose-900",
    item: "border-rose-100 bg-white",
  },
  smtp_catchall: {
    panel: "border-amber-200 bg-amber-50",
    label: "text-amber-700",
    value: "text-amber-900",
    item: "border-amber-100 bg-white",
  },
  smtp_unknown: {
    panel: "border-slate-200 bg-slate-50",
    label: "text-slate-600",
    value: "text-slate-900",
    item: "border-slate-200 bg-white",
  },
  smtp_policy_blocked: {
    panel: "border-amber-200 bg-amber-50",
    label: "text-amber-700",
    value: "text-amber-900",
    item: "border-amber-100 bg-white",
  },
  smtp_temp_failure: {
    panel: "border-blue-200 bg-blue-50",
    label: "text-blue-700",
    value: "text-blue-900",
    item: "border-blue-100 bg-white",
  },
  smtp_mailbox_full: {
    panel: "border-orange-200 bg-orange-50",
    label: "text-orange-700",
    value: "text-orange-900",
    item: "border-orange-100 bg-white",
  },
  smtp_mailbox_disabled: {
    panel: "border-slate-200 bg-slate-50",
    label: "text-slate-700",
    value: "text-slate-900",
    item: "border-slate-200 bg-white",
  },
  smtp_bad_mailbox: {
    panel: "border-red-200 bg-red-50",
    label: "text-red-700",
    value: "text-red-900",
    item: "border-red-100 bg-white",
  },
  smtp_bad_domain: {
    panel: "border-rose-200 bg-rose-50",
    label: "text-rose-700",
    value: "text-rose-900",
    item: "border-rose-100 bg-white",
  },
  smtp_network_error: {
    panel: "border-neutral-200 bg-neutral-50",
    label: "text-neutral-700",
    value: "text-neutral-900",
    item: "border-neutral-200 bg-white",
  },
  smtp_protocol_error: {
    panel: "border-stone-200 bg-stone-50",
    label: "text-stone-700",
    value: "text-stone-900",
    item: "border-stone-200 bg-white",
  },
  smtp_timeout: {
    panel: "border-zinc-200 bg-zinc-50",
    label: "text-zinc-700",
    value: "text-zinc-900",
    item: "border-zinc-200 bg-white",
  },
} as const;

export type VerifyBucketKey = keyof typeof verifyBucketTone;

const verifyBucketIcon: Record<VerifyBucketKey, typeof CheckCircle> = {
  final_alive: CheckCircle,
  final_dead: XCircle,
  final_unknown: AlertCircle,
  mx_has_mx: CheckCircle,
  mx_a_fallback: FolderOpen,
  mx_dead: AlertCircle,
  mx_inconclusive: SearchCheck,
  mx_parked: ShieldCheck,
  mx_disposable: Trash2,
  mx_typo: SearchCheck,
  smtp_deliverable: CheckCircle,
  smtp_rejected: XCircle,
  smtp_catchall: ShieldCheck,
  smtp_unknown: AlertCircle,
  smtp_policy_blocked: ShieldCheck,
  smtp_temp_failure: AlertCircle,
  smtp_mailbox_full: Trash2,
  smtp_mailbox_disabled: XCircle,
  smtp_bad_mailbox: UserX,
  smtp_bad_domain: XCircle,
  smtp_network_error: WifiOff,
  smtp_protocol_error: ServerCrash,
  smtp_timeout: Clock,
};

export function VerifyHeroCard({
  bucket,
  label,
  value,
  fileName,
}: {
  bucket: VerifyBucketKey;
  label: string;
  value: string;
  fileName: string;
}) {
  const Icon = verifyBucketIcon[bucket];
  const tone = verifyBucketTone[bucket];
  const solidIconBg = {
    final_alive: "bg-emerald-600 shadow-emerald-600/30",
    final_dead: "bg-red-600 shadow-red-600/30",
    final_unknown: "bg-amber-500 shadow-amber-500/30",
    mx_has_mx: "bg-emerald-600 shadow-emerald-600/30",
    mx_a_fallback: "bg-cyan-600 shadow-cyan-600/30",
    mx_dead: "bg-red-600 shadow-red-600/30",
    mx_inconclusive: "bg-amber-500 shadow-amber-500/30",
    mx_parked: "bg-yellow-500 shadow-yellow-500/30",
    mx_disposable: "bg-orange-500 shadow-orange-500/30",
    mx_typo: "bg-violet-600 shadow-violet-600/30",
    smtp_deliverable: "bg-emerald-600 shadow-emerald-600/30",
    smtp_rejected: "bg-rose-600 shadow-rose-600/30",
    smtp_catchall: "bg-amber-500 shadow-amber-500/30",
    smtp_unknown: "bg-slate-600 shadow-slate-600/30",
    smtp_policy_blocked: "bg-amber-500 shadow-amber-500/30",
    smtp_temp_failure: "bg-blue-600 shadow-blue-600/30",
    smtp_mailbox_full: "bg-orange-500 shadow-orange-500/30",
    smtp_mailbox_disabled: "bg-slate-600 shadow-slate-600/30",
    smtp_bad_mailbox: "bg-red-600 shadow-red-600/30",
    smtp_bad_domain: "bg-rose-600 shadow-rose-600/30",
    smtp_network_error: "bg-neutral-600 shadow-neutral-600/30",
    smtp_protocol_error: "bg-stone-600 shadow-stone-600/30",
    smtp_timeout: "bg-zinc-600 shadow-zinc-600/30",
  } as const;

  return (
    <div className={`flex items-center gap-4 rounded-2xl border p-4 ${tone.panel}`}>
      <div className={`shrink-0 rounded-xl p-2.5 text-white shadow-lg ${solidIconBg[bucket]}`}>
        <Icon className="h-6 w-6" />
      </div>
      <div className="min-w-0 flex-1">
        <p className={`text-[10px] font-bold uppercase tracking-widest ${tone.label}`}>{label}</p>
        <p className={`text-2xl font-extrabold leading-tight ${tone.value}`}>{value}</p>
        <p className={`truncate text-[11px] font-medium ${tone.label}`}>{fileName}</p>
      </div>
    </div>
  );
}

export function VerifySummaryCard({
  bucket,
  label,
  value,
}: {
  bucket: VerifyBucketKey;
  label: string;
  value: string;
}) {
  const Icon = verifyBucketIcon[bucket];
  const tone = verifyBucketTone[bucket];

  return (
    <div className={`rounded-2xl border p-4 ${tone.panel}`}>
      <div className="flex items-start gap-3">
        <div className={`rounded-xl p-2 ${tone.item}`}>
          <Icon className={`h-4 w-4 ${tone.label}`} />
        </div>
        <div>
          <div className={`text-[10px] font-bold uppercase tracking-widest ${tone.label}`}>{label}</div>
          <div className={`mt-2 text-xl font-extrabold ${tone.value}`}>{value}</div>
        </div>
      </div>
    </div>
  );
}

function VerifyHistoryBucketItem({
  bucket,
  formattedValue,
  label,
}: {
  bucket: VerifyBucketKey;
  formattedValue: string;
  label: string;
}) {
  const Icon = verifyBucketIcon[bucket];
  const tone = verifyBucketTone[bucket];

  return (
    <div className={`flex items-center justify-between rounded-lg border px-3 py-2 shadow-sm ${tone.item}`}>
      <div className="flex items-center gap-2">
        <Icon className={`h-3.5 w-3.5 ${tone.label}`} />
        <div className={`text-[11px] font-medium uppercase ${tone.label}`}>{label}</div>
      </div>
      <div className={`text-sm font-bold ${tone.value}`}>{formattedValue}</div>
    </div>
  );
}

export function VerifyHistoryGroup({
  title,
  titleClassName,
  className,
  buckets,
  getValue,
  getLabel,
  formatValue,
}: {
  title: string;
  titleClassName: string;
  className: string;
  buckets: VerifyBucketKey[];
  getValue: (bucket: VerifyBucketKey) => number;
  getLabel: (bucket: VerifyBucketKey) => string;
  formatValue: (value: number) => string;
}) {
  return (
    <div className={className}>
      <div className={`mb-2 text-[10px] font-bold uppercase tracking-widest ${titleClassName}`}>{title}</div>
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {buckets.map((bucket) => {
          const value = getValue(bucket);
          return (
            <VerifyHistoryBucketItem
              key={bucket}
              bucket={bucket}
              formattedValue={formatValue(value)}
              label={getLabel(bucket)}
            />
          );
        })}
      </div>
    </div>
  );
}
