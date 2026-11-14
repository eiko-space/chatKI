import { useEffect, useRef, useState } from "react";
import { Ear, Eye, ExternalLink, Keyboard, Maximize2, Palette } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { useAppState } from "../lib/app-state";
import { api } from "../lib/api";
import { useTelemetry } from "../lib/telemetry";
import { isTauri } from "../lib/tauri";

function Steps({ items }: { items: string[] }) {
  return (
    <ol className="mt-3 list-decimal space-y-1.5 pl-5 text-sm text-muted">
      {items.map((s) => <li key={s}>{s}</li>)}
    </ol>
  );
}

/** Sicht-Boost: Regler wird sofort angewendet (leicht verzögert, damit Ziehen flüssig bleibt). */
function VisionCard() {
  const [level, setLevel] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (isTauri()) api.getVision().then(setLevel).catch(() => {});
  }, []);

  const change = (v: number) => {
    setLevel(v);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      api.setVision(v).then(() => setError(null)).catch((e) => setError(String(e)));
    }, 120);
  };

  return (
    <Card className="col-span-12 lg:col-span-6" title="Sicht-Boost" subtitle="Gegner in dunklen Ecken sofort sehen" action={<Eye size={16} className="text-accent" />}>
      <div className="flex items-center gap-4">
        <input
          type="range" min={0} max={100} step={5} value={level} disabled={!isTauri()}
          aria-label="Stärke des Sicht-Boosts"
          onChange={(e) => change(Number(e.target.value))}
          className="flex-1 accent-[var(--color-accent)]"
        />
        <span className="w-14 text-right text-lg font-semibold tabular-nums">{level === 0 ? "Aus" : `${level} %`}</span>
      </div>
      <div className="mt-4 flex flex-wrap gap-2">
        {[0, 30, 60, 90].map((v) => (
          <Button key={v} variant={level === v ? "primary" : "secondary"} className="h-8" onClick={() => change(v)} disabled={!isTauri()}>
            {v === 0 ? "Aus" : v === 30 ? "Leicht" : v === 60 ? "Mittel" : "Stark"}
          </Button>
        ))}
      </div>
      {error && <p className="mt-3 text-xs text-bad">{error}</p>}
      <p className="mt-3 text-xs text-faint">Hellt nur Schatten auf, helle Stellen bleiben gleich. Wird beim Beenden von GameBoost automatisch zurückgesetzt.</p>
    </Card>
  );
}

export default function Advantages() {
  const { settings } = useAppState();
  const { snapshot } = useTelemetry();
  const gpu = snapshot?.gpu.name?.toLowerCase() ?? "";
  const vendor: "nvidia" | "amd" | "other" = gpu.includes("nvidia") ? "nvidia" : gpu.includes("amd") || gpu.includes("radeon") ? "amd" : "other";

  return (
    <>
      <PageHeader title="Vorteile" description="Erlaubte Tricks, die Profis nutzen – kein Eingriff ins Spiel, kein Bann-Risiko." />
      <div className="grid grid-cols-12 gap-4">
        <VisionCard />

        <Card className="col-span-12 lg:col-span-6" title="Farb-Boost" subtitle="Gegner heben sich stärker vom Hintergrund ab" action={<Palette size={16} className="text-accent" />}>
          {vendor !== "amd" && (
            <>
              <div className="text-xs font-semibold text-text">NVIDIA – „Digitale Brillanz“</div>
              <Steps items={["NVIDIA Systemsteuerung öffnen", "Anzeige → Desktopfarbeinstellungen anpassen", "„Digitale Brillanz“ auf 70–80 % stellen → Übernehmen"]} />
              <Button className="mt-3 h-8" onClick={() => api.openSetting("nvidia")} disabled={!isTauri()}><ExternalLink size={13} /> NVIDIA Systemsteuerung</Button>
            </>
          )}
          {vendor !== "nvidia" && (
            <div className={vendor === "other" ? "mt-5" : ""}>
              <div className="text-xs font-semibold text-text">AMD – „Sättigung“</div>
              <Steps items={["AMD Software: Adrenalin öffnen", "Spielen → Anzeige → „Benutzerdefinierte Farbe“ an", "„Sättigung“ auf 130–150 stellen"]} />
              <Button className="mt-3 h-8" onClick={() => api.openSetting("amd")} disabled={!isTauri()}><ExternalLink size={13} /> AMD Software</Button>
            </div>
          )}
        </Card>

        <Card className="col-span-12 lg:col-span-6" title="Schritte früher hören" subtitle="Leise Geräusche lauter, laute leiser" action={<Ear size={16} className="text-accent" />}>
          <Steps
            items={[
              "Soundeinstellungen öffnen (Knopf unten)",
              "Dein Headset auswählen → Eigenschaften → Reiter „Erweiterungen“",
              "Haken bei „Lautheitsausgleich“ setzen → OK",
              "Im Spiel die Gesamtlautstärke etwas senken – Schritte bleiben gut hörbar",
            ]}
          />
          <Button className="mt-3 h-8" onClick={() => api.openSetting("sound")} disabled={!isTauri()}><ExternalLink size={13} /> Soundeinstellungen</Button>
          <p className="mt-3 text-xs text-faint">Fehlt der Reiter „Erweiterungen“, bietet dein Headset-Treiber das nicht an. Dann im Headset-Programm „Equalizer: FPS/Schritte“ wählen.</p>
        </Card>

        <Card className="col-span-12 lg:col-span-6" title="Gestreckte Auflösung" subtitle="Gegner wirken breiter und sind leichter zu treffen" action={<Maximize2 size={16} className="text-accent" />}>
          <div className="space-y-4 text-sm">
            <div>
              <div className="flex items-center gap-2 text-xs font-semibold text-text">Counter-Strike 2 <Badge tone="good">erlaubt</Badge></div>
              <Steps items={["Im Grafiktreiber Skalierung auf „Vollbild“ stellen (NVIDIA: Desktop-Größe und -Position)", "Im Spiel: Seitenverhältnis 4:3, Auflösung 1280×960, Modus „Vollbild“"]} />
            </div>
            <div>
              <div className="flex items-center gap-2 text-xs font-semibold text-text">Fortnite <Badge tone="good">erlaubt</Badge></div>
              <Steps items={["Datei %LOCALAPPDATA%\\FortniteGame\\Saved\\Config\\WindowsClient\\GameUserSettings.ini öffnen", "ResolutionSizeX/Y und LastUserConfirmedResolutionSizeX/Y z. B. auf 1680 × 1050 setzen", "Datei speichern und auf „Schreibgeschützt“ stellen"]} />
            </div>
            <div className="flex items-center gap-2 text-xs text-muted">Valorant <Badge tone="bad">nicht möglich</Badge> – das Spiel verhindert gestreckte Bilder.</div>
          </div>
        </Card>

        <Card className="col-span-12" title="Boost per Tastenkürzel" subtitle="Mitten im Spiel ein- und ausschalten" action={<Keyboard size={16} className="text-accent" />}>
          <p className="text-sm text-muted">
            Drücke <kbd className="rounded border border-line bg-surface-2 px-1.5 py-0.5 text-text">{settings?.boostHotkey || "Ctrl+Shift+B"}</kbd> –
            der Boost startet bzw. wird zurückgesetzt, ohne dass du das Spiel verlassen musst. Ändern kannst du das Kürzel unter Einstellungen.
          </p>
        </Card>
      </div>
    </>
  );
}
