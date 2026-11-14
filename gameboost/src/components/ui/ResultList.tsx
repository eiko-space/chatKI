import { Check, Minus, RotateCcw, SkipForward, X } from "lucide-react";
import type { TweakResult } from "../../lib/api";

const META = {
  applied: { icon: Check, cls: "text-good", label: "Angewendet" },
  reverted: { icon: RotateCcw, cls: "text-good", label: "Zurückgesetzt" },
  unchanged: { icon: Minus, cls: "text-muted", label: "Unverändert" },
  skipped: { icon: SkipForward, cls: "text-muted", label: "Übersprungen" },
  failed: { icon: X, cls: "text-bad", label: "Fehlgeschlagen" },
} as const;

/** Ehrliche Ergebnisliste: zeigt auch, was nicht geklappt hat oder übersprungen wurde. */
export function ResultList({ results }: { results: TweakResult[] }) {
  return (
    <ul className="divide-y divide-line">
      {results.map((r) => {
        const m = META[r.status] ?? META.unchanged;
        const Icon = m.icon;
        return (
          <li key={r.id + r.status} className="flex gap-3 py-2.5">
            <Icon size={16} className={`mt-0.5 shrink-0 ${m.cls}`} aria-label={m.label} />
            <div className="min-w-0">
              <div className="text-sm font-medium">{r.title}</div>
              <div className="text-xs text-muted">{r.message}</div>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
