import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

type Tone = "neutral" | "accent" | "good" | "bad";

const tones: Record<Tone, string> = {
  neutral: "bg-surface-3 text-muted",
  accent: "bg-accent-dim text-accent",
  good: "bg-good/15 text-good",
  bad: "bg-bad/15 text-bad",
};

export function Badge({ tone = "neutral", children }: { tone?: Tone; children: ReactNode }) {
  return (
    <span className={cn("inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[11px] font-medium", tones[tone])}>
      {children}
    </span>
  );
}
