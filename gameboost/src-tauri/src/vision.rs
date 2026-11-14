//! Sicht-Boost: hellt dunkle Bildbereiche über die Gamma-Kurve des Bildschirms auf.
//! Das ist eine reine Bildschirmeinstellung (wie im Grafiktreiber) – kein Eingriff ins Spiel.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Datei mit der Original-Gammakurve (für Wiederherstellung auch nach Absturz).
static BACKUP: Mutex<Option<PathBuf>> = Mutex::new(None);
static LEVEL: Mutex<u32> = Mutex::new(0);

/// Berechnet die Gammakurve. `level` 0–100: 0 = unverändert, 100 = stärkste Aufhellung der Schatten.
pub fn ramp(level: u32) -> [u16; 768] {
    let gamma = 1.0 + (level.min(100) as f64 / 100.0) * 0.9; // bis Gamma 1,9
    let mut r = [0u16; 768];
    for i in 0..256 {
        let x = i as f64 / 255.0;
        let y = x.powf(1.0 / gamma);
        // Windows lehnt zu starke Abweichungen ab → auf zulässigen Bereich begrenzen
        let max = ((i as f64 + 128.0) * 256.0).min(65535.0);
        let v = (y * 65535.0).min(max).round() as u16;
        r[i] = v;
        r[i + 256] = v;
        r[i + 512] = v;
    }
    r
}

pub fn init(data_dir: &Path) {
    let file = data_dir.join("gamma-original.bin");
    // Liegt noch eine Sicherung herum, wurde die App nicht sauber beendet → Original wiederherstellen.
    if let Ok(bytes) = std::fs::read(&file) {
        if let Some(orig) = from_bytes(&bytes) {
            let _ = platform::set(&orig);
        }
        let _ = std::fs::remove_file(&file);
    }
    *BACKUP.lock().unwrap() = Some(file);
}

fn to_bytes(r: &[u16; 768]) -> Vec<u8> {
    r.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn from_bytes(b: &[u8]) -> Option<[u16; 768]> {
    if b.len() != 1536 {
        return None;
    }
    let mut r = [0u16; 768];
    for (i, c) in b.chunks(2).enumerate() {
        r[i] = u16::from_le_bytes([c[0], c[1]]);
    }
    Some(r)
}

pub fn level() -> u32 {
    *LEVEL.lock().unwrap()
}

pub fn set_level(level: u32) -> Result<(), String> {
    let file = BACKUP.lock().unwrap().clone().ok_or("Nicht bereit.")?;
    if level == 0 {
        return reset();
    }
    // Original nur beim ersten Einschalten sichern
    if !file.exists() {
        let orig = platform::get().ok_or("Bildschirm-Gamma nicht lesbar.")?;
        std::fs::write(&file, to_bytes(&orig)).map_err(|e| e.to_string())?;
    }
    platform::set(&ramp(level)).map_err(|_| "Windows hat die Einstellung abgelehnt (Grafiktreiber erlaubt keine Gamma-Änderung).".to_string())?;
    *LEVEL.lock().unwrap() = level;
    Ok(())
}

/// Originalzustand wiederherstellen (beim Ausschalten und Beenden).
pub fn reset() -> Result<(), String> {
    let Some(file) = BACKUP.lock().unwrap().clone() else { return Ok(()) };
    if let Ok(bytes) = std::fs::read(&file) {
        if let Some(orig) = from_bytes(&bytes) {
            platform::set(&orig).map_err(|_| "Zurücksetzen fehlgeschlagen.".to_string())?;
        }
        let _ = std::fs::remove_file(&file);
    }
    *LEVEL.lock().unwrap() = 0;
    Ok(())
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows_sys::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

    pub fn get() -> Option<[u16; 768]> {
        let mut r = [0u16; 768];
        // SAFETY: Bildschirm-DC wird geholt und freigegeben; Puffer hat die geforderte Größe (3×256 WORD).
        unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let ok = GetDeviceGammaRamp(dc, r.as_mut_ptr().cast()) != 0;
            ReleaseDC(std::ptr::null_mut(), dc);
            ok.then_some(r)
        }
    }

    pub fn set(r: &[u16; 768]) -> Result<(), ()> {
        // SAFETY: wie oben.
        unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let ok = SetDeviceGammaRamp(dc, r.as_ptr().cast()) != 0;
            ReleaseDC(std::ptr::null_mut(), dc);
            if ok { Ok(()) } else { Err(()) }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn get() -> Option<[u16; 768]> {
        None
    }
    pub fn set(_r: &[u16; 768]) -> Result<(), ()> {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_lifts_shadows_but_keeps_ends() {
        let flat = ramp(0);
        let boosted = ramp(100);
        assert_eq!(flat[0], 0);
        assert_eq!(boosted[255], 65535);
        assert!(boosted[40] > flat[40], "dunkle Werte werden heller");
        // monoton steigend
        assert!(boosted.windows(2).take(255).all(|w| w[1] >= w[0]));
        // innerhalb der Windows-Grenze
        assert!((0..256).all(|i| (boosted[i] as f64) <= (i as f64 + 128.0) * 256.0 + 1.0));
    }

    #[test]
    fn bytes_roundtrip() {
        let r = ramp(50);
        assert_eq!(from_bytes(&to_bytes(&r)), Some(r));
    }
}
