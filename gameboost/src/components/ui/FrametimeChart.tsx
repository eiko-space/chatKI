import { useEffect, useRef, useState } from "react";
import { num } from "../../lib/format";

interface Props {
  /** Frametimes in ms, älteste zuerst. */
  data: number[];
  height?: number;
}

const PAD = { top: 10, right: 8, bottom: 8, left: 44 };

/** Gut lesbare Achsen-Obergrenze (5, 10, 20, 25, 50, 100 … ms). */
function niceMax(v: number): number {
  const steps = [5, 10, 20, 25, 40, 50, 75, 100, 150, 200, 300, 500, 1000];
  return steps.find((s) => s >= v) ?? Math.ceil(v / 500) * 500;
}

function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/**
 * Frametime-Verlauf: jeder Punkt = ein Bild. Spitzen nach oben = Ruckler.
 * Eine Linie, ruhiges Raster, Fadenkreuz + Tooltip beim Überfahren.
 */
export function FrametimeChart({ data, height = 140 }: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [width, setWidth] = useState(0);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setWidth(Math.floor(e.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const n = data.length;
  const yMax = niceMax(Math.max(10, ...data) * 1.1);
  const plotW = Math.max(1, width - PAD.left - PAD.right);
  const plotH = height - PAD.top - PAD.bottom;
  const x = (i: number) => PAD.left + (n <= 1 ? plotW : (i / (n - 1)) * plotW);
  const y = (v: number) => PAD.top + plotH - (Math.min(v, yMax) / yMax) * plotH;

  useEffect(() => {
    const c = canvas.current;
    if (!c || width === 0) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = width * dpr;
    c.height = height * dpr;
    const ctx = c.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    // Raster + Beschriftung (zurückhaltend)
    ctx.font = "11px 'Inter Variable', sans-serif";
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    const ticks = [0, yMax / 2, yMax];
    for (const t of ticks) {
      ctx.strokeStyle = cssVar("--color-line");
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(PAD.left, Math.round(y(t)) + 0.5);
      ctx.lineTo(width - PAD.right, Math.round(y(t)) + 0.5);
      ctx.stroke();
      ctx.fillStyle = cssVar("--color-faint");
      ctx.fillText(`${num(t, t % 1 ? 1 : 0)} ms`, PAD.left - 8, y(t));
    }

    if (n < 2) return;
    ctx.strokeStyle = cssVar("--color-accent");
    ctx.lineWidth = 2;
    ctx.lineJoin = "round";
    ctx.beginPath();
    data.forEach((v, i) => (i === 0 ? ctx.moveTo(x(i), y(v)) : ctx.lineTo(x(i), y(v))));
    ctx.stroke();

    if (hover !== null && hover < n) {
      const hx = x(hover);
      ctx.strokeStyle = cssVar("--color-muted");
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(Math.round(hx) + 0.5, PAD.top);
      ctx.lineTo(Math.round(hx) + 0.5, PAD.top + plotH);
      ctx.stroke();
      ctx.fillStyle = cssVar("--color-accent");
      ctx.strokeStyle = cssVar("--color-surface");
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(hx, y(data[hover]), 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, width, height, hover, yMax]);

  const onMove = (e: React.MouseEvent) => {
    if (n < 2) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const rel = (e.clientX - rect.left - PAD.left) / plotW;
    setHover(Math.max(0, Math.min(n - 1, Math.round(rel * (n - 1)))));
  };

  const hv = hover !== null && hover < n ? data[hover] : null;

  return (
    <div ref={wrap} className="relative" style={{ height }} onMouseMove={onMove} onMouseLeave={() => setHover(null)}>
      <canvas ref={canvas} style={{ width: "100%", height }} role="img" aria-label="Frametime-Verlauf" />
      {hv !== null && hover !== null && (
        <div
          className="pointer-events-none absolute top-0 rounded-md border border-line bg-surface-2 px-2 py-1 text-[11px] shadow-lg"
          style={{ left: Math.min(Math.max(x(hover) + 10, PAD.left), width - 110) }}
        >
          <div className="font-semibold text-text">{num(hv, 1)} ms</div>
          <div className="text-muted">≈ {num(1000 / hv)} FPS</div>
        </div>
      )}
    </div>
  );
}
