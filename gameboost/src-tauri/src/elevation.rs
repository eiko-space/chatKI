//! Administratorrechte: prüfen und – nur auf Wunsch des Nutzers – die App mit Admin-Rechten neu starten.

/// Läuft der Prozess mit Administratorrechten?
#[cfg(windows)]
pub fn is_elevated() -> bool {
    // SAFETY: IsUserAnAdmin hat keine Parameter und keine Vorbedingungen.
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Prozess-ID des Programms im Vordergrund (für die FPS-Zielauswahl).
#[cfg(windows)]
pub fn foreground_pid() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: Beide Funktionen sind ohne Vorbedingungen aufrufbar; Null-Fenster wird geprüft.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        (pid != 0).then_some(pid)
    }
}

#[cfg(not(windows))]
pub fn foreground_pid() -> Option<u32> {
    None
}

const WAIT_ARG: &str = "--wait-pid";

/// Startet die App mit Admin-Rechten neu. Windows zeigt dabei die Benutzerkontensteuerung (UAC).
/// Gibt `Ok(())` zurück, wenn der neue Prozess gestartet wurde – der Aufrufer beendet dann die alte Instanz.
#[cfg(windows)]
pub fn relaunch_elevated() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};

    if cfg!(debug_assertions) {
        return Err("Im Entwicklungsmodus kann sich die App nicht selbst neu starten. \
                    Starte das Befehlsfenster als Administrator und führe dort „npm run tauri dev“ aus."
            .into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let params = wide(&format!("{WAIT_ARG} {}", std::process::id()));

    // SAFETY: Struktur nullinitialisiert, alle Zeiger zeigen auf gültige, nullterminierte Strings.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = 1; // SW_SHOWNORMAL
    if unsafe { ShellExecuteExW(&mut info) } != 0 {
        Ok(())
    } else {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        Err(if code == 1223 {
            "Du hast die Admin-Abfrage abgelehnt. Die FPS-Messung bleibt aus.".into()
        } else {
            format!("Neustart mit Admin-Rechten fehlgeschlagen (Fehler {code}).")
        })
    }
}

#[cfg(not(windows))]
pub fn relaunch_elevated() -> Result<(), String> {
    Err("Nur unter Windows verfügbar.".into())
}

/// Wurde die App per Neustart mit Admin-Rechten gestartet, wartet sie, bis die alte Instanz beendet ist
/// (sonst würde die Einzelinstanz-Sperre den neuen Start verhindern).
pub fn wait_for_previous_instance() {
    let args: Vec<String> = std::env::args().collect();
    let Some(pid) = args.iter().position(|a| a == WAIT_ARG).and_then(|i| args.get(i + 1)).and_then(|p| p.parse::<u32>().ok()) else {
        return;
    };
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
        // SAFETY: Handle wird geprüft und wieder geschlossen.
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if !h.is_null() {
                WaitForSingleObject(h, 15_000);
                CloseHandle(h);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = pid;
}
