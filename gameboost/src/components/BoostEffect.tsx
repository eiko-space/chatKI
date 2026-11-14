import { ArrowDownRight, ArrowUpRight, Minus, Sparkles } from "lucide-react";
import { Card } from "./ui/Card";
import { useTelemetry, type FpsStats, type PingStats } from "../lib/telemetry";
import { compare, type Compared } from "../lib/compare";
import type { Measurement } from "../lib/api";
import { num } from "../lib/format";

const WINDOW = 60;

function toMeasurement(fps: FpsStats | null, ping: PingStats): Measurement {
  return {
    game: fps?.game ?? fps?.app ?? null,
    durationSecs: WINDOW,
    frames: fps?.frameCount ?? 0,
    fpsAvg: fps?.average ?? null,
    low1: fps?.low1 ?? null,
    low01: fps?.low01 ?? null,
    frametimeStdDevMs: fps?.frametimeStdDevMs ?? null,
    stuttersPerMin: null,
    pingAvgMs: ping.averageMs,
    jitterMs: ping.jitterMs,
    lossPercent: ping.lossPercent,
    spikesPerMin: ping.spikes != null && ping.windowSecs > 5 ? (ping.spikes * 60) / ping.windowSecs : null,
    cpuAvg: null,
    gpuAvg: null,
    inputLatencyMs: fps?.inputLatencyMs ?? null,
  };
}

const SHOWN = ["fpsAvg", "low1", "inputLatencyMs", "pingAvgMs", "jitterMs", "spikesPerMin"];

function Tile({ row, delay }: { row: Compared; delay: number }) {
  const good = row.verdict === "better";
  const bad = row.verdict === "worse";
  const Icon = row.verdict === "same" || row.verdict === "unknown" ? Minus : (row.changePercent ?? 0) > 0 ? ArrowUpRight : ArrowDownRight;
  const pct = row.changePercent;
  return (
    <div className="rounded-xl bg-surface-2 p-4 animate-pop-in" style={{ animationDelay: `${delay}ms` }}>
      <div className="text-xs font-medium text-muted">{row.def.label}</div>
      <div className="mt-1 flex items-baseline gap-1.5">
        <span className="text-2xl font-semibold tracking-tight">{num(row.after, row.def.digits) ?? "—"}</span>
        <span className="text-xs text-muted">{row.def.unit}</span>
      </div>
      <div className={`mt-1 flex items-center gap-1 whitespace-nowrap text-xs font-medium ${good ? "text-good" : bad ? "text-bad" : "text-faint"}`}>
        <Icon size={13} className="shrink-0" />
        {row.verdict === "unknown"
          ? "keine Daten"
          : row.verdict === "same"
            ? "unverändert"
            : `${pct != null && pct > 0 ? "+" : ""}${num(pct, Math.abs(pct ?? 0) < 10 ? 1 : 0)} %`}
      </div>
      <div className="mt-0.5 text-[11px] text-faint">vorher {num(row.before, row.def.digits) ?? "—"} {row.def.unit}</div>
    </div>
  );
}

/** Live-Wirkung: echte Werte der 60 s vor dem Boost vs. die 60 s danach. */
export function BoostEffect() {
  const { snapshot } = useTelemetry();
  const base = snapshot?.boostBaseline;
  if (!snapshot || !base) return null;

  const since = base.secondsSince;
  const ready = since >= WINDOW;
  const rows = compare(toMeasurement(base.fps, base.ping), toMeasurement(snapshot.fps, snapshot.network.internet)).filter((r) => SHOWN.includes(r.def.key));
  const better = rows.filter((r) => r.verdict === "better").length;

  return (
    <Card
      className="col-span-12"
      title="Wirkung des Boosts"
      subtitle={ready ? "Letzte 60 s mit Boost im Vergleich zu den 60 s davor – echte Messwerte" : "Messe die Wirkung …"}
      action={<Sparkles size={16} className="text-accent" />}
    >
      {!ready ? (
        <div>
          <div className="h-2 overflow-hidden rounded-full bg-surface-3">
            <div className="h-full rounded-full bg-accent transition-[width] duration-200" style={{ width: `${(since / WINDOW) * 100}%` }} />
          </div>
          <p className="mt-2 text-xs text-muted">Noch {num(WINDOW - since)} s – spiel einfach weiter.</p>
        </div>
      ) : (
        <>
          <p className="mb-4 text-sm">
            {better > 0 ? (
              <span className="text-good">{better} Werte haben sich messbar verbessert.</span>
            ) : (
              <span className="text-muted">Kein messbarer Unterschied – dein PC lief schon gut optimiert.</span>
            )}
          </p>
          <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-6">
            {rows.map((r, i) => <Tile key={r.def.key} row={r} delay={i * 70} />)}
          </div>
        </>
      )}
    </Card>
  );
}
