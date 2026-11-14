# GameBoost

Ehrliche PC-Gaming-Optimierung für Windows 10/11 – Tauri 2 (Rust) + React + TypeScript + Tailwind.

**Grundregel:** Die App zeigt nur echte, gemessene Werte. Was (noch) nicht gemessen wird, erscheint als „—“.

## Voraussetzungen (Windows)

1. [Node.js 20+](https://nodejs.org)
2. [Rust](https://rustup.rs) (Standard-Toolchain `stable-x86_64-pc-windows-msvc`)
3. [Visual Studio Build Tools](https://visualstudio.microsoft.com/de/visual-cpp-build-tools/) mit „Desktopentwicklung mit C++“
4. WebView2 (bei Windows 10/11 meist schon installiert)

## Starten

```bash
cd gameboost
npm install
npm run tauri dev      # App mit Hot-Reload
npm run tauri build    # Installer (.exe/.msi) unter src-tauri/target/release/bundle
npm run dev            # nur Oberfläche im Browser (ohne Systemzugriff)
```

## FPS-Messung (PresentMon)

`npm install` lädt automatisch [Intel PresentMon](https://github.com/GameTechDev/PresentMon) 2.4.1 (MIT-Lizenz)
nach `src-tauri/bin/` und prüft die SHA-256-Prüfsumme. Manuell: `npm run fetch-presentmon`.

Die FPS-Messung braucht Administratorrechte (Windows-ETW). Im Entwicklungsmodus: Rechtsklick auf
`GameBoost-Start.bat` → „Als Administrator ausführen“.

## Konto & Cloud-Sync einrichten (optional)

Ohne diese Schritte läuft GameBoost im Gast-Modus – alle Grundfunktionen gehen trotzdem.

1. Kostenloses Projekt auf [supabase.com](https://supabase.com) anlegen.
2. **SQL Editor** → Inhalt von `supabase/migrations/0001_gameboost.sql` einfügen → **Run**.
3. **Project Settings → API**: `Project URL` und `anon public` Key kopieren.
   Im Ordner `gameboost` die Datei `.env.example` als `.env.local` kopieren und die Werte eintragen.
4. **Authentication → URL Configuration → Redirect URLs**: `http://127.0.0.1:53682/callback` hinzufügen.
5. **Authentication → Providers**: Discord und Google aktivieren
   (Client-ID/Secret aus dem [Discord Developer Portal](https://discord.com/developers/applications) bzw. der
   [Google Cloud Console](https://console.cloud.google.com/apis/credentials); dort als Redirect die
   Callback-URL eintragen, die Supabase beim Provider anzeigt).
6. **Authentication → Email Templates**: In „Confirm signup“ und „Reset Password“ zusätzlich `{{ .Token }}`
   einfügen – dann bekommt man einen Code, den man direkt in der App eingeben kann.

Anmelde-Tokens werden in der **Windows-Anmeldeinformationsverwaltung** gespeichert, nicht in einer Datei.
Synchronisiert werden nur geräteunabhängige Einstellungen und die Messungen. Die Spalte `profiles.plan`
(free/pro) ist für eine spätere Pro-Version vorbereitet und kann nur serverseitig geändert werden.

## Tests

```bash
npm test   # Rust-Unit-Tests (FPS-/Ping-Berechnung, PresentMon-CSV-Parser)
```

## Struktur

```
src/                    React-Oberfläche
  styles.css            Design-Tokens (Farben, Schrift, Radius, Animationen)
  components/ui/        Card, Metric, Badge, Button, Details, BoostButton …
  components/layout/    Seitenleiste
  pages/                Dashboard, Boost, Netzwerk, PC-Check, Verlauf, Profile, Einstellungen, Konto
  lib/tauri.ts          Brücke zum Rust-Backend
src-tauri/              Rust-Backend
  src/lib.rs            App-Start, Single-Instance, Schließen → Tray
  src/tray.rs           System-Tray mit Menü
  src/commands/         Befehle, die die Oberfläche aufrufen kann
  src/telemetry/        Live-Messung: frames (FPS), presentmon, ping, gpu, system
  src/elevation.rs      Admin-Rechte prüfen / Neustart mit Admin-Rechten
```

## Phasen

1. ✅ Grundgerüst, Design-System, Navigation, Tray
2. ✅ Live-Dashboard mit echten Messwerten
3. ✅ Boost-Funktionen + Rückgängig-System + Wiederherstellungspunkt
4. ✅ Vorher-/Nachher-Messung
5. ✅ Netzwerk-Tools und PC-Check
6. ✅ Login (Supabase) + Cloud-Sync
7. Overlay, Auto-Boost, Installer
