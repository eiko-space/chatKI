//! Ruckler-Detektiv: hält bei jedem Ruckler / jeder Ping-Spitze fest, was in dem Moment auf dem PC los war,
//! und nennt die wahrscheinlichste Ursache in einfachen Worten.

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

const MAX_EVENTS: usize = 100;
const COOLDOWN: Duration = Duration::from_secs(3);

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopProcess {
    pub exe: String,
    pub cpu: f32,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub cpu: Option<f32>,
    pub gpu: Option<f32>,
    pub ram: Option<f32>,
    pub disk: Option<f32>,
    pub download_mbps: Option<f32>,
    pub upload_mbps: Option<f32>,
    pub wlan: bool,
    pub top: Vec<TopProcess>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub time: u64,
    /// "stutter" | "ping"
    pub kind: &'static str,
    /// Dauer des Bildes bzw. Ping in ms
    pub value_ms: f32,
    /// Normalwert zum Vergleich (Ø Frametime bzw. Median-Ping)
    pub normal_ms: Option<f32>,
    pub game: Option<String>,
    pub cause: String,
    pub context: Context,
}

/// Ermittelt die wahrscheinlichste Ursache. Reine Logik (testbar).
pub fn cause(kind: &str, c: &Context, game_exe: Option<&str>) -> String {
    let other = c.top.iter().find(|p| game_exe.is_none_or(|g| !p.exe.eq_ignore_ascii_case(g)) && p.cpu >= 15.0);
    if kind == "ping" {
        let down = c.download_mbps.unwrap_or(0.0);
        let up = c.upload_mbps.unwrap_or(0.0);
        if down >= 8.0 || up >= 3.0 {
            return format!("Gleichzeitiger Datenverkehr auf deinem PC ({down:.0} Mbit/s runter, {up:.1} Mbit/s hoch) – z. B. Update, Cloud-Sync oder Stream.");
        }
        if c.wlan {
            return "WLAN-Schwankung – mit LAN-Kabel passiert das deutlich seltener.".into();
        }
        return "Spitze im Internet bzw. beim Anbieter – dein PC hat gerade nichts heruntergeladen.".into();
    }
    if c.disk.is_some_and(|d| d >= 80.0) {
        return match other {
            Some(p) => format!("Festplatte voll ausgelastet, gleichzeitig war {} aktiv.", p.exe),
            None => "Festplatte voll ausgelastet – z. B. Nachladen, Update oder Virenscan.".into(),
        };
    }
    if let Some(p) = other {
        return format!("{} hat gerade viel CPU belegt ({:.0} %).", p.exe, p.cpu);
    }
    if c.cpu.is_some_and(|v| v >= 90.0) {
        return "Prozessor am Limit – das Spiel braucht mehr CPU, als frei ist.".into();
    }
    if c.ram.is_some_and(|v| v >= 90.0) {
        return "Arbeitsspeicher fast voll – Windows muss auslagern.".into();
    }
    if c.gpu.is_some_and(|v| v >= 98.0) {
        return "Grafikkarte am Limit – ein FPS-Limit macht das Bild gleichmäßiger.".into();
    }
    "Keine Ursache außerhalb des Spiels erkennbar – oft lädt das Spiel gerade etwas nach (Shader, Map-Bereich).".into()
}

/// Erkennt Ruckler: Bild dauert mehr als 2,5× so lang wie üblich und mindestens 15 ms länger.
pub fn is_stutter(frame_ms: f32, avg_ms: f32) -> bool {
    frame_ms > avg_ms * 2.5 && frame_ms > avg_ms + 15.0
}

pub struct Detective {
    events: VecDeque<Event>,
    last: Option<Instant>,
    cpu_prev: HashMap<u32, (String, u64)>,
    cpu_at: Option<Instant>,
    pub top: Vec<TopProcess>,
    disk: platform::Disk,
    pub disk_busy: Option<f32>,
}

impl Detective {
    pub fn new() -> Self {
        Self { events: VecDeque::new(), last: None, cpu_prev: HashMap::new(), cpu_at: None, top: Vec::new(), disk: platform::Disk::new(), disk_busy: None }
    }

    /// Alle ~2 s: CPU je Programm und Festplattenauslastung aktualisieren.
    pub fn sample(&mut self) {
        self.disk_busy = self.disk.busy();
        #[cfg(windows)]
        {
            let now = Instant::now();
            let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
            let mut next = HashMap::new();
            let mut top = Vec::new();
            for p in crate::win::process::list().into_iter().filter(|p| p.pid > 4) {
                let Some(t) = crate::win::process::cpu_time(p.pid) else { continue };
                if let (Some((_, prev)), Some(at)) = (self.cpu_prev.get(&p.pid), self.cpu_at) {
                    let secs = now.duration_since(at).as_secs_f64();
                    if secs > 0.0 {
                        let pct = (t.saturating_sub(*prev) as f64 / 1e7) / secs / cores * 100.0;
                        if pct >= 1.0 {
                            top.push(TopProcess { exe: p.name.clone(), cpu: pct as f32 });
                        }
                    }
                }
                next.insert(p.pid, (p.name, t));
            }
            // Gleichnamige Prozesse (z. B. viele chrome.exe) zusammenfassen
            let mut merged: Vec<TopProcess> = Vec::new();
            for t in top {
                match merged.iter_mut().find(|m| m.exe.eq_ignore_ascii_case(&t.exe)) {
                    Some(m) => m.cpu += t.cpu,
                    None => merged.push(t),
                }
            }
            merged.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            merged.truncate(5);
            self.top = merged;
            self.cpu_prev = next;
            self.cpu_at = Some(now);
        }
    }

    pub fn record(&mut self, kind: &'static str, value_ms: f32, normal_ms: Option<f32>, game: Option<String>, game_exe: Option<&str>, context: Context) {
        if self.last.is_some_and(|l| l.elapsed() < COOLDOWN) {
            return;
        }
        self.last = Some(Instant::now());
        let cause = cause(kind, &context, game_exe);
        self.events.push_front(Event { time: crate::storage::unix_now(), kind, value_ms, normal_ms, game, cause, context });
        self.events.truncate(MAX_EVENTS);
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::Performance::*;

    pub struct Disk {
        query: isize,
        counter: isize,
    }
    // SAFETY: Handles werden nur vom Telemetrie-Thread benutzt.
    unsafe impl Send for Disk {}

    impl Disk {
        pub fn new() -> Self {
            let mut d = Self { query: 0, counter: 0 };
            // SAFETY: Ausgabezeiger gültig.
            unsafe {
                if PdhOpenQueryW(std::ptr::null(), 0, &mut d.query) == 0 {
                    let path = crate::win::wide(r"\PhysicalDisk(_Total)\% Idle Time");
                    PdhAddEnglishCounterW(d.query, path.as_ptr(), 0, &mut d.counter);
                    PdhCollectQueryData(d.query);
                }
            }
            d
        }

        /// Auslastung in % (100 − Leerlauf).
        pub fn busy(&mut self) -> Option<f32> {
            if self.counter == 0 {
                return None;
            }
            // SAFETY: gültige Handles.
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let mut v: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
                (PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) == 0)
                    .then(|| (100.0 - v.Anonymous.doubleValue).clamp(0.0, 100.0) as f32)
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub struct Disk;
    impl Disk {
        pub fn new() -> Self {
            Self
        }
        pub fn busy(&mut self) -> Option<f32> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context::default()
    }

    #[test]
    fn stutter_rule() {
        assert!(is_stutter(40.0, 7.0));
        assert!(!is_stutter(15.0, 7.0)); // nur 8 ms länger
        assert!(!is_stutter(20.0, 10.0));
    }

    #[test]
    fn causes() {
        let mut c = ctx();
        c.top = vec![TopProcess { exe: "cs2.exe".into(), cpu: 60.0 }, TopProcess { exe: "MsMpEng.exe".into(), cpu: 22.0 }];
        assert!(cause("stutter", &c, Some("cs2.exe")).contains("MsMpEng.exe"));
        c.disk = Some(95.0);
        assert!(cause("stutter", &c, Some("cs2.exe")).starts_with("Festplatte"));
        let mut p = ctx();
        p.download_mbps = Some(40.0);
        assert!(cause("ping", &p, None).contains("Datenverkehr"));
        p.download_mbps = Some(0.1);
        p.wlan = true;
        assert!(cause("ping", &p, None).contains("WLAN"));
        assert!(cause("stutter", &ctx(), None).contains("Keine Ursache"));
    }
}
