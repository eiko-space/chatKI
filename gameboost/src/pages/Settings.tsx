import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";
import { Card } from "../components/ui/Card";
import type { AppInfo } from "../lib/tauri";

export default function SettingsPage({ info }: { info: AppInfo | null }) {
  return (
    <>
      <PageHeader title="Einstellungen" />
      <div className="grid gap-4 md:grid-cols-2">
        <Card title="App-Verhalten">
          <p className="text-sm text-muted">
            Beim Schließen des Fensters läuft GameBoost im System-Tray weiter. Über das Tray-Symbol
            kannst du die App wieder öffnen oder ganz beenden.
          </p>
          {info && <p className="mt-3 text-xs text-faint">Version {info.version} · {info.os} {info.osVersion}</p>}
        </Card>
        <Planned phase={7} title="Später" items={["In-Game-Overlay (Position, Größe, Hotkey)", "Benachrichtigungen bei Überhitzung oder Ping-Spike", "Automatische Updates", "Mit Windows starten"]} />
      </div>
    </>
  );
}
