//! Lag-Test fürs Heimnetz: Ping im Ruhezustand vs. Ping unter voller Last (Download/Upload).
//! Steigt der Ping unter Last stark an („Bufferbloat“), liegt das am Router – nicht am PC.

use serde::Serialize;
use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DOWN_URL: &str = "https://speed.cloudflare.com/__down?bytes=25000000";
const UP_URL: &str = "https://speed.cloudflare.com/__up";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LagResult {
    pub idle_ms: Option<f32>,
    pub download_load_ms: Option<f32>,
    pub upload_load_ms: Option<f32>,
    pub download_mbps: Option<f32>,
    pub upload_mbps: Option<f32>,
    /// Zusätzlicher Ping unter Last (schlechterer Wert von Download/Upload)
    pub added_ms: Option<f32>,
    /// Note wie in der Schule: "A+", "A", "B", "C", "D", "F"
    pub grade: String,
    pub verdict: String,
    pub tips: Vec<String>,
}

/// Bewertung des zusätzlichen Pings unter Last.
pub fn grade(added_ms: f32) -> (&'static str, &'static str) {
    match added_ms {
        x if x < 5.0 => ("A+", "Perfekt – dein Ping bleibt auch bei voller Leitung stabil."),
        x if x < 30.0 => ("A", "Sehr gut – kaum Lag, wenn jemand nebenbei streamt oder lädt."),
        x if x < 60.0 => ("B", "Gut – bei großen Downloads merkst du leichte Lags."),
        x if x < 200.0 => ("C", "Spürbar – wenn zu Hause jemand streamt oder lädt, laggt es im Spiel."),
        x if x < 400.0 => ("D", "Schlecht – Downloads im Heimnetz verursachen deutliche Lags."),
        _ => ("F", "Sehr schlecht – sobald die Leitung ausgelastet ist, wird Online-Spielen kaum möglich."),
    }
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f32::total_cmp);
    Some(v[v.len() / 2])
}

/// Latenz per TCP-Verbindungsaufbau zu Cloudflare (funktioniert auch, wenn Ping blockiert ist).
fn rtt(addr: SocketAddr) -> Option<f32> {
    let t = Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_millis(2000)).ok().map(|_| t.elapsed().as_secs_f32() * 1000.0)
}

fn sample_latency(addr: SocketAddr, dur: Duration) -> Vec<f32> {
    let end = Instant::now() + dur;
    let mut v = Vec::new();
    while Instant::now() < end {
        if let Some(ms) = rtt(addr) {
            v.push(ms);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    v
}

/// Erzeugt für `dur` Last (Download oder Upload) und misst nebenbei den Ping.
fn under_load(addr: SocketAddr, upload: bool, dur: Duration) -> (Vec<f32>, Option<f32>) {
    let stop = Arc::new(AtomicBool::new(false));
    let bytes = Arc::new(AtomicU64::new(0));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .tls_connector(Arc::new(native_tls::TlsConnector::new().expect("TLS")))
        .build();
    let workers: Vec<_> = (0..if upload { 3 } else { 4 })
        .map(|_| {
            let (stop, bytes, agent) = (stop.clone(), bytes.clone(), agent.clone());
            std::thread::spawn(move || {
                let chunk = vec![0u8; 4 * 1024 * 1024];
                while !stop.load(Ordering::Relaxed) {
                    if upload {
                        if agent.post(UP_URL).send_bytes(&chunk).is_ok() {
                            bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    } else {
                        let Ok(resp) = agent.get(DOWN_URL).call() else { break };
                        let mut r = resp.into_reader();
                        let mut buf = [0u8; 64 * 1024];
                        while !stop.load(Ordering::Relaxed) {
                            match r.read(&mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => {
                                    bytes.fetch_add(n as u64, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }
            })
        })
        .collect();
    // Kurz anlaufen lassen, damit die Leitung wirklich voll ist
    std::thread::sleep(Duration::from_millis(1500));
    let start_bytes = bytes.load(Ordering::Relaxed);
    let t = Instant::now();
    let lat = sample_latency(addr, dur);
    let secs = t.elapsed().as_secs_f64();
    let mbps = ((bytes.load(Ordering::Relaxed) - start_bytes) as f64 * 8.0 / 1e6 / secs) as f32;
    stop.store(true, Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }
    (lat, (mbps > 0.1).then_some(mbps))
}

pub fn run(progress: &dyn Fn(u32, &str)) -> Result<LagResult, String> {
    use std::net::ToSocketAddrs;
    let addr = ("1.1.1.1", 443).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("Keine Verbindung.")?;

    progress(5, "Ping im Ruhezustand …");
    let idle = median(sample_latency(addr, Duration::from_secs(5))).ok_or("Keine Internetverbindung.")?;
    progress(30, "Ping beim Download …");
    let (dl_lat, dl_mbps) = under_load(addr, false, Duration::from_secs(8));
    progress(65, "Ping beim Upload …");
    let (ul_lat, ul_mbps) = under_load(addr, true, Duration::from_secs(7));
    progress(100, "Fertig");

    let dl = median(dl_lat);
    let ul = median(ul_lat);
    let added = [dl, ul].into_iter().flatten().map(|v| (v - idle).max(0.0)).reduce(f32::max);
    let (grade, verdict) = added.map(grade).unwrap_or(("?", "Unter Last kam keine Antwort – die Leitung war komplett verstopft."));
    let mut tips = Vec::new();
    if added.is_none_or(|a| a >= 30.0) {
        tips.push("Im Router „QoS“, „Smart Queue“, „SQM“ oder „Gaming-Priorisierung“ einschalten (FRITZ!Box: Internet → Filter → Priorisierung → deinen PC als „Echtzeitanwendung“ eintragen).".into());
        tips.push("Wenn es das gibt: die Bandbreite im Router auf ca. 90 % deiner echten Geschwindigkeit begrenzen – das verhindert die Warteschlange.".into());
        tips.push("Große Downloads (Spiele-Updates, Cloud-Backups) nicht während des Spielens laufen lassen.".into());
    }
    Ok(LagResult {
        idle_ms: Some(idle),
        download_load_ms: dl,
        upload_load_ms: ul,
        download_mbps: dl_mbps,
        upload_mbps: ul_mbps,
        added_ms: added,
        grade: grade.into(),
        verdict: verdict.into(),
        tips,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades() {
        assert_eq!(grade(2.0).0, "A+");
        assert_eq!(grade(25.0).0, "A");
        assert_eq!(grade(120.0).0, "C");
        assert_eq!(grade(900.0).0, "F");
    }
}
