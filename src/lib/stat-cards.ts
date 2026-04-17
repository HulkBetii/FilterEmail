import {
  CheckCircle,
  Copy,
  FolderOpen,
  Mail,
  SearchCheck,
  ShieldCheck,
  Target,
  Trash2,
  Users,
  XCircle,
  type LucideIcon,
} from "lucide-react";

export type StatCardKey =
  | "invalid"
  | "public"
  | "edu"
  | "targeted"
  | "custom"
  | "duplicates"
  | "mx_disposable"
  | "mx_has_mx"
  | "mx_a_fallback"
  | "mx_typo"
  | "mx_parked";

export type StatCardDefinition = {
  key: StatCardKey;
  icon: LucideIcon;
  chip: string;
};

export const STAT_CARDS: StatCardDefinition[] = [
  {
    key: "invalid",
    icon: XCircle,
    chip: "bg-red-50 text-red-700 ring-red-100",
  },
  {
    key: "public",
    icon: Users,
    chip: "bg-blue-50 text-blue-700 ring-blue-100",
  },
  {
    key: "edu",
    icon: ShieldCheck,
    chip: "bg-emerald-50 text-emerald-700 ring-emerald-100",
  },
  {
    key: "targeted",
    icon: Target,
    chip: "bg-fuchsia-50 text-fuchsia-700 ring-fuchsia-100",
  },
  {
    key: "custom",
    icon: Mail,
    chip: "bg-amber-50 text-amber-700 ring-amber-100",
  },
  {
    key: "duplicates",
    icon: Copy,
    chip: "bg-slate-50 text-slate-700 ring-slate-200",
  },
  {
    key: "mx_disposable",
    icon: Trash2,
    chip: "bg-orange-50 text-orange-700 ring-orange-100",
  },
  {
    key: "mx_has_mx",
    icon: CheckCircle,
    chip: "bg-emerald-50 text-emerald-700 ring-emerald-100",
  },
  {
    key: "mx_a_fallback",
    icon: FolderOpen,
    chip: "bg-cyan-50 text-cyan-700 ring-cyan-100",
  },
  {
    key: "mx_typo",
    icon: SearchCheck,
    chip: "bg-violet-50 text-violet-700 ring-violet-100",
  },
  {
    key: "mx_parked",
    icon: ShieldCheck,
    chip: "bg-amber-50 text-amber-700 ring-amber-100",
  },
];
