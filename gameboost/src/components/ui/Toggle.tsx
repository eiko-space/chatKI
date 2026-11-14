import { cn } from "../../lib/cn";

interface ToggleProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  label: string;
}

/** Ein/Aus-Schalter. `label` wird für Screenreader genutzt. */
export function Toggle({ checked, onChange, disabled, label }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40",
        checked ? "bg-accent" : "bg-surface-3",
      )}
    >
      <span
        className={cn(
          "inline-block size-5 rounded-full bg-text shadow transition-transform duration-150 ease-out-soft",
          checked ? "translate-x-[22px] bg-bg" : "translate-x-0.5",
        )}
      />
    </button>
  );
}
