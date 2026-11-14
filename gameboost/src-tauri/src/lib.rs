#![cfg_attr(not(windows), allow(dead_code))]

mod boost;
mod commands;
mod elevation;
mod games;
mod sessions;
mod settings;
mod storage;
mod telemetry;
mod tray;
mod win;

use boost::{BoostEngine, BoostStatus, TweakResult};
use settings::SettingsStore;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use telemetry::Hub;

/// Boost starten (vom Knopf, vom Tray oder vom Auto-Boost).
pub(crate) fn boost_now<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<BoostStatus, String> {
    let settings = app.state::<SettingsStore>().get();
    let game = boost::find_running_game(&settings);
    let status = app.state::<BoostEngine>().apply(&settings, game, elevation::is_elevated())?;
    let _ = app.emit("boost-changed", &status);
    Ok(status)
}

/// Boost beenden und alles zurücksetzen.
pub(crate) fn revert_now<R: tauri::Runtime>(app: &AppHandle<R>) -> Vec<TweakResult> {
    let engine = app.state::<BoostEngine>();
    let results = engine.revert();
    let _ = app.emit("boost-changed", engine.status());
    results
}

pub fn run() {
    elevation::wait_for_previous_instance();

    let app = tauri::Builder::default()
        // Nur eine Instanz: ein zweiter Start holt das vorhandene Fenster nach vorne.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            app.manage(SettingsStore::load(&data_dir));
            app.manage(BoostEngine::load(&data_dir));
            app.manage(sessions::SessionStore::load(&data_dir));
            tray::create(app.handle())?;

            let presentmon = telemetry::presentmon::locate(app.path().resource_dir().ok());
            let hub = Hub::new(presentmon);
            hub.start(app.handle().clone());
            // Mit Admin-Rechten startet die FPS-Messung direkt (sie liest nur mit, greift nicht ein).
            if elevation::is_elevated() {
                let _ = hub.start_capture();
            }
            app.manage(hub);
            Ok(())
        })
        // Fenster schließen = in den Tray minimieren. Beenden geht über das Tray-Menü.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_info,
            commands::system::restart_as_admin,
            commands::telemetry::start_fps_capture,
            commands::telemetry::stop_fps_capture,
            commands::telemetry::start_measurement,
            commands::telemetry::cancel_measurement,
            commands::telemetry::list_sessions,
            commands::telemetry::save_session,
            commands::telemetry::delete_session,
            commands::boost::get_boost_catalog,
            commands::boost::get_boost_status,
            commands::boost::apply_boost,
            commands::boost::revert_boost,
            commands::boost::create_restore_point,
            commands::boost::get_change_log,
            commands::boost::get_protected_running,
            commands::boost::list_running_apps,
            commands::boost::get_settings,
            commands::boost::save_settings,
            commands::boost::set_fullscreen_optimizations,
        ])
        .build(tauri::generate_context!())
        .expect("GameBoost konnte nicht gestartet werden");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // PresentMon sauber beenden, damit keine Messung im Hintergrund weiterläuft.
            if let Some(hub) = app.try_state::<Arc<Hub>>() {
                hub.stop_capture();
            }
        }
    });
}
