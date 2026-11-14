//! Spiele-Werkzeuge: installierte Spiele finden, Overlays erkennen, Shader-Cache leeren.

#[cfg(windows)]
use crate::storage_info::is_hdd;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub name: String,
    pub exe: String,
    pub path: String,
    pub drive: String,
    /// true = langsame Festplatte (HDD), false = SSD, None = unbekannt
    pub on_hdd: Option<bool>,
}

/// Erkennt bei laufenden Spielen das Laufwerk und ob es eine HDD ist.
#[cfg(windows)]
pub fn running_games_storage(profiles: &[crate::settings::GameProfile]) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    for p in crate::win::process::list() {
        let Some(profile) = profiles.iter().find(|g| g.exe.eq_ignore_ascii_case(&p.name)) else { continue };
        let Some(path) = crate::win::process::path(p.pid) else { continue };
        let drive = path.chars().next().filter(|c| c.is_ascii_alphabetic()).unwrap_or('C').to_ascii_uppercase();
        if out.iter().any(|g: &InstalledGame| g.exe.eq_ignore_ascii_case(&p.name)) {
            continue;
        }
        out.push(InstalledGame {
            name: profile.name.clone(),
            exe: p.name.clone(),
            path,
            drive: format!("{drive}:"),
            on_hdd: is_hdd(drive),
        });
    }
    out
}

#[cfg(not(windows))]
pub fn running_games_storage(_p: &[crate::settings::GameProfile]) -> Vec<InstalledGame> {
    Vec::new()
}

// ---------------------------------------------------------------- Overlays

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    pub exe: String,
    pub name: &'static str,
    /// Wo man es abschaltet
    pub how: &'static str,
}

/// Bekannte In-Game-Overlays (Prozessname → Anzeigename + Abschaltweg).
const OVERLAYS: &[(&str, &str, &str)] = &[
    ("GameOverlayUI.exe", "Steam-Overlay", "Steam → Einstellungen → Im Spiel → „Steam-Overlay im Spiel aktivieren“ aus"),
    ("Discord.exe", "Discord-Overlay", "Discord → Einstellungen → Spiel-Overlay → „In-Game-Overlay aktivieren“ aus"),
    ("nvcontainer.exe", "NVIDIA-Overlay (GeForce Experience / App)", "NVIDIA App → Einstellungen → „Im Spiel-Overlay“ aus"),
    ("EpicGamesLauncher.exe", "Epic-Overlay", "Epic Games Launcher → Einstellungen → „Enable overlay“ aus"),
    ("Spotify.exe", "Spotify-Overlay", "Spotify → Einstellungen → „Spiele-Overlay anzeigen“ aus"),
    ("rsi.exe", "Razer-Overlay", "Razer Synapse → entsprechendes Overlay aus"),
    ("MSIAfterburner.exe", "RivaTuner/Afterburner-Overlay", "Nur, wenn du es nicht brauchst – zeigt FPS/Stats an"),
    ("ANSEL", "NVIDIA Ansel", "In der NVIDIA App deaktivierbar"),
];

#[cfg(windows)]
pub fn active_overlays() -> Vec<Overlay> {
    let running = crate::win::process::list();
    OVERLAYS
        .iter()
        .filter(|(exe, _, _)| running.iter().any(|p| p.name.eq_ignore_ascii_case(exe)))
        .map(|(exe, name, how)| Overlay { exe: exe.to_string(), name, how })
        .collect()
}

#[cfg(not(windows))]
pub fn active_overlays() -> Vec<Overlay> {
    Vec::new()
}

// ---------------------------------------------------------------- Shader-Cache

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ShaderCache {
    pub label: String,
    pub path: String,
    pub size_mb: f32,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                total += dir_size(&e.path());
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn cache_dirs() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let temp = std::env::var_os("TEMP").map(PathBuf::from);
    if let Some(l) = &local {
        v.push(("NVIDIA DXCache".into(), l.join("NVIDIA").join("DXCache")));
        v.push(("NVIDIA GLCache".into(), l.join("NVIDIA").join("GLCache")));
        v.push(("D3D-Shader-Cache".into(), l.join("D3DSCache")));
        v.push(("AMD DxCache".into(), l.join("AMD").join("DxCache")));
        v.push(("AMD GLCache".into(), l.join("AMD").join("GLCache")));
    }
    if let Some(t) = &temp {
        v.push(("NVIDIA Temp-Cache".into(), t.join("NVIDIA Corporation").join("NV_Cache")));
    }
    v
}

pub fn shader_caches() -> Vec<ShaderCache> {
    cache_dirs()
        .into_iter()
        .filter(|(_, p)| p.is_dir())
        .map(|(label, path)| ShaderCache { label, size_mb: dir_size(&path) as f32 / 1_048_576.0, path: path.to_string_lossy().into() })
        .filter(|c| c.size_mb > 0.1)
        .collect()
}

/// Leert die Shader-Caches. Windows baut sie beim nächsten Spielen neu auf.
pub fn clear_shader_caches() -> Result<String, String> {
    let mut freed = 0.0f32;
    let mut count = 0;
    for (_, path) in cache_dirs().into_iter().filter(|(_, p)| p.is_dir()) {
        let before = dir_size(&path) as f32 / 1_048_576.0;
        if let Ok(entries) = std::fs::read_dir(&path) {
            for e in entries.flatten() {
                // Dateien löschen, Unterordner leeren – der Cache-Ordner selbst bleibt.
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let _ = std::fs::remove_dir_all(e.path());
                } else {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let after = dir_size(&path) as f32 / 1_048_576.0;
        if before - after > 0.1 {
            freed += before - after;
            count += 1;
        }
    }
    if count == 0 {
        Ok("Es gab nichts zu leeren.".into())
    } else {
        Ok(format!("{count} Cache(s) geleert, {freed:.0} MB frei. Beim nächsten Spielstart ruckelt es kurz, während der Cache neu entsteht – danach läuft es runder."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_struct_builds() {
        assert_eq!(OVERLAYS.len(), 8);
        assert!(OVERLAYS.iter().any(|(e, _, _)| *e == "Discord.exe"));
    }
}
