import { useEffect, useState } from "react";
import { Lock, Mic, ShieldCheck } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Badge } from "../components/ui/Badge";
import { Toggle } from "../components/ui/Toggle";
import { Details } from "../components/ui/Details";
import { ResultList } from "../components/ui/ResultList";
import { BoostControl } from "../components/BoostControl";
import { MeasureWizard } from "../components/MeasureWizard";
import { useAppState } from "../lib/app-state";
import { api, type LogEntry, type ProtectedApp, type TweakInfo } from "../lib/api";
import { isTauri, type AppInfo } from "../lib/tauri";

export default function Boost({ info }: { info: AppInfo | null }) {
  const { catalog, settings, boost, lastRevert, updateSettings } = useAppState();
  const [protectedApps, setProtectedApps] = useState<ProtectedApp[]>([]);
  const [log, setLog] = useState<LogEntry[]>([]);

  useEffect(() => {
    if (!isTauri()) return;
    const load = () => {
      api.protectedRunning().then(setProtectedApps).catch(() => {});
      api.changeLog().then(setLog).catch(() => {});
    };
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, [boost?.active]);

  const toggle = (id: string, on: boolean) =>
    updateSettings((s) => ({ ...s, enabledTweaks: on ? [...s.enabledTweaks, id] : s.enabledTweaks.filter((t) => t !== id) }));

  const group = (cat: TweakInfo["category"]) => catalog.filter((t) => t.category === cat);

  return (
    <>
      <PageHeader title="Boost" description="Alles einzeln schaltbar und komplett rückgängig machbar." />

      <div className="grid grid-cols-12 gap-4">
        <Card className="col-span-12 flex flex-col items-center justify-center py-8 lg:col-span-5">
          <BoostControl info={info} />
        </Card>

        <Card
          className="col-span-12 lg:col-span-7"
          title={boost?.active ? "Aktive Änderungen" : lastRevert ? "Zurückgesetzt" : "Ergebnis"}
          subtitle={boost?.active && boost.startedAt ? `seit ${new Date(boost.startedAt * 1000).toLocaleTimeString("de-DE")}` : undefined}
          action={boost?.active ? <Badge tone="accent">Aktiv</Badge> : undefined}
        >
          {boost?.active && boost.results.length > 0 ? (
            <ResultList results={boost.results} />
          ) : lastRevert && lastRevert.length > 0 ? (
            <ResultList results={lastRevert} />
          ) : (
            <p className="text-sm text-faint">Noch kein Boost aktiv.</p>
          )}

        </Card>

        <div className="col-span-12">
          <MeasureWizard />
        </div>

        {(["fps", "network"] as const).map((cat) => (
          <Card key={cat} className="col-span-12 lg:col-span-6" title={cat === "fps" ? "FPS & Input Delay" : "Netzwerk"}>
            <ul className="divide-y divide-line">
              {group(cat).map((t) => {
                const locked = t.needsAdmin && !info?.isElevated;
                return (
                  <li key={t.id} className="py-3 first:pt-0 last:pb-0">
                    <div className="flex items-start justify-between gap-4">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2 text-sm font-medium">
                          {t.title}
                          {t.needsAdmin && <Badge tone={locked ? "neutral" : "accent"}><Lock size={10} /> Admin</Badge>}
                          {t.oneShot && <Badge>einmalig</Badge>}
                        </div>
                        <p className="mt-0.5 text-xs text-muted">{t.summary}</p>
                      </div>
                      <Toggle
                        label={t.title}
                        checked={settings?.enabledTweaks.includes(t.id) ?? false}
                        disabled={!settings || boost?.active}
                        onChange={(v) => toggle(t.id, v)}
                      />
                    </div>
                  </li>
                );
              })}
            </ul>
          </Card>
        ))}

        <Card className="col-span-12 lg:col-span-6" title="Geschützt" subtitle="Wird nie beendet" action={<ShieldCheck size={16} className="text-good" />}>
          {protectedApps.length === 0 ? (
            <p className="text-sm text-faint">{isTauri() ? "Gerade läuft nichts aus der Schutzliste." : "Nur in der App verfügbar."}</p>
          ) : (
            <ul className="grid grid-cols-1 gap-x-6 gap-y-1.5 sm:grid-cols-2">
              {protectedApps.map((p) => (
                <li key={p.exe} className="flex items-center justify-between gap-2 text-sm">
                  <span className="truncate">{p.exe}</span>
                  <span className={`flex shrink-0 items-center gap-1 text-[11px] ${p.reason.startsWith("Mikrofon") ? "text-accent" : "text-faint"}`}>
                    {p.reason.startsWith("Mikrofon") && <Mic size={11} />} {p.reason}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <Details label="Wie funktioniert der Schutz?">
            Fest geschützt sind Windows-Systemprozesse, Virenschutz, Anti-Cheat, Voice-Chat & Anrufe (Discord, TeamSpeak, Teams, Zoom …),
            Audio-/Streaming-Software, Maus-/Headset-Software und Spiele-Launcher. Zusätzlich prüft GameBoost bei jedem Boost, welche
            Programme gerade dein Mikrofon benutzen (dieselbe Info, die Windows für das Mikrofon-Symbol nutzt) – die bleiben immer an.
          </Details>
        </Card>

        <Card className="col-span-12 lg:col-span-6" title="Änderungsprotokoll" >
          {log.length === 0 ? (
            <p className="text-sm text-faint">Noch keine Änderungen.</p>
          ) : (
            <ul className="max-h-72 space-y-2 overflow-y-auto pr-1 text-xs">
              {log.map((e, i) => (
                <li key={i} className="flex gap-3">
                  <span className="shrink-0 tabular-nums text-faint">{new Date(e.time * 1000).toLocaleString("de-DE", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" })}</span>
                  <span className="min-w-0">
                    <span className={e.status === "failed" ? "text-bad" : "text-text"}>{e.title}</span>
                    {e.message && <span className="text-muted"> – {e.message}</span>}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  );
}
