import { useEffect, useState } from "react";
import { Loader2, Play, Route, Trophy, Wifi, WifiOff } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { Metric } from "../components/ui/Metric";
import { toolsApi, type NetApp, type RegionResult, type TraceResult } from "../lib/api";
import { useTelemetry } from "../lib/telemetry";
import { isTauri } from "../lib/tauri";
import { num } from "../lib/format";

const KIND: Record<string, { label: string; tone: "neutral" | "accent" | "good" | "bad" }> = {
  home: { label: "Heimnetz", tone: "neutral" },
  isp: { label: "Anbieter", tone: "neutral" },
  internet: { label: "Internet", tone: "neutral" },
  target: { label: "Ziel", tone: "accent" },
  unknown: { label: "keine Antwort", tone: "neutral" },
};

export default function Network() {
  const { snapshot } = useTelemetry();
  const net = snapshot?.network;
  const [regions, setRegions] = useState<RegionResult[] | null>(null);
  const [regionBusy, setRegionBusy] = useState(false);
  const [target, setTarget] = useState("1.1.1.1");
  const [trace, setTrace] = useState<TraceResult | null>(null);
  const [traceBusy, setTraceBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [apps, setApps] = useState<NetApp[]>([]);

  useEffect(() => {
    if (!isTauri()) return;
    const load = () => toolsApi.netApps().then(setApps).catch(() => {});
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  const runRegions = async () => {
    setRegionBusy(true);
    setError(null);
    try {
      setRegions(await toolsApi.regionPing());
    } catch (e) {
      setError(String(e));
    } finally {
      setRegionBusy(false);
    }
  };

  const runTrace = async () => {
    setTraceBusy(true);
    setError(null);
    setTrace(null);
    try {
      setTrace(await toolsApi.traceroute(target));
    } catch (e) {
      setError(String(e));
    } finally {
      setTraceBusy(false);
    }
  };

  const best = regions?.find((r) => r.medianMs != null);
  const maxMs = Math.max(1, ...(regions ?? []).map((r) => r.medianMs ?? 0));

  return (
    <>
      <PageHeader title="Netzwerk" description="Ping-Spitzen finden und verstehen." />
      {error && <p className="mb-4 text-sm text-bad">{error}</p>}
      <div className="grid grid-cols-12 gap-4">
        <Card
          className="col-span-12 lg:col-span-7"
          title="Live-Verbindung"
          subtitle="Referenz: Cloudflare 1.1.1.1"
          action={net?.connection ? <Badge tone={net.connection === "wlan" ? "bad" : "good"}>{net.connection === "wlan" ? <WifiOff size={11} /> : <Wifi size={11} />} {net.connection === "wlan" ? "WLAN" : "LAN"}</Badge> : undefined}
        >
          <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
            <Metric label="Ping" value={num(net?.internet.lastMs)} unit="ms" size="lg" />
            <Metric label="Jitter" value={num(net?.internet.jitterMs, 1)} unit="ms" />
            <Metric label="Verlust" value={num(net?.internet.lossPercent, 1)} unit="%" />
            <Metric label="Router-Ping" value={num(net?.router.lastMs)} unit="ms" />
          </div>
          {net?.connection === "wlan" && (
            <p className="mt-4 text-xs text-muted">WLAN – ein LAN-Kabel ist für stabilen Ping fast immer besser.</p>
          )}
        </Card>

        <Card className="col-span-12 lg:col-span-5" title="Programme mit Internetverbindung" subtitle="Anzahl aktiver Verbindungen">
          {apps.length === 0 ? (
            <p className="text-sm text-faint">{isTauri() ? "Keine aktiven Verbindungen gefunden." : "Nur in der App verfügbar."}</p>
          ) : (
            <ul className="max-h-48 space-y-1.5 overflow-y-auto pr-1 text-sm">
              {apps.slice(0, 12).map((a) => (
                <li key={a.exe} className="flex justify-between gap-3">
                  <span className="truncate">{a.exe}</span>
                  <span className="shrink-0 tabular-nums text-muted">{a.connections}</span>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card
          className="col-span-12"
          title="Beste Server-Region finden"
          subtitle="Rechenzentren, in denen viele Spielserver laufen"
          action={
            <Button variant="primary" onClick={runRegions} disabled={regionBusy || !isTauri()}>
              {regionBusy ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />} {regionBusy ? "Messe …" : "Test starten"}
            </Button>
          }
        >
          {!regions ? (
            <p className="text-sm text-faint">Dauert ca. 5 Sekunden.</p>
          ) : (
            <>
              {best && (
                <p className="mb-4 flex items-center gap-2 text-sm">
                  <Trophy size={16} className="text-accent" /> Beste Region: <b>{best.city}</b> · {num(best.medianMs)} ms
                </p>
              )}
              <ul className="space-y-2">
                {regions.map((r) => (
                  <li key={r.id} className="grid grid-cols-[150px_1fr_110px] items-center gap-3 text-sm">
                    <span className="truncate">{r.city} <span className="text-faint">· {r.area}</span></span>
                    <span className="h-2 overflow-hidden rounded-full bg-surface-3">
                      {r.medianMs != null && (
                        <span className={`block h-full rounded-full ${r === best ? "bg-accent" : "bg-faint"}`} style={{ width: `${(r.medianMs / maxMs) * 100}%` }} />
                      )}
                    </span>
                    <span className="text-right tabular-nums">
                      {r.medianMs != null ? `${num(r.medianMs)} ms` : "nicht erreichbar"}
                      {r.failed > 0 && r.medianMs != null && <span className="text-bad"> · {r.failed}× weg</span>}
                    </span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </Card>

        <Card
          className="col-span-12"
          title="Route-Analyse"
          subtitle="Wo entsteht der Ping?"
          action={<Route size={16} className="text-faint" />}
        >
          <form
            className="flex flex-wrap gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              runTrace();
            }}
          >
            <input
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              aria-label="Ziel (IP oder Adresse)"
              className="h-9 w-64 rounded-lg border border-line bg-surface-2 px-3 text-sm text-text"
            />
            {best && (
              <Button type="button" variant="ghost" onClick={() => setTarget(best.host)}>Beste Region einsetzen</Button>
            )}
            <Button type="submit" variant="primary" disabled={traceBusy || !isTauri()}>
              {traceBusy ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />} {traceBusy ? "Analysiere … (bis 1 min)" : "Analysieren"}
            </Button>
          </form>

          {trace && (
            <div className="mt-5 space-y-4">
              <ul className="space-y-1.5 rounded-xl bg-surface-2 p-4 text-sm">
                {trace.analysis.map((a) => <li key={a}>{a}</li>)}
              </ul>
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-xs text-faint">
                    <th className="pb-2 font-medium">#</th>
                    <th className="pb-2 font-medium">Station</th>
                    <th className="pb-2 font-medium">Bereich</th>
                    <th className="pb-2 text-right font-medium">Bester</th>
                    <th className="pb-2 text-right font-medium">Ø</th>
                    <th className="pb-2 text-right font-medium">Verlust</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-line">
                  {trace.hops.map((h) => (
                    <tr key={h.hop}>
                      <td className="py-1.5 tabular-nums text-faint">{h.hop}</td>
                      <td className="py-1.5 tabular-nums">{h.address ?? "*"}</td>
                      <td className="py-1.5"><Badge tone={KIND[h.kind].tone}>{KIND[h.kind].label}</Badge></td>
                      <td className="py-1.5 text-right tabular-nums">{h.bestMs != null ? `${num(h.bestMs)} ms` : "—"}</td>
                      <td className="py-1.5 text-right tabular-nums">{h.avgMs != null ? `${num(h.avgMs)} ms` : "—"}</td>
                      <td className={`py-1.5 text-right tabular-nums ${h.lossPercent > 0 ? "text-bad" : "text-muted"}`}>{num(h.lossPercent)} %</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
      </div>
    </>
  );
}
