import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";

export default function Profiles() {
  return (
    <>
      <PageHeader title="Profile" description="Eigene Einstellungen pro Spiel." />
      <Planned phase={3} title="Spiele-Erkennung" items={[
        "Fortnite, CS2, Valorant, Apex, Warzone, Minecraft, Rocket League, LoL",
        "Eigene .exe hinzufügen",
        "Pro Spiel festlegen, welche Tweaks aktiv sind",
      ]} />
    </>
  );
}
