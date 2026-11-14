//! Boost-Engine: wendet Tweaks an, speichert vorher den alten Zustand auf der Festplatte
//! und kann alles wieder rückgängig machen – auch nach einem Absturz oder Neustart.

pub mod permanent;
pub mod protect;
pub mod tweaks;

use crate::settings::Settings;
use crate::storage::{unix_now, write_json};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

// ------------------------------------------------------------------ Rückgängig-Daten

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RegValue {
    Dword(u32),
    Sz(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegEntry {
    pub path: String,
    pub name: String,
    /// `None` = Wert existierte vorher nicht (wird beim Zurücksetzen gelöscht)
    pub previous: Option<RegValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClosedApp {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind")]
pub enum Revert {
    PowerPlan { previous: String },
    Priority { pid: u32, exe: String, previous: u32 },
    ClosedApps { apps: Vec<ClosedApp> },
    Registry { entries: Vec<RegEntry> },
    Services { names: Vec<String> },
    Wlan { interfaces: Vec<(String, bool, bool)> },
    /// Mausbeschleunigung: [Schwelle1, Schwelle2, Beschleunigung]
    Mouse { previous: [i32; 3] },
    /// Energiespar-Werte im Energieplan: (Untergruppe, Einstellung, Netz, Akku)
    DevicePower { scheme: String, values: Vec<(String, String, u32, u32)> },
    StickyKeys { flags: u32 },
    /// Energiespar-Drosselung eines Prozesses wieder Windows überlassen
    PowerThrottle { pid: u32, exe: String },
    /// Prioritäten mehrerer Prozesse: (PID, Name, vorherige Priorität)
    Priorities { items: Vec<(u32, String, u32)> },
    /// Mehrere Änderungen eines Tweaks (werden rückwärts zurückgesetzt)
    Multi { parts: Vec<Revert> },
}

// ------------------------------------------------------------------ Katalog

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct TweakInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub details: &'static str,
    /// "fps" | "network"
    pub category: &'static str,
    pub needs_admin: bool,
    pub default_on: bool,
    /// Einmalige Aktion ohne Rückgängig (z. B. Speicher leeren)
    pub one_shot: bool,
}

const GAME_DVR: &[(&str, &str, u32)] = &[
    (r"System\GameConfigStore", "GameDVR_Enabled", 0),
    (r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 0),
    (r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "HistoricalCaptureEnabled", 0),
];
const GAME_MODE: &[(&str, &str, u32)] = &[(r"Software\Microsoft\GameBar", "AutoGameModeEnabled", 1)];

pub fn catalog() -> &'static [TweakInfo] {
    &[
        TweakInfo {
            id: "power_plan", title: "Energieplan „Höchstleistung“", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "CPU taktet nicht mehr zum Stromsparen herunter.",
            details: "Wechselt auf den Windows-Energieplan „Höchstleistung“ und danach zurück auf deinen bisherigen Plan. Hilft vor allem gegen kurze Ruckler, wenn die CPU aus dem Stromsparmodus hochtaktet. Bei Laptops steigt der Stromverbrauch.",
        },
        TweakInfo {
            id: "game_priority", title: "Spiel bevorzugen", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Hohe Priorität fürs Spiel, keine Energiespar-Drosselung.",
            details: "Setzt nur die Prozess-Priorität des laufenden Spiels auf „Hoch“ – wie im Task-Manager. Kein Eingriff in den Spielprozess. Manche Anti-Cheat-Systeme verbieten das; dann wird es übersprungen. Bringt am meisten, wenn die CPU stark ausgelastet ist.",
        },
        TweakInfo {
            id: "cpu_full", title: "CPU-Kerne immer wach", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Kein Kern-Parken, volle Mindestleistung, schneller Turbo.",
            details: "Setzt im aktiven Energieplan die minimale Prozessorleistung auf 100 %, verhindert das Abschalten (Parken) von CPU-Kernen und stellt den Turbo auf „Aggressiv“. Weniger Ruckler, wenn das Spiel plötzlich mehr Leistung braucht.",
        },
        TweakInfo {
            id: "background_priority", title: "Browser & Co. drosseln", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Browser laufen weiter, bekommen aber weniger CPU als dein Spiel.",
            details: "Senkt die Priorität von Browsern (Chrome, Edge, Firefox, Opera, Brave …) auf „Niedriger als normal“. Nichts wird beendet – Musik und Videos laufen weiter.",
        },
        TweakInfo {
            id: "close_apps", title: "Hintergrund-Apps beenden", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Beendet nur Apps aus deiner Liste – und startet sie danach wieder.",
            details: "Beendet die Programme aus deiner Liste (Einstellungen → Hintergrund-Apps). Nie beendet werden: Virenschutz, Anti-Cheat, Windows-Prozesse, Voice-Chat (Discord, TeamSpeak …), Spiele-Launcher, Maus-/Headset-Software und alles, was gerade dein Mikrofon benutzt.",
        },
        TweakInfo {
            id: "game_mode", title: "Windows-Spielmodus an", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Windows hält Updates und Hintergrundarbeit vom Spiel fern.",
            details: "Schaltet den Windows-Spielmodus ein (Einstellungen → Spielen → Spielmodus). Bei den meisten PCs ist er schon an – dann wird nichts geändert.",
        },
        TweakInfo {
            id: "game_dvr", title: "Xbox-Aufnahme im Hintergrund aus", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Keine ständige Videoaufzeichnung im Hintergrund.",
            details: "Schaltet die Hintergrundaufnahme der Xbox Game Bar (Game DVR) aus. Sie zeichnet sonst ständig die letzten Minuten auf und kostet GPU-Leistung. Die Game Bar selbst bleibt nutzbar.",
        },
        TweakInfo {
            id: "device_power", title: "Energiesparen von USB, PCIe & WLAN aus", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Maus, Grafikkarte und Netzwerk werden nicht mehr schlafen gelegt.",
            details: "Schaltet im aktiven Energieplan das selektive USB-Energiesparen, die PCI-Express-Verbindungsenergieverwaltung und den WLAN-Energiesparmodus aus. Verhindert kurze Aufwach-Verzögerungen.",
        },
        TweakInfo {
            id: "mouse_accel", title: "Mausbeschleunigung aus", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Gleiche Handbewegung = gleiche Zielbewegung.",
            details: "Schaltet „Zeigerbeschleunigung verbessern“ aus. Dadurch hängt der Weg des Mauszeigers nur noch von der Strecke ab, nicht vom Tempo – wichtig für präzises Zielen (in Spielen mit Raw Input ohne Wirkung).",
        },
        TweakInfo {
            id: "no_interruptions", title: "Keine Störungen im Spiel", category: "fps", needs_admin: false, default_on: true, one_shot: false,
            summary: "Keine Windows-Popups und kein Einrastetasten-Fenster beim Shift-Spammen.",
            details: "Blendet Windows-Benachrichtigungen aus und deaktiviert das Tastenkürzel für die Einrastfunktion (5× Shift), das sonst mitten im Spiel ein Fenster öffnet.",
        },
        TweakInfo {
            id: "standby_list", title: "Standby-Speicher leeren", category: "fps", needs_admin: true, default_on: false, one_shot: true,
            summary: "Gibt zwischengespeicherten RAM frei. Hilft nur bei knappem RAM.",
            details: "Leert einmalig die Standby-Liste des Arbeitsspeichers (wie „RAMMap“ oder „ISLC“). Das hilft nur, wenn dein freier Arbeitsspeicher knapp ist – sonst bringt es nichts. Die App zeigt dir danach die echten Werte vorher/nachher.",
        },
        TweakInfo {
            id: "pause_updates", title: "Updates & Windows-Suche pausieren", category: "network", needs_admin: true, default_on: true, one_shot: false,
            summary: "Keine Downloads und keine Festplatten-Indexierung im Spiel.",
            details: "Hält die Dienste „Windows Update“, „Hintergrundübertragung (BITS)“ und „Übermittlungsoptimierung“ an und startet sie nach dem Boost wieder. Downloads in Steam/Epic pausierst du am besten im Launcher selbst – GameBoost beendet Launcher bewusst nicht.",
        },
        TweakInfo {
            id: "wlan_low_latency", title: "WLAN-Hintergrundsuche aus", category: "network", needs_admin: false, default_on: true, one_shot: false,
            summary: "Verhindert regelmäßige Ping-Spitzen durch WLAN-Netzsuche.",
            details: "Windows sucht im Hintergrund regelmäßig nach WLAN-Netzen. Währenddessen steigt der Ping kurz an. GameBoost schaltet diese Suche während des Spiels aus und aktiviert den Streaming-Modus. Bei LAN-Kabel passiert nichts.",
        },
    ]
}

// ------------------------------------------------------------------ Zustand & Ergebnisse

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TweakResult {
    pub id: String,
    pub title: String,
    /// "applied" | "unchanged" | "skipped" | "failed" | "reverted"
    pub status: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct AppliedTweak {
    id: String,
    title: String,
    revert: Revert,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct BoostState {
    started_at: u64,
    game: Option<String>,
    applied: Vec<AppliedTweak>,
    results: Vec<TweakResult>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BoostStatus {
    pub active: bool,
    pub started_at: Option<u64>,
    pub game: Option<String>,
    pub results: Vec<TweakResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub time: u64,
    pub action: String,
    pub title: String,
    pub status: String,
    pub message: String,
}

pub struct GameTarget {
    pub pid: u32,
    pub exe: String,
    pub name: String,
}

pub struct BoostEngine {
    state_path: PathBuf,
    log_path: PathBuf,
    state: Mutex<Option<BoostState>>,
}

impl BoostEngine {
    pub fn load(dir: &std::path::Path) -> Self {
        let state_path = dir.join("boost-state.json");
        // Existiert die Datei noch, wurde ein Boost nicht beendet (z. B. Absturz) → bleibt aktiv, damit man zurücksetzen kann.
        let state = std::fs::read_to_string(&state_path).ok().and_then(|s| serde_json::from_str(&s).ok());
        Self { state_path, log_path: dir.join("changes.log"), state: Mutex::new(state) }
    }

    pub fn status(&self) -> BoostStatus {
        match &*self.state.lock().unwrap() {
            Some(s) => BoostStatus { active: true, started_at: Some(s.started_at), game: s.game.clone(), results: s.results.clone() },
            None => BoostStatus { active: false, started_at: None, game: None, results: Vec::new() },
        }
    }

    fn persist(&self, s: &BoostState) {
        if let Err(e) = write_json(&self.state_path, s) {
            eprintln!("Boost-Zustand konnte nicht gespeichert werden: {e}");
        }
    }

    pub fn log(&self, action: &str, title: &str, status: &str, message: &str) {
        let entry = LogEntry { time: unix_now(), action: action.into(), title: title.into(), status: status.into(), message: message.into() };
        if let Some(dir) = self.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.log_path) {
            let _ = writeln!(f, "{}", serde_json::to_string(&entry).unwrap_or_default());
        }
    }

    pub fn change_log(&self, limit: usize) -> Vec<LogEntry> {
        let text = std::fs::read_to_string(&self.log_path).unwrap_or_default();
        let mut entries: Vec<LogEntry> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        entries.reverse();
        entries.truncate(limit);
        entries
    }

    pub fn apply(&self, settings: &Settings, game: Option<GameTarget>, admin: bool, progress: &dyn Fn(usize, usize, &str)) -> Result<BoostStatus, String> {
        // Sperre für die ganze Dauer: verhindert, dass zwei Boosts gleichzeitig starten.
        let mut guard = self.state.lock().unwrap();
        if guard.is_some() {
            return Err("Der Boost ist bereits aktiv.".into());
        }
        let mic = protect::microphone_in_use();
        let mut state = BoostState { started_at: unix_now(), game: game.as_ref().map(|g| g.name.clone()), applied: Vec::new(), results: Vec::new() };
        self.log("boost", "Boost gestartet", "info", &state.game.clone().map(|g| format!("Spiel: {g}")).unwrap_or_else(|| "Kein Spiel erkannt".into()));

        let enabled = |id: &str| match game.as_ref().and_then(|g| settings.profiles.iter().find(|p| p.exe.eq_ignore_ascii_case(&g.exe))).and_then(|p| p.tweaks.as_ref()) {
            Some(per_game) => per_game.iter().any(|t| t == id),
            None => settings.enabled_tweaks.iter().any(|t| t == id),
        };

        let todo: Vec<&TweakInfo> = catalog().iter().filter(|t| enabled(t.id)).collect();
        for (n, t) in todo.iter().enumerate() {
            progress(n, todo.len(), t.title);
            let outcome: tweaks::Applied = if t.needs_admin && !admin {
                Err("SKIP:Benötigt Administratorrechte.".into())
            } else {
                match t.id {
                    "power_plan" => tweaks::power_plan(),
                    "game_priority" => match &game {
                        Some(g) => tweaks::game_priority(g.pid, &g.exe),
                        None => Err("SKIP:Kein laufendes Spiel erkannt.".into()),
                    },
                    "cpu_full" => tweaks::cpu_full(),
                    "background_priority" => tweaks::background_priority(&mic),
                    "close_apps" => {
                        let apps: Vec<String> = settings.background_apps.iter().filter(|a| a.enabled).map(|a| a.exe.clone()).collect();
                        tweaks::close_apps(&apps, &mic)
                    }
                    "game_mode" => tweaks::registry(GAME_MODE),
                    "game_dvr" => tweaks::registry(GAME_DVR),
                    "standby_list" => tweaks::purge_standby_list(),
                    "device_power" => tweaks::device_power(),
                    "mouse_accel" => tweaks::mouse_accel(),
                    "no_interruptions" => tweaks::no_interruptions(),
                    "pause_updates" => tweaks::stop_update_services(),
                    "wlan_low_latency" => tweaks::wlan_low_latency(),
                    _ => Err("SKIP:Unbekannter Tweak.".into()),
                }
            };
            let result = match outcome {
                Ok((message, Some(revert))) => {
                    state.applied.push(AppliedTweak { id: t.id.into(), title: t.title.into(), revert });
                    TweakResult { id: t.id.into(), title: t.title.into(), status: "applied".into(), message }
                }
                Ok((message, None)) => {
                    let status = if t.one_shot { "applied" } else { "unchanged" };
                    TweakResult { id: t.id.into(), title: t.title.into(), status: status.into(), message }
                }
                Err(e) => match e.strip_prefix("SKIP:") {
                    Some(m) => TweakResult { id: t.id.into(), title: t.title.into(), status: "skipped".into(), message: m.into() },
                    None => TweakResult { id: t.id.into(), title: t.title.into(), status: "failed".into(), message: e },
                },
            };
            self.log("apply", t.title, &result.status, &result.message);
            state.results.push(result);
            // Nach jedem Schritt speichern: stürzt etwas ab, ist trotzdem alles rückgängig machbar.
            self.persist(&state);
        }
        progress(todo.len(), todo.len(), "Fertig");

        let status = BoostStatus { active: true, started_at: Some(state.started_at), game: state.game.clone(), results: state.results.clone() };
        *guard = Some(state);
        Ok(status)
    }

    pub fn revert(&self, progress: &dyn Fn(usize, usize, &str)) -> Vec<TweakResult> {
        let Some(state) = self.state.lock().unwrap().take() else { return Vec::new() };
        let mut results = Vec::new();
        let mut failed = Vec::new();
        let total = state.applied.len();
        for (n, a) in state.applied.iter().rev().enumerate() {
            progress(n, total, &a.title);
            let (status, message) = match tweaks::revert(&a.revert) {
                Ok(m) => ("reverted", m),
                Err(e) => {
                    failed.push(a.clone());
                    ("failed", e)
                }
            };
            self.log("revert", &a.title, status, &message);
            results.push(TweakResult { id: a.id.clone(), title: a.title.clone(), status: status.into(), message });
        }
        if failed.is_empty() {
            let _ = std::fs::remove_file(&self.state_path);
        } else {
            // Fehlgeschlagene Rücksetzungen bleiben gespeichert → erneuter Versuch möglich.
            let rest = BoostState { applied: failed, ..state };
            self.persist(&rest);
            *self.state.lock().unwrap() = Some(rest);
        }
        progress(total, total, "Fertig");
        self.log("boost", "Boost beendet", "info", "");
        results
    }
}

/// Sucht ein laufendes Spiel: zuerst eigene Profile, dann bekannte Spiele.
#[cfg(windows)]
pub fn find_running_game(settings: &Settings) -> Option<GameTarget> {
    let procs = crate::win::process::list();
    for p in &procs {
        if let Some(profile) = settings.profiles.iter().find(|g| g.exe.eq_ignore_ascii_case(&p.name)) {
            return Some(GameTarget { pid: p.pid, exe: p.name.clone(), name: profile.name.clone() });
        }
        if let Some(name) = crate::games::known_game(&p.name) {
            return Some(GameTarget { pid: p.pid, exe: p.name.clone(), name: name.into() });
        }
    }
    None
}

#[cfg(not(windows))]
pub fn find_running_game(_settings: &Settings) -> Option<GameTarget> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revert_state_roundtrip() {
        let r = Revert::Registry { entries: vec![RegEntry { path: "a".into(), name: "b".into(), previous: None }] };
        let json = serde_json::to_string(&r).unwrap();
        let back: Revert = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Revert::Registry { .. }));
    }

    #[test]
    fn catalog_ids_unique() {
        let mut ids: Vec<_> = catalog().iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), catalog().len());
    }

    #[test]
    fn engine_persists_and_logs() {
        let dir = std::env::temp_dir().join(format!("gb-test-{}", std::process::id()));
        let e = BoostEngine::load(&dir);
        assert!(!e.status().active);
        e.log("test", "Titel", "info", "Nachricht");
        assert_eq!(e.change_log(10).len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
