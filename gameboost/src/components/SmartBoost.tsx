import { useEffect, useState } from "react";
import { Brain, Check, Minus, Play, SkipForward, Square, X } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { Card } from "./ui/Card";
import { Button } from "./ui/Button";
import { ComparisonTable } from "./ComparisonTable";
import { smartApi, type SmartState } from "../lib/api";
import { useTelemetry } from "../lib/telemetry";
import { isTauri } from "../lib/tauri";

const VERDICT = {
  better: { icon: Check, cls: "text-good", label: "hilft – bleibt an" },
  neutral: { icon: Minus, cls: "text-muted", label: "kein Unterschied – bleibt an" },
  worse: { icon: X, cls: "text-bad", label: "schadet – entfernt" },
  skipped: { icon: SkipForward, cls: "text-faint", label: "übersprungen" },
} as const;

/** Smart-Boost: testet jeden Tweak einzeln und behält nur, was auf diesem PC hilft. */
export function SmartBoost() {
  const [state, setState] = useState<SmartState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { snapshot } = useTelemetry();
  const ready = isTauri() && snapshot?.capture.state === "running" && !!snapshot.fps;

  useEffect(() => {
    if (!isTauri()) return;
    smartApi.state().then(setState).catch(() => {});
    const un = listen<SmartState>("smartboost", ({ payload }) => setState(payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const start = async () => {
    setError(null);
    try {
      await smartApi.start();
    } catch (e) {
      setError(String(e));
    }
  };

  const running = state?.running ?? false;
  const mins = state ? Math.ceil(state.remainingSecs / 60) : 0;

  return (
    <Card
      title="Smart-Boost"
      subtitle="Findet die beste Kombination für deinen PC – Tweak für Tweak gemessen"
      action={<Brain size={16} className="text-accent" />}
    >
      {!running && !state?.finished && (
        <div className="space-y-3">
          <p className="text-sm text-muted">
            Spiel ca. 6 Minuten ganz normal (z. B. Fortnite). GameBoost schaltet jeden Tweak einzeln zu und misst nach.
            Was schadet, fliegt raus. Das Ergebnis wird als dein persönlicher Boost für dieses Spiel gespeichert.
          </p>
          {!ready && <p className="text-xs text-faint">{!isTauri() ? "Nur in der App verfügbar." : "Starte zuerst ein Spiel und die FPS-Messung (Admin-Rechte nötig)."}</p>}
          {(error || state?.error) && <p className="text-xs text-bad">{error ?? state?.error}</p>}
          <Button variant="primary" onClick={start} disabled={!ready}>
            <Play size={14} /> Smart-Boost starten
          </Button>
        </div>
      )}

      {running && state && (
        <div className="space-y-3">
          <div className="flex justify-between text-sm">
            <span className="min-w-0 truncate">{state.step}</span>
            <span className="shrink-0 pl-3 tabular-nums text-muted">{state.percent} % · noch ca. {mins} min</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-surface-3">
            <div className="h-full rounded-full bg-accent transition-[width] duration-500" style={{ width: `${state.percent}%` }} />
          </div>
          <Button variant="ghost" onClick={() => smartApi.cancel()}>
            <Square size={14} /> Abbrechen (setzt alles zurück)
          </Button>
        </div>
      )}

      {state && state.results.length > 0 && (
        <ul className="mt-4 divide-y divide-line">
          {state.results.map((r, i) => {
            const v = VERDICT[r.verdict];
            const Icon = v.icon;
            return (
              <li key={r.id} className="flex gap-3 py-2 animate-pop-in" style={{ animationDelay: `${i * 30}ms` }}>
                <Icon size={16} className={`mt-0.5 shrink-0 ${v.cls}`} />
                <div className="min-w-0">
                  <div className="text-sm">
                    {r.title} <span className={`text-xs ${v.cls}`}>· {v.label}</span>
                  </div>
                  <div className="truncate text-[11px] text-faint">{r.detail}</div>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {state?.finished && state.before && state.after && (
        <div className="mt-5 space-y-4 border-t border-line pt-4">
          <p className="text-sm font-medium text-good">Fertig – dein persönlicher Boost ist aktiv und für {state.game ?? "dieses Spiel"} gespeichert.</p>
          <ComparisonTable before={state.before} after={state.after} />
          <Button onClick={start} disabled={!ready}><Play size={14} /> Erneut testen</Button>
        </div>
      )}
    </Card>
  );
}
