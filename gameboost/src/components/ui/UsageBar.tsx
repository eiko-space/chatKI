import { num } from "../../lib/format";

/** Auslastungsbalken mit Zahl. `null` = nicht gemessen. */
export function UsageBar({ label, value, detail }: { label: string; value: number | null; detail?: string | null }) {
  const pct = value === null ? 0 : Math.min(100, Math.max(0, value));
  return (
    <div>
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-xs font-medium text-muted">{label}</span>
        <span className={value === null ? "text-sm font-semibold text-faint" : "text-sm font-semibold"}>
          {value === null ? "—" : `${num(value)} %`}
        </span>
      </div>
      <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-3">
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-200 ease-out-soft"
          style={{ width: `${pct}%` }}
        />
      </div>
      {detail && <div className="mt-1 truncate text-[11px] text-faint">{detail}</div>}
    </div>
  );
}
