//! Live-Messungen: sammelt alle Werte und schickt 2× pro Sekunde einen Schnappschuss an die Oberfläche.

pub mod detective;
pub mod frames;
pub mod gpu;
pub mod measure;
pub mod ping;
pub mod presentmon;
pub mod system;

use frames::{FpsStats, FrameStore};
use gpu::{GpuSampler, GpuStats};
use measure::{MeasurementProgress, Recording};
use ping::{PingStats, PingStore};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use system::{LinkStats, Sampler, SystemStats};
use tauri::{AppHandle, Emitter, Runtime};

const TICK: Duration = Duration::from_millis(500);

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    /// "idle" | "running" | "missingBinary" | "error"
    pub state: &'static str,
    pub message: Option<String>,
}

impl CaptureStatus {
    fn idle() -> Self {
        Self { state: "idle", message: None }
    }
    fn running() -> Self {
        Self { state: "running", message: None }
    }
    fn missing() -> Self {
        Self { state: "missingBinary", message: Some("PresentMon.exe wurde nicht gefunden.".into()) }
    }
    pub fn error(message: String) -> Self {
        Self { state: "error", message: Some(message) }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStats {
    pub internet: PingStats,
    pub router: PingStats,
    #[serde(flatten)]
    pub link: LinkStats,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub capture: CaptureStatus,
    pub fps: Option<FpsStats>,
    /// Neue Frametimes (ms) seit dem letzten Schnappschuss – für den Graphen.
    pub frametimes: Vec<f32>,
    pub system: SystemStats,
    pub gpu: GpuStats,
    /// Windows stellt die CPU-Temperatur ohne Zusatztreiber nicht bereit → bewusst immer `null`.
    pub cpu_temperature: Option<f32>,
    pub network: NetworkStats,
    /// Laufende 60-s-Messung (Vorher/Nachher)
    pub measurement: Option<MeasurementProgress>,
    /// Werte der letzten 60 s vor dem Boost – zum Live-Vergleich auf dem Dashboard
    pub boost_baseline: Option<BoostBaseline>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BoostBaseline {
    /// Sekunden seit Boost-Start
    pub seconds_since: f32,
    pub fps: Option<FpsStats>,
    pub ping: PingStats,
}

pub struct Hub {
    frames: Arc<Mutex<FrameStore>>,
    status: Arc<Mutex<CaptureStatus>>,
    generation: Arc<AtomicU64>,
    child: Mutex<Option<Child>>,
    internet: Arc<Mutex<PingStore>>,
    router: Arc<Mutex<PingStore>>,
    presentmon: Option<PathBuf>,
    recording: Mutex<Option<Recording>>,
    latest: Mutex<Option<Snapshot>>,
    baseline: Mutex<Option<(Instant, Option<FpsStats>, PingStats)>>,
    pub detective: Mutex<detective::Detective>,
    /// Ergebnisse abgeschlossener Messungen (für den Smart-Boost, der darauf wartet)
    results: Mutex<std::collections::HashMap<String, Result<measure::Measurement, String>>>,
}

impl Hub {
    pub fn new(presentmon: Option<PathBuf>) -> Arc<Self> {
        let status = if presentmon.is_some() { CaptureStatus::idle() } else { CaptureStatus::missing() };
        Arc::new(Self {
            frames: Default::default(),
            status: Arc::new(Mutex::new(status)),
            generation: Default::default(),
            child: Mutex::new(None),
            internet: Arc::new(Mutex::new(PingStore::new("Internet (Cloudflare)"))),
            router: Arc::new(Mutex::new(PingStore::new("Router"))),
            presentmon,
            recording: Mutex::new(None),
            latest: Mutex::new(None),
            baseline: Mutex::new(None),
            detective: Mutex::new(detective::Detective::new()),
            results: Mutex::new(Default::default()),
        })
    }

    /// Startet Ping-Messungen und den Sampler, der regelmäßig Schnappschüsse sendet.
    pub fn start<R: Runtime>(self: &Arc<Self>, app: AppHandle<R>) {
        ping::spawn(self.internet.clone(), ping::internet_target);
        ping::spawn(self.router.clone(), ping::router_target);

        let hub = self.clone();
        std::thread::Builder::new()
            .name("telemetry".into())
            .spawn(move || {
                let mut sampler = Sampler::default();
                let mut gpu = GpuSampler::new();
                let (mut sys, mut gpu_stats, mut link) = (SystemStats::default(), GpuStats::default(), LinkStats::default());
                let mut tick: u64 = 0;
                loop {
                    let started = Instant::now();
                    if tick % 2 == 0 {
                        // Hardware 1× pro Sekunde
                        sys = sampler.system();
                        gpu_stats = gpu.sample();
                        link = sampler.link();
                    }
                    if tick % 4 == 0 {
                        // CPU je Programm & Festplatte alle 2 s (für den Ruckler-Detektiv)
                        hub.detective.lock().unwrap().sample();
                    }
                    let snapshot = hub.snapshot(sys.clone(), gpu_stats.clone(), link.clone());
                    hub.detect(&snapshot);
                    if let Some(result) = hub.feed_recording(&snapshot) {
                        let _ = app.emit("measurement-finished", result);
                    }
                    *hub.latest.lock().unwrap() = Some(snapshot.clone());
                    let _ = app.emit("telemetry", snapshot);
                    tick += 1;
                    std::thread::sleep(TICK.saturating_sub(started.elapsed()));
                }
            })
            .expect("Telemetrie-Thread konnte nicht gestartet werden");
    }

    fn snapshot(&self, system: SystemStats, gpu: GpuStats, link: LinkStats) -> Snapshot {
        let (fps, frametimes) = {
            let mut f = self.frames.lock().unwrap();
            f.select_target(Instant::now(), crate::elevation::foreground_pid());
            (f.stats(), f.take_new_frametimes())
        };
        Snapshot {
            capture: self.status.lock().unwrap().clone(),
            fps,
            frametimes,
            system,
            gpu,
            cpu_temperature: None,
            network: NetworkStats {
                internet: self.internet.lock().unwrap().stats(),
                router: self.router.lock().unwrap().stats(),
                link,
            },
            measurement: self.recording.lock().unwrap().as_ref().map(|r| r.progress()),
            boost_baseline: self.baseline.lock().unwrap().as_ref().map(|(at, fps, ping)| BoostBaseline {
                seconds_since: at.elapsed().as_secs_f32(),
                fps: fps.clone(),
                ping: ping.clone(),
            }),
        }
    }

    /// Prüft den neuesten Schnappschuss auf Ruckler und Ping-Spitzen und hält den Zusammenhang fest.
    fn detect(&self, snap: &Snapshot) {
        let mut det = self.detective.lock().unwrap();
        let context = || detective::Context {
            cpu: snap.system.cpu_usage,
            gpu: snap.gpu.usage,
            ram: snap.system.ram_usage,
            disk: det.disk_busy,
            download_mbps: snap.network.link.download_mbps,
            upload_mbps: snap.network.link.upload_mbps,
            wlan: snap.network.link.connection == Some("wlan"),
            top: det.top.clone(),
        };
        if let Some(fps) = &snap.fps {
            // Erst ab 100 Bildern, damit der Normalwert stabil ist
            if fps.frame_count >= 100 {
                if let Some(worst) = snap.frametimes.iter().copied().reduce(f32::max) {
                    if detective::is_stutter(worst, fps.frametime_avg_ms) {
                        let ctx = context();
                        let game = Some(fps.game.map(str::to_string).unwrap_or_else(|| fps.app.clone()));
                        det.record("stutter", worst, Some(fps.frametime_avg_ms), game, Some(&fps.app), ctx);
                        return;
                    }
                }
            }
        }
        let p = &snap.network.internet;
        if let (Some(last), Some(th)) = (p.last_ms, p.spike_threshold_ms) {
            // Ping-Spitzen nur festhalten, während gespielt wird
            if last > th && snap.fps.is_some() {
                let ctx = context();
                // Schwelle = Median + max(30 ms, 50 %) → Median zurückrechnen
                let normal = p.spike_threshold_ms.map(|t| if t / 1.5 >= 60.0 { t / 1.5 } else { t - 30.0 });
                let game = snap.fps.as_ref().map(|f| f.game.map(str::to_string).unwrap_or_else(|| f.app.clone()));
                det.record("ping", last, normal, game, None, ctx);
            }
        }
    }

    /// Beim Boost-Start: aktuelle 60-s-Werte als Vergleichsbasis merken.
    pub fn capture_baseline(&self) {
        let fps = self.frames.lock().unwrap().stats();
        let ping = self.internet.lock().unwrap().stats();
        *self.baseline.lock().unwrap() = Some((Instant::now(), fps, ping));
    }

    pub fn clear_baseline(&self) {
        // (auch vom Smart-Boost genutzt)
        *self.baseline.lock().unwrap() = None;
    }

    /// Letzter Schnappschuss (z. B. für Overlay, Benachrichtigungen, Auto-Boost).
    pub fn latest(&self) -> Option<Snapshot> {
        self.latest.lock().unwrap().clone()
    }

    pub fn start_measurement(&self, label: String) -> Result<(), String> {
        let mut rec = self.recording.lock().unwrap();
        if rec.is_some() {
            return Err("Es läuft bereits eine Messung.".into());
        }
        *rec = Some(Recording::new(label));
        Ok(())
    }

    /// Messung mit eigener Dauer starten (Smart-Boost).
    pub fn start_measurement_for(&self, label: String, secs: u64) -> Result<(), String> {
        let mut rec = self.recording.lock().unwrap();
        if rec.is_some() {
            return Err("Es läuft bereits eine Messung.".into());
        }
        self.results.lock().unwrap().remove(&label);
        *rec = Some(Recording::with_duration(label, Duration::from_secs(secs)));
        Ok(())
    }

    /// Ergebnis einer Messung abholen (sobald fertig).
    pub fn take_result(&self, label: &str) -> Option<Result<measure::Measurement, String>> {
        self.results.lock().unwrap().remove(label)
    }

    pub fn cancel_measurement(&self) {
        *self.recording.lock().unwrap() = None;
    }

    /// Gibt neue Daten an eine laufende Messung; liefert das Ergebnis, wenn sie fertig ist.
    fn feed_recording(&self, snap: &Snapshot) -> Option<Result<measure::Measurement, String>> {
        let mut guard = self.recording.lock().unwrap();
        let rec = guard.as_mut()?;
        rec.add(
            snap.fps.as_ref().map(|f| f.pid),
            snap.fps.as_ref().map(|f| f.game.map(str::to_string).unwrap_or_else(|| f.app.clone())),
            &snap.frametimes,
            snap.system.cpu_usage,
            snap.gpu.usage,
        );
        rec.add_latency(snap.fps.as_ref().and_then(|f| f.input_latency_ms));
        if !rec.done() {
            return None;
        }
        let rec = guard.take()?;
        let ping = self.internet.lock().unwrap().stats_since(Some(rec.started()));
        let result = match rec.problem.clone() {
            Some(p) => Err(p),
            None => Ok(rec.finish(&ping)),
        };
        self.results.lock().unwrap().insert(rec.label.clone(), result.clone());
        Some(result)
    }

    pub fn start_capture(&self) -> Result<(), String> {
        let Some(path) = &self.presentmon else {
            *self.status.lock().unwrap() = CaptureStatus::missing();
            return Err("PresentMon.exe wurde nicht gefunden.".into());
        };
        if !crate::elevation::is_elevated() {
            return Err("Für die FPS-Messung werden Administratorrechte benötigt.".into());
        }
        let mut child = self.child.lock().unwrap();
        if child.is_some() {
            return Ok(());
        }
        self.frames.lock().unwrap().clear();
        self.generation.fetch_add(1, Ordering::SeqCst);
        match presentmon::spawn(path, self.frames.clone(), self.status.clone(), self.generation.clone()) {
            Ok(c) => {
                *child = Some(c);
                *self.status.lock().unwrap() = CaptureStatus::running();
                Ok(())
            }
            Err(e) => {
                let msg = format!("PresentMon konnte nicht gestartet werden: {e}");
                *self.status.lock().unwrap() = CaptureStatus::error(msg.clone());
                Err(msg)
            }
        }
    }

    pub fn stop_capture(&self) {
        // Generation erhöhen → der Lese-Thread meldet das Ende nicht als Fehler.
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
            if let Some(path) = &self.presentmon {
                presentmon::terminate_session(path);
            }
        }
        self.frames.lock().unwrap().clear();
        let mut st = self.status.lock().unwrap();
        if st.state != "missingBinary" {
            *st = CaptureStatus::idle();
        }
    }
}
