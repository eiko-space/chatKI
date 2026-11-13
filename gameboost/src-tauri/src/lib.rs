mod commands;
mod tray;

use tauri::WindowEvent;

pub fn run() {
    tauri::Builder::default()
        // Nur eine Instanz: ein zweiter Start holt das vorhandene Fenster nach vorne.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .setup(|app| {
            tray::create(app.handle())?;
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
        .invoke_handler(tauri::generate_handler![commands::system::get_app_info])
        .run(tauri::generate_context!())
        .expect("GameBoost konnte nicht gestartet werden");
}
