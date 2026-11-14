//! Konto-Hilfen: sichere Token-Speicherung (Windows-Anmeldeinformationsverwaltung) und
//! Empfang des Logins aus dem Browser (Discord/Google) über eine lokale Adresse.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};

const SERVICE: &str = "GameBoost";
/// Windows erlaubt pro Eintrag nur ~2,5 KB – längere Werte werden aufgeteilt.
const CHUNK: usize = 1200;
/// Feste lokale Adresse für den Login-Rückweg (muss in Supabase als Redirect-URL eingetragen sein).
pub const OAUTH_PORT: u16 = 53682;

mod store {
    #[cfg(windows)]
    pub fn get(key: &str) -> Option<String> {
        keyring::Entry::new(super::SERVICE, key).ok()?.get_password().ok()
    }
    #[cfg(windows)]
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(super::SERVICE, key).and_then(|e| e.set_password(value)).map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    pub fn delete(key: &str) {
        if let Ok(e) = keyring::Entry::new(super::SERVICE, key) {
            let _ = e.delete_credential();
        }
    }

    // Außerhalb von Windows (nur Entwicklung/Tests): flüchtiger Speicher im Arbeitsspeicher.
    #[cfg(not(windows))]
    static MEM: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> = std::sync::Mutex::new(None);
    #[cfg(not(windows))]
    pub fn get(key: &str) -> Option<String> {
        MEM.lock().unwrap().as_ref()?.get(key).cloned()
    }
    #[cfg(not(windows))]
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        MEM.lock().unwrap().get_or_insert_with(Default::default).insert(key.into(), value.into());
        Ok(())
    }
    #[cfg(not(windows))]
    pub fn delete(key: &str) {
        if let Some(m) = MEM.lock().unwrap().as_mut() {
            m.remove(key);
        }
    }
}

pub fn secure_get(key: &str) -> Option<String> {
    let count: usize = store::get(&format!("{key}#n"))?.parse().ok()?;
    let mut out = String::new();
    for i in 0..count {
        out.push_str(&store::get(&format!("{key}#{i}"))?);
    }
    Some(out)
}

pub fn secure_set(key: &str, value: &str) -> Result<(), String> {
    secure_delete(key);
    let chars: Vec<char> = value.chars().collect();
    let parts: Vec<String> = chars.chunks(CHUNK).map(|c| c.iter().collect()).collect();
    for (i, p) in parts.iter().enumerate() {
        store::set(&format!("{key}#{i}"), p)?;
    }
    store::set(&format!("{key}#n"), &parts.len().to_string())
}

pub fn secure_delete(key: &str) {
    if let Some(n) = store::get(&format!("{key}#n")).and_then(|n| n.parse::<usize>().ok()) {
        for i in 0..n {
            store::delete(&format!("{key}#{i}"));
        }
    }
    store::delete(&format!("{key}#n"));
}

const DONE_PAGE: &str = "<!doctype html><html lang=de><meta charset=utf-8><title>GameBoost</title>\
<body style=\"background:#0B0D12;color:#E7EAF0;font-family:Segoe UI,sans-serif;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1 style=\"color:#00E5FF\">Angemeldet ✓</h1><p>Du kannst dieses Fenster schließen und zu GameBoost zurückkehren.</p></div>";

/// Wartet (max. 5 Minuten) auf den Rückweg vom Browser-Login und meldet die URL an die Oberfläche.
pub fn listen_for_oauth<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let listener = TcpListener::bind(("127.0.0.1", OAUTH_PORT))
        .map_err(|_| format!("Port {OAUTH_PORT} ist belegt. Schließe andere Login-Fenster und versuche es erneut."))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(300);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/").to_string();
                    if !path.starts_with("/callback") {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                        continue;
                    }
                    let body = DONE_PAGE.as_bytes();
                    let _ = stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes());
                    let _ = stream.write_all(body);
                    let _ = app.emit("oauth-callback", format!("http://127.0.0.1:{OAUTH_PORT}{path}"));
                    crate::tray::show_main_window(&app);
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
                Err(_) => return,
            }
        }
    });
    Ok(format!("http://127.0.0.1:{OAUTH_PORT}/callback"))
}

/// Öffnet eine Adresse im Standard-Browser.
pub fn open_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://127.0.0.1")) {
        return Err("Nur sichere Adressen erlaubt.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer.exe").arg(url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_roundtrip() {
        let long: String = "äbc".repeat(1500);
        secure_set("test-key", &long).unwrap();
        assert_eq!(secure_get("test-key").as_deref(), Some(long.as_str()));
        secure_delete("test-key");
        assert_eq!(secure_get("test-key"), None);
    }

    #[test]
    fn rejects_insecure_urls() {
        assert!(open_url("file:///C:/Windows/System32/calc.exe").is_err());
        assert!(open_url("javascript:alert(1)").is_err());
    }
}
