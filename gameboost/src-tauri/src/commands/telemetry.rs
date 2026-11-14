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

#[tauri::command]
pub fn start_measurement(hub: State<'_, Arc<Hub>>, label: String) -> Result<(), String> {
    hub.start_measurement(label)
}

#[tauri::command]
pub fn cancel_measurement(hub: State<'_, Arc<Hub>>) {
    hub.cancel_measurement();
}

#[tauri::command]
pub fn list_sessions(store: State<'_, crate::sessions::SessionStore>) -> Vec<crate::sessions::Session> {
    store.list()
}

#[tauri::command]
pub fn save_session(store: State<'_, crate::sessions::SessionStore>, session: crate::sessions::Session) -> Result<(), String> {
    store.add(session)
}

#[tauri::command]
pub fn delete_session(store: State<'_, crate::sessions::SessionStore>, id: String) -> Result<(), String> {
    store.delete(&id)
}
