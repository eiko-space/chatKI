//! Smart-Boost: testet jeden Tweak einzeln während des Spielens und behält nur, was auf DIESEM PC hilft.
//!
//! Ablauf: Basis-Messung ohne Boost → Tweak für Tweak zuschalten und nachmessen →
//! schadet er messbar, wird er sofort wieder entfernt → Ergebnis als Spielprofil speichern.

use crate::boost::{self, BoostEngine};
use crate::settings::SettingsStore;
use crate::telemetry::measure::Measurement;
use crate::telemetry::Hub;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Läuft gerade ein Smart-Boost? (Dann sind Knopf, Tastenkürzel und Auto-Boost gesperrt.)
pub static RUNNING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<SmartState>> = Mutex::new(None);

const BASELINE_SECS: u64 = 25;
const STEP_SECS: u64 = 18;
const SETTLE: Duration = Duration::from_secs(3);

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SmartResult {
    pub id: String,
    pub title: String,
    /// "better" | "neutral" | "worse" | "skipped"
    pub verdict: &'static str,
    pub detail: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SmartState {
    pub running: bool,
    pub percent: u32,
    pub step: String,
    pub game: Option<String>,
    pub results: Vec<SmartResult>,
    pub before: Option<Measurement>,
    pub after: Option<Measurement>,
    pub kept: Vec<String>,
    pub error: Option<String>,
    pub finished: bool,
    /// Geschätzte Restzeit in Sekunden
    pub remaining_secs: u32,
}

fn rel(after: Option<f32>, before: Option<f32>) -> Option<f32> {
    match (after, before) {
        (Some(a), Some(b)) if b.abs() > f32::EPSILON => Some((a - b) / b.abs()),
        _ => None,
    }
}

/// Bewertet einen Tweak anhand von Vorher/Nachher. Reine Logik (testbar).
pub fn verdict(before: &Measurement, after: &Measurement) -> (&'static str, String) {
    let low1 = rel(after.low1, before.low1);
    let avg = rel(after.fps_avg, before.fps_avg);
    let std = rel(after.frametime_std_dev_ms, before.frametime_std_dev_ms);
    let lat = rel(after.input_latency_ms, before.input_latency_ms);

    let worse = low1.is_some_and(|v| v <= -0.04) || avg.is_some_and(|v| v <= -0.04) || std.is_some_and(|v| v >= 0.15);
    let better = low1.is_some_and(|v| v >= 0.03) || std.is_some_and(|v| v <= -0.08) || lat.is_some_and(|v| v <= -0.05);

    let pct = |v: Option<f32>| v.map(|x| format!("{}{:.0} %", if x >= 0.0 { "+" } else { "" }, x * 100.0)).unwrap_or_else(|| "–".into());
    let detail = format!("1 %-Lows {} · Ø FPS {} · Schwankung {} · Input-Delay {}", pct(low1), pct(avg), pct(std), pct(lat));
    let v = match (better, worse) {
        (true, false) => "better",
        (false, true) => "worse",
        _ => "neutral",
    };
    (v, detail)
}

fn update<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce(&mut SmartState)) {
    let mut g = STATE.lock().unwrap();
    let s = g.get_or_insert_with(SmartState::default);
    f(s);
    let _ = app.emit("smartboost", s.clone());
}

pub fn state() -> Option<SmartState> {
    STATE.lock().unwrap().clone()
}

pub fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Misst `secs` Sekunden und wartet auf das Ergebnis.
fn measure<R: Runtime>(app: &AppHandle<R>, label: &str, secs: u64) -> Result<Measurement, String> {
    let hub = app.state::<Arc<Hub>>();
    hub.start_measurement_for(label.into(), secs)?;
    let deadline = Instant::now() + Duration::from_secs(secs + 15);
    loop {
        if CANCEL.load(Ordering::SeqCst) {
            hub.cancel_measurement();
            return Err("Abgebrochen.".into());
        }
        if let Some(r) = hub.take_result(label) {
            return r.and_then(|m| if m.frames < 100 { Err("Zu wenige Bilder gemessen – läuft das Spiel im Vordergrund?".into()) } else { Ok(m) });
        }
        if Instant::now() > deadline {
            hub.cancel_measurement();
            return Err("Messung hat zu lange gedauert.".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn sleep_cancelable(d: Duration) -> Result<(), String> {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if CANCEL.load(Ordering::SeqCst) {
            return Err("Abgebrochen.".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

pub fn start<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Smart-Boost läuft bereits.".into());
    }
    CANCEL.store(false, Ordering::SeqCst);
    *STATE.lock().unwrap() = Some(SmartState { running: true, step: "Vorbereiten …".into(), ..Default::default() });
    std::thread::Builder::new()
        .name("smartboost".into())
        .spawn(move || {
            let result = run(&app);
            let engine = app.state::<BoostEngine>();
            if let Err(e) = &result {
                // Bei Abbruch/Fehler alles zurücksetzen – nichts bleibt halb verändert.
                engine.revert(&|_, _, _| {});
                update(&app, |s| {
                    s.error = Some(e.clone());
                    s.step = "Abgebrochen – alles zurückgesetzt.".into();
                });
            }
            update(&app, |s| {
                s.running = false;
                s.finished = result.is_ok();
            });
            RUNNING.store(false, Ordering::SeqCst);
            let _ = app.emit("boost-changed", engine.status());
        })
        .map_err(|e| {
            RUNNING.store(false, Ordering::SeqCst);
            e.to_string()
        })?;
    Ok(())
}

fn run<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let engine = app.state::<BoostEngine>();
    let store = app.state::<SettingsStore>();
    let settings = store.get();
    let game = boost::find_running_game(&settings).ok_or("Kein Spiel erkannt. Starte zuerst ein Spiel (z. B. Fortnite) und lass es im Vordergrund laufen.")?;
    let hub = app.state::<Arc<Hub>>();
    if hub.latest().is_none_or(|s| s.capture.state != "running") {
        return Err("Die FPS-Messung läuft nicht. Starte GameBoost mit Admin-Rechten und aktiviere „FPS messen“.".into());
    }
    let admin = crate::elevation::is_elevated();
    let candidates: Vec<&boost::TweakInfo> = boost::catalog().iter().filter(|t| !t.one_shot && (!t.needs_admin || admin)).collect();
    let total_secs = BASELINE_SECS + candidates.len() as u64 * (STEP_SECS + SETTLE.as_secs());
    update(app, |s| {
        s.game = Some(game.name.clone());
        s.remaining_secs = total_secs as u32;
    });

    // 1) Sauberer Ausgangszustand: laufenden Boost zurücksetzen
    if engine.status().active {
        update(app, |s| s.step = "Setze laufenden Boost zurück …".into());
        engine.revert(&|_, _, _| {});
        hub.clear_baseline();
    }
    sleep_cancelable(SETTLE)?;

    // 2) Basis-Messung ohne Boost
    update(app, |s| s.step = format!("Messe ohne Boost ({BASELINE_SECS} s) – spiel einfach weiter …"));
    let baseline = measure(app, "smart-basis", BASELINE_SECS)?;
    let mut reference = baseline.clone();
    update(app, |s| s.before = Some(baseline.clone()));

    // 3) Jeden Tweak einzeln zuschalten und nachmessen
    let mut kept = Vec::new();
    for (i, t) in candidates.iter().enumerate() {
        let left = (candidates.len() - i) as u64 * (STEP_SECS + SETTLE.as_secs());
        update(app, |s| {
            s.step = format!("Teste: {}", t.title);
            s.percent = (((i as f32) / candidates.len() as f32) * 100.0) as u32;
            s.remaining_secs = left as u32;
        });
        let res = engine.apply_one(t.id, &settings, Some(&game), admin);
        if res.status != "applied" {
            let (verdict, detail) = match res.status.as_str() {
                "unchanged" => ("neutral", format!("War schon optimal eingestellt. {}", res.message)),
                _ => ("skipped", res.message.clone()),
            };
            if res.status == "unchanged" {
                kept.push(t.id.to_string());
            }
            update(app, |s| s.results.push(SmartResult { id: t.id.into(), title: t.title.into(), verdict, detail }));
            continue;
        }
        sleep_cancelable(SETTLE)?;
        let m = measure(app, &format!("smart-{}", t.id), STEP_SECS)?;
        let (verdict, detail) = verdict(&reference, &m);
        if verdict == "worse" {
            let _ = engine.revert_one(t.id);
        } else {
            kept.push(t.id.to_string());
            reference = m;
        }
        update(app, |s| s.results.push(SmartResult { id: t.id.into(), title: t.title.into(), verdict, detail }));
    }

    // 4) RAM-Wächter (einmalig) übernehmen, wenn er vorher aktiv war
    if admin && settings.enabled_tweaks.iter().any(|t| t == "standby_list") {
        kept.push("standby_list".into());
    }

    // 5) Als persönliches Profil für dieses Spiel speichern
    let exe = game.exe.clone();
    let kept_ids = kept.clone();
    let _ = store.update(|s| {
        if let Some(p) = s.profiles.iter_mut().find(|p| p.exe.eq_ignore_ascii_case(&exe)) {
            p.tweaks = Some(kept_ids);
        }
    });
    let _ = app.emit("settings-changed", ());

    update(app, |s| {
        s.after = Some(reference.clone());
        s.kept = kept;
        s.percent = 100;
        s.remaining_secs = 0;
        s.step = "Fertig – dein persönlicher Boost ist aktiv und gespeichert.".into();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(low1: f32, avg: f32, std: f32, lat: Option<f32>) -> Measurement {
        Measurement { low1: Some(low1), fps_avg: Some(avg), frametime_std_dev_ms: Some(std), input_latency_ms: lat, frames: 5000, ..Default::default() }
    }

    #[test]
    fn verdicts() {
        let base = m(100.0, 150.0, 2.0, Some(20.0));
        assert_eq!(verdict(&base, &m(108.0, 152.0, 1.9, Some(19.5))).0, "better");
        assert_eq!(verdict(&base, &m(100.5, 150.5, 2.05, Some(20.0))).0, "neutral");
        assert_eq!(verdict(&base, &m(90.0, 140.0, 2.6, Some(21.0))).0, "worse");
        // gemischt (Lows besser, Schwankung deutlich schlechter) → neutral
        assert_eq!(verdict(&base, &m(105.0, 150.0, 2.4, None)).0, "neutral");
        // Input-Delay deutlich besser reicht
        assert_eq!(verdict(&base, &m(100.0, 150.0, 2.0, Some(17.0))).0, "better");
    }
}
