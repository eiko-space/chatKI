import { useEffect, useRef, useState } from "react";
import { Zap } from "lucide-react";
import { cn } from "../../lib/cn";

interface BoostButtonProps {
  active: boolean;
  /** Fortschritt 0–100 während des Boostens/Zurücksetzens, sonst `null` */
  progress: number | null;
  disabled?: boolean;
  onClick: () => void;
}

const R = 104;
const C = 2 * Math.PI * R;

/**
 * Großer runder Boost-Knopf.
 * Ruhe: sanftes Atmen · Arbeit: Fortschrittsring mit Prozent · Aktiviert: Druckwelle + Leuchten.
 */
export function BoostButton({ active, progress, disabled, onClick }: BoostButtonProps) {
  const [waves, setWaves] = useState<number[]>([]);
  const prev = useRef(active);
  const working = progress !== null;
  // Druckwelle erst, wenn der Fortschrittsring fertig ist
  const shownActive = active && !working;

  useEffect(() => {
    if (shownActive && !prev.current) {
      const t = Date.now();
      setWaves([t, t + 1]);
      const id = setTimeout(() => setWaves([]), 1200);
      prev.current = shownActive;
      return () => clearTimeout(id);
    }
    prev.current = shownActive;
  }, [shownActive]);

  return (
    <div className="relative grid size-56 place-items-center">
      {!working && (
        <span aria-hidden className={cn("absolute inset-2 rounded-full border border-accent/40 animate-breathe", active ? "bg-accent/10" : "bg-accent/5")} />
      )}
      {waves.map((w, i) => (
        <span key={w} aria-hidden className="absolute inset-8 rounded-full border-2 border-accent animate-shockwave" style={{ animationDelay: `${i * 180}ms` }} />
      ))}
      {working && (
        <svg viewBox="0 0 224 224" className="absolute inset-0 -rotate-90" aria-hidden>
          <circle cx="112" cy="112" r={R} fill="none" stroke="var(--color-surface-3)" strokeWidth="6" />
          <circle
            cx="112" cy="112" r={R} fill="none" stroke="var(--color-accent)" strokeWidth="6" strokeLinecap="round"
            strokeDasharray={`${(progress / 100) * C} ${C}`}
            style={{ filter: "drop-shadow(0 0 6px var(--color-accent))" }}
          />
        </svg>
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
          working
            ? "border-accent/60 bg-surface-2 text-accent"
            : active
              ? "border-accent bg-accent text-bg animate-glow"
              : "border-accent/70 bg-surface-2 text-accent hover:scale-[1.03] hover:bg-surface-3 hover:shadow-[0_0_36px_-10px_var(--color-accent)]",
        )}
      >
        {working ? (
          <span className="flex flex-col items-center">
            <span className="text-3xl font-semibold tabular-nums tracking-tight">{progress}%</span>
          </span>
        ) : (
          <span className="flex flex-col items-center gap-1">
            <Zap size={34} strokeWidth={2.2} className={cn("transition-transform duration-200", active && "scale-110")} />
            <span className="text-sm font-semibold tracking-wide">{active ? "AKTIV" : "BOOST"}</span>
          </span>
        )}
      </button>
    </div>
  );
}
