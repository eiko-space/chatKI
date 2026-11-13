import { Zap } from "lucide-react";
import { cn } from "../../lib/cn";

interface BoostButtonProps {
  active: boolean;
  disabled?: boolean;
  onClick: () => void;
}

/** Großer runder Boost-Knopf mit dezentem „Atmen“ im Hintergrund. */
export function BoostButton({ active, disabled, onClick }: BoostButtonProps) {
  return (
    <div className="relative grid size-52 place-items-center">
      <span
        aria-hidden
        className={cn(
          "absolute inset-0 rounded-full border border-accent/40 animate-breathe",
          active ? "bg-accent/10" : "bg-accent/5",
        )}
      />
      <span aria-hidden className="absolute inset-4 rounded-full border border-line" />
      <button
        onClick={onClick}
        disabled={disabled}
        aria-pressed={active}
        className={cn(
          "relative grid size-36 place-items-center rounded-full border-2",
          "transition-[transform,background-color,box-shadow,border-color] duration-200 ease-out-soft",
          "active:scale-95 disabled:cursor-not-allowed",
          active
            ? "border-accent bg-accent text-bg shadow-[0_0_48px_-6px_var(--color-accent)]"
            : "border-accent/70 bg-surface-2 text-accent hover:bg-surface-3 hover:shadow-[0_0_36px_-10px_var(--color-accent)]",
        )}
      >
        <span className="flex flex-col items-center gap-1">
          <Zap size={34} strokeWidth={2.2} />
          <span className="text-sm font-semibold tracking-wide">{active ? "AKTIV" : "BOOST"}</span>
        </span>
      </button>
    </div>
  );
}
