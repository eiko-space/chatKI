import { useEffect, useState } from "react";
import { Activity, Cpu, HardDrive, Wifi } from "lucide-react";
import { Card } from "./ui/Card";
import { Button } from "./ui/Button";
import { detectiveApi, type StutterEvent } from "../lib/api";
import { isTauri } from "../lib/tauri";
import { num } from "../lib/format";

/** Ruckler-Detektiv: zeigt, was bei jedem Ruckler/jeder Ping-Spitze los war. */
export function StutterLog() {
  const [events, setEvents] = useState<StutterEvent[]>([]);
  useEffect(() => {
    if (!isTauri()) return;
    const load = () => detectiveApi.events().then(setEvents).catch(() => {});
    load();
    const t = setInterval(load, 3000);
    return () => clearInterval(t);
  }, []);

  return (
    <Card
      className="col-span-12"
      title="Ruckler-Detektiv"
      subtitle="Findet die Ursache von Rucklern und Ping-Spitzen – im Moment, in dem sie passieren"
      action={events.length > 0 ? <Button variant="ghost" onClick={() => detectiveApi.clear().then(() => setEvents([]))}>Leeren</Button> : <Activity size={16} className="text-faint" />}
    >
      {events.length === 0 ? (
        <p className="text-sm text-faint">
          {isTauri() ? "Noch nichts aufgefallen. Spiele mit aktiver FPS-Messung – sobald es ruckelt oder der Ping springt, erscheint hier die Ursache." : "Nur in der App verfügbar."}
        </p>
      ) : (
        <ul className="space-y-2">
          {events.slice(0, 15).map((e, i) => (
            <li key={i} className="flex items-start gap-3 rounded-xl bg-surface-2 p-3 animate-pop-in" style={{ animationDelay: `${i * 40}ms` }}>
              <span className={`mt-0.5 shrink-0 ${e.kind === "ping" ? "text-accent" : "text-bad"}`}>{e.kind === "ping" ? <Wifi size={16} /> : <Cpu size={16} />}</span>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-baseline justify-between gap-x-3">
                  <span className="text-sm font-medium">{e.kind === "ping" ? "Ping-Spitze" : "Ruckler"} {e.game && <span className="text-faint">· {e.game}</span>}</span>
                  <span className="text-xs tabular-nums text-muted">
                    {num(e.valueMs)} ms{e.normalMs != null && <span className="text-faint"> (normal {num(e.normalMs)} ms)</span>} · {new Date(e.time * 1000).toLocaleTimeString("de-DE")}
                  </span>
                </div>
                <p className="mt-0.5 text-xs text-muted">{e.cause}</p>
                {e.context.top.length > 0 && (
                  <div className="mt-1 flex flex-wrap gap-1.5">
                    {e.context.top.slice(0, 3).map((p) => (
                      <span key={p.exe} className="rounded bg-surface-3 px-1.5 py-0.5 text-[10px] text-faint">{p.exe} {num(p.cpu)}%</span>
                    ))}
                    {e.context.disk != null && e.context.disk >= 50 && (
                      <span className="inline-flex items-center gap-1 rounded bg-surface-3 px-1.5 py-0.5 text-[10px] text-faint"><HardDrive size={9} /> Disk {num(e.context.disk)}%</span>
                    )}
                  </div>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
