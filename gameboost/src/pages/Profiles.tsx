import { useState } from "react";
import { Gamepad2, Lightbulb, Plus, Trash2, Zap } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { Toggle } from "../components/ui/Toggle";
import { useAppState } from "../lib/app-state";
import { api, type GameProfile } from "../lib/api";
import { tipsFor } from "../lib/games";
import { isTauri } from "../lib/tauri";

export default function Profiles() {
  const { settings, catalog, updateSettings } = useAppState();
  const [open, setOpen] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [exe, setExe] = useState("");
  const [msg, setMsg] = useState<string | null>(null);

  const update = (target: string, fn: (p: GameProfile) => GameProfile) =>
    updateSettings((s) => ({ ...s, profiles: s.profiles.map((p) => (p.exe === target ? fn(p) : p)) }));

  const addCustom = () => {
    const e = exe.trim().toLowerCase().endsWith(".exe") ? exe.trim() : `${exe.trim()}.exe`;
    if (!name.trim() || e === ".exe") return;
    updateSettings((s) =>
      s.profiles.some((p) => p.exe.toLowerCase() === e.toLowerCase())
        ? s
        : { ...s, profiles: [...s.profiles, { name: name.trim(), exe: e, tweaks: null, disableFullscreenOptimizations: false, exePath: null, custom: true }] },
    );
    setName("");
    setExe("");
  };

  const setFso = async (p: GameProfile, disable: boolean) => {
    if (!p.exePath) return;
    try {
      await api.setFullscreenOptimizations(p.exePath, disable);
      update(p.exe, (x) => ({ ...x, disableFullscreenOptimizations: disable }));
      setMsg(`${p.name}: Vollbild-Optimierungen ${disable ? "aus" : "an"} – wirkt ab dem nächsten Spielstart.`);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <>
      <PageHeader title="Profile" description="Eigene Einstellungen pro Spiel. GameBoost erkennt diese Spiele automatisch." />
      {msg && <p className="mb-4 text-sm text-muted">{msg}</p>}
      <div className="grid grid-cols-12 gap-4">
        <Card className="col-span-12 lg:col-span-8" title="Spiele">
          {!settings ? (
            <p className="text-sm text-faint">Nur in der App verfügbar.</p>
          ) : (
            <ul className="divide-y divide-line">
              {settings.profiles.map((p) => {
                const tips = tipsFor(p.exe);
                const custom = p.tweaks !== null;
                return (
                  <li key={p.exe} className="py-3">
                    <div className="flex items-center justify-between gap-3">
                      <button className="flex min-w-0 flex-1 items-center gap-3 text-left" onClick={() => setOpen(open === p.exe ? null : p.exe)} aria-expanded={open === p.exe}>
                        <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-surface-3 text-muted"><Gamepad2 size={16} /></span>
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium">{p.name}</span>
                          <span className="block truncate text-xs text-faint">{p.exe}</span>
                        </span>
                      </button>
                      <span className="flex shrink-0 items-center gap-2">
                        {custom && <Badge tone="accent">eigene Tweaks</Badge>}
                        {p.custom && (
                          <Button variant="ghost" className="px-2" aria-label={`${p.name} entfernen`} onClick={() => updateSettings((s) => ({ ...s, profiles: s.profiles.filter((x) => x.exe !== p.exe) }))}>
                            <Trash2 size={14} />
                          </Button>
                        )}
                      </span>
                    </div>

                    {open === p.exe && (
                      <div className="mt-4 grid animate-fade-in gap-4 pl-12 md:grid-cols-2">
                        <div>
                          <div className="flex items-center justify-between">
                            <span className="text-xs font-medium text-muted">Eigene Tweak-Auswahl</span>
                            <Toggle label="Eigene Tweak-Auswahl" checked={custom} onChange={(v) => update(p.exe, (x) => ({ ...x, tweaks: v ? [...settings.enabledTweaks] : null }))} />
                          </div>
                          {custom ? (
                            <ul className="mt-2 space-y-1.5">
                              {catalog.map((t) => (
                                <li key={t.id}>
                                  <label className="flex items-center gap-2 text-sm">
                                    <input
                                      type="checkbox"
                                      className="accent-[var(--color-accent)]"
                                      checked={p.tweaks!.includes(t.id)}
                                      onChange={(e) => update(p.exe, (x) => ({ ...x, tweaks: e.target.checked ? [...x.tweaks!, t.id] : x.tweaks!.filter((id) => id !== t.id) }))}
                                    />
                                    {t.title}
                                  </label>
                                </li>
                              ))}
                            </ul>
                          ) : (
                            <p className="mt-1 text-xs text-faint">Nutzt die Auswahl von der Boost-Seite.</p>
                          )}

                          <div className="mt-4 flex items-center justify-between gap-3">
                            <span>
                              <span className="block text-xs font-medium text-muted">Vollbild-Optimierungen aus</span>
                              <span className="block text-[11px] text-faint">{p.exePath ? "Wirkt ab dem nächsten Spielstart" : "Starte das Spiel einmal, damit GameBoost den Pfad kennt"}</span>
                            </span>
                            <Toggle label="Vollbild-Optimierungen aus" checked={p.disableFullscreenOptimizations} disabled={!p.exePath} onChange={(v) => setFso(p, v)} />
                          </div>
                        </div>
                        <div>
                          <div className="flex items-center gap-1.5 text-xs font-medium text-muted"><Lightbulb size={13} /> Empfohlen im Spiel</div>
                          <ul className="mt-2 list-disc space-y-1 pl-4 text-sm text-muted">
                            {tips.tips.map((t) => <li key={t}>{t}</li>)}
                          </ul>
                          {tips.reflex && <p className="mt-2 text-[11px] text-faint">Reflex gibt es nur mit NVIDIA-Grafikkarten. Bei AMD: „Anti-Lag“ im Treiber.</p>}
                        </div>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </Card>

        <div className="col-span-12 space-y-4 lg:col-span-4">
          <Card title="Auto-Boost" action={<Zap size={16} className="text-faint" />}>
            <div className="flex items-start justify-between gap-4">
              <p className="text-sm text-muted">Boost startet beim Spielstart und endet mit dem Spiel.</p>
              <Toggle label="Auto-Boost" checked={settings?.autoBoost ?? false} disabled={!settings} onChange={(v) => updateSettings((s) => ({ ...s, autoBoost: v }))} />
            </div>
          </Card>
          <Card title="Eigenes Spiel hinzufügen">
            <form className="space-y-3" onSubmit={(e) => { e.preventDefault(); addCustom(); }}>
              <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Name, z. B. The Finals" aria-label="Name"
                className="h-9 w-full rounded-lg border border-line bg-surface-2 px-3 text-sm text-text placeholder:text-faint" />
              <input value={exe} onChange={(e) => setExe(e.target.value)} placeholder="Programm, z. B. Discovery.exe" aria-label="Programmname"
                className="h-9 w-full rounded-lg border border-line bg-surface-2 px-3 text-sm text-text placeholder:text-faint" />
              <Button type="submit" disabled={!isTauri()}><Plus size={14} /> Hinzufügen</Button>
            </form>
            <p className="mt-3 text-[11px] text-faint">Den Programmnamen findest du im Task-Manager (Rechtsklick auf das Spiel → „Details“).</p>
          </Card>
        </div>
      </div>
    </>
  );
}
