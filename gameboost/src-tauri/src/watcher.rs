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
            let mut was_active = false;
            let mut last_purge: Option<Instant> = None;
            // Hat der Nutzer den Auto-Boost für dieses Spiel von Hand beendet? Dann bis Spielende nicht erneut starten.
            let mut stopped_by_user: Option<u32> = None;
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
                if game.is_none() {
                    stopped_by_user = None;
                }
                if was_active && !active {
                    // Boost lief (egal ob automatisch oder per Knopf) und ist jetzt aus → der Nutzer hat ihn beendet.
                    // Für dieses Spiel nicht erneut automatisch starten.
                    auto_started = false;
                    stopped_by_user = game.as_ref().map(|g| g.pid);
                }
                was_active = active;
                if settings.auto_boost {
                    match (&game, active) {
                        (Some(g), false) if stopped_by_user != Some(g.pid) => {
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
                }

                // ---------------- RAM-Wächter: Standby-Speicher leeren, wenn freier RAM knapp wird
                let ram_guard = settings.enabled_tweaks.iter().any(|t| t == "standby_list");
                if active && ram_guard && crate::elevation::is_elevated() && last_purge.is_none_or(|t| t.elapsed() > Duration::from_secs(30)) {
                    if let Some((free_gb, standby_gb)) = crate::boost::tweaks::memory_status() {
                        if free_gb < 1.0 && standby_gb > 1.0 {
                            if let Ok((msg, _)) = crate::boost::tweaks::purge_standby_list() {
                                app.state::<BoostEngine>().log("ram_guard", "RAM-Wächter", "applied", &msg);
                            }
                            last_purge = Some(Instant::now());
                        }
                    }
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
