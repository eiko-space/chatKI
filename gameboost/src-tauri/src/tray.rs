use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Erstellt das System-Tray-Symbol mit Menü.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "GameBoost öffnen", true, None::<&str>)?;
    let boost = MenuItem::with_id(app, "boost", "Boost starten / beenden", true, None::<&str>)?;
    let overlay = MenuItem::with_id(app, "overlay", "Overlay ein-/ausblenden", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(app, &[&open, &sep()?, &boost, &overlay, &sep()?, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("GameBoost")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "boost" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    if app.state::<crate::boost::BoostEngine>().status().active {
                        crate::revert_now(&app);
                    } else {
                        let _ = crate::boost_now(&app);
                    }
                });
            }
            "overlay" => crate::overlay::toggle(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Linksklick auf das Symbol öffnet das Fenster.
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
