use crate::telemetry::Hub;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn start_fps_capture(hub: State<'_, Arc<Hub>>) -> Result<(), String> {
    hub.start_capture()
}

#[tauri::command]
pub fn stop_fps_capture(hub: State<'_, Arc<Hub>>) {
    hub.stop_capture();
}
