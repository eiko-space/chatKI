import { useEffect, useRef, useState } from "react";
import { Zap } from "lucide-react";
import { cn } from "../../lib/cn";

interface BoostButtonProps {
  active: boolean;
  working?: boolean;
  disabled?: boolean;
  onClick: () => void;
}

/**
 * Großer runder Boost-Knopf.
 * Ruhe: sanftes Atmen · Arbeit: rotierender Ring · Aktiviert: Druckwelle + Leuchten.
 */
export function BoostButton({ active, working, disabled, onClick }: BoostButtonProps) {
  const [waves, setWaves] = useState<number[]>([]);
  const prev = useRef(active);

  // Beim Einschalten zwei Druckwellen auslösen
  useEffect(() => {
    if (active && !prev.current) {
      const t = Date.now();
      setWaves([t, t + 1]);
      const id = setTimeout(() => setWaves([]), 1200);
      prev.current = active;
      return () => clearTimeout(id);
    }
    prev.current = active;
  }, [active]);

  return (
    <div className="relative grid size-56 place-items-center">
      {!working && (
        <span aria-hidden className={cn("absolute inset-2 rounded-full border border-accent/40 animate-breathe", active ? "bg-accent/10" : "bg-accent/5")} />
      )}
      {waves.map((w, i) => (
        <span key={w} aria-hidden className="absolute inset-8 rounded-full border-2 border-accent animate-shockwave" style={{ animationDelay: `${i * 180}ms` }} />
      ))}
      {working && (
        <span
          aria-hidden
          className="absolute inset-5 rounded-full animate-spin-slow"
          style={{ background: "conic-gradient(from 0deg, transparent 0deg, var(--color-accent) 90deg, transparent 160deg)", mask: "radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2px))", WebkitMask: "radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2px))" }}
        />
      )}
      <span aria-hidden className="absolute inset-6 rounded-full border border-line" />
      <button
        onClick={onClick}
        disabled={disabled || working}
        aria-pressed={active}
        aria-busy={working}
        className={cn(
          "relative grid size-36 place-items-center rounded-full border-2",
          "transition-[transform,background-color,box-shadow,border-color,color] duration-200 ease-out-soft",
          "active:scale-95 disabled:cursor-not-allowed",
          active
            ? "border-accent bg-accent text-bg animate-glow"
            : "border-accent/70 bg-surface-2 text-accent hover:scale-[1.03] hover:bg-surface-3 hover:shadow-[0_0_36px_-10px_var(--color-accent)]",
          working && "scale-95 opacity-90",
        )}
      >
        <span className="flex flex-col items-center gap-1">
          <Zap size={34} strokeWidth={2.2} className={cn("transition-transform duration-200", active && "scale-110")} />
          <span className="text-sm font-semibold tracking-wide">{working ? "…" : active ? "AKTIV" : "BOOST"}</span>
        </span>
      </button>
    </div>
  );
}
