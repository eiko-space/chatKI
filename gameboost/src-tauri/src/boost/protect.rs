//! Schutzliste: Programme, die GameBoost NIEMALS beendet – auch nicht, wenn man sie selbst einträgt.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    System,
    Antivirus,
    AntiCheat,
    Voice,
    Audio,
    Peripheral,
    Launcher,
    Game,
    MicrophoneInUse,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Reason {
    pub fn label(self) -> &'static str {
        match self {
            Reason::System => "Windows-Systemprozess",
            Reason::Antivirus => "Virenschutz",
            Reason::AntiCheat => "Anti-Cheat",
            Reason::Voice => "Voice-Chat / Anrufe",
            Reason::Audio => "Audio / Streaming",
            Reason::Peripheral => "Maus-/Tastatur-/Headset-Software",
            Reason::Launcher => "Spiele-Launcher",
            Reason::Game => "Spiel",
            Reason::MicrophoneInUse => "Mikrofon gerade aktiv",
        }
    }
}

/// Exakte Namen von Windows-Kernprozessen.
const SYSTEM: &[&str] = &[
    "system", "registry", "memory compression", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe",
    "lsass.exe", "lsaiso.exe", "svchost.exe", "dwm.exe", "explorer.exe", "fontdrvhost.exe", "sihost.exe", "ctfmon.exe",
    "audiodg.exe", "conhost.exe", "taskhostw.exe", "runtimebroker.exe", "searchhost.exe", "searchindexer.exe",
    "startmenuexperiencehost.exe", "shellexperiencehost.exe", "textinputhost.exe", "spoolsv.exe", "wudfhost.exe",
    "dllhost.exe", "msedgewebview2.exe", "applicationframehost.exe", "systemsettings.exe", "lockapp.exe",
    "gameboost.exe", "presentmon.exe", "taskmgr.exe", "wmiprvse.exe", "dashost.exe", "smartscreen.exe",
    "securityhealthsystray.exe", "securityhealthservice.exe", "gamebarpresencewriter.exe", "gamingservices.exe",
    "gamingservicesnet.exe", "nvcontainer.exe", "nvdisplay.container.exe", "atiesrxx.exe", "atieclxx.exe",
    "igfxem.exe", "amdrsserv.exe",
];

/// Namensbestandteile (kleingeschrieben) je Kategorie. Lieber zu viel schützen als zu wenig.
const PATTERNS: &[(Reason, &[&str])] = &[
    (Reason::Antivirus, &[
        "msmpeng", "nissrv", "mpdefender", "defender", "avast", "avg", "avira", "avguard", "bitdefender", "bdagent",
        "bdservicehost", "vsserv", "eset", "ekrn", "egui", "kaspersky", "avp", "malwarebytes", "mbam", "mcafee",
        "mcshield", "mfemms", "norton", "nortonsecurity", "sophos", "trend", "webroot", "wrsa", "panda", "f-secure",
        "fsecure", "comodo", "gdata", "avgnt", "360tray", "zonealarm", "emsisoft", "hitmanpro", "totalav",
    ]),
    (Reason::AntiCheat, &[
        "vgc", "vgtray", "vanguard", "easyanticheat", "eac_", "beservice", "battleye", "faceit", "esea", "gameguard",
        "xigncode", "anticheat", "ricochet", "punkbuster", "pnkbstr", "nprotect",
    ]),
    (Reason::Voice, &[
        "discord", "teamspeak", "ts3client", "ts5client", "mumble", "skype", "teams", "zoom", "whatsapp", "telegram",
        "signal", "slack", "guilded", "webex", "revolt", "element", "viber", "facetime", "jitsi", "ventrilo",
    ]),
    (Reason::Audio, &[
        "obs", "streamlabs", "xsplit", "voicemeeter", "vbaudio", "nvbroadcast", "nvidia broadcast", "rtkaud",
        "realtek", "nahimic", "sonar", "wavelink", "elgato", "spotify", "equalizer", "audiorelay", "krisp",
    ]),
    (Reason::Peripheral, &[
        "lghub", "logi", "razer", "synapse", "icue", "corsair", "armourycrate", "armoury", "steelseries", "hyperx",
        "ngenuity", "wooting", "roccat", "glorious", "msi center", "dragoncenter", "asus",
    ]),
    (Reason::Launcher, &[
        "steam", "epicgameslauncher", "epicwebhelper", "epiconlineservices", "riotclient", "leagueclient",
        "battle.net", "agent.exe", "eadesktop", "eabackgroundservice", "origin", "ubisoftconnect", "upc.exe",
        "uplaywebcore", "galaxyclient", "xboxapp", "xboxpcapp", "minecraftlauncher", "rockstar", "launcher",
    ]),
];

/// Prüft, ob ein Programm geschützt ist. `mic_apps` = Programme, die gerade das Mikrofon nutzen.
pub fn check(exe: &str, mic_apps: &[String]) -> Option<Reason> {
    let lower = exe.to_ascii_lowercase();
    if mic_apps.iter().any(|m| m.eq_ignore_ascii_case(exe)) {
        return Some(Reason::MicrophoneInUse);
    }
    if SYSTEM.contains(&lower.as_str()) {
        return Some(Reason::System);
    }
    if crate::games::known_game(exe).is_some() {
        return Some(Reason::Game);
    }
    PATTERNS.iter().find(|(_, pats)| pats.iter().any(|p| lower.contains(p))).map(|(r, _)| *r)
}

/// Programme, die laut Windows gerade das Mikrofon benutzen
/// (dieselbe Information, die Windows für das Mikrofon-Symbol in der Taskleiste nutzt).
#[cfg(windows)]
pub fn microphone_in_use() -> Vec<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    const BASE: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

    let mut out = Vec::new();
    let in_use = |key: &RegKey| key.get_value::<u64, _>("LastUsedTimeStop").ok() == Some(0)
        && key.get_value::<u64, _>("LastUsedTimeStart").is_ok_and(|v| v > 0);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Klassische Programme: Schlüsselname ist der Pfad mit # statt \ (z. B. C:#Users#…#Discord.exe)
    if let Ok(np) = hkcu.open_subkey(format!(r"{BASE}\NonPackaged")) {
        for name in np.enum_keys().flatten() {
            if np.open_subkey(&name).is_ok_and(|k| in_use(&k)) {
                if let Some(exe) = name.rsplit('#').next() {
                    out.push(exe.to_string());
                }
            }
        }
    }
    // Store-Apps: Schlüsselname ist der Paketname (z. B. MicrosoftTeams_8wekyb3d8bbwe)
    if let Ok(base) = hkcu.open_subkey(BASE) {
        for name in base.enum_keys().flatten().filter(|n| n != "NonPackaged") {
            if base.open_subkey(&name).is_ok_and(|k| in_use(&k)) {
                out.push(name);
            }
        }
    }
    out
}

#[cfg(not(windows))]
pub fn microphone_in_use() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_critical_apps() {
        let none: Vec<String> = vec![];
        assert_eq!(check("MsMpEng.exe", &none), Some(Reason::Antivirus));
        assert_eq!(check("AvastUI.exe", &none), Some(Reason::Antivirus));
        assert_eq!(check("Discord.exe", &none), Some(Reason::Voice));
        assert_eq!(check("ts3client_win64.exe", &none), Some(Reason::Voice));
        assert_eq!(check("vgc.exe", &none), Some(Reason::AntiCheat));
        assert_eq!(check("EasyAntiCheat_EOS.exe", &none), Some(Reason::AntiCheat));
        assert_eq!(check("steam.exe", &none), Some(Reason::Launcher));
        assert_eq!(check("lghub.exe", &none), Some(Reason::Peripheral));
        assert_eq!(check("svchost.exe", &none), Some(Reason::System));
        assert_eq!(check("cs2.exe", &none), Some(Reason::Game));
    }

    #[test]
    fn microphone_overrides_everything() {
        let mic = vec!["MyCallApp.exe".to_string()];
        assert_eq!(check("mycallapp.exe", &mic), Some(Reason::MicrophoneInUse));
    }

    #[test]
    fn normal_apps_are_not_protected() {
        let none: Vec<String> = vec![];
        assert_eq!(check("OneDrive.exe", &none), None);
        assert_eq!(check("Dropbox.exe", &none), None);
        assert_eq!(check("CCXProcess.exe", &none), None);
    }
}
