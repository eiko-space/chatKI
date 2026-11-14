use tauri::AppHandle;

/// Schlüssel-Präfix, damit nur Konto-Daten von GameBoost gelesen/geschrieben werden können.
fn key(k: &str) -> Result<String, String> {
    if k.is_empty() || k.len() > 200 || !k.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        return Err("Ungültiger Schlüssel".into());
    }
    Ok(format!("auth.{k}"))
}

#[tauri::command]
pub fn secure_get(k: String) -> Result<Option<String>, String> {
    Ok(crate::auth::secure_get(&key(&k)?))
}

#[tauri::command]
pub fn secure_set(k: String, value: String) -> Result<(), String> {
    crate::auth::secure_set(&key(&k)?, &value)
}

#[tauri::command]
pub fn secure_delete(k: String) -> Result<(), String> {
    crate::auth::secure_delete(&key(&k)?);
    Ok(())
}

#[tauri::command]
pub fn start_oauth_listener(app: AppHandle) -> Result<String, String> {
    crate::auth::listen_for_oauth(app)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    crate::auth::open_url(&url)
}
