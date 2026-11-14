//! Speichert Frametimes aus PresentMon und berechnet FPS-Kennzahlen.
//! Alles hier ist reine Rechenlogik ohne Windows-Abhängigkeit (testbar).

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Zeitfenster für Durchschnitt und Lows.
pub const WINDOW: Duration = Duration::from_secs(60);
/// Ein Prozess gilt als „aktiv“, wenn er in dieser Zeit Frames geliefert hat.
const ACTIVE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
struct Frame {
    seq: u64,
    at: Instant,
    pid: u32,
    ms: f32,
    input_latency: Option<f32>,
    pc_latency: Option<f32>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FpsStats {
    pub app: String,
    pub game: Option<&'static str>,
    pub pid: u32,
    pub current: f32,
    pub average: f32,
    /// `None`, solange zu wenige Frames für eine aussagekräftige Berechnung vorliegen.
    pub low1: Option<f32>,
    pub low01: Option<f32>,
    pub frametime_avg_ms: f32,
    pub frametime_std_dev_ms: f32,
    pub frame_count: usize,
    pub window_secs: f32,
    pub input_latency_ms: Option<f32>,
    pub pc_latency_ms: Option<f32>,
}

#[derive(Default)]
pub struct FrameStore {
    frames: VecDeque<Frame>,
    names: HashMap<u32, String>,
    next_seq: u64,
    target: Option<u32>,
    last_sent_seq: u64,
}

impl FrameStore {
    pub fn push(&mut self, pid: u32, app: &str, ms: f32, input_latency: Option<f32>, pc_latency: Option<f32>) {
        if !(ms.is_finite() && ms > 0.0 && ms < 5_000.0) {
            return;
        }
        self.names.entry(pid).or_insert_with(|| app.to_string());
        self.next_seq += 1;
        self.frames.push_back(Frame { seq: self.next_seq, at: Instant::now(), pid, ms, input_latency, pc_latency });
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn prune(&mut self, now: Instant) {
        while let Some(f) = self.frames.front() {
            if now.duration_since(f.at) > WINDOW {
                self.frames.pop_front();
            } else {
                break;
            }
        }
        let frames = &self.frames;
        self.names.retain(|pid, _| frames.iter().any(|f| f.pid == *pid));
    }

    /// Wählt aus, welches Programm angezeigt wird:
    /// bekanntes Spiel > Vordergrund-Fenster > Programm mit den meisten Frames.
    pub fn select_target(&mut self, now: Instant, foreground: Option<u32>) {
        self.prune(now);
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for f in self.frames.iter().filter(|f| now.duration_since(f.at) <= ACTIVE) {
            *counts.entry(f.pid).or_default() += 1;
        }
        let is_game = |pid: &u32| self.names.get(pid).and_then(|n| crate::games::known_game(n)).is_some();
        let best_game = counts.iter().filter(|(p, _)| is_game(p)).max_by_key(|(_, c)| **c).map(|(p, _)| *p);

        let new_target = match self.target {
            Some(t) if counts.contains_key(&t) && (is_game(&t) || best_game.is_none()) => Some(t),
            _ => best_game
                .or_else(|| foreground.filter(|p| counts.contains_key(p)))
                .or_else(|| counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p)),
        };
        if new_target != self.target {
            self.target = new_target;
            self.last_sent_seq = self.next_seq; // Graph startet für das neue Ziel frisch
        }
    }

    pub fn stats(&self) -> Option<FpsStats> {
        let pid = self.target?;
        let frames: Vec<&Frame> = self.frames.iter().filter(|f| f.pid == pid).collect();
        if frames.is_empty() {
            return None;
        }
        let ms: Vec<f32> = frames.iter().map(|f| f.ms).collect();
        let summary = summarize(&ms)?;

        // Aktuelle FPS: die neuesten Frames, die zusammen ~1 Sekunde ergeben.
        let mut sum = 0.0f32;
        let mut n = 0usize;
        for v in ms.iter().rev() {
            sum += v;
            n += 1;
            if sum >= 1000.0 {
                break;
            }
        }
        let current = 1000.0 * n as f32 / sum;

        let app = self.names.get(&pid).cloned().unwrap_or_default();
        Some(FpsStats {
            game: crate::games::known_game(&app),
            app,
            pid,
            current,
            average: summary.average_fps,
            low1: summary.low1,
            low01: summary.low01,
            frametime_avg_ms: summary.mean_ms,
            frametime_std_dev_ms: summary.std_dev_ms,
            frame_count: ms.len(),
            window_secs: ms.iter().sum::<f32>() / 1000.0,
            input_latency_ms: mean(frames.iter().filter_map(|f| f.input_latency)),
            pc_latency_ms: mean(frames.iter().filter_map(|f| f.pc_latency)),
        })
    }

    /// Frametimes des Ziels, die seit dem letzten Aufruf neu hinzugekommen sind (für den Graphen).
    pub fn take_new_frametimes(&mut self) -> Vec<f32> {
        let Some(pid) = self.target else { return Vec::new() };
        let since = self.last_sent_seq;
        let out: Vec<f32> = self.frames.iter().filter(|f| f.seq > since && f.pid == pid).map(|f| f.ms).collect();
        self.last_sent_seq = self.next_seq;
        out
    }
}

fn mean(it: impl Iterator<Item = f32>) -> Option<f32> {
    let (s, n) = it.filter(|v| v.is_finite() && *v > 0.0).fold((0.0f32, 0usize), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| s / n as f32)
}

#[derive(Debug, PartialEq)]
pub struct Summary {
    pub average_fps: f32,
    pub low1: Option<f32>,
    pub low01: Option<f32>,
    pub mean_ms: f32,
    pub std_dev_ms: f32,
}

/// Berechnet Kennzahlen aus Frametimes (ms).
///
/// * Durchschnitt = 1000 / mittlere Frametime (zeitgewichtet, wie bei CapFrameX/PresentMon üblich)
/// * 1 %-Low = 1000 / Mittelwert der langsamsten 1 % Frametimes (erst ab 100 Frames)
/// * 0,1 %-Low analog mit den langsamsten 0,1 % (erst ab 1000 Frames)
pub fn summarize(ms: &[f32]) -> Option<Summary> {
    if ms.is_empty() {
        return None;
    }
    let n = ms.len();
    let mean_ms = ms.iter().sum::<f32>() / n as f32;
    let var = ms.iter().map(|v| (v - mean_ms).powi(2)).sum::<f32>() / n as f32;

    let mut sorted = ms.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a)); // langsamste zuerst
    let worst = |count: usize| {
        let s: f32 = sorted[..count].iter().sum();
        1000.0 * count as f32 / s
    };

    Some(Summary {
        average_fps: 1000.0 / mean_ms,
        low1: (n >= 100).then(|| worst(n / 100)),
        low01: (n >= 1000).then(|| worst(n / 1000)),
        mean_ms,
        std_dev_ms: var.sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_frametimes() {
        let s = summarize(&[10.0; 1000]).unwrap();
        assert!((s.average_fps - 100.0).abs() < 0.01);
        assert!((s.low1.unwrap() - 100.0).abs() < 0.01);
        assert!((s.low01.unwrap() - 100.0).abs() < 0.01);
        assert!(s.std_dev_ms < 1e-4);
    }

    #[test]
    fn lows_capture_stutter() {
        let mut ms = vec![10.0; 990];
        ms.extend(vec![50.0; 10]); // 1 % Ruckler à 50 ms
        let s = summarize(&ms).unwrap();
        assert!((s.low1.unwrap() - 20.0).abs() < 0.01);
        assert!((s.low01.unwrap() - 20.0).abs() < 0.01);
        assert!(s.average_fps < 100.0 && s.average_fps > 95.0);
    }

    #[test]
    fn not_enough_frames_for_lows() {
        let s = summarize(&[16.6; 50]).unwrap();
        assert_eq!(s.low1, None);
        assert_eq!(s.low01, None);
    }

    #[test]
    fn prefers_known_game_and_streams_new_frames() {
        let mut st = FrameStore::default();
        for _ in 0..10 {
            st.push(1, "chrome.exe", 16.0, None, None);
            st.push(2, "cs2.exe", 4.0, Some(20.0), None);
        }
        st.select_target(Instant::now(), Some(1));
        let s = st.stats().unwrap();
        assert_eq!(s.pid, 2);
        assert_eq!(s.game, Some("Counter-Strike 2"));
        assert_eq!(s.input_latency_ms, Some(20.0));
        assert!(st.take_new_frametimes().is_empty()); // neues Ziel → Graph beginnt ab jetzt
        st.push(2, "cs2.exe", 5.0, None, None);
        assert_eq!(st.take_new_frametimes(), vec![5.0]);
    }
}
