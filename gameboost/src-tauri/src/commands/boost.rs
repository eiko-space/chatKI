use crate::boost::{self, protect, BoostEngine, BoostStatus, LogEntry, TweakInfo, TweakResult};
use crate::settings::{Settings, SettingsStore};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_boost_catalog() -> Vec<TweakInfo> {
    boost::catalog().to_vec()
}

#[tauri::command]
pub fn get_boost_status(engine: State<'_, BoostEngine>) -> BoostStatus {
    engine.status()
}

/// Startet den Boost. Läuft in einem eigenen Thread, damit die Oberfläche flüssig bleibt.
#[tauri::command]
pub async fn apply_boost(app: AppHandle) -> Result<BoostStatus, String> {
    tauri::async_runtime::spawn_blocking(move || crate::boost_now(&app)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn revert_boost(app: AppHandle) -> Result<Vec<TweakResult>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::revert_now(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_restore_point(app: AppHandle) -> Result<String, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let engine = app.state::<BoostEngine>();
        let res = boost::tweaks::create_restore_point("GameBoost – vor dem ersten Boost");
        match &res {
            Ok(m) => {
                engine.log("restore_point", "Wiederherstellungspunkt", "applied", m);
                let _ = app.state::<SettingsStore>().update(|s| s.restore_point_created_at = Some(crate::storage::unix_now()));
            }
            Err(e) => engine.log("restore_point", "Wiederherstellungspunkt", "failed", e),
        }
        let _ = app.emit("settings-changed", ());
        res
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_change_log(engine: State<'_, BoostEngine>) -> Vec<LogEntry> {
    engine.change_log(300)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedApp {
    exe: String,
    reason: &'static str,
}

/// Laufende Programme, die der Boost garantiert nicht anfasst (ohne Windows-Kernprozesse).
#[tauri::command]
pub fn get_protected_running() -> Vec<ProtectedApp> {
    let mic = protect::microphone_in_use();
    let mut out: Vec<ProtectedApp> = Vec::new();
    #[cfg(windows)]
    for p in crate::win::process::list() {
        if let Some(r) = protect::check(&p.name, &mic) {
            if r != protect::Reason::System && !out.iter().any(|o| o.exe.eq_ignore_ascii_case(&p.name)) {
                out.push(ProtectedApp { exe: p.name, reason: r.label() });
            }
        }
    }
    // Mikrofon-Nutzer zuerst, dann alphabetisch
    out.sort_by(|a, b| (a.reason != "Mikrofon gerade aktiv").cmp(&(b.reason != "Mikrofon gerade aktiv")).then(a.exe.to_lowercase().cmp(&b.exe.to_lowercase())));
    let _ = &mic;
    out
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    exe: String,
    memory_mb: Option<f32>,
}

/// Laufende Benutzerprogramme, die man in die Hintergrund-Liste aufnehmen kann.
#[tauri::command]
pub fn list_running_apps() -> Vec<RunningApp> {
    #[allow(unused_mut)]
    let mut out: Vec<RunningApp> = Vec::new();
    #[cfg(windows)]
    {
        use crate::win::process;
        let mic = protect::microphone_in_use();
        for p in process::list() {
            if protect::check(&p.name, &mic).is_some() || !process::in_my_session(p.pid) {
                continue;
            }
            let mem = process::memory_mb(p.pid);
            match out.iter_mut().find(|o| o.exe.eq_ignore_ascii_case(&p.name)) {
                Some(o) => o.memory_mb = Some(o.memory_mb.unwrap_or(0.0) + mem.unwrap_or(0.0)),
                None => out.push(RunningApp { exe: p.name, memory_mb: mem }),
            }
        }
        out.sort_by(|a, b| b.memory_mb.unwrap_or(0.0).total_cmp(&a.memory_mb.unwrap_or(0.0)));
    }
    out
}

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, store: State<'_, SettingsStore>, settings: Settings) -> Result<(), String> {
    store.save(settings)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// Vollbild-Optimierungen für eine .exe dauerhaft an/aus.
#[tauri::command]
pub fn set_fullscreen_optimizations(engine: State<'_, BoostEngine>, exe_path: String, disable: bool) -> Result<(), String> {
    let res = boost::tweaks::set_fullscreen_optimizations(&exe_path, disable);
    let msg = format!("{exe_path}: Vollbild-Optimierungen {}", if disable { "aus" } else { "an (Standard)" });
    engine.log("profile", "Vollbild-Optimierungen", if res.is_ok() { "applied" } else { "failed" }, &msg);
    res
}

fn game_paths(store: &SettingsStore) -> Vec<String> {
    store.get().profiles.into_iter().filter_map(|p| p.exe_path).collect()
}

#[tauri::command]
pub fn get_permanent(store: State<'_, SettingsStore>) -> Vec<boost::permanent::PermanentItem> {
    boost::permanent::status(&game_paths(&store))
}

#[tauri::command]
pub fn set_permanent(engine: State<'_, BoostEngine>, store: State<'_, SettingsStore>, id: String, on: bool) -> Result<String, String> {
    let res = boost::permanent::set(&id, on, &game_paths(&store));
    match &res {
        Ok(m) => engine.log("permanent", &id, "applied", m),
        Err(e) => engine.log("permanent", &id, "failed", e),
    }
    res
}
