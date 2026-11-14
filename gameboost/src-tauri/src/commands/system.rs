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
