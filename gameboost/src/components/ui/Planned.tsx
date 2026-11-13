import { Clock } from "lucide-react";
import { Card } from "./Card";
import { Badge } from "./Badge";

interface PlannedProps {
  phase: number;
  title: string;
  items: string[];
}

/** Ehrlicher Platzhalter: zeigt, was hier in welcher Phase kommt – ohne Fake-Werte. */
export function Planned({ phase, title, items }: PlannedProps) {
  return (
    <Card title={title} action={<Badge><Clock size={11} /> Phase {phase}</Badge>}>
      <ul className="space-y-2">
        {items.map((item) => (
          <li key={item} className="flex gap-2.5 text-sm text-muted">
            <span className="mt-[7px] size-1.5 shrink-0 rounded-full bg-faint" />
            {item}
          </li>
        ))}
      </ul>
    </Card>
  );
}
