#![cfg_attr(not(windows), allow(dead_code))]

mod auth;
mod boost;
mod commands;
mod elevation;
mod games;
mod netdiag;
mod overlay;
mod pccheck;
mod sessions;
mod settings;
mod storage;
mod telemetry;
mod tray;
mod watcher;
mod win;

use boost::{BoostEngine, BoostStatus, TweakResult};
use settings::SettingsStore;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use telemetry::Hub;

/// Gesperrt, solange im „Nur mit Einladung“-Modus niemand angemeldet ist (setzt die Oberfläche).
pub(crate) static LOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn is_locked() -> bool {
    LOCKED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Boost starten (vom Knopf, vom Tray oder vom Auto-Boost).
pub(crate) fn boost_now<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<BoostStatus, String> {
    if is_locked() {
        return Err("Bitte zuerst anmelden.".into());
    }
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
    // Im Einladungs-Modus ist alles gesperrt, bis sich jemand anmeldet.
    if option_env!("VITE_INVITE_ONLY") == Some("true") {
        LOCKED.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let app = tauri::Builder::default()
        // Nur eine Instanz: ein zweiter Start holt das vorhandene Fenster nach vorne.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        overlay::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Beim Windows-Autostart (--minimized) nur im Tray starten
            if !std::env::args().any(|a| a == "--minimized") {
                tray::show_main_window(app.handle());
            }
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
            watcher::start(app.handle().clone());
            if let Err(e) = overlay::apply(app.handle()) {
                eprintln!("Overlay: {e}");
            }
            Ok(())
        })
        // Fenster schließen = in den Tray minimieren. Beenden geht über das Tray-Menü.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Nur das Hauptfenster wird in den Tray verschoben; das Overlay darf sich schließen.
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_info,
            commands::system::restart_as_admin,
            commands::system::get_autostart,
            commands::system::set_autostart,
            commands::system::apply_overlay,
            commands::system::set_locked,
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
            commands::auth::secure_get,
            commands::auth::secure_set,
            commands::auth::secure_delete,
            commands::auth::start_oauth_listener,
            commands::auth::open_url,
            commands::tools::region_ping,
            commands::tools::traceroute,
            commands::tools::net_apps,
            commands::tools::run_pc_check,
            commands::tools::run_pc_fix,
        ])
        .build(tauri::generate_context!())
        .expect("GameBoost konnte nicht gestartet werden");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // PresentMon sauber beenden, damit keine Messung im Hintergrund weiterläuft.
            if let Some(hub) = app.try_state::<Arc<Hub>>() {
                hub.stop_capture();
            }
            // Läuft beim Beenden noch ein Boost, wird alles zurückgesetzt – nichts bleibt verändert.
            if app.try_state::<BoostEngine>().is_some_and(|e| e.status().active) {
                revert_now(app);
            }
        }
    });
}
