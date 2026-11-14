//! Dauerhafte Optimierungen: wirken ab dem nächsten Spielstart und bleiben, bis man sie ausschaltet.

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PermanentItem {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: String,
    pub enabled: bool,
    pub available: bool,
}

/// Setzt `key=value;` in einer DirectX-Einstellungszeichenkette (andere Einträge bleiben erhalten).
pub fn set_dx_option(current: &str, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = current.split(';').map(str::trim).filter(|p| !p.is_empty() && !p.starts_with(&format!("{key}="))).map(String::from).collect();
    parts.push(format!("{key}={value}"));
    parts.join(";") + ";"
}

pub fn dx_option(current: &str, key: &str) -> Option<String> {
    current.split(';').map(str::trim).find_map(|p| p.strip_prefix(&format!("{key}=")).map(String::from))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const DX: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
    const GLOBAL: &str = "DirectXUserGlobalSettings";

    fn key() -> Result<RegKey, String> {
        RegKey::predef(HKEY_CURRENT_USER).create_subkey(DX).map(|(k, _)| k).map_err(|e| e.to_string())
    }

    const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

    /// Spiele-Programme, die beim Start hohe CPU-/Festplatten-Priorität bekommen (ohne javaw.exe – das wäre jedes Java-Programm).
    fn priority_exes() -> Vec<&'static str> {
        crate::games::KNOWN_GAMES.iter().flat_map(|g| g.exes.iter().copied()).filter(|e| !e.eq_ignore_ascii_case("javaw.exe")).collect()
    }

    fn priority_enabled() -> bool {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        priority_exes().iter().all(|e| {
            hklm.open_subkey(format!(r"{IFEO}\{e}\PerfOptions")).ok().and_then(|k| k.get_value::<u32, _>("CpuPriorityClass").ok()) == Some(3)
        })
    }

    fn set_priority(on: bool) -> Result<String, String> {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        if !crate::elevation::is_elevated() {
            return Err("Benötigt Administratorrechte.".into());
        }
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for e in priority_exes() {
            let path = format!(r"{IFEO}\{e}\PerfOptions");
            if on {
                let (k, _) = hklm.create_subkey(&path).map_err(|e| e.to_string())?;
                k.set_value("CpuPriorityClass", &3u32).map_err(|e| e.to_string())?; // 3 = Hoch
                k.set_value("IoPriority", &3u32).map_err(|e| e.to_string())?; // 3 = Hoch (Festplatte)
            } else {
                let _ = hklm.delete_subkey(&path);
            }
        }
        Ok(if on { "Eingeschaltet – deine Spiele starten ab jetzt immer mit hoher CPU- und Festplatten-Priorität.".into() } else { "Ausgeschaltet.".into() })
    }

    pub fn status(game_paths: &[String]) -> Vec<PermanentItem> {
        let k = key().ok();
        let global: String = k.as_ref().and_then(|k| k.get_value(GLOBAL).ok()).unwrap_or_default();
        let gpu_set = game_paths.iter().filter(|p| k.as_ref().and_then(|k| k.get_value::<String, _>(p.as_str()).ok()).is_some_and(|v| dx_option(&v, "GpuPreference").as_deref() == Some("2"))).count();
        vec![
            PermanentItem {
                id: "start_priority",
                title: "Spiele starten mit hoher Priorität",
                summary: "Hohe CPU- und Festplatten-Priorität ab Spielstart – weniger Nachlade-Ruckler (z. B. in Fortnite Creative). Braucht Admin-Rechte.".into(),
                enabled: priority_enabled(),
                available: crate::elevation::is_elevated() || priority_enabled(),
            },
            PermanentItem {
                id: "windowed_flip",
                title: "Optimierungen für Spiele im Fenstermodus",
                summary: "Weniger Input-Delay in „Randloses Fenster“ (Windows 11, DirectX 10/11).".into(),
                enabled: dx_option(&global, "SwapEffectUpgradeEnable").as_deref() == Some("1"),
                available: true,
            },
            PermanentItem {
                id: "gpu_high_perf",
                title: "Spiele immer auf der starken Grafikkarte",
                summary: if game_paths.is_empty() {
                    "Starte deine Spiele einmal mit GameBoost, dann kann es aktiviert werden.".into()
                } else {
                    format!("Für {} erkannte(s) Spiel(e). Wichtig bei Laptops mit zwei Grafikchips.", game_paths.len())
                },
                enabled: !game_paths.is_empty() && gpu_set == game_paths.len(),
                available: !game_paths.is_empty(),
            },
        ]
    }

    pub fn set(id: &str, on: bool, game_paths: &[String]) -> Result<String, String> {
        let k = key()?;
        match id {
            "start_priority" => set_priority(on),
            "windowed_flip" => {
                let cur: String = k.get_value(GLOBAL).unwrap_or_default();
                k.set_value(GLOBAL, &set_dx_option(&cur, "SwapEffectUpgradeEnable", if on { "1" } else { "0" })).map_err(|e| e.to_string())?;
                Ok(if on { "Eingeschaltet – wirkt ab dem nächsten Spielstart." } else { "Ausgeschaltet." }.into())
            }
            "gpu_high_perf" => {
                for p in game_paths {
                    let cur: String = k.get_value(p.as_str()).unwrap_or_default();
                    k.set_value(p.as_str(), &set_dx_option(&cur, "GpuPreference", if on { "2" } else { "0" })).map_err(|e| e.to_string())?;
                }
                Ok(if on { format!("Für {} Spiel(e) eingeschaltet – wirkt ab dem nächsten Spielstart.", game_paths.len()) } else { "Ausgeschaltet (Windows entscheidet wieder selbst).".into() })
            }
            _ => Err("Unbekannte Optimierung.".into()),
        }
    }
}

#[cfg(windows)]
pub use imp::{set, status};

#[cfg(not(windows))]
pub fn status(_p: &[String]) -> Vec<PermanentItem> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn set(_id: &str, _on: bool, _p: &[String]) -> Result<String, String> {
    Err("Nur unter Windows verfügbar.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dx_option_merge_keeps_others() {
        let s = set_dx_option("VRROptimizeEnable=0;SwapEffectUpgradeEnable=0;", "SwapEffectUpgradeEnable", "1");
        assert_eq!(s, "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;");
        assert_eq!(dx_option(&s, "SwapEffectUpgradeEnable").as_deref(), Some("1"));
        assert_eq!(set_dx_option("", "GpuPreference", "2"), "GpuPreference=2;");
    }
}
