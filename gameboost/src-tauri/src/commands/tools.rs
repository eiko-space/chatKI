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
