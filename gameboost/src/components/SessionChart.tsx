import { useState } from "react";
import type { Session } from "../lib/api";
import { num } from "../lib/format";

const H = 180;
const PAD = { top: 12, right: 8, bottom: 28, left: 40 };

/**
 * 1 %-Lows je Session: Vorher (grau) neben Nachher (Cyan).
 * Zwei Reihen → Legende darüber; Werte per Hover.
 */
export function SessionChart({ sessions, width }: { sessions: Session[]; width: number }) {
  const [hover, setHover] = useState<number | null>(null);
  const data = sessions.filter((s) => s.before.low1 != null && s.after.low1 != null).slice(-12);
  if (data.length === 0) return <p className="text-sm text-faint">Noch keine Messungen mit 1 %-Lows.</p>;

  const max = Math.max(...data.flatMap((s) => [s.before.low1!, s.after.low1!])) * 1.1;
  const plotW = width - PAD.left - PAD.right;
  const plotH = H - PAD.top - PAD.bottom;
  const group = plotW / data.length;
  const bar = Math.min(18, (group - 10) / 2);
  const y = (v: number) => PAD.top + plotH - (v / max) * plotH;
  const ticks = [0, max / 2, max].map((t) => Math.round(t));

  return (
    <div className="relative">
      <div className="mb-2 flex gap-4 text-xs text-muted">
        <span className="inline-flex items-center gap-1.5"><span className="size-2.5 rounded-sm bg-faint" /> Vorher</span>
        <span className="inline-flex items-center gap-1.5"><span className="size-2.5 rounded-sm bg-accent" /> Nachher (mit Boost)</span>
      </div>
      <svg width={width} height={H} role="img" aria-label="1 %-Lows vorher und nachher je Messung">
        {ticks.map((t) => (
          <g key={t}>
            <line x1={PAD.left} x2={width - PAD.right} y1={y(t)} y2={y(t)} stroke="var(--color-line)" />
            <text x={PAD.left - 8} y={y(t)} textAnchor="end" dominantBaseline="middle" fontSize={11} fill="var(--color-faint)">{t}</text>
          </g>
        ))}
        {data.map((s, i) => {
          const cx = PAD.left + group * i + group / 2;
          const b = s.before.low1!;
          const a = s.after.low1!;
          return (
            <g key={s.id} onMouseEnter={() => setHover(i)} onMouseLeave={() => setHover(null)}>
              <rect x={cx - group / 2} y={PAD.top} width={group} height={plotH} fill={hover === i ? "var(--color-surface-2)" : "transparent"} />
              <rect x={cx - bar - 1} y={y(b)} width={bar} height={plotH + PAD.top - y(b)} rx={4} fill="var(--color-faint)" />
              <rect x={cx + 1} y={y(a)} width={bar} height={plotH + PAD.top - y(a)} rx={4} fill="var(--color-accent)" />
              <text x={cx} y={H - 10} textAnchor="middle" fontSize={11} fill="var(--color-faint)">
                {new Date(s.createdAt * 1000).toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit" })}
              </text>
            </g>
          );
        })}
      </svg>
      {hover !== null && data[hover] && (
        <div
          className="pointer-events-none absolute rounded-md border border-line bg-surface-2 px-2.5 py-1.5 text-[11px] shadow-lg"
          style={{ left: Math.min(PAD.left + group * hover + group / 2 + 12, width - 150), top: 28 }}
        >
          <div className="font-semibold text-text">{data[hover].game ?? "Spiel"}</div>
          <div className="text-muted">1 %-Lows: {num(data[hover].before.low1)} → {num(data[hover].after.low1)} FPS</div>
        </div>
      )}
    </div>
  );
}
