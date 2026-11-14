import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { BoostButton } from "./ui/BoostButton";
import { Button } from "./ui/Button";
import { Dialog } from "./ui/Dialog";
import { useAppState } from "../lib/app-state";
import { api } from "../lib/api";
import { isTauri, type AppInfo } from "../lib/tauri";

/**
 * Der große Boost-Knopf inkl. Sicherheitsabfrage vor dem allerersten Boost
 * (mit Windows-Wiederherstellungspunkt).
 */
export function BoostControl({ info }: { info: AppInfo | null }) {
  const { boost, settings, catalog, busy, error, startBoost, stopBoost, updateSettings } = useAppState();
  const [confirm, setConfirm] = useState(false);
  const [withRestore, setWithRestore] = useState(true);
  const [restoreMsg, setRestoreMsg] = useState<string | null>(null);
  const [working, setWorking] = useState(false);

  const active = boost?.active ?? false;
  const enabled = catalog.filter((t) => settings?.enabledTweaks.includes(t.id));

  const onClick = () => {
    if (!isTauri()) return;
    if (active) return stopBoost();
    if (!settings?.firstBoostConfirmed) return setConfirm(true);
    return startBoost();
  };

  const onConfirm = async () => {
    setWorking(true);
    if (withRestore && info?.isElevated && !settings?.restorePointCreatedAt) {
      try {
        setRestoreMsg(await api.createRestorePoint());
      } catch (e) {
        setRestoreMsg(`Wiederherstellungspunkt nicht möglich: ${e}`);
      }
    }
    await updateSettings((s) => ({ ...s, firstBoostConfirmed: true }));
    setWorking(false);
    setConfirm(false);
    await startBoost();
  };

  return (
    <div className="flex flex-col items-center">
      <BoostButton active={active} disabled={busy || !isTauri()} onClick={onClick} />
      <p className="mt-5 max-w-xs text-center text-sm text-muted">
        {!isTauri()
          ? "Nur in der App verfügbar."
          : busy
            ? active ? "Setze alles zurück …" : "Wende Optimierungen an …"
            : active
              ? `Boost aktiv${boost?.game ? ` für ${boost.game}` : ""}. Klicke erneut, um alles zurückzusetzen.`
              : "Ein Klick optimiert Windows fürs Spielen. Alles lässt sich vollständig rückgängig machen."}
      </p>
      {error && <p className="mt-2 max-w-xs text-center text-xs text-bad">{error}</p>}
      {restoreMsg && <p className="mt-2 max-w-xs text-center text-xs text-muted">{restoreMsg}</p>}

      <Dialog
        open={confirm}
        title="Bevor es losgeht"
        onClose={() => !working && setConfirm(false)}
        actions={
          <>
            <Button variant="ghost" onClick={() => setConfirm(false)} disabled={working}>Abbrechen</Button>
            <Button variant="primary" onClick={onConfirm} disabled={working}>
              <ShieldCheck size={14} /> {working ? "Bitte warten …" : "Boost starten"}
            </Button>
          </>
        }
      >
        <p>Der Boost ändert folgende Einstellungen – jede davon wird beim Beenden zurückgesetzt:</p>
        <ul className="list-disc space-y-1 pl-5">
          {enabled.map((t) => (
            <li key={t.id}>
              <span className="text-text">{t.title}</span>
              {t.needsAdmin && !info?.isElevated && <span className="text-faint"> (übersprungen – braucht Admin-Rechte)</span>}
            </li>
          ))}
        </ul>
        <p>Virenschutz, Anti-Cheat, Voice-Chat und alles, was gerade dein Mikrofon nutzt, wird nie beendet.</p>
        {!settings?.restorePointCreatedAt && (
          <label className="flex items-start gap-2 rounded-lg bg-surface-2 p-3">
            <input
              type="checkbox"
              className="mt-0.5 accent-[var(--color-accent)]"
              checked={withRestore}
              disabled={!info?.isElevated}
              onChange={(e) => setWithRestore(e.target.checked)}
            />
            <span>
              <span className="text-text">Vorher Windows-Wiederherstellungspunkt erstellen</span> (empfohlen)
              {!info?.isElevated && <span className="block text-xs text-faint">Braucht Admin-Rechte – starte GameBoost als Administrator, um ihn zu erstellen.</span>}
            </span>
          </label>
        )}
      </Dialog>
    </div>
  );
}
