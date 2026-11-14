//! Netzwerk-Werkzeuge: Region-Ping, Routen-Analyse (wie WinMTR), Programme mit Internetverbindungen.

use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ Region-Ping

/// Rechenzentren, in denen viele Online-Spiele ihre Server betreiben (Amazon AWS).
const REGIONS: &[(&str, &str, &str)] = &[
    ("eu-central-1", "Frankfurt", "EU"),
    ("eu-central-2", "Zürich", "EU"),
    ("eu-west-1", "Irland", "EU"),
    ("eu-west-2", "London", "EU"),
    ("eu-west-3", "Paris", "EU"),
    ("eu-north-1", "Stockholm", "EU"),
    ("eu-south-1", "Mailand", "EU"),
    ("eu-south-2", "Spanien", "EU"),
    ("me-central-1", "VAE", "Naher Osten"),
    ("us-east-1", "Virginia (US-Ost)", "Nordamerika"),
    ("us-east-2", "Ohio (US-Mitte)", "Nordamerika"),
    ("us-west-2", "Oregon (US-West)", "Nordamerika"),
    ("sa-east-1", "São Paulo", "Südamerika"),
    ("ap-northeast-1", "Tokio", "Asien"),
    ("ap-southeast-1", "Singapur", "Asien"),
    ("ap-southeast-2", "Sydney", "Ozeanien"),
];

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RegionResult {
    pub id: String,
    pub city: String,
    pub area: String,
    pub host: String,
    /// Bester (kleinster) Wert – entspricht am ehesten der reinen Leitungszeit
    pub min_ms: Option<f32>,
    pub median_ms: Option<f32>,
    pub jitter_ms: Option<f32>,
    pub failed: u32,
    pub attempts: u32,
}

/// Misst die Verbindungszeit (TCP-Handshake) – funktioniert ohne Admin und auch, wenn Ping (ICMP) geblockt ist.
fn tcp_rtt(addr: SocketAddr, timeout: Duration) -> Option<f32> {
    let t = Instant::now();
    TcpStream::connect_timeout(&addr, timeout).ok().map(|_| t.elapsed().as_secs_f32() * 1000.0)
}

pub fn region_ping(attempts: u32) -> Vec<RegionResult> {
    let handles: Vec<_> = REGIONS
        .iter()
        .map(|(id, city, area)| {
            let (id, city, area) = (id.to_string(), city.to_string(), area.to_string());
            std::thread::spawn(move || {
                let host = format!("dynamodb.{id}.amazonaws.com");
                let mut r = RegionResult { id, city, area, host: host.clone(), min_ms: None, median_ms: None, jitter_ms: None, failed: 0, attempts };
                // DNS vorab auflösen, damit nur die Leitungszeit gemessen wird.
                let Some(addr) = (host.as_str(), 443).to_socket_addrs().ok().and_then(|mut a| a.find(|x| x.is_ipv4())) else {
                    r.failed = attempts;
                    return r;
                };
                let mut v = Vec::new();
                for _ in 0..attempts {
                    match tcp_rtt(addr, Duration::from_millis(1500)) {
                        Some(ms) => v.push(ms),
                        None => r.failed += 1,
                    }
                    std::thread::sleep(Duration::from_millis(150));
                }
                if !v.is_empty() {
                    let mut s = v.clone();
                    s.sort_by(f32::total_cmp);
                    r.min_ms = Some(s[0]);
                    r.median_ms = Some(s[s.len() / 2]);
                    if v.len() > 1 {
                        r.jitter_ms = Some(v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / (v.len() - 1) as f32);
                    }
                }
                r
            })
        })
        .collect();
    let mut out: Vec<RegionResult> = handles.into_iter().filter_map(|h| h.join().ok()).collect();
    out.sort_by(|a, b| a.median_ms.unwrap_or(f32::MAX).total_cmp(&b.median_ms.unwrap_or(f32::MAX)));
    out
}

// ------------------------------------------------------------------ Routen-Analyse

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hop {
    pub hop: u32,
    pub address: Option<String>,
    /// "home" (Heimnetz), "isp"/"internet" oder "target"
    pub kind: &'static str,
    pub best_ms: Option<f32>,
    pub avg_ms: Option<f32>,
    pub worst_ms: Option<f32>,
    pub loss_percent: f32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TraceResult {
    pub target: String,
    pub resolved: Option<String>,
    pub hops: Vec<Hop>,
    pub reached: bool,
    /// Ehrliche Einschätzung in einfachem Deutsch
    pub analysis: Vec<String>,
}

pub fn resolve(host: &str) -> Option<Ipv4Addr> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(ip);
    }
    (host, 0).to_socket_addrs().ok()?.find_map(|a| match a.ip() {
        std::net::IpAddr::V4(v4) => Some(v4),
        _ => None,
    })
}

pub fn traceroute(target: &str, probes: u32) -> Result<TraceResult, String> {
    let ip = resolve(target).ok_or_else(|| format!("„{target}“ konnte nicht aufgelöst werden."))?;
    let pinger = platform::Pinger::new().ok_or("Ping-Funktion von Windows nicht verfügbar.")?;
    let mut hops = Vec::new();
    let mut reached = false;
    for ttl in 1..=30u8 {
        let mut addr = None;
        let mut times = Vec::new();
        let mut at_target = false;
        for _ in 0..probes {
            if let Some((from, ms, done)) = pinger.probe(ip, ttl, 1000) {
                addr = Some(from);
                times.push(ms);
                at_target |= done;
            }
        }
        let avg = (!times.is_empty()).then(|| times.iter().sum::<f32>() / times.len() as f32);
        let kind = match addr {
            _ if at_target => "target",
            Some(a) if a.is_private() || a.is_link_local() => "home",
            Some(a) if is_cgnat(a) => "isp",
            Some(_) => "internet",
            None => "unknown",
        };
        hops.push(Hop {
            hop: ttl as u32,
            address: addr.map(|a| a.to_string()),
            kind,
            best_ms: times.iter().copied().reduce(f32::min),
            avg_ms: avg,
            worst_ms: times.iter().copied().reduce(f32::max),
            loss_percent: 100.0 * (probes as usize - times.len()) as f32 / probes as f32,
        });
        if at_target {
            reached = true;
            break;
        }
    }
    let analysis = analyse(&hops, reached);
    Ok(TraceResult { target: target.into(), resolved: Some(ip.to_string()), hops, reached, analysis })
}

fn is_cgnat(a: Ipv4Addr) -> bool {
    let o = a.octets();
    o[0] == 100 && (64..128).contains(&o[1])
}

/// Erklärt in einfachen Worten, wo der Ping entsteht.
pub fn analyse(hops: &[Hop], reached: bool) -> Vec<String> {
    let mut out = Vec::new();
    let answered: Vec<&Hop> = hops.iter().filter(|h| h.avg_ms.is_some()).collect();
    if answered.is_empty() {
        return vec!["Keine Station hat geantwortet – evtl. blockiert eine Firewall Ping-Anfragen.".into()];
    }
    if let Some(first) = hops.first() {
        match (first.kind, first.avg_ms) {
            ("home", Some(ms)) if ms > 5.0 => out.push(format!("Schon dein Router antwortet langsam ({ms:.0} ms). Das deutet auf WLAN-Probleme oder ein überlastetes Heimnetz hin – ein LAN-Kabel hilft meist.")),
            ("home", Some(_)) if first.loss_percent > 0.0 => out.push("Zum Router gehen Pakete verloren – typisch für schwaches WLAN.".into()),
            ("home", Some(_)) => out.push("Heimnetz in Ordnung (Router antwortet schnell).".into()),
            _ => {}
        }
    }
    // Größter Sprung zwischen zwei antwortenden Stationen
    let mut best: Option<(f32, &Hop, &Hop)> = None;
    for w in answered.windows(2) {
        let jump = w[1].avg_ms.unwrap() - w[0].avg_ms.unwrap();
        if best.is_none_or(|(b, _, _)| jump > b) {
            best = Some((jump, w[0], w[1]));
        }
    }
    if let Some((jump, a, b)) = best {
        if jump >= 15.0 {
            out.push(format!(
                "Der größte Anstieg (+{jump:.0} ms) liegt zwischen Station {} und {}. Dort entsteht der meiste Ping – oft eine weite Strecke oder ein Übergang zwischen Anbietern. Darauf hast du selbst keinen Einfluss.",
                a.hop, b.hop
            ));
        }
    }
    let lossy: Vec<u32> = hops.iter().filter(|h| h.loss_percent > 0.0 && h.avg_ms.is_some()).map(|h| h.hop).collect();
    if !lossy.is_empty() && reached {
        out.push("Einzelne Stationen verwerfen Ping-Pakete. Das ist oft harmlos (Router beantworten Pings nur nachrangig), solange das Ziel selbst keine Verluste hat.".into());
    }
    if !reached {
        out.push("Das Ziel hat nicht geantwortet. Viele Server blockieren Ping – die Strecke bis dahin ist trotzdem aussagekräftig.".into());
    }
    out
}

// ------------------------------------------------------------------ Programme mit Internetverbindungen

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetApp {
    pub exe: String,
    pub connections: u32,
}

/// Programme mit aktiven TCP-Verbindungen ins Internet (Anzahl, nicht Datenmenge).
pub fn apps_with_connections() -> Vec<NetApp> {
    let mut out: Vec<NetApp> = Vec::new();
    for (pid, n) in platform::tcp_connections_by_pid() {
        let exe = platform::process_name(pid).unwrap_or_else(|| format!("PID {pid}"));
        match out.iter_mut().find(|a| a.exe.eq_ignore_ascii_case(&exe)) {
            Some(a) => a.connections += n,
            None => out.push(NetApp { exe, connections: n }),
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.connections));
    out
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::net::Ipv4Addr;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::NetworkManagement::IpHelper::*;

    pub struct Pinger(HANDLE);
    impl Pinger {
        pub fn new() -> Option<Self> {
            // SAFETY: Rückgabe wird geprüft.
            let h = unsafe { IcmpCreateFile() };
            (!h.is_null() && h != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE).then_some(Self(h))
        }

        /// Sendet ein Paket mit begrenzter Lebensdauer (TTL). Liefert (antwortende Station, ms, Ziel erreicht?).
        pub fn probe(&self, ip: Ipv4Addr, ttl: u8, timeout: u32) -> Option<(Ipv4Addr, f32, bool)> {
            let payload = [0x47u8; 32];
            let mut buf = [0u64; 32];
            let opts = IP_OPTION_INFORMATION { Ttl: ttl, Tos: 0, Flags: 0, OptionsSize: 0, OptionsData: std::ptr::null_mut() };
            let t = std::time::Instant::now();
            // SAFETY: Handle gültig, Puffer ausreichend und ausgerichtet.
            unsafe {
                IcmpSendEcho(self.0, u32::from_ne_bytes(ip.octets()), payload.as_ptr().cast(), payload.len() as u16, &opts, buf.as_mut_ptr().cast(), std::mem::size_of_val(&buf) as u32, timeout);
                let reply = &*(buf.as_ptr() as *const ICMP_ECHO_REPLY);
                let from = Ipv4Addr::from(reply.Address.to_ne_bytes());
                match reply.Status {
                    IP_SUCCESS => Some((from, reply.RoundTripTime.max(1) as f32, true)),
                    // Bei abgelaufener TTL ist RoundTripTime oft 0 → eigene Zeitmessung
                    IP_TTL_EXPIRED_TRANSIT => Some((from, t.elapsed().as_secs_f32() * 1000.0, false)),
                    _ => None,
                }
            }
        }
    }
    impl Drop for Pinger {
        fn drop(&mut self) {
            // SAFETY: Handle stammt aus IcmpCreateFile.
            unsafe { IcmpCloseHandle(self.0) };
        }
    }

    pub fn tcp_connections_by_pid() -> HashMap<u32, u32> {
        use windows_sys::Win32::Networking::WinSock::AF_INET;
        let mut out = HashMap::new();
        let mut size = 0u32;
        // SAFETY: Erst Größe ermitteln, dann mit ausreichend großem, ausgerichtetem Puffer lesen.
        unsafe {
            GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, AF_INET as u32, TCP_TABLE_OWNER_PID_CONNECTIONS, 0);
            if size == 0 {
                return out;
            }
            let mut buf = vec![0u64; (size as usize).div_ceil(8) + 8];
            size = (buf.len() * 8) as u32;
            if GetExtendedTcpTable(buf.as_mut_ptr().cast(), &mut size, 0, AF_INET as u32, TCP_TABLE_OWNER_PID_CONNECTIONS, 0) != 0 {
                return out;
            }
            let table = &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
            for r in rows {
                let remote = Ipv4Addr::from(r.dwRemoteAddr.to_ne_bytes());
                if r.dwState == MIB_TCP_STATE_ESTAB as u32 && !remote.is_loopback() && !remote.is_private() && !remote.is_link_local() {
                    *out.entry(r.dwOwningPid).or_insert(0) += 1;
                }
            }
        }
        out
    }

    pub fn process_name(pid: u32) -> Option<String> {
        crate::win::process::list().into_iter().find(|p| p.pid == pid).map(|p| p.name)
    }
}

#[cfg(not(windows))]
mod platform {
    use std::collections::HashMap;
    use std::net::Ipv4Addr;
    pub struct Pinger;
    impl Pinger {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn probe(&self, _ip: Ipv4Addr, _ttl: u8, _t: u32) -> Option<(Ipv4Addr, f32, bool)> {
            None
        }
    }
    pub fn tcp_connections_by_pid() -> HashMap<u32, u32> {
        HashMap::new()
    }
    pub fn process_name(_pid: u32) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hop(n: u32, kind: &'static str, avg: Option<f32>, loss: f32) -> Hop {
        Hop { hop: n, address: Some("1.2.3.4".into()), kind, best_ms: avg, avg_ms: avg, worst_ms: avg, loss_percent: loss }
    }

    #[test]
    fn finds_biggest_jump_and_slow_router() {
        let hops = vec![hop(1, "home", Some(12.0), 0.0), hop(2, "isp", Some(15.0), 0.0), hop(3, "internet", Some(48.0), 0.0), hop(4, "target", Some(50.0), 0.0)];
        let a = analyse(&hops, true);
        assert!(a[0].contains("Router"));
        assert!(a.iter().any(|s| s.contains("zwischen Station 2 und 3")));
    }

    #[test]
    fn no_answers() {
        let hops = vec![Hop { hop: 1, address: None, kind: "unknown", best_ms: None, avg_ms: None, worst_ms: None, loss_percent: 100.0 }];
        assert!(analyse(&hops, false)[0].contains("Firewall"));
    }
}
