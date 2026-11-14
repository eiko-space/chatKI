use crate::boost::BoostEngine;
use crate::netdiag::{self, NetApp, RegionResult, TraceResult};
use crate::pccheck::{self, CheckReport};
use crate::telemetry::Hub;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn region_ping() -> Result<Vec<RegionResult>, String> {
    blocking(|| netdiag::region_ping(5)).await
}

#[tauri::command]
pub async fn traceroute(target: String) -> Result<TraceResult, String> {
    blocking(move || netdiag::traceroute(target.trim(), 3)).await?
}

#[tauri::command]
pub async fn net_apps() -> Result<Vec<NetApp>, String> {
    blocking(netdiag::apps_with_connections).await
}

#[tauri::command]
pub async fn run_pc_check(app: AppHandle) -> Result<CheckReport, String> {
    let snap = app.state::<Arc<Hub>>().latest();
    let gpu_name = snap.as_ref().and_then(|s| s.gpu.name.clone());
    let gpu_temp = snap.as_ref().and_then(|s| s.gpu.temperature);
    let wlan = snap.as_ref().and_then(|s| s.network.link.connection) == Some("wlan");
    blocking(move || pccheck::run_checks(gpu_name, gpu_temp, wlan)).await
}

#[tauri::command]
pub async fn run_pc_fix(app: AppHandle, id: String) -> Result<String, String> {
    blocking(move || {
        let res = pccheck::run_fix(&id);
        let engine = app.state::<BoostEngine>();
        match &res {
            Ok(m) => engine.log("pc_check", &format!("PC-Check: {id}"), "applied", m),
            Err(e) => engine.log("pc_check", &format!("PC-Check: {id}"), "failed", e),
        }
        res
    })
    .await?
}

#[tauri::command]
pub async fn lag_test(app: AppHandle) -> Result<crate::lagtest::LagResult, String> {
    use tauri::Emitter;
    blocking(move || {
        crate::lagtest::run(&|pct, step| {
            let _ = app.emit("lagtest-progress", serde_json::json!({ "percent": pct, "step": step }));
        })
    })
    .await?
}

#[tauri::command]
pub fn games_storage(store: tauri::State<'_, crate::settings::SettingsStore>) -> Vec<crate::gametools::InstalledGame> {
    crate::gametools::running_games_storage(&store.get().profiles)
}

#[tauri::command]
pub fn active_overlays() -> Vec<crate::gametools::Overlay> {
    crate::gametools::active_overlays()
}

#[tauri::command]
pub fn shader_caches() -> Vec<crate::gametools::ShaderCache> {
    crate::gametools::shader_caches()
}

#[tauri::command]
pub async fn clear_shader_caches(app: AppHandle) -> Result<String, String> {
    blocking(move || {
        let res = crate::gametools::clear_shader_caches();
        let engine = app.state::<BoostEngine>();
        match &res {
            Ok(m) => engine.log("shader", "Shader-Cache geleert", "applied", m),
            Err(e) => engine.log("shader", "Shader-Cache geleert", "failed", e),
        }
        res
    })
    .await?
}

#[tauri::command]
pub fn game_configs() -> Vec<crate::gameconfig::ConfigGame> {
    crate::gameconfig::status()
}

#[tauri::command]
pub fn apply_game_config(app: AppHandle, id: String) -> Result<String, String> {
    let res = crate::gameconfig::apply(&id);
    let engine = app.state::<BoostEngine>();
    match &res {
        Ok(m) => engine.log("gameconfig", &format!("Spiel-Einstellungen: {id}"), "applied", m),
        Err(e) => engine.log("gameconfig", &format!("Spiel-Einstellungen: {id}"), "failed", e),
    }
    res
}

#[tauri::command]
pub fn restore_game_config(app: AppHandle, id: String) -> Result<String, String> {
    let res = crate::gameconfig::restore(&id);
    app.state::<BoostEngine>().log("gameconfig", &format!("Spiel-Einstellungen zurück: {id}"), if res.is_ok() { "reverted" } else { "failed" }, res.as_ref().map(|s| s.as_str()).unwrap_or_else(|e| e));
    res
}

#[tauri::command]
pub fn fps_advice(app: AppHandle) -> crate::fpslimit::FpsAdvice {
    let snap = app.state::<Arc<Hub>>().latest();
    let fps = snap.as_ref().and_then(|s| s.fps.as_ref().map(|f| f.average));
    let gpu = snap.as_ref().and_then(|s| s.gpu.usage);
    let cpu = snap.as_ref().and_then(|s| s.system.cpu_usage);
    let hz = crate::pccheck::primary_monitor_hz();
    crate::fpslimit::advise(fps, gpu, cpu, hz)
}
