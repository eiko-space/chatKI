// Lädt Intel PresentMon (Open Source, MIT-Lizenz) herunter und prüft die Prüfsumme.
// Wird automatisch nach `npm install` ausgeführt. Manuell: `npm run fetch-presentmon`.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = "2.4.1";
const URL = `https://github.com/GameTechDev/PresentMon/releases/download/v${VERSION}/PresentMon-${VERSION}-x64.exe`;
const SHA256 = "d74183e7ae630f72cd3690be0373ecbfdc6cbb86578148aab8fa2a7166068f34";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = join(root, "src-tauri", "bin", "PresentMon.exe");
const sha = (buf) => createHash("sha256").update(buf).digest("hex");

if (existsSync(target) && sha(readFileSync(target)) === SHA256) {
  console.log(`PresentMon ${VERSION} ist bereits vorhanden.`);
  process.exit(0);
}

try {
  console.log(`Lade PresentMon ${VERSION} herunter …`);
  const res = await fetch(URL);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  const got = sha(buf);
  if (got !== SHA256) throw new Error(`Prüfsumme stimmt nicht (${got}). Datei wird verworfen.`);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, buf);
  console.log("PresentMon erfolgreich geladen und geprüft.");
} catch (err) {
  // Kein harter Fehler: Die App läuft auch ohne – nur die FPS-Messung ist dann deaktiviert.
  console.warn(`\n⚠  PresentMon konnte nicht geladen werden: ${err.message}`);
  console.warn("   Die FPS-Messung bleibt deaktiviert. Später erneut versuchen mit: npm run fetch-presentmon\n");
}
