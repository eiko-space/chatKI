//! Ping-Messung per ICMP (Windows-API, keine Admin-Rechte nötig) und Auswertung.

use serde::Serialize;
use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const INTERVAL: Duration = Duration::from_millis(500);
const WINDOW: Duration = Duration::from_secs(60);
const TIMEOUT_MS: u32 = 1000;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PingStats {
    pub label: String,
    pub target: Option<String>,
    pub last_ms: Option<f32>,
    pub average_ms: Option<f32>,
    /// Mittlere Abweichung aufeinanderfolgender Pings.
    pub jitter_ms: Option<f32>,
    pub loss_percent: Option<f32>,
    /// Anzahl Ping-Spitzen im Messfenster (siehe `spike_threshold`).
    pub spikes: Option<u32>,
    pub spike_threshold_ms: Option<f32>,
    pub samples: u32,
    pub window_secs: f32,
}

pub struct PingStore {
    label: String,
    target: Option<Ipv4Addr>,
    samples: VecDeque<(Instant, Option<f32>)>,
}

impl PingStore {
    pub fn new(label: &str) -> Self {
        Self { label: label.into(), target: None, samples: VecDeque::new() }
    }

    pub fn set_target(&mut self, target: Option<Ipv4Addr>) {
        if self.target != target {
            self.target = target;
            self.samples.clear();
        }
    }

    pub fn push(&mut self, rtt_ms: Option<f32>) {
        let now = Instant::now();
        self.samples.push_back((now, rtt_ms));
        while self.samples.front().is_some_and(|(t, _)| now.duration_since(*t) > WINDOW) {
            self.samples.pop_front();
        }
    }

    pub fn stats(&self) -> PingStats {
        self.stats_since(None)
    }

    /// Statistik nur über Pings ab `since` (für Messungen mit festem Start).
    pub fn stats_since(&self, since: Option<Instant>) -> PingStats {
        let window: Vec<&(Instant, Option<f32>)> = self.samples.iter().filter(|(t, _)| since.is_none_or(|s| *t >= s)).collect();
        let rtts: Vec<f32> = window.iter().filter_map(|(_, r)| *r).collect();
        let total = window.len();
        let mut s = PingStats {
            label: self.label.clone(),
            target: self.target.map(|t| t.to_string()),
            samples: total as u32,
            window_secs: window.first().map_or(0.0, |(t, _)| t.elapsed().as_secs_f32()),
            ..Default::default()
        };
        if total == 0 {
            return s;
        }
        s.last_ms = window.last().and_then(|(_, r)| *r);
        s.loss_percent = Some(100.0 * (total - rtts.len()) as f32 / total as f32);
        if rtts.is_empty() {
            return s;
        }
        s.average_ms = Some(rtts.iter().sum::<f32>() / rtts.len() as f32);
        if rtts.len() >= 2 {
            let diffs: f32 = rtts.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
            s.jitter_ms = Some(diffs / (rtts.len() - 1) as f32);
        }
        if rtts.len() >= 10 {
            let threshold = spike_threshold(median(&rtts));
            s.spike_threshold_ms = Some(threshold);
            s.spikes = Some(count_spikes(&rtts, threshold));
        }
        s
    }
}

fn median(v: &[f32]) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    s[s.len() / 2]
}

/// Ein Spike ist ein Ping, der deutlich über dem Normalwert (Median) liegt:
/// mindestens 30 ms bzw. 50 % darüber – je nachdem, was größer ist.
pub fn spike_threshold(median: f32) -> f32 {
    median + (median * 0.5).max(30.0)
}

/// Zählt zusammenhängende Spitzen nur einmal (eine Spitze über 3 Pings = 1 Spike).
fn count_spikes(rtts: &[f32], threshold: f32) -> u32 {
    let mut count = 0;
    let mut inside = false;
    for &r in rtts {
        let high = r > threshold;
        if high && !inside {
            count += 1;
        }
        inside = high;
    }
    count
}

/// Startet einen Hintergrund-Thread, der das Ziel alle 500 ms anpingt.
/// `resolve` liefert das (ggf. wechselnde) Ziel, z. B. den aktuellen Router.
pub fn spawn(store: Arc<Mutex<PingStore>>, resolve: fn() -> Option<Ipv4Addr>) {
    std::thread::Builder::new()
        .name("ping".into())
        .spawn(move || {
            let pinger = platform::Pinger::new();
            let mut target = None;
            let mut last_resolve: Option<Instant> = None;
            loop {
                let started = Instant::now();
                if last_resolve.is_none_or(|t| t.elapsed() > Duration::from_secs(10)) {
                    target = resolve();
                    last_resolve = Some(started);
                    store.lock().unwrap().set_target(target);
                }
                if let (Some(p), Some(ip)) = (&pinger, target) {
                    let rtt = p.ping(ip, TIMEOUT_MS);
                    store.lock().unwrap().push(rtt);
                }
                std::thread::sleep(INTERVAL.saturating_sub(started.elapsed()));
            }
        })
        .expect("Ping-Thread konnte nicht gestartet werden");
}

/// Referenzziel „Internet“: Cloudflare-DNS, weltweit erreichbar und antwortet auf Ping.
pub fn internet_target() -> Option<Ipv4Addr> {
    Some(Ipv4Addr::new(1, 1, 1, 1))
}

/// Router (Standard-Gateway). Hilft zu unterscheiden: liegt das Problem im Heimnetz/WLAN oder beim Anbieter?
pub fn router_target() -> Option<Ipv4Addr> {
    platform::default_gateway()
}

#[cfg(windows)]
mod platform {
    use std::net::Ipv4Addr;
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetBestRoute, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY, MIB_IPFORWARDROW,
    };

    pub struct Pinger(HANDLE);

    impl Pinger {
        pub fn new() -> Option<Self> {
            // SAFETY: Keine Parameter; Rückgabe wird geprüft.
            let h = unsafe { IcmpCreateFile() };
            (h != INVALID_HANDLE_VALUE && !h.is_null()).then_some(Self(h))
        }

        pub fn ping(&self, ip: Ipv4Addr, timeout_ms: u32) -> Option<f32> {
            let payload = [0x47u8; 32];
            // Puffer: Antwort-Struktur + Nutzdaten + 8 Byte ICMP-Fehlerinfo (laut Doku), großzügig bemessen.
            let mut buf = [0u64; 32];
            let dest = u32::from_ne_bytes(ip.octets());
            // SAFETY: Handle gültig, Puffer groß genug und 8-Byte-ausgerichtet.
            let n = unsafe {
                IcmpSendEcho(
                    self.0,
                    dest,
                    payload.as_ptr().cast(),
                    payload.len() as u16,
                    std::ptr::null(),
                    buf.as_mut_ptr().cast(),
                    std::mem::size_of_val(&buf) as u32,
                    timeout_ms,
                )
            };
            if n == 0 {
                return None;
            }
            // SAFETY: Bei Erfolg beginnt der Puffer mit einer ICMP_ECHO_REPLY-Struktur.
            let reply = unsafe { &*(buf.as_ptr() as *const ICMP_ECHO_REPLY) };
            (reply.Status == 0).then_some(reply.RoundTripTime as f32)
        }
    }

    impl Drop for Pinger {
        fn drop(&mut self) {
            // SAFETY: Handle stammt von IcmpCreateFile.
            unsafe { IcmpCloseHandle(self.0) };
        }
    }

    pub fn default_gateway() -> Option<Ipv4Addr> {
        // SAFETY: MIB_IPFORWARDROW ist eine reine Datenstruktur; Nullinitialisierung ist gültig.
        let mut row: MIB_IPFORWARDROW = unsafe { std::mem::zeroed() };
        let dest = u32::from_ne_bytes([1, 1, 1, 1]);
        // SAFETY: Zeiger auf gültige Struktur.
        if unsafe { GetBestRoute(dest, 0, &mut row) } != 0 {
            return None;
        }
        let hop = Ipv4Addr::from(row.dwForwardNextHop.to_ne_bytes());
        (!hop.is_unspecified()).then_some(hop)
    }
}

#[cfg(not(windows))]
mod platform {
    use std::net::Ipv4Addr;
    pub struct Pinger;
    impl Pinger {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn ping(&self, _ip: Ipv4Addr, _timeout_ms: u32) -> Option<f32> {
            None
        }
    }
    pub fn default_gateway() -> Option<Ipv4Addr> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_loss_jitter_spikes() {
        let mut s = PingStore::new("Test");
        for r in [20.0, 22.0, 20.0, 21.0, 20.0, 120.0, 130.0, 20.0, 21.0, 20.0] {
            s.push(Some(r));
        }
        s.push(None);
        s.push(None);
        let st = s.stats();
        assert_eq!(st.samples, 12);
        assert!((st.loss_percent.unwrap() - 100.0 * 2.0 / 12.0).abs() < 0.01);
        assert_eq!(st.spikes, Some(1)); // 120 + 130 hängen zusammen
        assert_eq!(st.spike_threshold_ms, Some(51.0)); // Median 21 + 30
        assert!(st.jitter_ms.unwrap() > 0.0);
        assert_eq!(st.last_ms, None);
    }

    #[test]
    fn empty_store_has_no_values() {
        let st = PingStore::new("Test").stats();
        assert_eq!(st.average_ms, None);
        assert_eq!(st.loss_percent, None);
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    #[test]
    fn pings_loopback() {
        let p = super::platform::Pinger::new().expect("ICMP-Handle");
        let rtt = p.ping(std::net::Ipv4Addr::LOCALHOST, 1000);
        println!("loopback rtt: {rtt:?}, gateway: {:?}", super::platform::default_gateway());
        assert!(rtt.is_some());
    }
}
