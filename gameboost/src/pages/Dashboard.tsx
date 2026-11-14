import { useState } from "react";
import { BoostControl } from "../components/BoostControl";
import { BoostEffect } from "../components/BoostEffect";
import { StutterLog } from "../components/StutterLog";
import { AlertTriangle, Cpu, Gamepad2, Loader2, Play, ShieldCheck, Square, Wifi, WifiOff } from "lucide-react";
import { Card } from "../components/ui/Card";
import { Metric } from "../components/ui/Metric";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Dialog } from "../components/ui/Dialog";
import { PageHeader } from "../components/ui/PageHeader";
import { UsageBar } from "../components/ui/UsageBar";
import { FrametimeChart } from "../components/ui/FrametimeChart";
import { isTauri, restartAsAdmin, startFpsCapture, stopFpsCapture, type AppInfo } from "../lib/tauri";
import { useTelemetry, type PingStats, type Snapshot } from "../lib/telemetry";
import { num } from "../lib/format";

export default function Dashboard({ info }: { info: AppInfo | null }) {
  const { snapshot } = useTelemetry();
  const fps = snapshot?.fps ?? null;

  return (
    <>
      <PageHeader
        title="Dashboard"
        description="Live-Werte deines PCs – nur echte Messungen."
        action={
          fps ? (
            <Badge tone="accent"><Gamepad2 size={11} /> {fps.game ?? fps.app}</Badge>
          ) : (
            <Badge><Gamepad2 size={11} /> Kein Spiel erkannt</Badge>
          )
        }
      />

      <div className="grid grid-cols-12 gap-4">
        <Card className="col-span-12 flex flex-col items-center justify-center py-8 xl:col-span-4">
          <BoostControl info={info} />
        </Card>

        <FpsCard info={info} snapshot={snapshot} />
        <BoostEffect />
        <NetworkCard snapshot={snapshot} />
        <HardwareCard snapshot={snapshot} />
        <StutterLog />
      </div>
    </>
  );
}

/* ---------------------------------- FPS ---------------------------------- */

function FpsCard({ info, snapshot }: { info: AppInfo | null; snapshot: Snapshot | null }) {
  const { frametimes } = useTelemetry();
  const [dialog, setDialog] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const capture = snapshot?.capture;
  const fps = snapshot?.fps ?? null;
  const running = capture?.state === "running";

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onStart = () => (info?.isElevated ? run(startFpsCapture) : setDialog(true));

  let action = null;
  if (isTauri() && capture && capture.state !== "missingBinary") {
    action = running ? (
      <Button variant="ghost" onClick={() => run(stopFpsCapture)} disabled={busy}>
        <Square size={14} /> Stoppen
      </Button>
    ) : (
      <Button variant="primary" onClick={onStart} disabled={busy}>
        <Play size={14} /> FPS messen
      </Button>
    );
  }

  return (
    <Card className="col-span-12 xl:col-span-8" title="Bildrate (FPS)" subtitle={fps ? `${fps.game ?? fps.app} · letzte ${num(fps.windowSecs)} s` : "Gemessen mit Intel PresentMon"} action={action}>
      <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
        <Metric label="Aktuell" value={num(fps?.current)} unit="FPS" size="lg" />
        <Metric label="Durchschnitt" value={num(fps?.average)} unit="FPS" />
        <Metric label="1 %-Lows" value={num(fps?.low1)} unit="FPS" hint={fps && fps.low1 === null ? "ab 100 Bildern" : undefined} />
        <Metric label="0,1 %-Lows" value={num(fps?.low01)} unit="FPS" hint={fps && fps.low01 === null ? "ab 1000 Bildern" : undefined} />
      </div>

      <div className="mt-5 rounded-xl border border-line bg-surface-2/40 px-2 pt-2">
        {fps && frametimes.length > 1 ? (
          <FrametimeChart data={frametimes} />
        ) : (
          <div className="grid h-[140px] place-items-center px-4 text-center text-xs text-faint">
            <FpsEmptyState snapshot={snapshot} info={info} />
          </div>
        )}
      </div>

      {fps && (
        <div className="mt-4 grid grid-cols-2 gap-6 sm:grid-cols-4">
          <Metric label="Frametime Ø" value={num(fps.frametimeAvgMs, 1)} unit="ms" />
          <Metric label="Schwankung" value={num(fps.frametimeStdDevMs, 1)} unit="ms"  />
          <Metric
            label="Input-Delay"
            value={num(fps.inputLatencyMs, 1)}
            unit="ms"
            hint={fps.inputLatencyMs === null ? "Maus/Tastatur benutzen" : undefined}
          />
          <Metric
            label="PC-Latenz (Reflex)"
            value={num(fps.pcLatencyMs, 1)}
            unit="ms"
            hint={fps.pcLatencyMs === null ? "nur mit Reflex" : undefined}
          />
        </div>
      )}

      {(error || capture?.state === "error") && (
        <p className="mt-4 flex items-start gap-2 text-xs text-bad">
          <AlertTriangle size={14} className="mt-px shrink-0" /> {error ?? capture?.message}
        </p>
      )}


      <Dialog
        open={dialog}
        title="Admin-Rechte für die FPS-Messung"
        onClose={() => setDialog(false)}
        actions={
          info?.isDev ? (
            <Button variant="primary" onClick={() => setDialog(false)}>Verstanden</Button>
          ) : (
            <>
              <Button variant="ghost" onClick={() => setDialog(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={() => { setDialog(false); run(restartAsAdmin); }}>
                <ShieldCheck size={14} /> Mit Admin-Rechten neu starten
              </Button>
            </>
          )
        }
      >
        <p>
          Windows erlaubt das Mitlesen der Bildausgabe (ETW) nur mit Administratorrechten. GameBoost liest dabei nur mit –
          am Spiel und an deinem System wird nichts verändert.
        </p>
        {info?.isDev ? (
          <p>
            <b className="text-text">Entwicklungsmodus:</b> Schließe die App und das schwarze Fenster. Mach dann im Ordner
            „gameboost“ einen <b className="text-text">Rechtsklick auf „GameBoost-Start.bat“</b> und wähle{" "}
            <b className="text-text">„Als Administrator ausführen“</b>.
          </p>
        ) : (
          <p>GameBoost startet neu und Windows fragt dich einmal, ob du das erlaubst.</p>
        )}
      </Dialog>
    </Card>
  );
}

function FpsEmptyState({ snapshot, info }: { snapshot: Snapshot | null; info: AppInfo | null }) {
  if (!isTauri()) return <>Nur in der App verfügbar – die Oberfläche läuft gerade im Browser.</>;
  if (!snapshot) return <span className="inline-flex items-center gap-2"><Loader2 size={14} className="animate-spin" /> Verbinde …</span>;
  switch (snapshot.capture.state) {
    case "missingBinary":
      return <>PresentMon fehlt. Führe im Ordner „gameboost“ einmal <code className="text-muted">npm run fetch-presentmon</code> aus und starte die App neu.</>;
    case "running":
      return <span className="inline-flex items-center gap-2"><Loader2 size={14} className="animate-spin" /> Warte auf ein Spiel … Starte ein Spiel, dann erscheinen hier die Frametimes.</span>;
    case "error":
      return <>Messung gestoppt.</>;
    default:
      return info?.isElevated
        ? <>Klicke auf „FPS messen“, um die Messung zu starten.</>
        : <>Die FPS-Messung braucht einmalig Admin-Rechte. Klicke auf „FPS messen“.</>;
  }
}

/* -------------------------------- Netzwerk -------------------------------- */

function NetworkCard({ snapshot }: { snapshot: Snapshot | null }) {
  const net = snapshot?.network;
  const inet = net?.internet;
  const wlan = net?.connection === "wlan";

  return (
    <Card
      className="col-span-12 lg:col-span-6"
      title="Verbindung"
      subtitle="Referenz-Server 1.1.1.1"
      action={
        net?.connection ? (
          <Badge tone={wlan ? "bad" : "good"}>{wlan ? <WifiOff size={11} /> : <Wifi size={11} />} {wlan ? "WLAN" : net.connection === "lan" ? "LAN-Kabel" : "Andere"}</Badge>
        ) : undefined
      }
    >
      <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
        <Metric label="Ping" value={num(inet?.lastMs)} unit="ms" size="lg" />
        <Metric label="Jitter" value={num(inet?.jitterMs, 1)} unit="ms" />
        <Metric label="Paketverlust" value={num(inet?.lossPercent, 1)} unit="%" tone={(inet?.lossPercent ?? 0) > 0 ? "bad" : "neutral"} />
        <Metric
          label="Spikes (60 s)"
          value={inet?.spikes ?? null}
          tone={(inet?.spikes ?? 0) > 0 ? "bad" : "neutral"}
          hint={inet?.spikeThresholdMs ? `über ${num(inet.spikeThresholdMs)} ms` : undefined}
        />
      </div>

      <RouterRow router={net?.router} />

      {wlan && (
        <p className="mt-4 flex items-start gap-2 text-xs text-muted">
          <AlertTriangle size={14} className="mt-px shrink-0 text-bad" />
          WLAN – ein LAN-Kabel senkt Ping-Spitzen meist deutlich.
        </p>
      )}

    </Card>
  );
}

function RouterRow({ router }: { router?: PingStats }) {
  return (
    <div className="mt-5 flex items-center justify-between rounded-xl bg-surface-2 px-4 py-3 text-sm">
      <span className="text-muted">Router {router?.target ? <span className="text-faint">({router.target})</span> : null}</span>
      <span className="flex gap-5">
        <span><span className="text-faint">Ping </span>{router?.lastMs != null ? `${num(router.lastMs)} ms` : "—"}</span>
        <span><span className="text-faint">Jitter </span>{router?.jitterMs != null ? `${num(router.jitterMs, 1)} ms` : "—"}</span>
        <span><span className="text-faint">Verlust </span>{router?.lossPercent != null ? `${num(router.lossPercent, 1)} %` : "—"}</span>
      </span>
    </div>
  );
}

/* -------------------------------- Hardware -------------------------------- */

function HardwareCard({ snapshot }: { snapshot: Snapshot | null }) {
  const s = snapshot?.system;
  const g = snapshot?.gpu;
  const net = snapshot?.network;
  const gb = (used?: number | null, total?: number | null) =>
    used != null && total != null ? `${num(used, 1)} / ${num(total, 1)} GB` : null;

  return (
    <Card className="col-span-12 lg:col-span-6" title="Hardware" subtitle={g?.name ?? undefined} action={<Cpu size={16} className="text-faint" />}>
      <div className="grid grid-cols-2 gap-x-6 gap-y-4">
        <UsageBar label="CPU" value={s?.cpuUsage ?? null} />
        <UsageBar label="GPU" value={g?.usage ?? null} />
        <UsageBar label="RAM" value={s?.ramUsage ?? null} detail={gb(s?.ramUsedGb, s?.ramTotalGb)} />
        <UsageBar label="VRAM" value={g?.vramUsage ?? null} detail={gb(g?.vramUsedGb, g?.vramTotalGb)} />
      </div>
      <div className="mt-5 grid grid-cols-4 gap-4 border-t border-line pt-4">
        <Metric label="CPU-Temp." value={num(snapshot?.cpuTemperature)} unit="°C" hint="nicht verfügbar" />
        <Metric label="GPU-Temp." value={num(g?.temperature)} unit="°C" tone={(g?.temperature ?? 0) >= 85 ? "bad" : "neutral"} />
        <Metric label="Download" value={num(net?.downloadMbps, 1)} unit="Mbit/s" />
        <Metric label="Upload" value={num(net?.uploadMbps, 1)} unit="Mbit/s" />
      </div>
    </Card>
  );
}
