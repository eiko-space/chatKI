//! Hintergrund-Wächter: Auto-Boost beim Spielstart und Benachrichtigungen (Hitze, Ping-Spitzen).

use crate::boost::BoostEngine;
use crate::settings::SettingsStore;
use crate::telemetry::Hub;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

const INTERVAL: Duration = Duration::from_secs(3);
const COOLDOWN: Duration = Duration::from_secs(10 * 60);

fn notify<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

pub fn start<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("watcher".into())
        .spawn(move || {
            let mut auto_started = false;
            let mut last_temp: Option<Instant> = None;
            let mut last_ping: Option<Instant> = None;
            loop {
                std::thread::sleep(INTERVAL);
                let settings = app.state::<SettingsStore>().get();
                let game = crate::boost::find_running_game(&settings);

                // Pfad des Spiels merken (nötig für die Vollbild-Optimierungen im Profil)
                #[cfg(windows)]
                if let Some(g) = &game {
                    if let Some(p) = settings.profiles.iter().find(|p| p.exe.eq_ignore_ascii_case(&g.exe) && p.exe_path.is_none()) {
                        if let Some(path) = crate::win::process::path(g.pid) {
                            let exe = p.exe.clone();
                            let _ = app.state::<SettingsStore>().update(|s| {
                                if let Some(p) = s.profiles.iter_mut().find(|p| p.exe == exe) {
                                    p.exe_path = Some(path);
                                }
                            });
                        }
                    }
                }

                // ---------------- Auto-Boost
                let active = app.state::<BoostEngine>().status().active;
                if settings.auto_boost {
                    match (&game, active) {
                        (Some(g), false) => {
                            let name = g.name.clone();
                            if crate::boost_now(&app).is_ok() {
                                auto_started = true;
                                if settings.notifications {
                                    notify(&app, "Auto-Boost aktiv", &format!("{name} erkannt – GameBoost hat optimiert. Nach dem Spiel wird alles zurückgesetzt."));
                                }
                            }
                        }
                        (None, true) if auto_started => {
                            crate::revert_now(&app);
                            auto_started = false;
                            if settings.notifications {
                                notify(&app, "Boost beendet", "Spiel geschlossen – alle Änderungen wurden zurückgesetzt.");
                            }
                        }
                        _ => {}
                    }
                } else if !active {
                    auto_started = false;
                }

                // ---------------- Benachrichtigungen
                if !settings.notifications {
                    continue;
                }
                let Some(snap) = app.state::<Arc<Hub>>().latest() else { continue };
                if let Some(t) = snap.gpu.temperature {
                    if t >= settings.temp_warning_c && last_temp.is_none_or(|l| l.elapsed() > COOLDOWN) {
                        notify(&app, "Grafikkarte sehr warm", &format!("{t:.0} °C – ab etwa 85 °C kann die Leistung sinken. Lüfter und Staub prüfen."));
                        last_temp = Some(Instant::now());
                    }
                }
                // Ping-Warnungen nur während eines Spiels, sonst nerven sie.
                let in_game = game.is_some() || snap.fps.as_ref().is_some_and(|f| f.game.is_some());
                let p = &snap.network.internet;
                let bad_spikes = p.spikes.is_some_and(|s| s >= 3) && p.window_secs >= 30.0;
                let bad_loss = p.loss_percent.is_some_and(|l| l >= 5.0) && p.samples >= 20;
                if in_game && (bad_spikes || bad_loss) && last_ping.is_none_or(|l| l.elapsed() > COOLDOWN) {
                    let body = if bad_loss {
                        format!("{:.0} % Paketverlust in der letzten Minute.", p.loss_percent.unwrap_or(0.0))
                    } else {
                        format!("{} Ping-Spitzen in der letzten Minute (über {:.0} ms).", p.spikes.unwrap_or(0), p.spike_threshold_ms.unwrap_or(0.0))
                    };
                    let hint = if snap.network.link.connection == Some("wlan") { " Du bist per WLAN verbunden – ein Kabel hilft meist." } else { "" };
                    notify(&app, "Unruhige Verbindung", &format!("{body}{hint}"));
                    last_ping = Some(Instant::now());
                }
            }
        })
        .expect("Wächter-Thread konnte nicht gestartet werden");
}
