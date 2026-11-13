import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/cn";

/** Aufklappbare „Profi-Details“ – Standardansicht bleibt einfach. */
export function Details({ label = "Profi-Details", children }: { label?: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="mt-4 border-t border-line pt-3">
      <button
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-1.5 text-xs font-medium text-muted transition-colors duration-150 hover:text-text"
        aria-expanded={open}
      >
        <ChevronDown size={14} className={cn("transition-transform duration-150", open && "rotate-180")} />
        {label}
      </button>
      {open && <div className="mt-3 animate-fade-in text-xs leading-relaxed text-muted">{children}</div>}
    </div>
  );
}
