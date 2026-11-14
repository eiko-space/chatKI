//! In-Game-Overlay: ein eigenes, durchsichtiges Fenster, das immer im Vordergrund liegt.
//! Kein Eingriff ins Spiel (kein Hooking, keine Injection) → Anti-Cheat-sicher.
//! Hinweis: Im exklusiven Vollbildmodus kann Windows es nicht über dem Spiel anzeigen –
//! dafür im Spiel „Randloses Fenster“/„Vollbild (Fenster)“ wählen.

use crate::settings::{OverlaySettings, SettingsStore};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const LABEL: &str = "overlay";
const BASE_W: f64 = 210.0;
const BASE_H: f64 = 92.0;
const MARGIN: f64 = 16.0;

fn geometry<R: Runtime>(app: &AppHandle<R>, s: &OverlaySettings) -> Option<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let m = app.primary_monitor().ok().flatten()?;
    let scale = m.scale_factor();
    let size = m.size().to_logical::<f64>(scale);
    let origin = m.position().to_logical::<f64>(scale);
    let (w, h) = (BASE_W * s.scale as f64, BASE_H * s.scale as f64);
    let x = if s.position.ends_with("right") { origin.x + size.width - w - MARGIN } else { origin.x + MARGIN };
    let y = if s.position.starts_with("bottom") { origin.y + size.height - h - MARGIN } else { origin.y + MARGIN };
    Some((LogicalPosition::new(x, y), LogicalSize::new(w, h)))
}

/// Erstellt/aktualisiert das Overlay-Fenster und das Tastenkürzel gemäß den Einstellungen.
pub fn apply<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let s = app.state::<SettingsStore>().get().overlay;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    if !s.enabled {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.close();
        }
        return Ok(());
    }

    gs.register(s.hotkey.as_str()).map_err(|e| format!("Tastenkürzel „{}“ ist ungültig oder belegt: {e}", s.hotkey))?;

    let (pos, size) = geometry(app, &s).ok_or("Kein Bildschirm gefunden.")?;
    let window = match app.get_webview_window(LABEL) {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
            .title("GameBoost Overlay")
            .initialization_script("window.__GB_OVERLAY__ = true;")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?,
    };
    let _ = window.set_size(size);
    let _ = window.set_position(pos);
    let _ = window.set_ignore_cursor_events(true); // Klicks gehen durch ins Spiel
    let _ = window.show();
    Ok(())
}

/// Per Tastenkürzel ein-/ausblenden.
pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(LABEL) {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
        }
    }
}
