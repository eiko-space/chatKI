import { useState } from "react";
import { Cpu, Gamepad2, MemoryStick, Wifi } from "lucide-react";
import { Card } from "../components/ui/Card";
import { Metric } from "../components/ui/Metric";
import { Badge } from "../components/ui/Badge";
import { Details } from "../components/ui/Details";
import { PageHeader } from "../components/ui/PageHeader";
import { BoostButton } from "../components/ui/BoostButton";
import type { AppInfo } from "../lib/tauri";

/**
 * Phase 1: Layout des Live-Dashboards.
 * Alle Messwerte sind `null`, bis in Phase 2 echte Messquellen (PresentMon, Ping, Sensoren) angebunden sind.
 */
export default function Dashboard({ info }: { info: AppInfo | null }) {
  const [notice, setNotice] = useState(false);

  return (
    <>
      <PageHeader
        title="Dashboard"
        description="Live-Werte deines PCs. Es werden nur echte Messungen angezeigt – solange nichts gemessen wird, siehst du „—“."
        action={<Badge><Gamepad2 size={11} /> Kein Spiel erkannt</Badge>}
      />

      <div className="grid grid-cols-12 gap-4">
        {/* Boost-Zentrale */}
        <Card className="col-span-12 lg:col-span-5 flex flex-col items-center justify-center py-8">
          <BoostButton active={false} onClick={() => setNotice(true)} />
          <p className="mt-5 text-center text-sm text-muted">
            {notice
              ? "Boost-Funktionen werden in Phase 3 freigeschaltet – inklusive Wiederherstellungspunkt und Rückgängig-Funktion."
              : "Ein Klick optimiert Windows fürs Spielen. Alles lässt sich vollständig rückgängig machen."}
          </p>
        </Card>

        {/* FPS */}
        <Card
          className="col-span-12 lg:col-span-7"
          title="Bildrate (FPS)"
          subtitle="Gemessen über Intel PresentMon"
          action={<Badge>Phase 2</Badge>}
        >
          <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
            <Metric label="Aktuell" value={null} unit="FPS" size="lg" />
            <Metric label="Durchschnitt" value={null} unit="FPS" />
            <Metric label="1 %-Lows" value={null} unit="FPS" />
            <Metric label="0,1 %-Lows" value={null} unit="FPS" />
          </div>
          <div className="mt-6 grid h-32 place-items-center rounded-xl border border-dashed border-line bg-surface-2/50 text-xs text-faint">
            Frametime-Graph erscheint, sobald ein Spiel läuft
          </div>
          <Details>
            Die <b className="text-text">1 %-Lows</b> sind der Durchschnitt der langsamsten 1 % aller Frames.
            Sie zeigen Ruckler viel besser als die durchschnittliche FPS. Der Frametime-Graph zeigt
            die Zeit pro Bild in Millisekunden – jede Spitze ist ein spürbarer Ruckler.
          </Details>
        </Card>

        {/* Netzwerk */}
        <Card className="col-span-12 md:col-span-6 lg:col-span-4" title="Verbindung zum Spielserver" action={<Wifi size={16} className="text-faint" />}>
          <div className="grid grid-cols-3 gap-4">
            <Metric label="Ping" value={null} unit="ms" />
            <Metric label="Jitter" value={null} unit="ms" />
            <Metric label="Verlust" value={null} unit="%" />
          </div>
        </Card>

        {/* Hardware */}
        <Card className="col-span-12 md:col-span-6 lg:col-span-4" title="Auslastung" action={<Cpu size={16} className="text-faint" />}>
          <div className="grid grid-cols-2 gap-4">
            <Metric label="CPU" value={null} unit="%" />
            <Metric label="GPU" value={null} unit="%" />
            <Metric label="RAM" value={null} unit="%" />
            <Metric label="VRAM" value={null} unit="%" />
          </div>
        </Card>

        <Card className="col-span-12 lg:col-span-4" title="Temperaturen & Datenverkehr" action={<MemoryStick size={16} className="text-faint" />}>
          <div className="grid grid-cols-2 gap-4">
            <Metric label="CPU-Temp." value={null} unit="°C" />
            <Metric label="GPU-Temp." value={null} unit="°C" />
            <Metric label="Download" value={null} unit="Mbit/s" />
            <Metric label="Upload" value={null} unit="Mbit/s" />
          </div>
        </Card>

        {/* Systemstatus: bereits echte Werte aus dem Rust-Backend */}
        <Card className="col-span-12" title="Systemstatus" subtitle="Diese Werte kommen bereits live aus dem Rust-Backend">
          {info ? (
            <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
              <Metric label="Betriebssystem" value={info.os} hint={info.osVersion} />
              <Metric label="Architektur" value={info.arch} />
              <Metric label="App-Version" value={info.version} />
              <Metric
                label="Admin-Rechte"
                value={info.isElevated ? "Ja" : "Nein"}
                hint={info.isElevated ? undefined : "Wird nur bei Bedarf angefragt"}
              />
            </div>
          ) : (
            <p className="text-sm text-faint">
              Nicht verfügbar – die Oberfläche läuft gerade im Browser statt in der App.
            </p>
          )}
        </Card>
      </div>
    </>
  );
}
