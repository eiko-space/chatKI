import { useEffect, useState } from "react";
import { Gauge, HardDrive, Layers, Loader2, RotateCcw, Settings2, Signal, Trash2, Wrench } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { labApi, type ConfigGame, type FpsAdvice, type InstalledGame, type LagResult, type OverlayApp, type ShaderCache } from "../lib/api";
import { isTauri } from "../lib/tauri";
import { num } from "../lib/format";

function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  useEffect(() => {
    if (isTauri()) fn().then(setData).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return [data, setData] as const;
}

/* ------------------------------- Lag-Test ------------------------------- */
function LagTestCard() {
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ percent: number; step: string } | null>(null);
  const [result, setResult] = useState<LagResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<{ percent: number; step: string }>("lagtest-progress", ({ payload }) => setProgress(payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const run = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    setProgress({ percent: 0, step: "Starte …" });
    try {
      setResult(await labApi.lagTest());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  const gradeColor = (g: string) => (g.startsWith("A") ? "text-good" : g === "B" ? "text-good" : g === "C" ? "text-accent" : "text-bad");

  return (
    <Card
      title="Lag-Test fürs Heimnetz"
      subtitle="Bleibt dein Ping stabil, wenn die Leitung ausgelastet ist?"
      action={<Button variant="primary" onClick={run} disabled={busy || !isTauri()}>{busy ? <Loader2 size={14} className="animate-spin" /> : <Signal size={14} />} {busy ? "Teste …" : "Test starten"}</Button>}
    >
      {busy && progress && (
        <div>
          <div className="flex justify-between text-sm"><span>{progress.step}</span><span className="tabular-nums text-muted">{progress.percent} %</span></div>
          <div className="mt-2 h-2 overflow-hidden rounded-full bg-surface-3"><div className="h-full rounded-full bg-accent transition-[width] duration-300" style={{ width: `${progress.percent}%` }} /></div>
          <p className="mt-2 text-xs text-faint">Dauert ca. 20 Sekunden und lastet kurz deine Leitung aus.</p>
        </div>
      )}
      {error && <p className="text-sm text-bad">{error}</p>}
      {result && !busy && (
        <div className="space-y-4">
          <div className="flex items-center gap-4">
            <div className={`text-5xl font-semibold ${gradeColor(result.grade)}`}>{result.grade}</div>
            <p className="text-sm text-muted">{result.verdict}</p>
          </div>
          <div className="grid grid-cols-3 gap-3 text-center">
            <div className="rounded-xl bg-surface-2 p-3"><div className="text-xs text-muted">Ruhe</div><div className="text-lg font-semibold tabular-nums">{num(result.idleMs)} ms</div></div>
            <div className="rounded-xl bg-surface-2 p-3"><div className="text-xs text-muted">bei Download</div><div className="text-lg font-semibold tabular-nums">{num(result.downloadLoadMs)} ms</div></div>
            <div className="rounded-xl bg-surface-2 p-3"><div className="text-xs text-muted">bei Upload</div><div className="text-lg font-semibold tabular-nums">{num(result.uploadLoadMs)} ms</div></div>
          </div>
          {result.addedMs != null && (
            <p className="text-sm">Unter Last steigt dein Ping um <b className={gradeColor(result.grade)}>+{num(result.addedMs)} ms</b>.</p>
          )}
          {result.tips.length > 0 && (
            <div className="rounded-xl bg-surface-2 p-4">
              <div className="text-xs font-semibold text-text">Das hilft</div>
              <ul className="mt-1.5 list-disc space-y-1 pl-4 text-xs text-muted">{result.tips.map((t) => <li key={t}>{t}</li>)}</ul>
            </div>
          )}
        </div>
      )}
      {!result && !busy && !error && <p className="text-sm text-faint">Der häufigste Grund für plötzliche Lags: Jemand im Haushalt lädt etwas herunter. Dieser Test findet das heraus.</p>}
    </Card>
  );
}

/* ----------------------------- FPS-Limit -------------------------------- */
function FpsAdviceCard() {
  const [advice, setAdvice] = useState<FpsAdvice | null>(null);
  const load = () => labApi.fpsAdvice().then(setAdvice).catch(() => {});
  useEffect(() => {
    if (isTauri()) load();
  }, []);
  return (
    <Card title="FPS-Limit-Finder" subtitle="Das richtige Limit senkt den Input-Delay" action={<Button onClick={load} disabled={!isTauri()}><Gauge size={14} /> Analysieren</Button>}>
      {!advice ? (
        <p className="text-sm text-faint">Starte die FPS-Messung und spiele kurz, dann auf „Analysieren“.</p>
      ) : (
        <div className="space-y-3">
          <div className="flex items-start gap-3">
            {advice.recommendedCap != null && <div className="shrink-0 rounded-xl bg-accent-dim px-4 py-2 text-center"><div className="text-xs text-accent">Limit</div><div className="text-2xl font-semibold text-accent tabular-nums">{advice.recommendedCap}</div></div>}
            <div>
              <div className="text-sm font-medium">{advice.headline}</div>
              <p className="mt-1 text-xs text-muted">{advice.detail}</p>
            </div>
          </div>
          {advice.how.length > 0 && <ul className="list-disc space-y-1 pl-5 text-xs text-muted">{advice.how.map((h) => <li key={h}>{h}</li>)}</ul>}
        </div>
      )}
    </Card>
  );
}

/* --------------------------- Spiel-Config ------------------------------- */
function GameConfigCard() {
  const [games, setGames] = useAsync<ConfigGame[]>(labApi.gameConfigs);
  const [msg, setMsg] = useState<Record<string, string>>({});
  const act = async (id: string, apply: boolean) => {
    try {
      const m = apply ? await labApi.applyConfig(id) : await labApi.restoreConfig(id);
      setMsg((x) => ({ ...x, [id]: m }));
    } catch (e) {
      setMsg((x) => ({ ...x, [id]: String(e) }));
    }
    labApi.gameConfigs().then(setGames).catch(() => {});
  };
  return (
    <Card title="Spiel-Einstellungen optimieren" subtitle="Beste Wettkampf-Einstellungen per Klick – mit Backup" action={<Settings2 size={16} className="text-faint" />}>
      {!games || games.length === 0 ? (
        <p className="text-sm text-faint">{isTauri() ? "Lade …" : "Nur in der App verfügbar."}</p>
      ) : (
        <ul className="space-y-3">
          {games.map((g) => (
            <li key={g.id} className="rounded-xl bg-surface-2 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium">{g.name} {g.optimized && <Badge tone="good">aktiv</Badge>}</div>
                  <p className="text-xs text-muted">{g.summary}</p>
                </div>
                {!g.available ? (
                  <span className="text-xs text-faint">Spiel einmal starten</span>
                ) : g.optimized ? (
                  <Button onClick={() => act(g.id, false)}><RotateCcw size={13} /> Zurücksetzen</Button>
                ) : (
                  <Button variant="primary" onClick={() => act(g.id, true)}>Optimieren</Button>
                )}
              </div>
              <ul className="mt-2 list-disc pl-5 text-[11px] text-faint">{g.changes.map((c) => <li key={c}>{c}</li>)}</ul>
              {msg[g.id] && <p className="mt-2 text-xs text-muted">{msg[g.id]}</p>}
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

/* ---------------------- Festplatte / Overlay / Shader ------------------- */
function SystemToolsCard() {
  const [storage] = useAsync<InstalledGame[]>(labApi.gamesStorage);
  const [overlays] = useAsync<OverlayApp[]>(labApi.overlays);
  const [caches, setCaches] = useAsync<ShaderCache[]>(labApi.shaderCaches);
  const [shaderMsg, setShaderMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const clearShaders = async () => {
    setBusy(true);
    try {
      setShaderMsg(await labApi.clearShaders());
    } catch (e) {
      setShaderMsg(String(e));
    } finally {
      setBusy(false);
      labApi.shaderCaches().then(setCaches).catch(() => {});
    }
  };
  const totalMb = (caches ?? []).reduce((s, c) => s + c.sizeMb, 0);
  const hddGames = (storage ?? []).filter((g) => g.onHdd === true);

  return (
    <Card title="System-Werkzeuge" subtitle="Festplatte, Overlays und Shader-Cache" action={<Wrench size={16} className="text-faint" />}>
      <div className="space-y-5">
        <div>
          <div className="flex items-center gap-1.5 text-xs font-semibold text-text"><HardDrive size={13} /> Laufen deine Spiele auf einer SSD?</div>
          {!storage || storage.length === 0 ? (
            <p className="mt-1 text-xs text-faint">Kein laufendes Spiel erkannt. Starte ein Spiel und öffne diese Seite erneut.</p>
          ) : (
            <ul className="mt-2 space-y-1 text-sm">
              {storage.map((g) => (
                <li key={g.exe} className="flex items-center justify-between gap-2">
                  <span>{g.name} <span className="text-faint">({g.drive})</span></span>
                  {g.onHdd === true ? <Badge tone="bad">langsame HDD</Badge> : g.onHdd === false ? <Badge tone="good">SSD</Badge> : <Badge>unbekannt</Badge>}
                </li>
              ))}
            </ul>
          )}
          {hddGames.length > 0 && <p className="mt-2 text-xs text-bad">Verschiebe diese Spiele auf eine SSD – das behebt Nachlade-Ruckler, die kein Tweak beheben kann (im Launcher: Spiel → verschieben).</p>}
        </div>

        <div className="border-t border-line pt-4">
          <div className="flex items-center gap-1.5 text-xs font-semibold text-text"><Layers size={13} /> Aktive Overlays</div>
          {!overlays || overlays.length === 0 ? (
            <p className="mt-1 text-xs text-faint">Keine bekannten Overlays aktiv – gut.</p>
          ) : (
            <ul className="mt-2 space-y-1.5 text-sm">
              {overlays.map((o) => (
                <li key={o.exe}>
                  <span className="font-medium">{o.name}</span>
                  <span className="block text-[11px] text-muted">{o.how}</span>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="border-t border-line pt-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-1.5 text-xs font-semibold text-text"><Trash2 size={13} /> Shader-Cache</div>
              <p className="mt-0.5 text-xs text-muted">{totalMb > 0 ? `${num(totalMb)} MB belegt. Bei Rucklern nach Treiber-Update leeren.` : "Kein großer Cache gefunden."}</p>
            </div>
            <Button onClick={clearShaders} disabled={busy || totalMb < 1}>{busy ? <Loader2 size={14} className="animate-spin" /> : <Trash2 size={14} />} Leeren</Button>
          </div>
          {shaderMsg && <p className="mt-2 text-xs text-muted">{shaderMsg}</p>}
        </div>
      </div>
    </Card>
  );
}

export default function Tools() {
  return (
    <>
      <PageHeader title="Werkzeuge" description="Spezial-Analysen, die die echte Ursache von Lags und Rucklern finden." />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-12 lg:col-span-6"><LagTestCard /></div>
        <div className="col-span-12 lg:col-span-6"><FpsAdviceCard /></div>
        <div className="col-span-12 lg:col-span-6"><GameConfigCard /></div>
        <div className="col-span-12 lg:col-span-6"><SystemToolsCard /></div>
      </div>
    </>
  );
}
