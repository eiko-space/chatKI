/** Empfohlene Einstellungen im Spiel – allgemein gültig für kompetitives Spielen. */
export interface GameTips {
  reflex: boolean;
  tips: string[];
}

const COMMON = [
  "Vollbild bzw. „Randloses Fenster“ statt Fenstermodus",
  "VSync im Spiel aus",
  "Mit G-Sync/FreeSync: FPS-Limit ca. 3 FPS unter der Hz-Zahl deines Monitors (z. B. 141 bei 144 Hz)",
];

const TIPS: Record<string, GameTips> = {
  "cs2.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", "Multicore-Rendering an", ...COMMON] },
  "VALORANT-Win64-Shipping.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", "Multithreaded Rendering an", ...COMMON] },
  "FortniteClient-Win64-Shipping.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", "Für maximale FPS: Rendering-Modus „Performance“", ...COMMON] },
  "r5apex.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", "Adaptive Auflösung aus (Zielwert 0)", ...COMMON] },
  "r5apex_dx12.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", ...COMMON] },
  "cod.exe": { reflex: true, tips: ["NVIDIA Reflex: „Ein + Boost“", "Shader-Vorkompilierung vor dem ersten Match abwarten", ...COMMON] },
  "RocketLeague.exe": { reflex: false, tips: ["FPS-Limit „Unbegrenzt“ oder knapp unter deiner Hz-Zahl", ...COMMON] },
  "League of Legends.exe": { reflex: false, tips: ["FPS-Limit knapp unter deiner Hz-Zahl", ...COMMON] },
  "javaw.exe": { reflex: false, tips: ["Sichtweite moderat halten (z. B. 12 Chunks)", "Leistungs-Mods wie „Sodium“ (Fabric) bringen oft deutlich mehr FPS", "VSync aus"] },
};

export function tipsFor(exe: string): GameTips {
  return TIPS[exe] ?? { reflex: false, tips: COMMON };
}
