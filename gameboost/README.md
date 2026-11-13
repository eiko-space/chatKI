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
```

## Phasen

1. ✅ Grundgerüst, Design-System, Navigation, Tray
2. Live-Dashboard mit echten Messwerten
3. Boost-Funktionen + Rückgängig-System + Wiederherstellungspunkt
4. Vorher-/Nachher-Messung
5. Netzwerk-Tools und PC-Check
6. Login (Supabase) + Cloud-Sync
7. Overlay, Auto-Boost, Installer
