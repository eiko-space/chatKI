use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: String,
    os_version: String,
    arch: String,
    is_elevated: bool,
    is_dev: bool,
}

/// Liefert echte Basisinformationen über App und System.
#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    let info = os_info::get();
    AppInfo {
        version: app.package_info().version.to_string(),
        os: info.os_type().to_string(),
        os_version: info.version().to_string(),
        arch: std::env::consts::ARCH.to_string(),
        is_elevated: crate::elevation::is_elevated(),
        is_dev: cfg!(debug_assertions),
    }
}

/// Startet die App mit Admin-Rechten neu (nur nach ausdrücklicher Bestätigung in der Oberfläche).
#[tauri::command]
pub fn restart_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    crate::elevation::relaunch_elevated()?;
    app.exit(0);
    Ok(())
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Startet GameBoost mit Windows (minimiert im Tray)?
#[tauri::command]
pub fn get_autostart() -> bool {
    #[cfg(windows)]
    {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey(RUN_KEY)
            .and_then(|k| k.get_value::<String, _>("GameBoost"))
            .is_ok()
    }
    #[cfg(not(windows))]
    false
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            key.set_value("GameBoost", &format!("\"{}\" --minimized", exe.display())).map_err(|e| e.to_string())
        } else {
            match key.delete_value("GameBoost") {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (enabled, RUN_KEY);
        Err("Nur unter Windows verfügbar.".into())
    }
}

/// Overlay-Fenster und Tastenkürzel nach geänderten Einstellungen neu anwenden.
#[tauri::command]
pub fn apply_overlay(app: tauri::AppHandle) -> Result<(), String> {
    crate::overlay::apply(&app)
}

/// Sperrt Hintergrundfunktionen (Tray-Boost, Auto-Boost), solange niemand angemeldet ist.
#[tauri::command]
pub fn set_locked(locked: bool) {
    crate::LOCKED.store(locked, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub fn get_vision() -> u32 {
    crate::vision::level()
}

#[tauri::command]
pub fn set_vision(level: u32) -> Result<(), String> {
    crate::vision::set_level(level.min(100))
}

/// Öffnet eine Windows-/Treiber-Einstellung (nur feste, bekannte Ziele).
#[tauri::command]
pub fn open_setting(target: String) -> Result<(), String> {
    let (program, arg) = match target.as_str() {
        "sound" => ("control.exe", "mmsys.cpl"),
        "display" => ("explorer.exe", "ms-settings:display"),
        "nvidia" => ("explorer.exe", r"shell:AppsFolder\NVIDIACorp.NVIDIAControlPanel_56jk63ar0cjw8!NVIDIACorp.NVIDIAControlPanel"),
        "amd" => ("explorer.exe", r"C:\Program Files\AMD\CNext\CNext\RadeonSoftware.exe"),
        _ => return Err("Unbekanntes Ziel.".into()),
    };
    std::process::Command::new(program).arg(arg).spawn().map(|_| ()).map_err(|e| e.to_string())
}
