//! Windows-Implementierung der einzelnen Tweaks. Jede Funktion liefert die Daten, die zum
//! Rückgängigmachen nötig sind, BEVOR etwas verändert wird.

use super::{ClosedApp, RegEntry, RegValue, Revert};

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
pub use stub::*;

/// Ergebnis eines Tweaks: Meldung für den Nutzer + Daten zum Rückgängigmachen.
pub type Applied = Result<(String, Option<Revert>), String>;

#[cfg(not(windows))]
mod stub {
    use super::*;
    const NA: &str = "Nur unter Windows verfügbar.";
    pub fn power_plan() -> Applied { Err(NA.into()) }
    pub fn game_priority(_pid: u32, _exe: &str) -> Applied { Err(NA.into()) }
    pub fn close_apps(_apps: &[String], _mic: &[String]) -> Applied { Err(NA.into()) }
    pub fn registry(_entries: &[(&str, &str, u32)]) -> Applied { Err(NA.into()) }
    pub fn stop_update_services() -> Applied { Err(NA.into()) }
    pub fn wlan_low_latency() -> Applied { Err(NA.into()) }
    pub fn purge_standby_list() -> Applied { Err(NA.into()) }
    pub fn memory_status() -> Option<(f64, f64)> { None }
    pub fn device_power() -> Applied { Err(NA.into()) }
    pub fn cpu_full() -> Applied { Err(NA.into()) }
    pub fn close_background(_mic: &[String], _game: Option<&str>) -> Applied { Err(NA.into()) }
    pub fn background_priority(_mic: &[String]) -> Applied { Err(NA.into()) }
    pub fn mouse_accel() -> Applied { Err(NA.into()) }
    pub fn no_interruptions() -> Applied { Err(NA.into()) }
    pub fn revert(_r: &Revert) -> Result<String, String> { Err(NA.into()) }
    pub fn create_restore_point(_d: &str) -> Result<String, String> { Err(NA.into()) }
    pub fn set_fullscreen_optimizations(_p: &str, _d: bool) -> Result<(), String> { Err(NA.into()) }
    #[allow(dead_code)]
    fn _use(_: ClosedApp, _: RegEntry, _: RegValue) {}
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::boost::protect;
    use crate::win::{process, wide};
    use windows_sys::core::GUID;
    use winreg::enums::*;
    use winreg::RegKey;

    // ------------------------------------------------------------------ Energieplan

    /// Windows-Energieplan „Höchstleistung“ (gibt es auf jedem Windows, außer auf manchen Modern-Standby-Laptops).
    const HIGH_PERFORMANCE: GUID = GUID::from_u128(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);

    fn guid_str(g: &GUID) -> String {
        format!("{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{}", g.data1, g.data2, g.data3, g.data4[0], g.data4[1],
            g.data4[2..].iter().map(|b| format!("{b:02x}")).collect::<String>())
    }

    fn parse_guid(s: &str) -> Option<GUID> {
        u128::from_str_radix(&s.replace('-', ""), 16).ok().map(GUID::from_u128)
    }

    fn active_scheme() -> Option<GUID> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::System::Power::PowerGetActiveScheme;
        let mut p: *mut GUID = std::ptr::null_mut();
        // SAFETY: Windows reserviert den Speicher; wir geben ihn mit LocalFree frei.
        unsafe {
            if PowerGetActiveScheme(std::ptr::null_mut(), &mut p) != 0 || p.is_null() {
                return None;
            }
            let g = *p;
            LocalFree(p.cast());
            Some(g)
        }
    }

    fn set_scheme(g: &GUID) -> Result<(), String> {
        use windows_sys::Win32::System::Power::PowerSetActiveScheme;
        // SAFETY: Zeiger auf gültige GUID.
        match unsafe { PowerSetActiveScheme(std::ptr::null_mut(), g) } {
            0 => Ok(()),
            e => Err(format!("Windows-Fehler {e}")),
        }
    }

    pub fn power_plan() -> Applied {
        let prev = active_scheme().ok_or("Aktueller Energieplan konnte nicht gelesen werden.")?;
        if guid_str(&prev) == guid_str(&HIGH_PERFORMANCE) {
            return Ok(("„Höchstleistung“ war bereits aktiv – nichts geändert.".into(), None));
        }
        set_scheme(&HIGH_PERFORMANCE).map_err(|e| {
            format!("„Höchstleistung“ ist auf diesem PC nicht verfügbar (häufig bei Laptops mit Modern Standby). {e}")
        })?;
        Ok(("Energieplan auf „Höchstleistung“ gestellt.".into(), Some(Revert::PowerPlan { previous: guid_str(&prev) })))
    }

    // ------------------------------------------------------------------ Priorität

    pub fn game_priority(pid: u32, exe: &str) -> Applied {
        let mut parts = Vec::new();
        let mut done = Vec::new();
        let previous = process::get_priority(pid)?;
        if previous != process::PRIORITY_HIGH {
            process::set_priority(pid, process::PRIORITY_HIGH)?;
            parts.push(Revert::Priority { pid, exe: exe.into(), previous });
            done.push("hohe Priorität");
        }
        // Auch auf der Grafikkarte bevorzugen (Windows-Grafikplaner)
        if let Some(prev_gpu) = process::get_gpu_priority(pid) {
            if prev_gpu < 4 && process::set_gpu_priority(pid, 4).is_ok() {
                parts.push(Revert::GpuPriority { pid, exe: exe.into(), previous: prev_gpu });
                done.push("hohe GPU-Priorität");
            }
        }
        // Windows soll das Spiel nie zum Stromsparen drosseln (wichtig bei CPUs mit P- und E-Kernen)
        if process::set_power_throttling_off(pid, true).is_ok() {
            parts.push(Revert::PowerThrottle { pid, exe: exe.into() });
            done.push("keine Energiespar-Drosselung");
        }
        if parts.is_empty() {
            return Ok((format!("{exe} lief bereits bevorzugt."), None));
        }
        Ok((format!("{exe}: {}.", done.join(", ")), Some(Revert::Multi { parts })))
    }

    /// Browser bekommen weniger CPU als das Spiel – nichts wird beendet.
    const BROWSERS: &[&str] = &["chrome.exe", "msedge.exe", "firefox.exe", "opera.exe", "opera_gx.exe", "brave.exe", "vivaldi.exe", "librewolf.exe", "zen.exe"];

    pub fn background_priority(mic: &[String]) -> Applied {
        let mut items = Vec::new();
        for p in process::list() {
            if !BROWSERS.iter().any(|b| b.eq_ignore_ascii_case(&p.name)) || protect::check(&p.name, mic).is_some() || !process::in_my_session(p.pid) {
                continue;
            }
            if let Ok(prev) = process::get_priority(p.pid) {
                if prev == process::PRIORITY_NORMAL && process::set_priority(p.pid, process::PRIORITY_BELOW_NORMAL).is_ok() {
                    items.push((p.pid, p.name.clone(), prev));
                }
            }
        }
        if items.is_empty() {
            return Ok(("Kein Browser geöffnet – nichts zu tun.".into(), None));
        }
        let mut names: Vec<String> = items.iter().map(|(_, n, _)| n.clone()).collect();
        names.dedup();
        Ok((format!("Gedrosselt: {}.", names.join(", ")), Some(Revert::Priorities { items })))
    }

    // ------------------------------------------------------------------ Hintergrund-Apps

    pub fn close_apps(apps: &[String], mic: &[String]) -> Applied {
        let running = process::list();
        let mut closed: Vec<ClosedApp> = Vec::new();
        let mut notes = Vec::new();
        for want in apps {
            if let Some(reason) = protect::check(want, mic) {
                notes.push(format!("{want} geschützt ({})", reason.label()));
                continue;
            }
            for p in running.iter().filter(|p| p.name.eq_ignore_ascii_case(want) && process::in_my_session(p.pid)) {
                let path = process::path(p.pid);
                match process::close(p.pid) {
                    Ok(()) => {
                        if let Some(path) = path {
                            if !closed.iter().any(|c| c.path == path) {
                                closed.push(ClosedApp { name: p.name.clone(), path });
                            }
                        }
                    }
                    Err(e) => notes.push(format!("{}: {e}", p.name)),
                }
            }
        }
        let mut msg = if closed.is_empty() {
            "Keine der eingetragenen Hintergrund-Apps lief.".to_string()
        } else {
            format!("Beendet: {}. Sie starten nach dem Boost wieder.", closed.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", "))
        };
        if !notes.is_empty() {
            msg.push_str(&format!(" Hinweise: {}.", notes.join("; ")));
        }
        let revert = (!closed.is_empty()).then_some(Revert::ClosedApps { apps: closed });
        Ok((msg, revert))
    }

    /// Programme der eigenen Sitzung, die OHNE sichtbares Fenster laufen und nicht geschützt sind: (Name, Pfad, PIDs).
    /// Reines Auswählen ohne Beenden – dadurch im Test gefahrlos.
    pub fn background_candidates(mic: &[String], game_exe: Option<&str>) -> Vec<(String, String, Vec<u32>)> {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()).to_ascii_lowercase();
        let visible = process::pids_with_visible_windows();
        let all = process::list();
        // Programme, von denen IRGENDEIN Prozess ein Fenster hat (z. B. Chrome mit seinen Tab-Prozessen), bleiben komplett an.
        let visible_names: std::collections::HashSet<String> = all.iter().filter(|p| visible.contains(&p.pid)).map(|p| p.name.to_ascii_lowercase()).collect();
        let me = std::process::id();
        // Alles, was GameBoost gestartet hat (Eltern-Kette), bleibt unangetastet.
        let parents = process::ancestors(me, &all);
        let mut out: Vec<(String, String, Vec<u32>)> = Vec::new();
        for p in &all {
            let lower = p.name.to_ascii_lowercase();
            if p.pid == me || p.pid <= 4 || parents.contains(&p.pid) || visible_names.contains(&lower) || out.iter().any(|(n, _, _)| n.eq_ignore_ascii_case(&p.name)) {
                continue;
            }
            if game_exe.is_some_and(|g| g.eq_ignore_ascii_case(&p.name)) || protect::check(&p.name, mic).is_some() || !process::in_my_session(p.pid) {
                continue;
            }
            // Alles aus dem Windows-Ordner gehört zu Windows → nie anfassen
            let Some(path) = process::path(p.pid) else { continue };
            let lpath = path.to_ascii_lowercase();
            if lpath.starts_with(&windir) || lpath.contains(r"\windowsapps\microsoft.windows") {
                continue;
            }
            let pids = all.iter().filter(|q| q.name.eq_ignore_ascii_case(&p.name) && !parents.contains(&q.pid) && q.pid != me).map(|q| q.pid).collect();
            out.push((p.name.clone(), path, pids));
        }
        out
    }

    /// Beendet alle Hintergrund-Programme aus `background_candidates`.
    pub fn close_background(mic: &[String], game_exe: Option<&str>) -> Applied {
        let mut closed: Vec<ClosedApp> = Vec::new();
        for (name, path, pids) in background_candidates(mic, game_exe) {
            let mut any = false;
            for pid in pids {
                any |= process::close(pid).is_ok();
            }
            if any {
                closed.push(ClosedApp { name, path });
            }
        }
        if closed.is_empty() {
            return Ok(("Keine unnötigen Hintergrund-Programme gefunden.".into(), None));
        }
        let names: Vec<&str> = closed.iter().map(|c| c.name.as_str()).collect();
        Ok((format!("{} beendet: {}. Sie starten nach dem Boost wieder.", closed.len(), names.join(", ")), Some(Revert::ClosedApps { apps: closed })))
    }

    // ------------------------------------------------------------------ Registry (nur HKCU)

    fn read(path: &str, name: &str) -> Option<RegValue> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(path).ok()?;
        if let Ok(v) = key.get_value::<u32, _>(name) {
            return Some(RegValue::Dword(v));
        }
        key.get_value::<String, _>(name).ok().map(RegValue::Sz)
    }

    fn write(path: &str, name: &str, value: &Option<RegValue>) -> Result<(), String> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(path).map_err(|e| e.to_string())?;
        match value {
            Some(RegValue::Dword(v)) => key.set_value(name, v),
            Some(RegValue::Sz(s)) => key.set_value(name, s),
            None => match key.delete_value(name) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                r => r,
            },
        }
        .map_err(|e| e.to_string())
    }

    /// Setzt DWORD-Werte unter HKEY_CURRENT_USER und merkt sich die alten Werte.
    pub fn registry(entries: &[(&str, &str, u32)]) -> Applied {
        let mut saved = Vec::new();
        for (path, name, value) in entries {
            let previous = read(path, name);
            if previous == Some(RegValue::Dword(*value)) {
                continue;
            }
            write(path, name, &Some(RegValue::Dword(*value)))?;
            saved.push(RegEntry { path: path.to_string(), name: name.to_string(), previous });
        }
        if saved.is_empty() {
            Ok(("War bereits so eingestellt – nichts geändert.".into(), None))
        } else {
            Ok(("Eingestellt.".into(), Some(Revert::Registry { entries: saved })))
        }
    }

    const FSO_PATH: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
    const FSO_FLAG: &str = "~ DISABLEDXMAXIMIZEDWINDOWEDMODE";

    /// Vollbild-Optimierungen für eine .exe dauerhaft an/aus (wirkt ab dem nächsten Spielstart).
    pub fn set_fullscreen_optimizations(exe_path: &str, disable: bool) -> Result<(), String> {
        write(FSO_PATH, exe_path, &disable.then(|| RegValue::Sz(FSO_FLAG.into())))
    }

    // ------------------------------------------------------------------ Dienste (Updates)

    mod svc {
        use crate::win::wide;
        use windows_sys::Win32::System::Services::*;

        pub struct H(pub SC_HANDLE);
        impl Drop for H {
            fn drop(&mut self) {
                // SAFETY: Handle stammt aus OpenSCManager/OpenService.
                unsafe { CloseServiceHandle(self.0) };
            }
        }

        pub fn open(name: &str) -> Result<H, String> {
            // SAFETY: Rückgaben werden geprüft.
            unsafe {
                let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT);
                if scm.is_null() {
                    return Err("Dienstverwaltung nicht erreichbar".into());
                }
                let scm = H(scm);
                let s = OpenServiceW(scm.0, wide(name).as_ptr(), SERVICE_STOP | SERVICE_START | SERVICE_QUERY_STATUS);
                if s.is_null() {
                    return Err(format!("{name}: kein Zugriff ({})", std::io::Error::last_os_error()));
                }
                Ok(H(s))
            }
        }

        pub fn running(h: &H) -> bool {
            // SAFETY: Struktur wird von Windows befüllt.
            let mut st: SERVICE_STATUS = unsafe { std::mem::zeroed() };
            unsafe { QueryServiceStatus(h.0, &mut st) != 0 && st.dwCurrentState == SERVICE_RUNNING }
        }

        pub fn stop(h: &H) -> Result<(), String> {
            let mut st: SERVICE_STATUS = unsafe { std::mem::zeroed() };
            // SAFETY: gültiges Handle.
            if unsafe { ControlService(h.0, SERVICE_CONTROL_STOP, &mut st) } != 0 { Ok(()) } else { Err(std::io::Error::last_os_error().to_string()) }
        }

        pub fn start(h: &H) -> Result<(), String> {
            // SAFETY: keine Argumente.
            if unsafe { StartServiceW(h.0, 0, std::ptr::null()) } != 0 { Ok(()) } else { Err(std::io::Error::last_os_error().to_string()) }
        }
    }

    /// Windows Update, Hintergrundübertragung (BITS) und Übermittlungsoptimierung.
    const UPDATE_SERVICES: &[(&str, &str)] = &[
        ("wuauserv", "Windows Update"),
        ("BITS", "Hintergrundübertragung"),
        ("DoSvc", "Übermittlungsoptimierung"),
        ("WSearch", "Windows-Suche"),
    ];

    pub fn stop_update_services() -> Applied {
        let mut stopped = Vec::new();
        let mut notes = Vec::new();
        for (name, label) in UPDATE_SERVICES {
            match svc::open(name) {
                Ok(h) if svc::running(&h) => match svc::stop(&h) {
                    Ok(()) => stopped.push(name.to_string()),
                    Err(e) => notes.push(format!("{label}: {e}")),
                },
                Ok(_) => {}
                Err(e) => notes.push(e),
            }
        }
        let mut msg = if stopped.is_empty() { "Es liefen gerade keine Update-Dienste.".to_string() } else { format!("Pausiert: {}.", stopped.join(", ")) };
        if !notes.is_empty() {
            msg.push_str(&format!(" Hinweise: {}.", notes.join("; ")));
        }
        Ok((msg, (!stopped.is_empty()).then_some(Revert::Services { names: stopped })))
    }

    // ------------------------------------------------------------------ WLAN

    mod wlan {
        use windows_sys::core::GUID;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::NetworkManagement::WiFi::*;

        pub struct Client(HANDLE);
        impl Drop for Client {
            fn drop(&mut self) {
                // SAFETY: Handle stammt aus WlanOpenHandle.
                unsafe { WlanCloseHandle(self.0, std::ptr::null()) };
            }
        }

        pub fn open() -> Option<Client> {
            let (mut ver, mut h) = (0u32, std::ptr::null_mut());
            // SAFETY: Ausgabezeiger gültig.
            (unsafe { WlanOpenHandle(2, std::ptr::null(), &mut ver, &mut h) } == 0).then_some(Client(h))
        }

        /// GUIDs aller verbundenen WLAN-Adapter.
        pub fn connected(c: &Client) -> Vec<GUID> {
            let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
            // SAFETY: Liste wird von Windows reserviert und mit WlanFreeMemory freigegeben.
            unsafe {
                if WlanEnumInterfaces(c.0, std::ptr::null(), &mut list) != 0 || list.is_null() {
                    return Vec::new();
                }
                let n = (*list).dwNumberOfItems as usize;
                let items = std::slice::from_raw_parts((*list).InterfaceInfo.as_ptr(), n);
                let out = items.iter().filter(|i| i.isState == wlan_interface_state_connected).map(|i| i.InterfaceGuid).collect();
                WlanFreeMemory(list.cast());
                out
            }
        }

        pub fn get(c: &Client, g: &GUID, op: WLAN_INTF_OPCODE) -> Option<bool> {
            let (mut size, mut data, mut ty) = (0u32, std::ptr::null_mut(), 0);
            // SAFETY: Windows liefert einen BOOL-Wert, den wir lesen und freigeben.
            unsafe {
                if WlanQueryInterface(c.0, g, op, std::ptr::null(), &mut size, &mut data, &mut ty) != 0 || data.is_null() {
                    return None;
                }
                let v = *(data as *const i32) != 0;
                WlanFreeMemory(data);
                Some(v)
            }
        }

        pub fn set(c: &Client, g: &GUID, op: WLAN_INTF_OPCODE, v: bool) -> Result<(), String> {
            let val: i32 = v as i32;
            // SAFETY: Zeiger auf gültigen BOOL mit korrekter Größe.
            match unsafe { WlanSetInterface(c.0, g, op, 4, (&val as *const i32).cast(), std::ptr::null()) } {
                0 => Ok(()),
                e => Err(format!("WLAN-Fehler {e}")),
            }
        }

        pub use windows_sys::Win32::NetworkManagement::WiFi::{wlan_intf_opcode_background_scan_enabled as BG_SCAN, wlan_intf_opcode_media_streaming_mode as STREAMING};
    }

    pub fn wlan_low_latency() -> Applied {
        let Some(c) = wlan::open() else { return Ok(("Kein WLAN auf diesem PC – nicht nötig.".into(), None)) };
        let ifaces = wlan::connected(&c);
        if ifaces.is_empty() {
            return Ok(("Kein WLAN verbunden – nicht nötig.".into(), None));
        }
        let mut saved = Vec::new();
        for g in &ifaces {
            let bg = wlan::get(&c, g, wlan::BG_SCAN).unwrap_or(true);
            let st = wlan::get(&c, g, wlan::STREAMING).unwrap_or(false);
            wlan::set(&c, g, wlan::BG_SCAN, false)?;
            let _ = wlan::set(&c, g, wlan::STREAMING, true);
            saved.push((guid_str(g), bg, st));
        }
        Ok(("WLAN-Hintergrundsuche aus, Streaming-Modus an.".into(), Some(Revert::Wlan { interfaces: saved })))
    }

    // ------------------------------------------------------------------ Standby-Liste

    #[repr(C)]
    #[derive(Default)]
    struct MemoryListInfo {
        zero: usize,
        free: usize,
        modified: usize,
        modified_no_write: usize,
        bad: usize,
        standby_by_priority: [usize; 8],
        repurposed_by_priority: [usize; 8],
        modified_page_file: usize,
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQuerySystemInformation(class: u32, info: *mut core::ffi::c_void, len: u32, ret: *mut u32) -> i32;
        fn NtSetSystemInformation(class: u32, info: *const core::ffi::c_void, len: u32) -> i32;
    }
    const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;
    const MEMORY_PURGE_STANDBY_LIST: u32 = 4;

    fn enable_privilege(name: &str) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, LUID};
        use windows_sys::Win32::Security::*;
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        // SAFETY: Token-Handle wird geprüft und geschlossen.
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut luid: LUID = std::mem::zeroed();
            let ok = LookupPrivilegeValueW(std::ptr::null(), wide(name).as_ptr(), &mut luid) != 0 && {
                let tp = TOKEN_PRIVILEGES { PrivilegeCount: 1, Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }] };
                AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(0)
            };
            CloseHandle(token);
            ok
        }
    }

    fn memory_lists() -> Option<MemoryListInfo> {
        let mut info = MemoryListInfo::default();
        // SAFETY: Puffergröße entspricht der Struktur.
        let st = unsafe {
            NtQuerySystemInformation(SYSTEM_MEMORY_LIST_INFORMATION, (&mut info as *mut MemoryListInfo).cast(), std::mem::size_of::<MemoryListInfo>() as u32, std::ptr::null_mut())
        };
        (st >= 0).then_some(info)
    }

    /// (freier RAM in GB, Standby-Speicher in GB)
    pub fn memory_status() -> Option<(f64, f64)> {
        let gb = |pages: usize| pages as f64 * 4096.0 / 1024f64.powi(3);
        memory_lists().map(|m| (gb(m.free + m.zero), gb(m.standby_by_priority.iter().sum())))
    }

    pub fn purge_standby_list() -> Applied {
        if !enable_privilege("SeProfileSingleProcessPrivilege") {
            return Err("Benötigt Administratorrechte.".into());
        }
        let gb = |pages: usize| pages as f64 * 4096.0 / 1024f64.powi(3);
        let before = memory_lists();
        let cmd = MEMORY_PURGE_STANDBY_LIST;
        // SAFETY: dokumentierter Befehlswert, Größe korrekt.
        let st = unsafe { NtSetSystemInformation(SYSTEM_MEMORY_LIST_INFORMATION, (&cmd as *const u32).cast(), 4) };
        if st < 0 {
            return Err(format!("Windows hat das Leeren abgelehnt (Status 0x{st:08X})."));
        }
        let after = memory_lists();
        let msg = match (before, after) {
            (Some(b), Some(a)) => format!(
                "Standby-Liste: {:.1} GB → {:.1} GB. Freier RAM: {:.1} GB → {:.1} GB.",
                gb(b.standby_by_priority.iter().sum()), gb(a.standby_by_priority.iter().sum()),
                gb(b.free + b.zero), gb(a.free + a.zero)
            ),
            _ => "Standby-Liste geleert.".into(),
        };
        Ok((msg, None))
    }

    // ------------------------------------------------------------------ Geräte-Energiesparen

    /// (Untergruppe, Einstellung, Name) – Zielwert ist jeweils 0 (= aus / maximale Leistung).
    const DEVICE_POWER: &[(u128, u128, &str)] = &[
        (0x2a737441_1930_4402_8d77_b2bebba308a3, 0x48e6b7a6_50f5_4782_a5d4_53bb8f07e226, "USB-Energiesparen"),
        (0x501a4d13_42af_4429_9fd1_a8218c268e20, 0xee12f906_d277_404b_b6da_e5fa1a576df5, "PCIe-Energiesparen"),
        (0x19cbb8fa_5279_450e_9fac_8a3d5fedd0c1, 0x12bbebe6_58d6_4636_95bb_3217ef867c1a, "WLAN-Energiesparen"),
    ];

    fn power_value(scheme: &GUID, sub: &GUID, set: &GUID) -> Option<(u32, u32)> {
        use windows_sys::Win32::System::Power::{PowerReadACValueIndex, PowerReadDCValueIndex};
        let (mut ac, mut dc) = (0u32, 0u32);
        // SAFETY: Zeiger auf gültige GUIDs und Ausgabevariablen.
        unsafe {
            (PowerReadACValueIndex(std::ptr::null_mut(), scheme, sub, set, &mut ac) == 0
                && PowerReadDCValueIndex(std::ptr::null_mut(), scheme, sub, set, &mut dc) == 0)
                .then_some((ac, dc))
        }
    }

    fn write_power_value(scheme: &GUID, sub: &GUID, set: &GUID, ac: u32, dc: u32) -> Result<(), String> {
        use windows_sys::Win32::System::Power::{PowerWriteACValueIndex, PowerWriteDCValueIndex};
        // SAFETY: wie oben.
        let r = unsafe {
            PowerWriteACValueIndex(std::ptr::null_mut(), scheme, sub, set, ac) | PowerWriteDCValueIndex(std::ptr::null_mut(), scheme, sub, set, dc)
        };
        if r == 0 { Ok(()) } else { Err(format!("Windows-Fehler {r}")) }
    }

    /// Prozessor-Einstellungen im Energieplan: (Einstellung, Zielwert, Name)
    const CPU_SUB: u128 = 0x54533251_82be_4824_96c1_47b60b740d00;
    const CPU_FULL: &[(u128, u32, &str)] = &[
        (0x893dee8e_2bef_41e0_89c6_b55d0929964c, 100, "Mindestleistung 100 %"),
        (0x0cc5b647_c1df_4637_891a_dec35c318583, 100, "kein Kern-Parken"),
        (0xbe337238_0d82_4146_a960_4f3749d470c7, 2, "Turbo aggressiv"),
    ];

    pub fn cpu_full() -> Applied {
        let scheme = active_scheme().ok_or("Aktueller Energieplan konnte nicht gelesen werden.")?;
        let sub = GUID::from_u128(CPU_SUB);
        let mut saved = Vec::new();
        let mut changed = Vec::new();
        for (set, target, name) in CPU_FULL {
            let set = GUID::from_u128(*set);
            let Some((ac, dc)) = power_value(&scheme, &sub, &set) else { continue };
            if ac == *target && dc == *target {
                continue;
            }
            write_power_value(&scheme, &sub, &set, *target, *target)?;
            saved.push((guid_str(&sub), guid_str(&set), ac, dc));
            changed.push(*name);
        }
        if saved.is_empty() {
            return Ok(("War bereits so eingestellt – nichts geändert.".into(), None));
        }
        set_scheme(&scheme)?;
        Ok((format!("{}.", changed.join(", ")), Some(Revert::DevicePower { scheme: guid_str(&scheme), values: saved })))
    }

    pub fn device_power() -> Applied {
        let scheme = active_scheme().ok_or("Aktueller Energieplan konnte nicht gelesen werden.")?;
        let mut saved = Vec::new();
        let mut changed = Vec::new();
        for (sub, set, name) in DEVICE_POWER {
            let (sub, set) = (GUID::from_u128(*sub), GUID::from_u128(*set));
            let Some((ac, dc)) = power_value(&scheme, &sub, &set) else { continue }; // Gerät/Einstellung nicht vorhanden
            if ac == 0 && dc == 0 {
                continue;
            }
            write_power_value(&scheme, &sub, &set, 0, 0)?;
            saved.push((guid_str(&sub), guid_str(&set), ac, dc));
            changed.push(*name);
        }
        if saved.is_empty() {
            return Ok(("War bereits aus – nichts geändert.".into(), None));
        }
        set_scheme(&scheme)?; // Änderungen sofort wirksam machen
        Ok((format!("Ausgeschaltet: {}.", changed.join(", ")), Some(Revert::DevicePower { scheme: guid_str(&scheme), values: saved })))
    }

    // ------------------------------------------------------------------ Maus & Störungen

    fn spi_get<T>(action: u32, value: &mut T, size: u32) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;
        // SAFETY: Puffer passt zur jeweiligen Aktion.
        unsafe { SystemParametersInfoW(action, size, (value as *mut T).cast(), 0) != 0 }
    }

    fn spi_set<T>(action: u32, value: &mut T, size: u32, by_value: bool) -> Result<(), String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE};
        // SAFETY: wie oben.
        let ok = unsafe { SystemParametersInfoW(action, if by_value { 0 } else { size }, (value as *mut T).cast(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE) != 0 };
        if ok { Ok(()) } else { Err(std::io::Error::last_os_error().to_string()) }
    }

    pub fn mouse_accel() -> Applied {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETMOUSE, SPI_SETMOUSE};
        let mut prev = [0i32; 3];
        if !spi_get(SPI_GETMOUSE, &mut prev, 0) {
            return Err("Mauseinstellungen nicht lesbar.".into());
        }
        if prev[2] == 0 {
            return Ok(("Mausbeschleunigung war bereits aus.".into(), None));
        }
        let mut off = [0i32; 3];
        spi_set(SPI_SETMOUSE, &mut off, 0, true)?;
        Ok(("Mausbeschleunigung ausgeschaltet.".into(), Some(Revert::Mouse { previous: prev })))
    }

    #[repr(C)]
    struct StickyKeys {
        cb_size: u32,
        flags: u32,
    }
    const SKF_HOTKEYACTIVE: u32 = 0x4;

    fn sticky_keys() -> Option<StickyKeys> {
        use windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETSTICKYKEYS;
        let mut sk = StickyKeys { cb_size: 8, flags: 0 };
        spi_get(SPI_GETSTICKYKEYS, &mut sk, 8).then_some(sk)
    }

    fn set_sticky_flags(flags: u32) -> Result<(), String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::SPI_SETSTICKYKEYS;
        let mut sk = StickyKeys { cb_size: 8, flags };
        spi_set(SPI_SETSTICKYKEYS, &mut sk, 8, false)
    }

    pub fn no_interruptions() -> Applied {
        let mut parts = Vec::new();
        let mut done = Vec::new();
        if let Ok((_, Some(r))) = registry(&[(r"Software\Microsoft\Windows\CurrentVersion\PushNotifications", "ToastEnabled", 0)]) {
            parts.push(r);
            done.push("Benachrichtigungen aus");
        }
        if let Some(sk) = sticky_keys() {
            if sk.flags & SKF_HOTKEYACTIVE != 0 {
                set_sticky_flags(sk.flags & !SKF_HOTKEYACTIVE)?;
                parts.push(Revert::StickyKeys { flags: sk.flags });
                done.push("Einrastfunktion-Kürzel aus");
            }
        }
        if parts.is_empty() {
            return Ok(("War bereits so eingestellt – nichts geändert.".into(), None));
        }
        Ok((format!("{}.", done.join(", ")), Some(Revert::Multi { parts })))
    }

    // ------------------------------------------------------------------ Rückgängig

    pub fn revert(r: &Revert) -> Result<String, String> {
        match r {
            Revert::PowerPlan { previous } => {
                let g = parse_guid(previous).ok_or("Ungültige Energieplan-ID")?;
                set_scheme(&g)?;
                Ok("Vorheriger Energieplan wiederhergestellt.".into())
            }
            Revert::Priority { pid, exe, previous } => {
                // Nur, wenn noch derselbe Prozess läuft (PIDs werden wiederverwendet).
                let same = process::list().into_iter().any(|p| p.pid == *pid && p.name.eq_ignore_ascii_case(exe));
                if !same {
                    return Ok(format!("{exe} läuft nicht mehr – nichts zu tun."));
                }
                process::set_priority(*pid, *previous)?;
                Ok(format!("Priorität von {exe} zurückgesetzt."))
            }
            Revert::ClosedApps { apps } => {
                let running = process::list();
                let mut started = Vec::new();
                for a in apps {
                    if running.iter().any(|p| p.name.eq_ignore_ascii_case(&a.name)) {
                        continue;
                    }
                    if process::launch_as_user(&a.path).is_ok() {
                        started.push(a.name.clone());
                    }
                }
                Ok(if started.is_empty() { "Apps liefen bereits wieder.".into() } else { format!("Wieder gestartet: {}.", started.join(", ")) })
            }
            Revert::Registry { entries } => {
                for e in entries.iter().rev() {
                    write(&e.path, &e.name, &e.previous)?;
                }
                Ok("Einstellungen wiederhergestellt.".into())
            }
            Revert::Services { names } => {
                // Diese Dienste startet Windows bei Bedarf ohnehin selbst – Fehler hier sind kein Problem.
                let mut started = 0;
                for n in names {
                    if svc::open(n).and_then(|h| if svc::running(&h) { Ok(()) } else { svc::start(&h) }).is_ok() {
                        started += 1;
                    }
                }
                Ok(if started == names.len() {
                    "Update-Dienste laufen wieder.".into()
                } else {
                    "Update-Dienste werden von Windows bei Bedarf automatisch wieder gestartet.".into()
                })
            }
            Revert::Wlan { interfaces } => {
                let c = wlan::open().ok_or("WLAN-Dienst nicht erreichbar")?;
                for (g, bg, st) in interfaces {
                    if let Some(g) = parse_guid(g) {
                        let _ = wlan::set(&c, &g, wlan::BG_SCAN, *bg);
                        let _ = wlan::set(&c, &g, wlan::STREAMING, *st);
                    }
                }
                Ok("WLAN-Einstellungen wiederhergestellt.".into())
            }
            Revert::Mouse { previous } => {
                use windows_sys::Win32::UI::WindowsAndMessaging::SPI_SETMOUSE;
                let mut p = *previous;
                spi_set(SPI_SETMOUSE, &mut p, 0, true)?;
                Ok("Mausbeschleunigung wiederhergestellt.".into())
            }
            Revert::DevicePower { scheme, values } => {
                let scheme = parse_guid(scheme).ok_or("Ungültige Energieplan-ID")?;
                for (sub, set, ac, dc) in values {
                    if let (Some(sub), Some(set)) = (parse_guid(sub), parse_guid(set)) {
                        write_power_value(&scheme, &sub, &set, *ac, *dc)?;
                    }
                }
                // Nur neu aktivieren, wenn der Plan noch aktiv ist (sonst würde er wieder eingeschaltet).
                if active_scheme().is_some_and(|a| guid_str(&a) == guid_str(&scheme)) {
                    set_scheme(&scheme)?;
                }
                Ok("Energiespar-Einstellungen wiederhergestellt.".into())
            }
            Revert::StickyKeys { flags } => {
                set_sticky_flags(*flags)?;
                Ok("Einrastfunktion wiederhergestellt.".into())
            }
            Revert::Multi { parts } => {
                // Alle Teile versuchen – ein Fehler darf die anderen nicht verhindern.
                let (mut ok, mut errs) = (Vec::new(), Vec::new());
                for p in parts.iter().rev() {
                    match revert(p) {
                        Ok(m) => ok.push(m),
                        Err(e) => errs.push(e),
                    }
                }
                if errs.is_empty() { Ok(ok.join(" ")) } else { Err(errs.join("; ")) }
            }
            Revert::GpuPriority { pid, exe, previous } => {
                if process::list().into_iter().any(|p| p.pid == *pid && p.name.eq_ignore_ascii_case(exe)) {
                    let _ = process::set_gpu_priority(*pid, *previous);
                }
                Ok("GPU-Priorität zurückgesetzt.".into())
            }
            Revert::PowerThrottle { pid, exe } => {
                if process::list().into_iter().any(|p| p.pid == *pid && p.name.eq_ignore_ascii_case(exe)) {
                    let _ = process::set_power_throttling_off(*pid, false);
                }
                Ok("Energiespar-Drosselung wieder Windows überlassen.".into())
            }
            Revert::Priorities { items } => {
                let running = process::list();
                for (pid, name, prev) in items {
                    if running.iter().any(|p| p.pid == *pid && p.name.eq_ignore_ascii_case(name)) {
                        let _ = process::set_priority(*pid, *prev);
                    }
                }
                Ok("Prioritäten zurückgesetzt.".into())
            }
        }
    }

    // ------------------------------------------------------------------ Wiederherstellungspunkt

    pub fn create_restore_point(description: &str) -> Result<String, String> {
        use windows_sys::Win32::System::Restore::*;
        let mut info = RESTOREPOINTINFOW { dwEventType: BEGIN_SYSTEM_CHANGE, dwRestorePtType: MODIFY_SETTINGS, llSequenceNumber: 0, szDescription: [0; 256] };
        for (i, c) in description.encode_utf16().take(255).enumerate() {
            info.szDescription[i] = c;
        }
        // SAFETY: Strukturen korrekt befüllt.
        let mut status: STATEMGRSTATUS = unsafe { std::mem::zeroed() };
        if unsafe { SRSetRestorePointW(&info, &mut status) } == 0 {
            let code = { status.nStatus };
            return Err(match code {
                1058 => "Der Computerschutz ist auf diesem PC ausgeschaltet. Du kannst ihn unter „Wiederherstellungspunkt erstellen“ in Windows aktivieren.".into(),
                5 => "Benötigt Administratorrechte.".into(),
                c => format!("Windows-Fehler {c}"),
            });
        }
        let end = RESTOREPOINTINFOW { dwEventType: END_SYSTEM_CHANGE, dwRestorePtType: MODIFY_SETTINGS, llSequenceNumber: { status.llSequenceNumber }, szDescription: [0; 256] };
        // SAFETY: wie oben.
        unsafe { SRSetRestorePointW(&end, &mut status) };
        Ok("Wiederherstellungspunkt erstellt. (Windows legt höchstens einen pro 24 Stunden an.)".into())
    }
}

/// Diese Tests verändern das System kurz und setzen es sofort zurück.
/// Sie laufen nur in der Windows-CI (Wegwerf-Rechner): `cargo test -- --include-ignored`.
#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    #[ignore = "verändert Systemeinstellungen – nur in CI"]
    fn power_plan_apply_and_revert() {
        let before = imp_active();
        let (msg, rev) = power_plan().expect("power_plan");
        println!("{msg}");
        if let Some(r) = rev {
            println!("{}", revert(&r).expect("revert"));
        }
        assert_eq!(imp_active(), before, "Energieplan muss wieder wie vorher sein");
    }

    fn imp_active() -> Option<String> {
        use windows_sys::Win32::System::Power::PowerGetActiveScheme;
        let mut p: *mut windows_sys::core::GUID = std::ptr::null_mut();
        unsafe {
            if PowerGetActiveScheme(std::ptr::null_mut(), &mut p) != 0 {
                return None;
            }
            let g = *p;
            Some(format!("{:?}{:?}{:?}{:?}", g.data1, g.data2, g.data3, g.data4))
        }
    }

    #[test]
    #[ignore = "verändert Systemeinstellungen – nur in CI"]
    fn registry_apply_and_revert() {
        use winreg::enums::HKEY_CURRENT_USER;
        let path = r"Software\GameBoostTest";
        let read = || winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(path).ok().and_then(|k| k.get_value::<u32, _>("Value").ok());
        assert_eq!(read(), None);
        let (_, rev) = registry(&[(path, "Value", 1)]).unwrap();
        assert_eq!(read(), Some(1));
        revert(&rev.unwrap()).unwrap();
        assert_eq!(read(), None, "Wert muss wieder gelöscht sein");
        let _ = winreg::RegKey::predef(HKEY_CURRENT_USER).delete_subkey(path);
    }

    #[test]
    #[ignore = "verändert Systemeinstellungen – nur in CI"]
    fn standby_purge_reports_real_values() {
        match purge_standby_list() {
            Ok((m, _)) => println!("{m}"),
            Err(e) => println!("nicht möglich (erwartbar ohne Admin): {e}"),
        }
    }

    #[test]
    #[ignore = "verändert Systemeinstellungen – nur in CI"]
    fn new_tweaks_apply_and_revert() {
        for (name, f) in [("device_power", device_power as fn() -> Applied), ("cpu_full", cpu_full), ("mouse_accel", mouse_accel), ("no_interruptions", no_interruptions)] {
            match f() {
                Ok((m, rev)) => {
                    println!("{name}: {m}");
                    if let Some(r) = rev {
                        println!("  zurück: {}", revert(&r).expect("revert"));
                    }
                }
                Err(e) => println!("{name}: nicht möglich: {e}"),
            }
        }
    }

    #[test]
    fn close_background_dry_run() {
        // Nur auflisten, NICHTS beenden – auf dem CI-Rechner würde Beenden den Test-Runner treffen.
        let list = background_candidates(&[], None);
        for (name, path, pids) in &list {
            println!("würde beenden: {name} ({} Prozess(e)) – {path}", pids.len());
        }
        // Der eigene Prozess darf nie in der Liste stehen.
        assert!(!list.iter().any(|(_, _, pids)| pids.contains(&std::process::id())));
    }

    #[test]
    fn microphone_detection_and_process_list_work() {
        println!("Mikrofon aktiv: {:?}", crate::boost::protect::microphone_in_use());
        let procs = crate::win::process::list();
        assert!(procs.iter().any(|p| p.pid == std::process::id()), "eigener Prozess muss in der Liste sein");
    }

    #[test]
    fn close_apps_skips_protected() {
        let (msg, rev) = close_apps(&["svchost.exe".into(), "MsMpEng.exe".into()], &[]).unwrap();
        println!("{msg}");
        assert!(rev.is_none(), "geschützte Prozesse dürfen nie beendet werden");
        assert!(msg.contains("geschützt"));
    }
}
