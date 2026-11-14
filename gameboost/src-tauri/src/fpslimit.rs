//! FPS-Limit-Finder: empfiehlt ein ideales FPS-Limit.
//! Läuft die GPU ständig bei ~100 %, steigt der Input-Delay stark. Ein Limit knapp darunter
//! macht die Frametimes gleichmäßiger und senkt die Verzögerung – oft spürbarer als mehr FPS.

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FpsAdvice {
    /// "gpu_bound" | "cpu_bound" | "headroom" | "unknown"
    pub situation: &'static str,
    pub headline: String,
    pub detail: String,
    /// Empfohlenes FPS-Limit (None, wenn keins nötig)
    pub recommended_cap: Option<u32>,
    pub how: Vec<String>,
}

/// Berechnet die Empfehlung aus Durchschnitts-FPS, GPU- und CPU-Auslastung und Monitor-Hz.
pub fn advise(avg_fps: Option<f32>, gpu: Option<f32>, cpu: Option<f32>, monitor_hz: Option<u32>) -> FpsAdvice {
    let how = |cap: u32| {
        vec![
            format!("Am besten im Spiel ein FPS-Limit von {cap} setzen (Grafikeinstellungen → „Max FPS“)."),
            "Alternativ im NVIDIA-/AMD-Treiber ein globales Limit setzen.".to_string(),
            "Mit NVIDIA Reflex „Ein + Boost“ setzt das Spiel das Limit automatisch optimal.".to_string(),
        ]
    };

    let (Some(fps), Some(gpu)) = (avg_fps, gpu) else {
        return FpsAdvice {
            situation: "unknown",
            headline: "Noch keine Daten".into(),
            detail: "Starte die FPS-Messung und spiele kurz – dann analysiere ich deine Lastverteilung.".into(),
            recommended_cap: None,
            how: Vec::new(),
        };
    };

    if gpu >= 97.0 {
        // GPU am Limit → Cap knapp unter aktuelle FPS (auf 5er gerundet)
        let cap = (((fps * 0.97) / 5.0).floor() * 5.0).max(30.0) as u32;
        return FpsAdvice {
            situation: "gpu_bound",
            headline: format!("Deine Grafikkarte läuft am Limit ({gpu:.0} %)"),
            detail: format!(
                "Dadurch staut sich die Bildausgabe und der Input-Delay steigt. Ein Limit von etwa {cap} FPS (knapp unter deinen {fps:.0} FPS) macht das Bild gleichmäßiger und die Maus direkter."
            ),
            recommended_cap: Some(cap),
            how: how(cap),
        };
    }

    if cpu.is_some_and(|c| c >= 85.0) && gpu < 90.0 {
        return FpsAdvice {
            situation: "cpu_bound",
            headline: "Dein Prozessor ist der Engpass, nicht die Grafikkarte".into(),
            detail: format!(
                "Die GPU hat noch Reserven ({gpu:.0} %), die CPU ist stark ausgelastet. Ein FPS-Limit bringt hier wenig. Mehr hilft: Hintergrund-Apps schließen (macht der Boost) und im Spiel CPU-lastige Optionen senken (Sichtweite, Schatten, Menschenmengen)."
            ),
            recommended_cap: None,
            how: Vec::new(),
        };
    }

    // GPU hat Luft → Limit passend zum Monitor, damit es gleichmäßig bleibt
    if let Some(hz) = monitor_hz {
        if fps > hz as f32 + 5.0 {
            let cap = hz.saturating_sub(3);
            return FpsAdvice {
                situation: "headroom",
                headline: format!("Du hast mehr FPS ({fps:.0}) als dein Monitor zeigen kann ({hz} Hz)"),
                detail: format!(
                    "Bei G-Sync/FreeSync ist ein Limit von {cap} FPS ideal – das hält die Synchronisation aktiv und vermeidet Tearing, ohne unnötige Hitze."
                ),
                recommended_cap: Some(cap),
                how: how(cap),
            };
        }
    }

    FpsAdvice {
        situation: "headroom",
        headline: "Gute Lastverteilung".into(),
        detail: format!("GPU bei {gpu:.0} %, genug Reserve. Ein FPS-Limit ist gerade nicht nötig. Bei G-Sync/FreeSync kannst du eins knapp unter deiner Monitor-Hz setzen."),
        recommended_cap: monitor_hz.map(|h| h.saturating_sub(3)),
        how: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_bound_recommends_cap() {
        let a = advise(Some(142.0), Some(99.0), Some(50.0), Some(144));
        assert_eq!(a.situation, "gpu_bound");
        assert!(a.recommended_cap.is_some_and(|c| c >= 130 && c < 142));
    }

    #[test]
    fn cpu_bound_no_cap() {
        let a = advise(Some(90.0), Some(70.0), Some(95.0), Some(144));
        assert_eq!(a.situation, "cpu_bound");
        assert_eq!(a.recommended_cap, None);
    }

    #[test]
    fn headroom_over_hz() {
        let a = advise(Some(300.0), Some(60.0), Some(40.0), Some(144));
        assert_eq!(a.recommended_cap, Some(141));
    }

    #[test]
    fn no_data() {
        assert_eq!(advise(None, None, None, None).situation, "unknown");
    }
}
