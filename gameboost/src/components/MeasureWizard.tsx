import { FlaskConical, Play, RotateCcw, X } from "lucide-react";
import { Card } from "./ui/Card";
import { Button } from "./ui/Button";
import { Details } from "./ui/Details";
import { ComparisonTable } from "./ComparisonTable";
import { useMeasure } from "../lib/measure";
import { useTelemetry } from "../lib/telemetry";
import { useAppState } from "../lib/app-state";
import { num } from "../lib/format";
import { isTauri } from "../lib/tauri";

function Progress({ label }: { label: string }) {
  const { snapshot } = useTelemetry();
  const m = snapshot?.measurement;
  const pct = m ? (m.elapsedSecs / m.durationSecs) * 100 : 0;
  return (
    <div>
      <div className="flex justify-between text-sm">
        <span>{label}</span>
        <span className="tabular-nums text-muted">{m ? `${num(m.durationSecs - m.elapsedSecs)} s` : "…"}</span>
      </div>
      <div className="mt-2 h-2 overflow-hidden rounded-full bg-surface-3">
        <div className="h-full rounded-full bg-accent transition-[width] duration-200" style={{ width: `${pct}%` }} />
      </div>
      <p className="mt-2 text-xs text-muted">Spiel einfach normal weiter – am besten immer an derselben Stelle bzw. im selben Modus.</p>
    </div>
  );
}

/** „Was hat der Boost gebracht?“ – geführte Vorher-/Nachher-Messung. */
export function MeasureWizard() {
  const m = useMeasure();
  const { snapshot } = useTelemetry();
  const { boost } = useAppState();
  const fps = snapshot?.fps;
  const capturing = snapshot?.capture.state === "running";
  const ready = isTauri() && capturing && !!fps;

  return (
    <Card title="Was bringt der Boost?" subtitle="Je 60 s ohne und mit Boost messen" action={<FlaskConical size={16} className="text-faint" />}>
      {m.error && <p className="mb-3 text-sm text-bad">{m.error}</p>}

      {m.step === "idle" && (
        <div className="space-y-3">
          <ol className="list-decimal space-y-1 pl-5 text-sm text-muted">
            <li>Spiel starten, 60 s ohne Boost messen</li>
            <li>Boost wird gestartet, dann 60 s erneut messen</li>
            <li>Du siehst, was sich wirklich verändert hat – auch wenn es nichts ist</li>
          </ol>
          {!ready && (
            <p className="text-xs text-faint">
              {!isTauri() ? "Nur in der App verfügbar." : !capturing ? "Starte zuerst die FPS-Messung im Dashboard." : "Starte ein Spiel – dann geht’s los."}
            </p>
          )}
          {boost?.active && <p className="text-xs text-faint">Der Boost ist gerade aktiv. Beende ihn, damit die „Vorher“-Messung ohne Boost läuft.</p>}
          <Button variant="primary" disabled={!ready || boost?.active} onClick={m.startBefore}>
            <Play size={14} /> Vorher messen{fps ? ` (${fps.game ?? fps.app})` : ""}
          </Button>
        </div>
      )}

      {m.step === "before" && (
        <div className="space-y-3">
          <Progress label="1/2 · Messe ohne Boost …" />
          <Button variant="ghost" onClick={m.cancel}><X size={14} /> Abbrechen</Button>
        </div>
      )}

      {m.step === "between" && (
        <div className="space-y-3">
          <p className="text-sm">
            Vorher-Messung fertig: <b>{num(m.before?.low1) ?? "—"}</b> FPS 1 %-Lows, <b>{num(m.before?.fpsAvg) ?? "—"}</b> FPS im Schnitt.
          </p>
          <Button variant="primary" disabled={!ready} onClick={m.startAfter}>
            <Play size={14} /> {boost?.active ? "Nachher messen" : "Boost starten & nachher messen"}
          </Button>
          <Button variant="ghost" onClick={m.reset}>Verwerfen</Button>
        </div>
      )}

      {m.step === "settling" && <p className="text-sm text-muted">Boost wird angewendet, Messung startet in wenigen Sekunden …</p>}

      {m.step === "after" && (
        <div className="space-y-3">
          <Progress label="2/2 · Messe mit Boost …" />
          <Button variant="ghost" onClick={m.cancel}><X size={14} /> Abbrechen</Button>
        </div>
      )}

      {m.step === "done" && m.before && m.after && (
        <div className="space-y-4">
          <ComparisonTable before={m.before} after={m.after} />
          <div className="flex items-center gap-2">
            <Button onClick={m.reset}><RotateCcw size={14} /> Neue Messung</Button>
            <span className="text-xs text-faint">Im Verlauf gespeichert.</span>
          </div>
        </div>
      )}

      <Details label="Warum kann das Ergebnis schwanken?">
        FPS hängen stark davon ab, was gerade im Bild passiert. Miss deshalb vorher und nachher möglichst an derselben Stelle
        (z. B. Übungsmodus, gleiche Map, gleiche Blickrichtung). Kleine Unterschiede werden als „gleich“ gewertet, damit
        Zufallsschwankungen nicht als Verbesserung verkauft werden.
      </Details>
    </Card>
  );
}
