//! Einstellungen, die dauerhaft gespeichert werden (lokal; mit Konto später auch in der Cloud).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundApp {
    pub exe: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameProfile {
    /// Anzeigename, z. B. „Counter-Strike 2“
    pub name: String,
    /// Prozessname, z. B. „cs2.exe“
    pub exe: String,
    /// Eigene Tweak-Auswahl für dieses Spiel (`None` = globale Auswahl)
    pub tweaks: Option<Vec<String>>,
    /// Vollbild-Optimierungen von Windows für dieses Spiel abschalten
    pub disable_fullscreen_optimizations: bool,
    /// Vollständiger Pfad der .exe (wird beim ersten Erkennen gespeichert; nötig für Vollbild-Optimierungen)
    #[serde(default)]
    pub exe_path: Option<String>,
    pub custom: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlaySettings {
    pub enabled: bool,
    pub position: String, // "top-left" | "top-right" | "bottom-left" | "bottom-right"
    pub scale: f32,
    pub hotkey: String,
    pub show_fps: bool,
    pub show_ping: bool,
    pub show_temp: bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            position: "top-left".into(),
            scale: 1.0,
            hotkey: "Ctrl+Shift+O".into(),
            show_fps: true,
            show_ping: true,
            show_temp: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub enabled_tweaks: Vec<String>,
    pub background_apps: Vec<BackgroundApp>,
    pub profiles: Vec<GameProfile>,
    pub auto_boost: bool,
    pub notifications: bool,
    pub temp_warning_c: f32,
    pub overlay: OverlaySettings,
    /// Zeitpunkt (Unix-Sekunden) des von GameBoost erstellten Wiederherstellungspunkts
    pub restore_point_created_at: Option<u64>,
    /// Hat der Nutzer den Hinweis vor dem ersten Boost bestätigt?
    pub first_boost_confirmed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_tweaks: crate::boost::catalog().iter().filter(|t| t.default_on).map(|t| t.id.to_string()).collect(),
            // Cloud-Sync-Programme: verursachen im Spiel Festplatten- und Netzwerklast, starten nach dem Spiel wieder.
            background_apps: ["OneDrive.exe", "Dropbox.exe", "GoogleDriveFS.exe", "CCXProcess.exe", "Creative Cloud.exe", "CoreSync.exe", "AdobeIPCBroker.exe"]
                .iter()
                .map(|e| BackgroundApp { exe: e.to_string(), enabled: true })
                .collect(),
            profiles: crate::games::KNOWN_GAMES
                .iter()
                .map(|g| GameProfile { name: g.name.into(), exe: g.exes[0].into(), tweaks: None, disable_fullscreen_optimizations: false, exe_path: None, custom: false })
                .collect(),
            auto_boost: false,
            notifications: true,
            temp_warning_c: 85.0,
            overlay: OverlaySettings::default(),
            restore_point_created_at: None,
            first_boost_confirmed: false,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    pub current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(dir: &std::path::Path) -> Self {
        let path = dir.join("settings.json");
        let current = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn save(&self, s: Settings) -> Result<(), String> {
        crate::storage::write_json(&self.path, &s)?;
        *self.current.lock().unwrap() = s;
        Ok(())
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut s = self.get();
        f(&mut s);
        self.save(s.clone())?;
        Ok(s)
    }
}
