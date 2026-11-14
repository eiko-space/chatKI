import type { Measurement } from "./api";

export type Verdict = "better" | "worse" | "same" | "unknown";

export interface MetricDef {
  key: keyof Measurement;
  label: string;
  unit: string;
  digits: number;
  /** true = größer ist besser (z. B. FPS); false = kleiner ist besser (z. B. Ping) */
  higherIsBetter: boolean;
  /** Unterschiede unterhalb dieser Grenzen gelten als Messrauschen → „gleich“ */
  minRelative: number;
  minAbsolute: number;
  hint: string;
}

export const METRICS: MetricDef[] = [
  { key: "low1", label: "1 %-Lows", unit: "FPS", digits: 0, higherIsBetter: true, minRelative: 0.03, minAbsolute: 1, hint: "Die langsamsten 1 % – zeigen Ruckler" },
  { key: "low01", label: "0,1 %-Lows", unit: "FPS", digits: 0, higherIsBetter: true, minRelative: 0.05, minAbsolute: 1, hint: "Die allerschlimmsten Einbrüche" },
  { key: "fpsAvg", label: "Ø FPS", unit: "FPS", digits: 0, higherIsBetter: true, minRelative: 0.03, minAbsolute: 1, hint: "Durchschnittliche Bildrate" },
  { key: "frametimeStdDevMs", label: "Frametime-Schwankung", unit: "ms", digits: 2, higherIsBetter: false, minRelative: 0.1, minAbsolute: 0.1, hint: "Je kleiner, desto gleichmäßiger" },
  { key: "stuttersPerMin", label: "Ruckler", unit: "/min", digits: 1, higherIsBetter: false, minRelative: 0.15, minAbsolute: 1, hint: "Bilder, die doppelt so lange dauern wie üblich" },
  { key: "inputLatencyMs", label: "Input-Delay", unit: "ms", digits: 1, higherIsBetter: false, minRelative: 0.05, minAbsolute: 0.5, hint: "Maus/Tastatur bis zum Bild" },
  { key: "pingAvgMs", label: "Ø Ping", unit: "ms", digits: 0, higherIsBetter: false, minRelative: 0.05, minAbsolute: 2, hint: "Zu 1.1.1.1 (Referenz)" },
  { key: "jitterMs", label: "Jitter", unit: "ms", digits: 1, higherIsBetter: false, minRelative: 0.15, minAbsolute: 0.5, hint: "Schwankung des Pings" },
  { key: "spikesPerMin", label: "Ping-Spikes", unit: "/min", digits: 1, higherIsBetter: false, minRelative: 0.2, minAbsolute: 1, hint: "Plötzliche Ping-Spitzen" },
  { key: "lossPercent", label: "Paketverlust", unit: "%", digits: 1, higherIsBetter: false, minRelative: 0.2, minAbsolute: 0.5, hint: "Verlorene Ping-Pakete" },
];

export interface Compared {
  def: MetricDef;
  before: number | null;
  after: number | null;
  changePercent: number | null;
  verdict: Verdict;
}

export function compare(before: Measurement, after: Measurement): Compared[] {
  return METRICS.map((def) => {
    const b = (before[def.key] ?? null) as number | null;
    const a = (after[def.key] ?? null) as number | null;
    if (b == null || a == null) return { def, before: b, after: a, changePercent: null, verdict: "unknown" as Verdict };
    const diff = a - b;
    const rel = b !== 0 ? diff / Math.abs(b) : diff === 0 ? 0 : Infinity;
    const significant = Math.abs(diff) >= def.minAbsolute && Math.abs(rel) >= def.minRelative;
    const improved = def.higherIsBetter ? diff > 0 : diff < 0;
    return {
      def,
      before: b,
      after: a,
      changePercent: Number.isFinite(rel) ? rel * 100 : null,
      verdict: !significant ? "same" : improved ? "better" : "worse",
    };
  });
}

/** Ein ehrlicher Satz zum Gesamtergebnis. */
export function summary(rows: Compared[]): string {
  const known = rows.filter((r) => r.verdict !== "unknown");
  const better = known.filter((r) => r.verdict === "better").length;
  const worse = known.filter((r) => r.verdict === "worse").length;
  if (known.length === 0) return "Es liegen zu wenige Messwerte für einen Vergleich vor.";
  if (better === 0 && worse === 0) return "Kein messbarer Unterschied – auf deinem PC hat der Boost hier nichts verändert.";
  if (worse === 0) return `${better} von ${known.length} Werten haben sich messbar verbessert.`;
  if (better === 0) return `${worse} Werte sind schlechter geworden. Das kann an einer anderen Spielszene liegen – oder der Boost hilft hier nicht.`;
  return `${better} Werte besser, ${worse} schlechter. Gemischtes Ergebnis – am besten an derselben Stelle im Spiel wiederholen.`;
}
