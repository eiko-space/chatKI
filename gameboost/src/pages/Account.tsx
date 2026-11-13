import { UserRound } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Planned } from "../components/ui/Planned";
import { Card } from "../components/ui/Card";
import { Badge } from "../components/ui/Badge";

export default function Account() {
  return (
    <>
      <PageHeader title="Konto" description="Ein Konto ist optional. Alle Grundfunktionen laufen offline im Gast-Modus." />
      <div className="grid gap-4 md:grid-cols-2">
        <Card title="Gast-Modus" action={<Badge tone="accent">Aktiv</Badge>}>
          <div className="flex items-center gap-3">
            <div className="grid size-11 place-items-center rounded-full bg-surface-3 text-muted"><UserRound size={20} /></div>
            <p className="text-sm text-muted">Deine Daten bleiben nur auf diesem PC.</p>
          </div>
        </Card>
        <Planned phase={6} title="Mit Konto" items={["Login per E-Mail, Discord oder Google", "Profile & Verlauf in der Cloud synchronisieren", "Sichere Token-Speicherung im Windows Credential Manager"]} />
      </div>
    </>
  );
}
