//! Kleine, sichere Hüllen um Windows-APIs, die mehrere Module brauchen.
#[cfg(windows)]
pub mod process;

/// Wandelt einen Rust-String in einen nullterminierten UTF-16-String um.
#[cfg(windows)]
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Liest einen nullterminierten UTF-16-Puffer.
#[cfg(windows)]
pub fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
