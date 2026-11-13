import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";

export default function Network() {
  return (
    <>
      <PageHeader title="Netzwerk" description="Ping-Spikes finden und abstellen – mit echten Messungen zu den Spielservern." />
      <div className="grid gap-4 md:grid-cols-2">
        <Planned phase={2} title="Live-Messung" items={["Ping, Jitter und Paketverlust", "Welches Programm gerade Internet nutzt", "Download/Upload live"]} />
        <Planned phase={5} title="Werkzeuge" items={["Ping-Test zu echten Spielservern pro Region", "Route-Analyse: wo entsteht der Ping?", "Warnung bei WLAN statt LAN"]} />
        <Planned phase={3} title="Netzwerk-Tweaks" items={["Downloads & Updates pausieren", "Übermittlungsoptimierung aus", "WLAN-Hintergrundsuche aus", "Energiesparmodus der Netzwerkkarte aus"]} />
      </div>
    </>
  );
}
