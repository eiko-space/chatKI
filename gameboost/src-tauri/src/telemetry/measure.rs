//! 60-Sekunden-Messung für den Vorher-/Nachher-Vergleich.

use super::frames::summarize;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const DURATION: Duration = Duration::from_secs(60);

/// Ergebnis einer Messung. Alle Werte sind echte Messungen; `None` = nicht messbar.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub game: Option<String>,
    pub duration_secs: f32,
    pub frames: usize,
    pub fps_avg: Option<f32>,
    pub low1: Option<f32>,
    pub low01: Option<f32>,
    pub frametime_std_dev_ms: Option<f32>,
    /// Bilder, die mehr als doppelt so lange dauerten wie üblich (Median), hochgerechnet auf 1 Minute
    pub stutters_per_min: Option<f32>,
    pub ping_avg_ms: Option<f32>,
    pub jitter_ms: Option<f32>,
    pub loss_percent: Option<f32>,
    pub spikes_per_min: Option<f32>,
    pub cpu_avg: Option<f32>,
    pub gpu_avg: Option<f32>,
    /// Eingabe → Bild (nur, wenn währenddessen Maus/Tastatur benutzt wurde)
    #[serde(default)]
    pub input_latency_ms: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementProgress {
    pub label: String,
    pub elapsed_secs: f32,
    pub duration_secs: f32,
    /// Grund, falls die Messung ungültig wurde (z. B. Spiel gewechselt)
    pub problem: Option<String>,
}

pub struct Recording {
    pub label: String,
    started: Instant,
    duration: Duration,
    pid: Option<u32>,
    game: Option<String>,
    frametimes: Vec<f32>,
    cpu: Vec<f32>,
    gpu: Vec<f32>,
    latency: Vec<f32>,
    pub problem: Option<String>,
}

impl Recording {
    pub fn new(label: String) -> Self {
        Self::with_duration(label, DURATION)
    }

    pub fn with_duration(label: String, duration: Duration) -> Self {
        Self { label, started: Instant::now(), duration, pid: None, game: None, frametimes: Vec::new(), cpu: Vec::new(), gpu: Vec::new(), latency: Vec::new(), problem: None }
    }

    pub fn started(&self) -> Instant {
        self.started
    }

    pub fn done(&self) -> bool {
        self.started.elapsed() >= self.duration
    }

    pub fn progress(&self) -> MeasurementProgress {
        MeasurementProgress {
            label: self.label.clone(),
            elapsed_secs: self.started.elapsed().as_secs_f32().min(self.duration.as_secs_f32()),
            duration_secs: self.duration.as_secs_f32(),
            problem: self.problem.clone(),
        }
    }

    /// Wird zweimal pro Sekunde mit den neuen Daten aufgerufen.
    pub fn add_latency(&mut self, ms: Option<f32>) {
        if let Some(v) = ms {
            self.latency.push(v);
        }
    }

    pub fn add(&mut self, pid: Option<u32>, game: Option<String>, frametimes: &[f32], cpu: Option<f32>, gpu: Option<f32>) {
        match (self.pid, pid) {
            (None, Some(p)) => {
                self.pid = Some(p);
                self.game = game;
            }
            (Some(a), Some(b)) if a != b => self.problem = Some("Das gemessene Programm hat gewechselt.".into()),
            _ => {}
        }
        if pid.is_some() && pid == self.pid {
            self.frametimes.extend_from_slice(frametimes);
        }
        if let Some(c) = cpu {
            self.cpu.push(c);
        }
        if let Some(g) = gpu {
            self.gpu.push(g);
        }
    }

    pub fn finish(&self, ping: &super::ping::PingStats) -> Measurement {
        let secs = self.started.elapsed().as_secs_f32();
        let avg = |v: &[f32]| (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32);
        let summary = summarize(&self.frametimes);
        let frame_secs: f32 = self.frametimes.iter().sum::<f32>() / 1000.0;
        Measurement {
            game: self.game.clone(),
            duration_secs: secs,
            frames: self.frametimes.len(),
            fps_avg: summary.as_ref().map(|s| s.average_fps),
            low1: summary.as_ref().and_then(|s| s.low1),
            low01: summary.as_ref().and_then(|s| s.low01),
            frametime_std_dev_ms: summary.as_ref().map(|s| s.std_dev_ms),
            stutters_per_min: (frame_secs > 5.0).then(|| stutters(&self.frametimes) as f32 * 60.0 / frame_secs),
            ping_avg_ms: ping.average_ms,
            jitter_ms: ping.jitter_ms,
            loss_percent: ping.loss_percent,
            spikes_per_min: ping.spikes.filter(|_| ping.window_secs > 5.0).map(|s| s as f32 * 60.0 / ping.window_secs),
            cpu_avg: avg(&self.cpu),
            gpu_avg: avg(&self.gpu),
            input_latency_ms: avg(&self.latency),
        }
    }
}

/// Zählt Ruckler: Bilder, die mehr als doppelt so lange brauchen wie der Median.
pub fn stutters(ms: &[f32]) -> usize {
    if ms.len() < 10 {
        return 0;
    }
    let mut s = ms.to_vec();
    s.sort_by(f32::total_cmp);
    let median = s[s.len() / 2];
    ms.iter().filter(|v| **v > median * 2.0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_stutters() {
        let mut v = vec![10.0; 100];
        v[10] = 25.0;
        v[50] = 40.0;
        v[60] = 19.0; // unter 2× Median → kein Ruckler
        assert_eq!(stutters(&v), 2);
    }

    #[test]
    fn detects_program_switch() {
        let mut r = Recording::new("vorher".into());
        r.add(Some(1), Some("A".into()), &[10.0, 10.0], Some(50.0), None);
        r.add(Some(2), Some("B".into()), &[5.0], None, None);
        assert!(r.problem.is_some());
        assert_eq!(r.frametimes.len(), 2);
    }
}
