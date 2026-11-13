use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: String,
    os_version: String,
    arch: String,
    is_elevated: bool,
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
        is_elevated: is_elevated(),
    }
}

/// Läuft der Prozess mit Administratorrechten?
/// Die App startet bewusst OHNE Admin-Rechte und fragt erst bei Bedarf (ab Phase 3).
#[cfg(windows)]
fn is_elevated() -> bool {
    // SAFETY: IsUserAnAdmin hat keine Parameter und keine Vorbedingungen.
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    false
}
