import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";

export default function Boost() {
  return (
    <>
      <PageHeader title="Boost" description="Jeder Tweak einzeln schaltbar, mit Erklärung und vollständig rückgängig machbar." />
      <div className="grid gap-4 md:grid-cols-2">
        <Planned phase={3} title="FPS & Input Delay" items={[
          "Energieplan „Höchstleistung“ / „Ultimative Leistung“",
          "Spiel auf hohe Priorität, Hintergrundprozesse niedriger",
          "Hintergrund-Apps beenden (eigene Liste, nichts Systemkritisches)",
          "Spielmodus an, Xbox Game Bar / DVR-Aufnahme aus",
          "GPU-Planung und Monitor-Hz prüfen",
          "Standby-Liste des Arbeitsspeichers leeren",
        ]} />
        <Planned phase={3} title="Sicherheit" items={[
          "Wiederherstellungspunkt vor dem ersten Boost",
          "Jede Änderung wird protokolliert und kann zurückgesetzt werden",
          "Kein Eingriff in Spielprozesse außer Priorität – Anti-Cheat-sicher",
        ]} />
        <Planned phase={4} title="Vorher / Nachher" items={[
          "60 Sekunden Basis-Messung im Spiel",
          "Gleiche Messung nach dem Boost",
          "Ehrlicher Vergleich – auch wenn ein Tweak nichts bringt",
        ]} />
        <Planned phase={7} title="Auto-Boost" items={[
          "Startet automatisch, wenn ein erkanntes Spiel läuft",
          "Setzt nach dem Spiel alles zurück",
        ]} />
      </div>
    </>
  );
}
