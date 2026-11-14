mod commands;
mod elevation;
mod games;
mod telemetry;
mod tray;

use std::sync::Arc;
use tauri::{Manager, RunEvent, WindowEvent};
use telemetry::Hub;

pub fn run() {
    elevation::wait_for_previous_instance();

    let app = tauri::Builder::default()
        // Nur eine Instanz: ein zweiter Start holt das vorhandene Fenster nach vorne.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .setup(|app| {
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
