import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";

export default function PcCheck() {
  return (
    <>
      <PageHeader title="PC-Check" description="Ein Systemscan mit Bewertung von 0–100 und konkreten Tipps in einfachem Deutsch." />
      <Planned phase={5} title="Prüfpunkte" items={[
        "Alter des Grafiktreibers", "Autostart-Programme", "Freier Festplattenplatz",
        "RAM-Takt (XMP/EXPO aktiv?)", "Temperaturen & Thermal Throttling",
        "Zu jedem Punkt eine Erklärung und – wenn möglich – ein „Fix“-Knopf",
      ]} />
    </>
  );
}
