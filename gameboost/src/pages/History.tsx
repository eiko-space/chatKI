import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";

export default function HistoryPage() {
  return (
    <>
      <PageHeader title="Verlauf" description="Alle Mess-Sessions und was der Boost wirklich gebracht hat." />
      <Planned phase={4} title="Sessions" items={["Verlauf aller Sessions als Diagramm", "Vorher/Nachher-Vergleichskarten", "Wöchentliche Statistik"]} />
    </>
  );
}
