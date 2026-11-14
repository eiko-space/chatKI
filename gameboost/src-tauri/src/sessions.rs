//! Gespeicherte Vorher-/Nachher-Vergleiche (Verlauf).

use crate::telemetry::measure::Measurement;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub created_at: u64,
    pub game: Option<String>,
    pub before: Measurement,
    pub after: Measurement,
    /// Titel der Tweaks, die beim „Nachher“ aktiv waren
    pub tweaks: Vec<String>,
}

pub struct SessionStore {
    path: PathBuf,
    sessions: Mutex<Vec<Session>>,
}

impl SessionStore {
    pub fn load(dir: &std::path::Path) -> Self {
        let path = dir.join("sessions.json");
        let sessions = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { path, sessions: Mutex::new(sessions) }
    }

    pub fn list(&self) -> Vec<Session> {
        self.sessions.lock().unwrap().clone()
    }

    pub fn add(&self, s: Session) -> Result<(), String> {
        let mut all = self.sessions.lock().unwrap();
        all.retain(|x| x.id != s.id);
        all.push(s);
        all.sort_by_key(|x| x.created_at);
        crate::storage::write_json(&self.path, &*all)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut all = self.sessions.lock().unwrap();
        all.retain(|x| x.id != id);
        crate::storage::write_json(&self.path, &*all)
    }

    /// Ersetzt alle Sessions (z. B. nach Cloud-Sync).
    pub fn replace_all(&self, list: Vec<Session>) -> Result<(), String> {
        let mut all = self.sessions.lock().unwrap();
        *all = list;
        all.sort_by_key(|x| x.created_at);
        crate::storage::write_json(&self.path, &*all)
    }
}
