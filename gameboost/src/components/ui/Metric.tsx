import { cn } from "../../lib/cn";

interface MetricProps {
  label: string;
  /** Echter Messwert. `null` = noch nicht gemessen → es wird „—“ angezeigt, niemals eine erfundene Zahl. */
  value: number | string | null;
  unit?: string;
  size?: "lg" | "md";
  tone?: "neutral" | "good" | "bad";
  hint?: string;
}

export function Metric({ label, value, unit, size = "md", tone = "neutral", hint }: MetricProps) {
  const empty = value === null;
  return (
    <div className="min-w-0">
      <div className="text-xs font-medium text-muted">{label}</div>
      <div className="mt-1 flex items-baseline gap-1">
        <span
          className={cn(
            "font-semibold tracking-tight",
            size === "lg" ? "text-4xl" : "text-2xl",
            empty ? "text-faint" : tone === "good" ? "text-good" : tone === "bad" ? "text-bad" : "text-text",
          )}
        >
          {empty ? "—" : value}
        </span>
        {!empty && unit && <span className="text-sm text-muted">{unit}</span>}
      </div>
      {hint && <div className="mt-0.5 text-[11px] text-faint">{hint}</div>}
    </div>
  );
}
