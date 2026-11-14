import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { compare, summary, type Compared } from "../lib/compare";
import type { Measurement } from "../lib/api";
import { num } from "../lib/format";

function Change({ row }: { row: Compared }) {
  if (row.verdict === "unknown") return <span className="text-faint">—</span>;
  const pct = row.changePercent;
  const text = pct == null ? "" : `${pct > 0 ? "+" : ""}${num(pct, Math.abs(pct) < 10 ? 1 : 0)} %`;
  if (row.verdict === "same")
    return (
      <span className="inline-flex items-center gap-1 text-muted">
        <Minus size={13} /> gleich {text && <span className="text-faint">({text})</span>}
      </span>
    );
  const good = row.verdict === "better";
  const up = (pct ?? 0) > 0;
  const Icon = up ? ArrowUpRight : ArrowDownRight;
  return (
    <span className={`inline-flex items-center gap-1 font-medium ${good ? "text-good" : "text-bad"}`}>
      <Icon size={14} /> {text} <span className="sr-only">{good ? "besser" : "schlechter"}</span>
    </span>
  );
}

/** Vergleich Vorher → Nachher. Rauschen (kleine Unterschiede) wird ehrlich als „gleich“ angezeigt. */
export function ComparisonTable({ before, after }: { before: Measurement; after: Measurement }) {
  const rows = compare(before, after);
  return (
    <div>
      <p className="mb-3 text-sm text-text">{summary(rows)}</p>
      <table className="w-full text-sm">
        <thead>
          <tr className="text-left text-xs text-faint">
            <th className="pb-2 font-medium">Wert</th>
            <th className="pb-2 text-right font-medium">Vorher</th>
            <th className="pb-2 text-right font-medium">Nachher</th>
            <th className="pb-2 pl-4 text-right font-medium">Änderung</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {rows.map((r) => (
            <tr key={r.def.key}>
              <td className="py-2">
                <div>{r.def.label}</div>
                <div className="text-[11px] text-faint">{r.def.hint}</div>
              </td>
              <td className="py-2 text-right tabular-nums text-muted">{r.before == null ? "—" : `${num(r.before, r.def.digits)} ${r.def.unit}`}</td>
              <td className="py-2 text-right tabular-nums">{r.after == null ? "—" : `${num(r.after, r.def.digits)} ${r.def.unit}`}</td>
              <td className="py-2 pl-4 text-right tabular-nums"><Change row={r} /></td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="mt-3 text-[11px] text-faint">
        Unterschiede unter der Messgenauigkeit (z. B. weniger als 3 % bei FPS) zählen als „gleich“.
        Gemessen: {before.frames.toLocaleString("de-DE")} → {after.frames.toLocaleString("de-DE")} Bilder in je {num(after.durationSecs)} s.
      </p>
    </div>
  );
}
