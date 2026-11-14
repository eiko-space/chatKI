//! Prozesse auflisten, Priorität setzen, Programme schließen.

use super::from_wide;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM, WPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, GWL_EXSTYLE, WM_CLOSE, WS_EX_TOOLWINDOW};

#[derive(Clone, Debug)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub parent: u32,
}

/// Alle laufenden Prozesse.
pub fn list() -> Vec<Proc> {
    let mut out = Vec::new();
    // SAFETY: Snapshot-Handle wird geprüft und geschlossen; Struktur mit dwSize initialisiert.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                out.push(Proc { pid: e.th32ProcessID, name: from_wide(&e.szExeFile), parent: e.th32ParentProcessID });
                if Process32NextW(snap, &mut e) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: Handle stammt aus OpenProcess.
        unsafe { CloseHandle(self.0) };
    }
}

fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Option<Handle> {
    // SAFETY: Rückgabe wird geprüft.
    let h = unsafe { OpenProcess(access, 0, pid) };
    (!h.is_null()).then_some(Handle(h))
}

/// Vollständiger Pfad der .exe (falls zugänglich).
pub fn path(pid: u32) -> Option<String> {
    let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: Puffer und Länge passen zusammen.
    (unsafe { QueryFullProcessImageNameW(h.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) } != 0)
        .then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

/// Arbeitsspeicher (Working Set) in MB.
pub fn memory_mb(pid: u32) -> Option<f32> {
    let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    // SAFETY: Struktur nullinitialisiert, Größe korrekt angegeben.
    let mut c: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    (unsafe { K32GetProcessMemoryInfo(h.0, &mut c, c.cb) } != 0).then(|| c.WorkingSetSize as f32 / 1_048_576.0)
}

/// Gehört der Prozess zur eigenen Windows-Sitzung (also zum angemeldeten Benutzer)?
pub fn in_my_session(pid: u32) -> bool {
    let (mut mine, mut theirs) = (0u32, 0u32);
    // SAFETY: Ausgabezeiger gültig.
    unsafe {
        ProcessIdToSessionId(std::process::id(), &mut mine) != 0
            && ProcessIdToSessionId(pid, &mut theirs) != 0
            && mine == theirs
            && theirs != 0
    }
}

pub fn get_priority(pid: u32) -> Result<u32, String> {
    let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok_or_else(|| os_err("Prozess nicht zugänglich"))?;
    // SAFETY: gültiges Handle.
    let p = unsafe { GetPriorityClass(h.0) };
    if p == 0 { Err(os_err("Priorität nicht lesbar")) } else { Ok(p) }
}

pub fn set_priority(pid: u32, class: u32) -> Result<(), String> {
    let h = open(pid, PROCESS_SET_INFORMATION).ok_or_else(|| os_err("Prozess erlaubt keine Änderung"))?;
    // SAFETY: gültiges Handle, gültige Prioritätsklasse.
    if unsafe { SetPriorityClass(h.0, class) } != 0 { Ok(()) } else { Err(os_err("Priorität konnte nicht gesetzt werden")) }
}

pub const PRIORITY_HIGH: u32 = HIGH_PRIORITY_CLASS;
pub const PRIORITY_NORMAL: u32 = NORMAL_PRIORITY_CLASS;
pub const PRIORITY_BELOW_NORMAL: u32 = BELOW_NORMAL_PRIORITY_CLASS;

/// Energiespar-Drosselung (EcoQoS) für einen Prozess aus (`true`) oder Windows überlassen (`false`).
pub fn set_power_throttling_off(pid: u32, off: bool) -> Result<(), String> {
    let h = open(pid, PROCESS_SET_INFORMATION).ok_or_else(|| os_err("Prozess erlaubt keine Änderung"))?;
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: if off { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
        StateMask: 0,
    };
    // SAFETY: gültiges Handle, Struktur mit korrekter Größe.
    let ok = unsafe {
        SetProcessInformation(h.0, ProcessPowerThrottling, (&state as *const PROCESS_POWER_THROTTLING_STATE).cast(), std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32)
    };
    if ok != 0 { Ok(()) } else { Err(os_err("Energiespar-Drosselung nicht änderbar")) }
}

unsafe extern "system" fn post_close(hwnd: HWND, lparam: LPARAM) -> i32 {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == lparam as u32 {
        PostMessageW(hwnd, WM_CLOSE, 0 as WPARAM, 0);
    }
    1
}

/// Schließt ein Programm: erst höflich (Fenster schließen), nach 3 s notfalls hart.
pub fn close(pid: u32) -> Result<(), String> {
    // SAFETY: Callback hält sich an die WNDENUMPROC-Signatur.
    unsafe { EnumWindows(Some(post_close), pid as LPARAM) };
    if let Some(h) = open(pid, PROCESS_SYNCHRONIZE | PROCESS_TERMINATE) {
        // SAFETY: gültiges Handle.
        unsafe {
            if WaitForSingleObject(h.0, 3000) == 0 {
                return Ok(()); // sauber beendet
            }
            if TerminateProcess(h.0, 0) != 0 {
                WaitForSingleObject(h.0, 2000);
                return Ok(());
            }
        }
    }
    Err(os_err("Programm konnte nicht beendet werden"))
}

/// Startet ein Programm wieder – als normaler Benutzer (über den Explorer), nicht mit Admin-Rechten.
pub fn launch_as_user(path: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn os_err(context: &str) -> String {
    let e = std::io::Error::last_os_error();
    match e.raw_os_error() {
        Some(5) => format!("{context} (Zugriff verweigert – z. B. durch Anti-Cheat-Schutz)"),
        _ => format!("{context} ({e})"),
    }
}

#[allow(dead_code)]
pub fn exists(pid: u32) -> bool {
    open(pid, PROCESS_QUERY_LIMITED_INFORMATION).is_some()
}

/// Verbrauchte CPU-Zeit (Kernel + Benutzer) eines Prozesses in 100-ns-Einheiten.
pub fn cpu_time(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::FILETIME;
    let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let z = || FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut c, mut e, mut k, mut u) = (z(), z(), z(), z());
    // SAFETY: gültiges Handle, Ausgabezeiger gültig.
    if unsafe { GetProcessTimes(h.0, &mut c, &mut e, &mut k, &mut u) } == 0 {
        return None;
    }
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    Some(ft(k) + ft(u))
}

/// GPU-Planungspriorität (Windows-Grafikkernel) lesen.
pub fn get_gpu_priority(pid: u32) -> Option<i32> {
    use windows_sys::Wdk::Graphics::Direct3D::D3DKMTGetProcessSchedulingPriorityClass;
    let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut c = 0i32;
    // SAFETY: gültiges Handle, Ausgabezeiger gültig.
    (unsafe { D3DKMTGetProcessSchedulingPriorityClass(h.0, &mut c) } >= 0).then_some(c)
}

/// GPU-Planungspriorität setzen (2 = normal, 4 = hoch).
pub fn set_gpu_priority(pid: u32, class: i32) -> Result<(), String> {
    use windows_sys::Wdk::Graphics::Direct3D::D3DKMTSetProcessSchedulingPriorityClass;
    let h = open(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION).ok_or_else(|| os_err("Prozess erlaubt keine Änderung"))?;
    // SAFETY: gültiges Handle und gültige Prioritätsklasse.
    let st = unsafe { D3DKMTSetProcessSchedulingPriorityClass(h.0, class) };
    if st >= 0 { Ok(()) } else { Err(format!("GPU-Priorität nicht änderbar (Status 0x{st:08X})")) }
}

unsafe extern "system" fn collect_visible(hwnd: HWND, lparam: LPARAM) -> i32 {
    let set = &mut *(lparam as *mut std::collections::HashSet<u32>);
    // Sichtbar, mit Titel und kein reines Werkzeugfenster = ein Fenster, das der Nutzer sieht
    if IsWindowVisible(hwnd) != 0 && GetWindowTextLengthW(hwnd) > 0 && (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW) == 0 {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        set.insert(pid);
    }
    1
}

/// Prozesse, die mindestens ein sichtbares Fenster haben (z. B. geöffnete Programme mit evtl. ungespeicherten Daten).
pub fn pids_with_visible_windows() -> std::collections::HashSet<u32> {
    let mut set = std::collections::HashSet::new();
    // SAFETY: Callback schreibt nur in das übergebene HashSet, das während des Aufrufs lebt.
    unsafe { EnumWindows(Some(collect_visible), &mut set as *mut _ as LPARAM) };
    set
}

/// PIDs aller „Vorfahren“ eines Prozesses (Eltern, Großeltern …) – die dürfen nie beendet werden.
pub fn ancestors(pid: u32, all: &[Proc]) -> std::collections::HashSet<u32> {
    let mut out = std::collections::HashSet::new();
    let mut cur = pid;
    for _ in 0..64 {
        let Some(p) = all.iter().find(|p| p.pid == cur) else { break };
        if p.parent == 0 || !out.insert(p.parent) {
            break;
        }
        cur = p.parent;
    }
    out
}
