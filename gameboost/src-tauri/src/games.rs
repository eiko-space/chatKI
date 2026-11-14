//! Bekannte Spiele (Prozessnamen). Wird für die FPS-Zielerkennung und später für Auto-Boost genutzt.

pub struct KnownGame {
    pub name: &'static str,
    pub exes: &'static [&'static str],
}

pub const KNOWN_GAMES: &[KnownGame] = &[
    KnownGame { name: "Fortnite", exes: &["FortniteClient-Win64-Shipping.exe"] },
    KnownGame { name: "Counter-Strike 2", exes: &["cs2.exe"] },
    KnownGame { name: "Valorant", exes: &["VALORANT-Win64-Shipping.exe"] },
    KnownGame { name: "Apex Legends", exes: &["r5apex.exe", "r5apex_dx12.exe"] },
    KnownGame { name: "Call of Duty: Warzone", exes: &["cod.exe", "ModernWarfare.exe"] },
    KnownGame { name: "Minecraft", exes: &["javaw.exe", "Minecraft.Windows.exe"] },
    KnownGame { name: "Rocket League", exes: &["RocketLeague.exe"] },
    KnownGame { name: "League of Legends", exes: &["League of Legends.exe"] },
];

/// Liefert den Anzeigenamen, falls die .exe zu einem bekannten Spiel gehört.
pub fn known_game(exe: &str) -> Option<&'static str> {
    KNOWN_GAMES
        .iter()
        .find(|g| g.exes.iter().any(|e| e.eq_ignore_ascii_case(exe)))
        .map(|g| g.name)
}
