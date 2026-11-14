import { useState } from "react";
import { AlertTriangle, CheckCircle2, Info, Loader2, Play, Wrench, XCircle } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Details } from "../components/ui/Details";
import { toolsApi, type CheckItem, type CheckReport } from "../lib/api";
import { isTauri, type AppInfo } from "../lib/tauri";

const ICON = {
  good: { icon: CheckCircle2, cls: "text-good", label: "In Ordnung" },
  warn: { icon: AlertTriangle, cls: "text-bad", label: "Verbesserbar" },
  bad: { icon: XCircle, cls: "text-bad", label: "Problem" },
  info: { icon: Info, cls: "text-muted", label: "Hinweis" },
} as const;

function ScoreRing({ score }: { score: number }) {
  const r = 54;
  const c = 2 * Math.PI * r;
  const color = score >= 80 ? "var(--color-good)" : score >= 60 ? "var(--color-accent)" : "var(--color-bad)";
  return (
    <div className="relative grid size-36 place-items-center">
      <svg viewBox="0 0 128 128" className="absolute inset-0 -rotate-90">
        <circle cx="64" cy="64" r={r} fill="none" stroke="var(--color-surface-3)" strokeWidth="10" />
        <circle cx="64" cy="64" r={r} fill="none" stroke={color} strokeWidth="10" strokeLinecap="round" strokeDasharray={`${(score / 100) * c} ${c}`} />
      </svg>
      <div className="text-center">
        <div className="text-4xl font-semibold tracking-tight">{score}</div>
        <div className="text-xs text-muted">von 100</div>
      </div>
    </div>
  );
}

export default function PcCheck({ info }: { info: AppInfo | null }) {
  const [report, setReport] = useState<CheckReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [fixMsg, setFixMsg] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const scan = async () => {
    setBusy(true);
    setError(null);
    try {
      setReport(await toolsApi.pcCheck());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const fix = async (item: CheckItem) => {
    if (!item.fix) return;
    try {
      const msg = await toolsApi.pcFix(item.fix);
      setFixMsg((m) => ({ ...m, [item.id]: msg }));
      if (!["driver_nvidia", "driver_amd", "driver_intel", "storage_settings", "startup_apps"].includes(item.fix)) scan();
    } catch (e) {
      setFixMsg((m) => ({ ...m, [item.id]: `Fehler: ${e}` }));
    }
  };

  const problems = report?.items.filter((i) => i.level === "bad" || i.level === "warn").length ?? 0;

  return (
    <>
      <PageHeader title="PC-Check" description="Findet Bremsen und sagt dir, wie du sie behebst." />
      {error && <p className="mb-4 text-sm text-bad">{error}</p>}
      <div className="grid grid-cols-12 gap-4">
        <Card className="col-span-12 flex flex-col items-center justify-center gap-4 py-8 lg:col-span-4">
          {report ? <ScoreRing score={report.score} /> : <div className="grid size-36 place-items-center rounded-full border-[10px] border-surface-3 text-3xl text-faint">—</div>}
          <p className="text-center text-sm text-muted">
            {!report ? "Starte den Scan – er dauert nur ein paar Sekunden." : problems === 0 ? "Keine Probleme gefunden." : `${problems} Punkt${problems === 1 ? "" : "e"} kannst du verbessern.`}
          </p>
          <Button variant="primary" onClick={scan} disabled={busy || !isTauri()}>
            {busy ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />} {report ? "Erneut prüfen" : "Scan starten"}
          </Button>
          {!isTauri() && <p className="text-xs text-faint">Nur in der App verfügbar.</p>}
        </Card>

        <Card className="col-span-12 lg:col-span-8" title="Ergebnis">
          {!report ? (
            <p className="text-sm text-faint">Treiber, Monitor-Hz, RAM-Takt, Speicher, Autostart, Temperaturen, Windows-Einstellungen und Internet.</p>
          ) : (
            <ul className="divide-y divide-line">
              {report.items.map((item, idx) => {
                const meta = ICON[item.level];
                const Icon = meta.icon;
                const locked = item.fixNeedsAdmin && !info?.isElevated;
                return (
                  <li key={item.id} className="py-3 first:pt-0 last:pb-0 animate-pop-in" style={{ animationDelay: `${idx * 60}ms` }}>
                    <div className="flex items-start gap-3">
                      <Icon size={18} className={`mt-0.5 shrink-0 ${meta.cls}`} aria-label={meta.label} />
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-baseline justify-between gap-x-3">
                          <span className="text-sm font-medium">{item.title}</span>
                          <span className="text-sm tabular-nums text-muted">{item.value}</span>
                        </div>
                        <p className="mt-0.5 text-xs leading-relaxed text-muted">{item.explanation}</p>
                        {item.steps.length > 0 && (
                          <div className="mt-3 rounded-lg bg-surface-2 p-3">
                            <div className="text-xs font-semibold text-text">So behebst du es</div>
                            <ol className="mt-1.5 list-decimal space-y-1 pl-4 text-xs text-muted">
                              {item.steps.map((st) => <li key={st}>{st}</li>)}
                            </ol>
                          </div>
                        )}
                        {item.fix && (
                          <div className="mt-2 flex flex-wrap items-center gap-2">
                            <Button className="h-8" onClick={() => fix(item)} disabled={locked}>
                              <Wrench size={13} /> {item.fixLabel}
                            </Button>
                            {locked && <span className="text-[11px] text-faint">braucht Admin-Rechte</span>}
                            {fixMsg[item.id] && <span className="text-xs text-muted">{fixMsg[item.id]}</span>}
                          </div>
                        )}
                        {item.details && <Details label="Details">{item.details}</Details>}
                      </div>
                    </div>
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
