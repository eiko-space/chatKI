//! PC-Check: prüft typische Bremsen und erklärt sie in einfachem Deutsch. Bewertung 0–100.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Good,
    Warn,
    Bad,
    Info,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub id: &'static str,
    pub title: &'static str,
    pub level: Level,
    /// Kurzer Messwert, z. B. „vor 412 Tagen“ oder „60 von 144 Hz“
    pub value: String,
    /// Erklärung in einfachem Deutsch
    pub explanation: String,
    /// Technische Details für Profis
    pub details: Option<String>,
    /// Kennung einer automatischen Korrektur (siehe `run_fix`)
    pub fix: Option<&'static str>,
    pub fix_label: Option<&'static str>,
    pub fix_needs_admin: bool,
    /// Konkrete Schritte „So behebst du es“ (leer, wenn alles in Ordnung ist)
    pub steps: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub score: u32,
    pub items: Vec<CheckItem>,
}

/// Bewertung: 100 minus Abzüge (Problem −15, Hinweis −6). Info zählt nicht.
pub fn score(items: &[CheckItem]) -> u32 {
    let minus: i32 = items.iter().map(|i| match i.level { Level::Bad => 15, Level::Warn => 6, _ => 0 }).sum();
    (100 - minus).clamp(0, 100) as u32
}

/// Grober Hinweis, ob XMP/EXPO fehlen könnte. Liefert Erklärung + Stufe.
pub fn ram_assessment(kind: &str, rated: u32, configured: u32) -> (Level, String) {
    if configured > 0 && rated > configured {
        return (Level::Warn, format!("Dein Arbeitsspeicher läuft mit {configured} MT/s, kann laut Modul aber {rated} MT/s. Im BIOS „XMP“ (Intel) bzw. „EXPO/DOCP“ (AMD) aktivieren bringt oft spürbar bessere 1 %-Lows."));
    }
    let low = match kind {
        "DDR4" => configured <= 2666,
        "DDR5" => configured <= 4800,
        _ => false,
    };
    if low {
        (Level::Warn, format!("{kind} mit {configured} MT/s ist der Standardtakt ohne XMP/EXPO. Falls dein RAM für mehr ausgelegt ist (steht auf dem Modul bzw. im Datenblatt), kannst du XMP/EXPO im BIOS aktivieren."))
    } else {
        (Level::Good, format!("{kind} läuft mit {configured} MT/s."))
    }
}

/// Liest Speichermodule aus der SMBIOS-Tabelle (Typ 17). Rückgabe: (Typ, Nenntakt, eingestellter Takt) je belegtem Modul.
pub fn parse_smbios_memory(raw: &[u8]) -> Vec<(String, u32, u32)> {
    let mut out = Vec::new();
    if raw.len() < 8 {
        return out;
    }
    let len = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]) as usize;
    let data = &raw[8..(8 + len).min(raw.len())];
    let mut i = 0;
    while i + 4 <= data.len() {
        let ty = data[i];
        let hlen = data[i + 1] as usize;
        if hlen < 4 || i + hlen > data.len() {
            break;
        }
        let s = &data[i..i + hlen];
        if ty == 17 && hlen > 0x16 {
            let size = u16::from_le_bytes([s[0x0C], s[0x0D]]);
            if size != 0 && size != 0xFFFF {
                let kind = match s[0x12] {
                    0x1A => "DDR4",
                    0x22 => "DDR5",
                    0x18 => "DDR3",
                    0x1E => "LPDDR4",
                    0x23 => "LPDDR5",
                    _ => "RAM",
                };
                let rated = u16::from_le_bytes([s[0x15], s[0x16]]) as u32;
                let configured = if hlen > 0x21 { u16::from_le_bytes([s[0x20], s[0x21]]) as u32 } else { 0 };
                out.push((kind.to_string(), rated, if configured == 0 { rated } else { configured }));
            }
        }
        if ty == 127 {
            break;
        }
        // Formatierter Bereich, danach Strings bis zu einer doppelten Null
        let mut j = i + hlen;
        while j + 1 < data.len() && !(data[j] == 0 && data[j + 1] == 0) {
            j += 1;
        }
        i = j + 2;
    }
    out
}

/// Datum aus der Registry (z. B. „8-14-2024“) → Tage bis heute.
pub fn days_since_driver_date(date: &str, now_unix: u64) -> Option<u64> {
    let p: Vec<u32> = date.split('-').filter_map(|x| x.trim().parse().ok()).collect();
    let [m, d, y] = p[..] else { return None };
    let days = days_from_civil(y as i64, m as i64, d as i64)?;
    let today = (now_unix / 86_400) as i64;
    Some((today - days).max(0) as u64)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

#[cfg(windows)]
pub use imp::{run_checks, run_fix};

/// Höchste Bildwiederholrate des Hauptmonitors (für den FPS-Limit-Finder).
#[cfg(windows)]
pub fn primary_monitor_hz() -> Option<u32> {
    imp::displays().first().map(|d| d.current.max(d.max))
}
#[cfg(not(windows))]
pub fn primary_monitor_hz() -> Option<u32> {
    None
}

#[cfg(not(windows))]
pub fn run_checks(_gpu_name: Option<String>, _gpu_temp: Option<f32>, _wlan: bool) -> CheckReport {
    CheckReport { score: 0, items: Vec::new() }
}
#[cfg(not(windows))]
pub fn run_fix(_id: &str) -> Result<String, String> {
    Err("Nur unter Windows verfügbar.".into())
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::win::wide;
    use winreg::enums::*;
    use winreg::RegKey;

    fn item(id: &'static str, title: &'static str, level: Level, value: String, explanation: String) -> CheckItem {
        CheckItem { id, title, level, value, explanation, details: None, fix: None, fix_label: None, fix_needs_admin: false, steps: Vec::new() }
    }

    pub fn run_checks(gpu_name: Option<String>, gpu_temp: Option<f32>, wlan: bool) -> CheckReport {
        let mut items = vec![
            driver_check(gpu_name.as_deref()),
            monitor_check(),
            ram_check(),
            disk_check(),
            autostart_check(),
            throttle_check(),
            temp_check(gpu_temp),
            game_mode_check(),
            dvr_check(),
            hags_check(),
            network_check(wlan),
        ];
        items.retain(|i| !i.value.is_empty() || i.level != Level::Info);
        let order = |l: &Level| match l { Level::Bad => 0, Level::Warn => 1, Level::Info => 2, Level::Good => 3 };
        items.sort_by_key(|i| order(&i.level));
        CheckReport { score: score(&items), items }
    }

    fn hklm(path: &str) -> Option<RegKey> {
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).ok()
    }
    fn hkcu(path: &str) -> Option<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(path).ok()
    }

    // ---------------------------------------------------------------- Grafiktreiber
    fn driver_check(gpu: Option<&str>) -> CheckItem {
        const CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
        let mut best: Option<(String, String, String)> = None; // (Name, Datum, Version)
        if let Some(class) = hklm(CLASS) {
            for sub in class.enum_keys().flatten().filter(|k| k.chars().all(|c| c.is_ascii_digit())) {
                let Ok(k) = class.open_subkey(&sub) else { continue };
                let (Ok(desc), Ok(date)) = (k.get_value::<String, _>("DriverDesc"), k.get_value::<String, _>("DriverDate")) else { continue };
                let version = k.get_value::<String, _>("DriverVersion").unwrap_or_default();
                let matches = gpu.is_some_and(|g| g.eq_ignore_ascii_case(&desc));
                let is_ms = desc.contains("Microsoft");
                if matches || (best.is_none() && !is_ms) {
                    best = Some((desc, date, version));
                    if matches {
                        break;
                    }
                }
            }
        }
        let Some((name, date, version)) = best else {
            return item("driver", "Grafiktreiber", Level::Info, String::new(), "Grafiktreiber konnte nicht gelesen werden.".into());
        };
        let days = days_since_driver_date(&date, crate::storage::unix_now());
        let (level, text) = match days {
            Some(d) if d > 365 => (Level::Bad, "Über ein Jahr alt. Neue Treiber bringen oft mehr FPS und beheben Fehler in aktuellen Spielen.".into()),
            Some(d) if d > 180 => (Level::Warn, "Älter als ein halbes Jahr. Ein Update lohnt sich besonders bei neuen Spielen.".into()),
            Some(_) => (Level::Good, "Dein Grafiktreiber ist aktuell genug.".into()),
            None => (Level::Info, "Treiberdatum unbekannt.".into()),
        };
        let mut i = item("driver", "Grafiktreiber", level.clone(), days.map(|d| format!("vor {d} Tagen")).unwrap_or_default(), text);
        i.details = Some(format!("{name} · Version {version} · Datum {date}"));
        if level != Level::Good {
            let lower = name.to_lowercase();
            let (fix, steps): (&'static str, [&str; 3]) = if lower.contains("nvidia") {
                ("driver_nvidia", ["Auf „Treiber-Seite öffnen“ klicken – oder die „NVIDIA App“ öffnen und unter „Treiber“ aktualisieren", "Deine Karte wählen und den „Game Ready“-Treiber herunterladen", "Installieren („Schnellinstallation“) und PC neu starten"])
            } else if lower.contains("amd") || lower.contains("radeon") {
                ("driver_amd", ["Auf „Treiber-Seite öffnen“ klicken – oder „AMD Software: Adrenalin“ öffnen → „Nach Updates suchen“", "Den empfohlenen Treiber herunterladen", "Installieren und PC neu starten"])
            } else {
                ("driver_intel", ["Auf „Treiber-Seite öffnen“ klicken", "„Intel Driver & Support Assistant“ laden und starten", "Vorgeschlagenen Grafiktreiber installieren und PC neu starten"])
            };
            i.fix = Some(fix);
            i.fix_label = Some("Treiber-Seite öffnen");
            i.steps = std::iter::once(format!("Deine Grafikkarte: {name}")).chain(steps.iter().map(|x| x.to_string())).collect();
        }
        i
    }

    // ---------------------------------------------------------------- Monitor-Hz
    pub(super) struct Display {
        pub device: Vec<u16>,
        pub name: String,
        pub width: u32,
        pub height: u32,
        pub current: u32,
        pub max: u32,
    }

    pub(super) fn displays() -> Vec<Display> {
        use windows_sys::Win32::Graphics::Gdi::*;
        let mut out = Vec::new();
        for i in 0.. {
            // SAFETY: Strukturen mit korrekter Größe initialisiert.
            let mut dd: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
            dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if unsafe { EnumDisplayDevicesW(std::ptr::null(), i, &mut dd, 0) } == 0 {
                break;
            }
            if dd.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
                continue;
            }
            let device: Vec<u16> = dd.DeviceName.iter().copied().take_while(|c| *c != 0).chain(std::iter::once(0)).collect();
            let mut cur: DEVMODEW = unsafe { std::mem::zeroed() };
            cur.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if unsafe { EnumDisplaySettingsW(device.as_ptr(), ENUM_CURRENT_SETTINGS, &mut cur) } == 0 {
                continue;
            }
            let mut max = cur.dmDisplayFrequency;
            for m in 0.. {
                let mut dm: DEVMODEW = unsafe { std::mem::zeroed() };
                dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                if unsafe { EnumDisplaySettingsW(device.as_ptr(), m, &mut dm) } == 0 {
                    break;
                }
                if dm.dmPelsWidth == cur.dmPelsWidth && dm.dmPelsHeight == cur.dmPelsHeight && dm.dmBitsPerPel == cur.dmBitsPerPel {
                    max = max.max(dm.dmDisplayFrequency);
                }
            }
            out.push(Display { name: crate::win::from_wide(&dd.DeviceString), device, width: cur.dmPelsWidth, height: cur.dmPelsHeight, current: cur.dmDisplayFrequency, max });
        }
        out
    }

    fn monitor_check() -> CheckItem {
        let ds = displays();
        if ds.is_empty() {
            return item("monitor", "Bildwiederholrate", Level::Info, String::new(), String::new());
        }
        let below: Vec<&Display> = ds.iter().filter(|d| d.current + 1 < d.max).collect();
        let value = ds.iter().map(|d| format!("{} von {} Hz", d.current, d.max)).collect::<Vec<_>>().join(", ");
        if below.is_empty() {
            let mut i = item("monitor", "Bildwiederholrate", Level::Good, value, "Dein Bildschirm läuft mit der höchsten möglichen Hz-Zahl.".into());
            i.details = Some(ds.iter().map(|d| format!("{}: {}×{} @ {} Hz", d.name, d.width, d.height, d.current)).collect::<Vec<_>>().join("; "));
            return i;
        }
        let mut i = item("monitor", "Bildwiederholrate", Level::Bad, value,
            format!("Dein Bildschirm könnte {} Hz, läuft aber nur mit {} Hz. Das ist einer der häufigsten Fehler – du siehst dann weniger Bilder, als dein PC liefert.", below[0].max, below[0].current));
        i.fix = Some("monitor_max_hz");
        i.fix_label = Some("Auf maximale Hz stellen");
        i.steps = vec![
            "Am einfachsten: auf „Auf maximale Hz stellen“ klicken".into(),
            "Oder von Hand: Windows-Einstellungen → System → Anzeige → Erweiterte Anzeige → Bildwiederholrate".into(),
            "Klappt es nicht: Prüfe, ob das Kabel High-Hz kann (DisplayPort oder HDMI 2.0+) und am Grafikkarten-Anschluss steckt – nicht am Mainboard".into(),
        ];
        i
    }

    fn set_max_hz() -> Result<String, String> {
        use windows_sys::Win32::Graphics::Gdi::*;
        let mut changed = Vec::new();
        for d in displays().into_iter().filter(|d| d.current + 1 < d.max) {
            // SAFETY: DEVMODEW korrekt initialisiert; erst Test, dann Übernahme.
            let mut dm: DEVMODEW = unsafe { std::mem::zeroed() };
            dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            unsafe { EnumDisplaySettingsW(d.device.as_ptr(), ENUM_CURRENT_SETTINGS, &mut dm) };
            dm.dmDisplayFrequency = d.max;
            dm.dmFields = DM_DISPLAYFREQUENCY | DM_PELSWIDTH | DM_PELSHEIGHT;
            let test = unsafe { ChangeDisplaySettingsExW(d.device.as_ptr(), &dm, std::ptr::null_mut(), CDS_TEST, std::ptr::null()) };
            if test != DISP_CHANGE_SUCCESSFUL {
                return Err(format!("{}: {} Hz wird vom Treiber abgelehnt.", d.name, d.max));
            }
            let r = unsafe { ChangeDisplaySettingsExW(d.device.as_ptr(), &dm, std::ptr::null_mut(), CDS_UPDATEREGISTRY, std::ptr::null()) };
            if r != DISP_CHANGE_SUCCESSFUL {
                return Err(format!("{}: Umstellen fehlgeschlagen (Code {r}).", d.name));
            }
            changed.push(format!("{} → {} Hz", d.name, d.max));
        }
        Ok(if changed.is_empty() { "Alle Bildschirme liefen bereits mit maximaler Hz-Zahl.".into() } else { format!("Umgestellt: {}", changed.join(", ")) })
    }

    // ---------------------------------------------------------------- RAM
    fn ram_check() -> CheckItem {
        use windows_sys::Win32::System::SystemInformation::GetSystemFirmwareTable;
        const RSMB: u32 = u32::from_be_bytes(*b"RSMB");
        // SAFETY: erst Größe ermitteln, dann lesen.
        let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
        let mut buf = vec![0u8; size as usize];
        let got = if size > 0 { unsafe { GetSystemFirmwareTable(RSMB, 0, buf.as_mut_ptr().cast(), size) } } else { 0 };
        let modules = if got > 0 { parse_smbios_memory(&buf[..got as usize]) } else { Vec::new() };
        let Some((kind, rated, configured)) = modules.first().cloned() else {
            return item("ram", "Arbeitsspeicher-Takt", Level::Info, String::new(), String::new());
        };
        let (level, text) = ram_assessment(&kind, rated, configured);
        let mut i = item("ram", "Arbeitsspeicher-Takt", level, format!("{kind} · {configured} MT/s"), text);
        i.details = Some(format!("{} Modul(e); Nenntakt laut Modul {rated} MT/s, eingestellt {configured} MT/s.", modules.len()));
        if i.level != Level::Good {
            i.steps = vec![
                "PC neu starten und beim Start mehrmals Entf oder F2 drücken – das BIOS öffnet sich".into(),
                "Nach „XMP“ (Intel), „EXPO“ oder „DOCP“ (AMD) suchen und „Profil 1“ wählen".into(),
                "Mit F10 speichern und neu starten. Startet der PC nicht, setzt er sich nach 3 Versuchen meist selbst zurück".into(),
            ];
        }
        i
    }

    // ---------------------------------------------------------------- Festplatte
    fn disk_check() -> CheckItem {
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let (mut free, mut total) = (0u64, 0u64);
        // SAFETY: Ausgabezeiger gültig.
        let ok = unsafe { GetDiskFreeSpaceExW(wide("C:\\").as_ptr(), &mut free, &mut total, std::ptr::null_mut()) } != 0;
        if !ok || total == 0 {
            return item("disk", "Speicherplatz (C:)", Level::Info, String::new(), String::new());
        }
        let pct = free as f64 / total as f64 * 100.0;
        let gb = free as f64 / 1024f64.powi(3);
        let (level, text) = if pct < 10.0 {
            (Level::Bad, "Auf deinem Systemlaufwerk ist kaum noch Platz. Windows und Spiele werden langsamer, Updates können scheitern.")
        } else if pct < 20.0 {
            (Level::Warn, "Auf deinem Systemlaufwerk wird der Platz knapp. Halte am besten mindestens 15–20 % frei.")
        } else {
            (Level::Good, "Genug freier Speicherplatz.")
        };
        let mut i = item("disk", "Speicherplatz (C:)", level.clone(), format!("{gb:.0} GB frei ({pct:.0} %)"), text.into());
        if level != Level::Good {
            i.fix = Some("storage_settings");
            i.fix_label = Some("Speicher aufräumen öffnen");
            i.steps = vec![
                "Auf „Speicher aufräumen öffnen“ klicken → „Temporäre Dateien“ löschen".into(),
                "Spiele, die du nicht mehr spielst, im Launcher deinstallieren".into(),
                "Große Spiele auf eine andere Festplatte verschieben (in Steam: Eigenschaften → Installierte Dateien → Verschieben)".into(),
            ];
        }
        i
    }

    // ---------------------------------------------------------------- Autostart
    fn autostart_check() -> CheckItem {
        let mut names: Vec<String> = Vec::new();
        let approved = |hive: &RegKey, name: &str| -> bool {
            // In „StartupApproved“ bedeutet erstes Byte 0x03 = vom Nutzer deaktiviert
            ["Run", "Run32"].iter().all(|sub| {
                hive.open_subkey(format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\{sub}"))
                    .ok()
                    .and_then(|k| k.get_raw_value(name).ok())
                    .is_none_or(|v| v.bytes.first() != Some(&3))
            })
        };
        let hkcu_root = RegKey::predef(HKEY_CURRENT_USER);
        let hklm_root = RegKey::predef(HKEY_LOCAL_MACHINE);
        for (root, path) in [
            (&hkcu_root, r"Software\Microsoft\Windows\CurrentVersion\Run"),
            (&hklm_root, r"Software\Microsoft\Windows\CurrentVersion\Run"),
            (&hklm_root, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"),
        ] {
            if let Ok(k) = root.open_subkey(path) {
                for (name, _) in k.enum_values().flatten() {
                    if approved(root, &name) && !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }
        let n = names.len();
        let (level, text) = if n > 15 {
            (Level::Warn, "Sehr viele Programme starten automatisch mit Windows. Sie kosten Startzeit und laufen oft unbemerkt im Hintergrund.")
        } else {
            (Level::Good, "Überschaubare Anzahl an Autostart-Programmen.")
        };
        let mut i = item("autostart", "Autostart-Programme", level, format!("{n} Programme"), text.into());
        i.details = Some(names.join(", "));
        i.fix = Some("startup_apps");
        i.fix_label = Some("Autostart verwalten");
        if n > 15 {
            i.steps = vec![
                "Auf „Autostart verwalten“ klicken".into(),
                "Alles ausschalten, was du nicht sofort nach dem Hochfahren brauchst (z. B. Launcher, Updater, Cloud-Programme)".into(),
                "Virenschutz und Treiber-Programme (Audio, Maus) eingeschaltet lassen".into(),
            ];
        }
        i
    }

    // ---------------------------------------------------------------- Drosselung
    fn throttle_check() -> CheckItem {
        use windows_sys::Win32::System::Performance::*;
        let mut q = 0isize;
        let mut c = 0isize;
        let mut val: Option<f64> = None;
        // SAFETY: PDH-Handles werden geprüft und geschlossen.
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut q) == 0 {
                if PdhAddEnglishCounterW(q, wide(r"\Processor Information(_Total)\% Performance Limit").as_ptr(), 0, &mut c) == 0 {
                    PdhCollectQueryData(q);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    PdhCollectQueryData(q);
                    let mut v: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
                    if PdhGetFormattedCounterValue(c, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) == 0 {
                        val = Some(v.Anonymous.doubleValue);
                    }
                }
                PdhCloseQuery(q);
            }
        }
        match val {
            Some(v) if v >= 5.0 => {
                let mut i = item("throttle", "CPU-Drosselung", Level::Bad, format!("{v:.0} % begrenzt"),
                    "Dein Prozessor wird gerade gedrosselt – meist wegen Hitze oder Strombegrenzung.".into());
                i.steps = vec![
                    "Laptop: Netzteil anschließen und auf einer harten, freien Fläche betreiben".into(),
                    "PC: Staub aus Lüftern und Kühlern entfernen (Druckluft), Gehäuselüfter prüfen".into(),
                    "Energieplan auf „Höchstleistung“ – das macht der Boost automatisch".into(),
                ];
                i
            }
            Some(_) => item("throttle", "CPU-Drosselung", Level::Good, "keine".into(), "Dein Prozessor wird gerade nicht gedrosselt.".into()),
            None => item("throttle", "CPU-Drosselung", Level::Info, String::new(), String::new()),
        }
    }

    fn temp_check(t: Option<f32>) -> CheckItem {
        match t {
            Some(t) if t >= 85.0 => {
                let mut i = item("gpu_temp", "Grafikkarten-Temperatur", Level::Bad, format!("{t:.0} °C"), "Deine Grafikkarte ist sehr heiß. Ab etwa 85 °C takten viele Karten herunter.".into());
                i.steps = vec![
                    "Staub aus Grafikkarte und Gehäuse entfernen (Druckluft, PC vorher ausschalten)".into(),
                    "Prüfen, ob alle Gehäuselüfter laufen und das Gehäuse Luft bekommt".into(),
                    "FPS begrenzen (z. B. auf deine Monitor-Hz) – spart Hitze ohne sichtbaren Nachteil".into(),
                ];
                i
            }
            Some(t) if t >= 78.0 => item("gpu_temp", "Grafikkarten-Temperatur", Level::Warn, format!("{t:.0} °C"), "Deine Grafikkarte ist warm. Unter Last ist das normal – im Leerlauf sollte sie deutlich kühler sein.".into()),
            Some(t) => item("gpu_temp", "Grafikkarten-Temperatur", Level::Good, format!("{t:.0} °C"), "Temperatur im grünen Bereich.".into()),
            None => item("gpu_temp", "Grafikkarten-Temperatur", Level::Info, String::new(), String::new()),
        }
    }

    // ---------------------------------------------------------------- Windows-Einstellungen
    fn game_mode_check() -> CheckItem {
        let on = hkcu(r"Software\Microsoft\GameBar").and_then(|k| k.get_value::<u32, _>("AutoGameModeEnabled").ok()).unwrap_or(1) == 1;
        let mut i = if on {
            item("game_mode", "Windows-Spielmodus", Level::Good, "an".into(), "Der Spielmodus ist an.".into())
        } else {
            item("game_mode", "Windows-Spielmodus", Level::Warn, "aus".into(), "Der Windows-Spielmodus ist aus. Er hält Windows-Updates und Hintergrundarbeit vom Spiel fern.".into())
        };
        if !on {
            i.fix = Some("game_mode_on");
            i.fix_label = Some("Dauerhaft einschalten");
            i.steps = vec!["Auf „Dauerhaft einschalten“ klicken – fertig".into()];
        }
        i
    }

    fn dvr_check() -> CheckItem {
        let on = hkcu(r"System\GameConfigStore").and_then(|k| k.get_value::<u32, _>("GameDVR_Enabled").ok()).unwrap_or(1) == 1
            && hkcu(r"Software\Microsoft\Windows\CurrentVersion\GameDVR").and_then(|k| k.get_value::<u32, _>("AppCaptureEnabled").ok()).unwrap_or(1) == 1;
        let mut i = if on {
            item("dvr", "Xbox-Hintergrundaufnahme", Level::Warn, "an".into(), "Die Xbox Game Bar darf im Hintergrund aufnehmen. Das kostet etwas GPU-Leistung. Der Boost schaltet es während des Spielens aus – oder du schaltest es hier dauerhaft ab.".into())
        } else {
            item("dvr", "Xbox-Hintergrundaufnahme", Level::Good, "aus".into(), "Keine Hintergrundaufnahme.".into())
        };
        if on {
            i.fix = Some("dvr_off");
            i.fix_label = Some("Dauerhaft ausschalten");
            i.steps = vec!["Auf „Dauerhaft ausschalten“ klicken – oder einfach den Boost nutzen, der das während des Spielens macht".into()];
        }
        i
    }

    fn hags_check() -> CheckItem {
        let mode = hklm(r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers").and_then(|k| k.get_value::<u32, _>("HwSchMode").ok());
        match mode {
            Some(2) => item("hags", "Hardwarebeschleunigte GPU-Planung", Level::Good, "an".into(), "Aktiv. Nötig z. B. für DLSS Frame Generation.".into()),
            Some(_) => {
                let mut i = item("hags", "Hardwarebeschleunigte GPU-Planung", Level::Info, "aus".into(),
                    "Ist aus. Bei neueren Grafikkarten (ab NVIDIA RTX 20 / AMD RX 5000) kann sie die Latenz leicht senken und ist für Frame Generation nötig. Die Wirkung ist von PC zu PC verschieden – Neustart erforderlich.".into());
                i.fix = Some("hags_on");
                i.fix_label = Some("Einschalten (Neustart nötig)");
                i.fix_needs_admin = true;
                i.steps = vec!["Auf „Einschalten“ klicken (braucht Admin-Rechte)".into(), "PC neu starten".into(), "Mit der Vorher-/Nachher-Messung prüfen, ob es bei dir etwas bringt".into()];
                i
            }
            None => item("hags", "Hardwarebeschleunigte GPU-Planung", Level::Info, String::new(), String::new()),
        }
    }

    fn network_check(wlan: bool) -> CheckItem {
        if wlan {
            {
                let mut i = item("network", "Internetverbindung", Level::Warn, "WLAN".into(), "WLAN schwankt deutlich stärker als ein Kabel und verursacht häufiger Ping-Spitzen.".into());
                i.steps = vec![
                    "Am besten: LAN-Kabel vom Router zum PC (bringt am meisten gegen Ping-Spitzen)".into(),
                    "Geht kein Kabel: mit dem 5-GHz-WLAN verbinden statt 2,4 GHz".into(),
                    "Router frei aufstellen, nicht hinter Metall oder im Schrank".into(),
                ];
                i
            }
        } else {
            item("network", "Internetverbindung", Level::Good, "LAN".into(), "Kabelverbindung – ideal zum Spielen.".into())
        }
    }

    // ---------------------------------------------------------------- Korrekturen
    pub fn run_fix(id: &str) -> Result<String, String> {
        let open = |target: &str| {
            std::process::Command::new("explorer.exe").arg(target).spawn().map(|_| "Geöffnet.".to_string()).map_err(|e| e.to_string())
        };
        match id {
            "driver_nvidia" => open("https://www.nvidia.com/de-de/drivers/"),
            "driver_amd" => open("https://www.amd.com/de/support/download/drivers.html"),
            "driver_intel" => open("https://www.intel.de/content/www/de/de/download-center/home.html"),
            "storage_settings" => open("ms-settings:storagesense"),
            "startup_apps" => open("ms-settings:startupapps"),
            "monitor_max_hz" => set_max_hz(),
            "game_mode_on" => {
                let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"Software\Microsoft\GameBar").map_err(|e| e.to_string())?;
                k.set_value("AutoGameModeEnabled", &1u32).map_err(|e| e.to_string())?;
                Ok("Spielmodus eingeschaltet.".into())
            }
            "dvr_off" => {
                let (a, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"System\GameConfigStore").map_err(|e| e.to_string())?;
                a.set_value("GameDVR_Enabled", &0u32).map_err(|e| e.to_string())?;
                let (b, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"Software\Microsoft\Windows\CurrentVersion\GameDVR").map_err(|e| e.to_string())?;
                b.set_value("AppCaptureEnabled", &0u32).map_err(|e| e.to_string())?;
                Ok("Hintergrundaufnahme dauerhaft ausgeschaltet.".into())
            }
            "hags_on" => {
                if !crate::elevation::is_elevated() {
                    return Err("Benötigt Administratorrechte.".into());
                }
                let (k, _) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers").map_err(|e| e.to_string())?;
                k.set_value("HwSchMode", &2u32).map_err(|e| e.to_string())?;
                Ok("Eingeschaltet. Wird nach einem Neustart aktiv.".into())
            }
            _ => Err("Unbekannte Korrektur.".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_age() {
        // 1.1.2025 = 20089 Tage seit 1970
        assert_eq!(days_since_driver_date("1-1-2025", 20089 * 86_400), Some(0));
        assert_eq!(days_since_driver_date("12-31-2024", 20089 * 86_400), Some(1));
        assert_eq!(days_since_driver_date("kaputt", 0), None);
    }

    #[test]
    fn ram_xmp_hints() {
        assert_eq!(ram_assessment("DDR4", 3600, 2133).0, Level::Warn);
        assert_eq!(ram_assessment("DDR4", 2666, 2666).0, Level::Warn);
        assert_eq!(ram_assessment("DDR5", 6000, 6000).0, Level::Good);
    }

    #[test]
    fn parses_smbios_type17() {
        // Minimaler SMBIOS-Puffer mit einem DDR5-Modul (Typ 17, Länge 0x28) und Ende-Struktur (127)
        let mut t17 = vec![0u8; 0x28];
        t17[0] = 17;
        t17[1] = 0x28;
        t17[0x0C] = 0x00;
        t17[0x0D] = 0x40; // 16 GB
        t17[0x12] = 0x22; // DDR5
        t17[0x15..0x17].copy_from_slice(&6000u16.to_le_bytes());
        t17[0x20..0x22].copy_from_slice(&4800u16.to_le_bytes());
        let mut table = t17;
        table.extend_from_slice(&[0, 0]); // keine Strings
        table.extend_from_slice(&[127, 4, 0, 0, 0, 0]);
        let mut raw = vec![0u8, 3, 4, 0];
        raw.extend_from_slice(&(table.len() as u32).to_le_bytes());
        raw.extend_from_slice(&table);
        let m = parse_smbios_memory(&raw);
        assert_eq!(m, vec![("DDR5".to_string(), 6000, 4800)]);
    }

    #[test]
    fn scoring() {
        let mk = |level| CheckItem { id: "x", title: "x", level, value: "v".into(), explanation: String::new(), details: None, fix: None, fix_label: None, fix_needs_admin: false, steps: Vec::new() };
        assert_eq!(score(&[mk(Level::Good), mk(Level::Info)]), 100);
        assert_eq!(score(&[mk(Level::Bad), mk(Level::Warn)]), 79);
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    #[test]
    fn pc_check_runs_on_windows() {
        let r = super::run_checks(None, None, false);
        for i in &r.items {
            println!("{:?} {} = {} | {}", i.level, i.title, i.value, i.details.clone().unwrap_or_default());
        }
        println!("Score: {}", r.score);
        assert!(!r.items.is_empty());
    }

    #[test]
    fn network_tools_run_on_windows() {
        let regions = crate::netdiag::region_ping(2);
        for r in regions.iter().take(4) {
            println!("{} {:?} ms (fehlgeschlagen {})", r.city, r.median_ms, r.failed);
        }
        match crate::netdiag::traceroute("1.1.1.1", 1) {
            Ok(t) => println!("Route: {} Stationen, erreicht: {}, {:?}", t.hops.len(), t.reached, t.analysis),
            Err(e) => println!("Traceroute: {e}"),
        }
        println!("Apps mit Verbindungen: {:?}", crate::netdiag::apps_with_connections().iter().take(5).collect::<Vec<_>>());
    }
}
