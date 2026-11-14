import { useEffect, useRef, useState } from "react";
import { Trash2 } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { ComparisonTable } from "../components/ComparisonTable";
import { SessionChart } from "../components/SessionChart";
import { measureApi, type Session } from "../lib/api";
import { compare } from "../lib/compare";
import { useMeasure } from "../lib/measure";
import { isTauri } from "../lib/tauri";
import { num } from "../lib/format";

const WEEK = 7 * 24 * 3600;

/** Wochenstatistik – nur aus echten Messungen; ohne Daten keine Zahl. */
function weekly(sessions: Session[]) {
  const recent = sessions.filter((s) => s.createdAt > Date.now() / 1000 - WEEK);
  const changes = recent
    .map((s) => (s.before.frametimeStdDevMs && s.after.frametimeStdDevMs ? (s.before.frametimeStdDevMs - s.after.frametimeStdDevMs) / s.before.frametimeStdDevMs : null))
    .filter((v): v is number => v !== null);
  const lows = recent
    .map((s) => (s.before.low1 && s.after.low1 ? (s.after.low1 - s.before.low1) / s.before.low1 : null))
    .filter((v): v is number => v !== null);
  const avg = (v: number[]) => (v.length ? v.reduce((a, b) => a + b, 0) / v.length : null);
  return { count: recent.length, stability: avg(changes), lows: avg(lows) };
}

export default function HistoryPage() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [open, setOpen] = useState<string | null>(null);
  const { session: latest } = useMeasure();
  const wrap = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(600);

  useEffect(() => {
    if (isTauri()) measureApi.listSessions().then(setSessions).catch(() => {});
  }, [latest]);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setWidth(Math.floor(e.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const w = weekly(sessions);
  const remove = async (id: string) => {
    await measureApi.deleteSession(id);
    setSessions((s) => s.filter((x) => x.id !== id));
  };

  return (
    <>
      <PageHeader title="Verlauf" description="Alle Vorher-/Nachher-Messungen." />
      <div className="grid grid-cols-12 gap-4">
        <Card className="col-span-12 lg:col-span-4" title="Diese Woche">
          {w.count === 0 ? (
            <p className="text-sm text-faint">Noch keine Messung in den letzten 7 Tagen. Starte eine unter „Boost → Was bringt der Boost?“.</p>
          ) : (
            <div className="space-y-4">
              <div>
                <div className="text-4xl font-semibold tracking-tight">
                  {w.stability == null ? "—" : `${w.stability > 0 ? "" : "−"}${num(Math.abs(w.stability * 100))} %`}
                </div>
                <p className="mt-1 text-sm text-muted">
                  {w.stability == null
                    ? "Zu wenige Frametime-Daten."
                    : w.stability > 0.05
                      ? "gleichmäßigere Frametimes mit Boost (weniger Schwankung)"
                      : w.stability < -0.05
                        ? "mehr Schwankung mit Boost – der Boost hilft hier nicht"
                        : "kein nennenswerter Unterschied bei der Gleichmäßigkeit"}
                </p>
              </div>
              <div className="text-sm text-muted">
                1 %-Lows im Schnitt: <span className="text-text">{w.lows == null ? "—" : `${w.lows >= 0 ? "+" : "−"}${num(Math.abs(w.lows * 100), 1)} %`}</span>
                <br />
                Messungen: <span className="text-text">{w.count}</span>
              </div>
            </div>
          )}
        </Card>

        <Card className="col-span-12 lg:col-span-8" title="1 %-Lows pro Messung">
          <div ref={wrap}>
            <SessionChart sessions={sessions} width={width} />
          </div>
        </Card>

        <Card className="col-span-12" title="Alle Messungen">
          {sessions.length === 0 ? (
            <p className="text-sm text-faint">{isTauri() ? "Noch keine Messungen." : "Nur in der App verfügbar."}</p>
          ) : (
            <ul className="divide-y divide-line">
              {[...sessions].reverse().map((s) => {
                const rows = compare(s.before, s.after);
                const better = rows.filter((r) => r.verdict === "better").length;
                const worse = rows.filter((r) => r.verdict === "worse").length;
                return (
                  <li key={s.id} className="py-3">
                    <div className="flex items-center justify-between gap-3">
                      <button className="min-w-0 flex-1 text-left" onClick={() => setOpen(open === s.id ? null : s.id)} aria-expanded={open === s.id}>
                        <div className="text-sm font-medium">{s.game ?? "Unbekanntes Spiel"}</div>
                        <div className="text-xs text-muted">
                          {new Date(s.createdAt * 1000).toLocaleString("de-DE")} · 1 %-Lows {num(s.before.low1) ?? "—"} → {num(s.after.low1) ?? "—"} FPS
                        </div>
                      </button>
                      <span className="flex shrink-0 items-center gap-2">
                        {better > 0 && <Badge tone="good">{better} besser</Badge>}
                        {worse > 0 && <Badge tone="bad">{worse} schlechter</Badge>}
                        {better === 0 && worse === 0 && <Badge>kein Unterschied</Badge>}
                        <Button variant="ghost" className="px-2" aria-label="Messung löschen" onClick={() => remove(s.id)}><Trash2 size={14} /></Button>
                      </span>
                    </div>
                    {open === s.id && (
                      <div className="mt-4 animate-fade-in">
                        <ComparisonTable before={s.before} after={s.after} />
                        {s.tweaks.length > 0 && <p className="mt-2 text-xs text-faint">Aktive Tweaks: {s.tweaks.join(", ")}</p>}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
      </div>
    </>
  );
}
