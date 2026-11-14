import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Toggle } from "../components/ui/Toggle";
import { Dialog } from "../components/ui/Dialog";
import { useAppState } from "../lib/app-state";
import { api, type RunningApp } from "../lib/api";
import { isTauri, type AppInfo } from "../lib/tauri";
import { num } from "../lib/format";

export default function SettingsPage({ info }: { info: AppInfo | null }) {
  const { settings, updateSettings } = useAppState();
  const [picker, setPicker] = useState(false);
  const [running, setRunning] = useState<RunningApp[]>([]);
  const [manual, setManual] = useState("");

  const openPicker = async () => {
    setPicker(true);
    setRunning(await api.runningApps().catch(() => []));
  };

  const addApp = (exe: string) => {
    const name = exe.trim();
    if (!name) return;
    const withExe = name.toLowerCase().endsWith(".exe") ? name : `${name}.exe`;
    updateSettings((s) =>
      s.backgroundApps.some((a) => a.exe.toLowerCase() === withExe.toLowerCase())
        ? s
        : { ...s, backgroundApps: [...s.backgroundApps, { exe: withExe, enabled: true }] },
    );
  };

  return (
    <>
      <PageHeader title="Einstellungen" />
      <div className="grid grid-cols-12 gap-4">
        <Card
          className="col-span-12 lg:col-span-7"
          title="Hintergrund-Apps"
          subtitle="Werden beim Boost beendet und danach wieder gestartet"
          action={
            <Button onClick={openPicker} disabled={!isTauri()}>
              <Plus size={14} /> Hinzufügen
            </Button>
          }
        >
          {!settings ? (
            <p className="text-sm text-faint">Nur in der App verfügbar.</p>
          ) : (
            <ul className="divide-y divide-line">
              {settings.backgroundApps.map((a) => (
                <li key={a.exe} className="flex items-center justify-between gap-3 py-2">
                  <span className="truncate text-sm">{a.exe}</span>
                  <span className="flex items-center gap-2">
                    <Toggle
                      label={a.exe}
                      checked={a.enabled}
                      onChange={(v) => updateSettings((s) => ({ ...s, backgroundApps: s.backgroundApps.map((b) => (b.exe === a.exe ? { ...b, enabled: v } : b)) }))}
                    />
                    <Button
                      variant="ghost"
                      className="px-2"
                      aria-label={`${a.exe} entfernen`}
                      onClick={() => updateSettings((s) => ({ ...s, backgroundApps: s.backgroundApps.filter((b) => b.exe !== a.exe) }))}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </span>
                </li>
              ))}
            </ul>
          )}
          <p className="mt-4 text-xs text-faint">
            Voreingestellt sind Cloud-Sync-Programme (OneDrive, Dropbox …), weil sie im Spiel Festplatte und Internet belasten.
            Programme aus der Schutzliste (Virenschutz, Discord, Launcher …) werden nie beendet, selbst wenn sie hier stehen.
          </p>
        </Card>

        <Card className="col-span-12 lg:col-span-5" title="App-Verhalten">
          <div className="space-y-4">
            <Row label="Benachrichtigungen" hint="Bei Überhitzung oder Ping-Spitzen">
              <Toggle label="Benachrichtigungen" checked={settings?.notifications ?? false} disabled={!settings} onChange={(v) => updateSettings((s) => ({ ...s, notifications: v }))} />
            </Row>
            <Row label="Warnung ab GPU-Temperatur" hint={`${num(settings?.tempWarningC) ?? "—"} °C`}>
              <input
                type="range"
                min={70}
                max={95}
                step={1}
                aria-label="Warnung ab GPU-Temperatur"
                value={settings?.tempWarningC ?? 85}
                disabled={!settings}
                onChange={(e) => updateSettings((s) => ({ ...s, tempWarningC: Number(e.target.value) }))}
                className="w-32 accent-[var(--color-accent)]"
              />
            </Row>
          </div>
          <p className="mt-5 text-sm text-muted">
            Beim Schließen des Fensters läuft GameBoost im System-Tray weiter. Über das Tray-Symbol öffnest oder beendest du die App.
          </p>
          {info && (
            <p className="mt-3 text-xs text-faint">
              Version {info.version} · {info.os} {info.osVersion} · Admin-Rechte: {info.isElevated ? "ja" : "nein"}
            </p>
          )}
        </Card>
      </div>

      <Dialog
        open={picker}
        title="Hintergrund-App hinzufügen"
        onClose={() => setPicker(false)}
        actions={<Button onClick={() => setPicker(false)}>Fertig</Button>}
      >
        <p>Laufende Programme (geschützte sind ausgeblendet), sortiert nach Arbeitsspeicher:</p>
        <ul className="max-h-64 divide-y divide-line overflow-y-auto rounded-lg border border-line">
          {running.length === 0 && <li className="p-3 text-faint">Lade …</li>}
          {running.map((r) => {
            const added = settings?.backgroundApps.some((a) => a.exe.toLowerCase() === r.exe.toLowerCase());
            return (
              <li key={r.exe} className="flex items-center justify-between gap-3 px-3 py-2">
                <span className="min-w-0 truncate text-text">{r.exe}</span>
                <span className="flex shrink-0 items-center gap-3">
                  <span className="text-xs text-faint">{r.memoryMb != null ? `${num(r.memoryMb)} MB` : ""}</span>
                  <Button variant={added ? "ghost" : "secondary"} disabled={added} onClick={() => addApp(r.exe)} className="h-7 px-3">
                    {added ? "Drin" : "Hinzufügen"}
                  </Button>
                </span>
              </li>
            );
          })}
        </ul>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            addApp(manual);
            setManual("");
          }}
        >
          <input
            value={manual}
            onChange={(e) => setManual(e.target.value)}
            placeholder="oder Name eingeben, z. B. Spotify.exe"
            className="h-9 flex-1 rounded-lg border border-line bg-surface-2 px-3 text-sm text-text placeholder:text-faint"
          />
          <Button type="submit">Hinzufügen</Button>
        </form>
      </Dialog>
    </>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <div className="text-sm font-medium">{label}</div>
        {hint && <div className="text-xs text-muted">{hint}</div>}
      </div>
      {children}
    </div>
  );
}
