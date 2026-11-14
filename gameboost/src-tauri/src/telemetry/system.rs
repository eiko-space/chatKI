//! CPU-, RAM- und Netzwerkdurchsatz über die Windows-API (keine Admin-Rechte nötig).

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_usage: Option<f32>,
    pub ram_usage: Option<f32>,
    pub ram_used_gb: Option<f32>,
    pub ram_total_gb: Option<f32>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    /// "lan", "wlan" oder "other"
    pub connection: Option<&'static str>,
    pub adapter: Option<String>,
    pub download_mbps: Option<f32>,
    pub upload_mbps: Option<f32>,
}

pub use platform::Sampler;

#[cfg(windows)]
mod platform {
    use super::{LinkStats, SystemStats};
    use std::time::Instant;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetBestInterface, GetIfEntry2, MIB_IF_ROW2};
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    const IF_TYPE_ETHERNET: u32 = 6;
    const IF_TYPE_WIFI: u32 = 71;

    #[derive(Default)]
    pub struct Sampler {
        cpu_prev: Option<(u64, u64)>, // (idle, total)
        net_prev: Option<(u32, u64, u64, Instant)>, // (ifindex, in, out, zeit)
    }

    fn ft(f: FILETIME) -> u64 {
        ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
    }

    impl Sampler {
        pub fn system(&mut self) -> SystemStats {
            let mut s = SystemStats::default();

            let (mut idle, mut kernel, mut user) = (FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 }, FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 }, FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 });
            // SAFETY: Zeiger auf gültige Strukturen.
            if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0 {
                // Kernel-Zeit enthält die Leerlaufzeit bereits.
                let (idle, total) = (ft(idle), ft(kernel) + ft(user));
                if let Some((pi, pt)) = self.cpu_prev {
                    let dt = total.saturating_sub(pt);
                    if dt > 0 {
                        let busy = dt.saturating_sub(idle.saturating_sub(pi));
                        s.cpu_usage = Some((100.0 * busy as f64 / dt as f64).clamp(0.0, 100.0) as f32);
                    }
                }
                self.cpu_prev = Some((idle, total));
            }

            // SAFETY: Struktur wird nullinitialisiert und dwLength gesetzt, wie von der API verlangt.
            let mut mem: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
            mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if unsafe { GlobalMemoryStatusEx(&mut mem) } != 0 && mem.ullTotalPhys > 0 {
                let gb = |b: u64| (b as f64 / 1024f64.powi(3)) as f32;
                let used = mem.ullTotalPhys - mem.ullAvailPhys;
                s.ram_total_gb = Some(gb(mem.ullTotalPhys));
                s.ram_used_gb = Some(gb(used));
                s.ram_usage = Some((100.0 * used as f64 / mem.ullTotalPhys as f64) as f32);
            }
            s
        }

        /// Durchsatz des Adapters, über den das Internet erreicht wird.
        pub fn link(&mut self) -> LinkStats {
            let mut out = LinkStats::default();
            let mut index = 0u32;
            // SAFETY: Zeiger auf gültige Variable.
            if unsafe { GetBestInterface(u32::from_ne_bytes([1, 1, 1, 1]), &mut index) } != 0 {
                self.net_prev = None;
                return out;
            }
            // SAFETY: Struktur nullinitialisiert, InterfaceIndex gesetzt – so verlangt es GetIfEntry2.
            let mut row: MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
            row.InterfaceIndex = index;
            if unsafe { GetIfEntry2(&mut row) } != 0 {
                self.net_prev = None;
                return out;
            }
            out.connection = Some(match row.Type {
                IF_TYPE_ETHERNET => "lan",
                IF_TYPE_WIFI => "wlan",
                _ => "other",
            });
            let len = row.Description.iter().position(|c| *c == 0).unwrap_or(row.Description.len());
            out.adapter = Some(String::from_utf16_lossy(&row.Description[..len]));

            let now = Instant::now();
            if let Some((pi, pin, pout, pt)) = self.net_prev {
                let secs = now.duration_since(pt).as_secs_f64();
                if pi == index && secs > 0.0 {
                    let mbps = |d: u64| (d as f64 * 8.0 / 1e6 / secs) as f32;
                    out.download_mbps = Some(mbps(row.InOctets.saturating_sub(pin)));
                    out.upload_mbps = Some(mbps(row.OutOctets.saturating_sub(pout)));
                }
            }
            self.net_prev = Some((index, row.InOctets, row.OutOctets, now));
            out
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{LinkStats, SystemStats};
    #[derive(Default)]
    pub struct Sampler;
    impl Sampler {
        pub fn system(&mut self) -> SystemStats {
            SystemStats::default()
        }
        pub fn link(&mut self) -> LinkStats {
            LinkStats::default()
        }
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::Sampler;

    #[test]
    fn reads_cpu_and_ram() {
        let mut s = Sampler::default();
        s.system();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let st = s.system();
        println!("{st:?}");
        assert!(st.cpu_usage.is_some_and(|v| (0.0..=100.0).contains(&v)));
        assert!(st.ram_total_gb.is_some_and(|v| v > 0.5));
        assert!(st.ram_usage.is_some_and(|v| v > 0.0 && v <= 100.0));
    }

    #[test]
    fn reads_link() {
        let mut s = Sampler::default();
        s.link();
        std::thread::sleep(std::time::Duration::from_millis(300));
        println!("{:?}", s.link());
    }
}
