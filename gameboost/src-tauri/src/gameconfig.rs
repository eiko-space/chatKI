//! Schreibt kompetitive Einstellungen direkt in die Konfig-Dateien der Spiele.
//! Immer mit Backup der Originaldatei – jederzeit zurücksetzbar.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigGame {
    pub id: &'static str,
    pub name: &'static str,
    /// true = Konfig-Datei gefunden
    pub available: bool,
    /// true = Backup existiert (Optimierung ist aktiv)
    pub optimized: bool,
    pub summary: &'static str,
    pub changes: Vec<&'static str>,
}

fn local() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

/// Pfad der Konfig-Datei je Spiel.
fn config_path(id: &str) -> Option<PathBuf> {
    match id {
        "fortnite" => local().map(|l| l.join(r"FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini")),
        _ => None,
    }
}

fn backup_path(id: &str) -> Option<PathBuf> {
    config_path(id).map(|p| p.with_extension("ini.gameboost-backup"))
}

const GAMES: &[(&str, &str, &str, &[&str])] = &[(
    "fortnite",
    "Fortnite",
    "Maximale FPS, kompetitive Grafik",
    &[
        "Grafikqualität auf niedrig (mehr FPS, bessere Sicht)",
        "Sichtweite „Episch“ (Gegner früher sehen)",
        "Bewegungsunschärfe aus",
        "VSync aus, FPS unbegrenzt",
        "Schatten aus",
    ],
)];

pub fn status() -> Vec<ConfigGame> {
    GAMES
        .iter()
        .map(|(id, name, summary, changes)| ConfigGame {
            id,
            name,
            available: config_path(id).is_some_and(|p| p.is_file()),
            optimized: backup_path(id).is_some_and(|p| p.is_file()),
            summary,
            changes: changes.to_vec(),
        })
        .collect()
}

/// Setzt `Key=Value` in der passenden `[Abschnitt]`-Sektion einer INI-Datei.
pub fn set_ini(content: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let header = format!("[{section}]");
    let sec_start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&header));

    let Some(start) = sec_start else {
        // Sektion fehlt → am Ende anfügen
        if !lines.is_empty() && !lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(format!("{key}={value}"));
        return lines.join("\r\n");
    };
    // Ende der Sektion suchen
    let end = lines.iter().skip(start + 1).position(|l| l.trim_start().starts_with('[')).map(|i| start + 1 + i).unwrap_or(lines.len());
    let prefix = format!("{}=", key.to_ascii_lowercase());
    if let Some(pos) = lines[start + 1..end].iter().position(|l| l.trim().to_ascii_lowercase().starts_with(&prefix)) {
        lines[start + 1 + pos] = format!("{key}={value}");
    } else {
        lines.insert(end, format!("{key}={value}"));
    }
    lines.join("\r\n")
}

const FORTNITE: &[(&str, &str, &str)] = &[
    ("ScalabilityGroups", "sg.ResolutionQuality", "100"),
    ("ScalabilityGroups", "sg.ViewDistanceQuality", "3"),
    ("ScalabilityGroups", "sg.ShadowQuality", "0"),
    ("ScalabilityGroups", "sg.PostProcessQuality", "0"),
    ("ScalabilityGroups", "sg.TextureQuality", "1"),
    ("ScalabilityGroups", "sg.EffectsQuality", "0"),
    ("ScalabilityGroups", "sg.FoliageQuality", "0"),
    ("/Script/FortniteGame.FortGameUserSettings", "bMotionBlur", "False"),
    ("/Script/FortniteGame.FortGameUserSettings", "FrameRateLimit", "0.000000"),
];

pub fn apply(id: &str) -> Result<String, String> {
    let path = config_path(id).ok_or("Unbekanntes Spiel.")?;
    if !path.is_file() {
        return Err("Konfig-Datei nicht gefunden. Starte das Spiel einmal und schließe es wieder.".into());
    }
    let backup = backup_path(id).unwrap();
    let original = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    // Backup nur beim ersten Mal anlegen
    if !backup.is_file() {
        std::fs::write(&backup, &original).map_err(|e| format!("Backup fehlgeschlagen: {e}"))?;
    }
    let mut content = original;
    if id == "fortnite" {
        for (sec, key, val) in FORTNITE {
            content = set_ini(&content, sec, key, val);
        }
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    // Schreibschutz, damit das Spiel die Werte nicht gleich überschreibt
    set_readonly(&path, true);
    Ok("Eingestellt. Die Datei ist schreibgeschützt, damit die Werte bleiben. Im Spiel nichts an der Grafik ändern.".into())
}

pub fn restore(id: &str) -> Result<String, String> {
    let path = config_path(id).ok_or("Unbekanntes Spiel.")?;
    let backup = backup_path(id).ok_or("Kein Backup.")?;
    if !backup.is_file() {
        return Err("Es gibt kein Backup zum Wiederherstellen.".into());
    }
    set_readonly(&path, false);
    let content = std::fs::read_to_string(&backup).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    std::fs::remove_file(&backup).map_err(|e| e.to_string())?;
    Ok("Originaleinstellungen wiederhergestellt.".into())
}

#[cfg(windows)]
fn set_readonly(path: &std::path::Path, ro: bool) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_readonly(ro);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(windows))]
fn set_readonly(_path: &std::path::Path, _ro: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_updates_existing_key() {
        let c = "[ScalabilityGroups]\r\nsg.ShadowQuality=3\r\nsg.TextureQuality=2\r\n";
        let out = set_ini(c, "ScalabilityGroups", "sg.ShadowQuality", "0");
        assert!(out.contains("sg.ShadowQuality=0"));
        assert!(out.contains("sg.TextureQuality=2"));
        assert_eq!(out.matches("sg.ShadowQuality").count(), 1);
    }

    #[test]
    fn ini_adds_missing_key_and_section() {
        let c = "[ScalabilityGroups]\r\nsg.ShadowQuality=3\r\n";
        let out = set_ini(c, "ScalabilityGroups", "sg.ViewDistanceQuality", "3");
        assert!(out.contains("sg.ViewDistanceQuality=3"));
        let out2 = set_ini(c, "/Script/FortniteGame.FortGameUserSettings", "bMotionBlur", "False");
        assert!(out2.contains("[/Script/FortniteGame.FortGameUserSettings]"));
        assert!(out2.contains("bMotionBlur=False"));
    }

    #[test]
    fn case_insensitive_key_match() {
        let c = "[Sec]\r\nFrameRateLimit=60.000000\r\n";
        let out = set_ini(c, "Sec", "FrameRateLimit", "0.000000");
        assert_eq!(out.matches("FrameRateLimit").count(), 1);
        assert!(out.contains("FrameRateLimit=0.000000"));
    }
}
