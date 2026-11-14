//! GPU-Auslastung, VRAM und Temperatur – herstellerunabhängig (NVIDIA, AMD, Intel).
//!
//! * Auslastung & VRAM-Belegung: Windows-Leistungsindikatoren „GPU Engine“ / „GPU Adapter Memory“
//! * Name, VRAM-Größe, Temperatur: Grafik-Kernel-Schnittstelle (D3DKMT)
//!
//! Das sind dieselben Quellen, die auch der Windows-Task-Manager nutzt. Keine Admin-Rechte, kein Treiber.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GpuStats {
    pub name: Option<String>,
    pub usage: Option<f32>,
    pub vram_used_gb: Option<f32>,
    pub vram_total_gb: Option<f32>,
    pub vram_usage: Option<f32>,
    pub temperature: Option<f32>,
}

pub use platform::GpuSampler;

/// Extrahiert die Adapter-LUID aus einem Instanznamen wie
/// `pid_1234_luid_0x00000000_0x0000D1C4_phys_0_eng_0_engtype_3D` → `0x00000000_0x0000d1c4`.
fn luid_of(instance: &str) -> Option<String> {
    let lower = instance.to_ascii_lowercase();
    let start = lower.find("luid_")? + 5;
    let rest = &lower[start..];
    let end = rest.find("_phys").unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

#[cfg(windows)]
mod platform {
    use super::{luid_of, GpuStats};
    use std::collections::HashMap;
    use windows_sys::Wdk::Graphics::Direct3D::*;
    use windows_sys::Win32::System::Performance::*;

    struct Adapter {
        handle: u32,
        luid: String,
        name: String,
        vram_total: u64,
    }

    pub struct GpuSampler {
        query: isize,
        engine: isize,
        memory: isize,
        adapter: Option<Adapter>,
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    impl GpuSampler {
        pub fn new() -> Self {
            let mut s = Self { query: 0, engine: 0, memory: 0, adapter: find_main_adapter() };
            // SAFETY: Ausgabezeiger sind gültig; Fehler führen nur dazu, dass Werte fehlen.
            unsafe {
                if PdhOpenQueryW(std::ptr::null(), 0, &mut s.query) == 0 {
                    let p = wide(r"\GPU Engine(*engtype_3D)\Utilization Percentage");
                    PdhAddEnglishCounterW(s.query, p.as_ptr(), 0, &mut s.engine);
                    let p = wide(r"\GPU Adapter Memory(*)\Dedicated Usage");
                    PdhAddEnglishCounterW(s.query, p.as_ptr(), 0, &mut s.memory);
                    PdhCollectQueryData(s.query);
                }
            }
            s
        }

        pub fn sample(&mut self) -> GpuStats {
            let mut out = GpuStats::default();
            if let Some(a) = &self.adapter {
                out.name = Some(a.name.clone());
                if a.vram_total > 0 {
                    out.vram_total_gb = Some(a.vram_total as f32 / 1024f32.powi(3));
                }
                out.temperature = temperature(a.handle);
            }
            if self.query == 0 || unsafe { PdhCollectQueryData(self.query) } != 0 {
                return out;
            }

            // 3D-Auslastung je Adapter summieren (über alle Prozesse/Engines).
            let mut per_luid: HashMap<String, f64> = HashMap::new();
            for (name, v) in read_array(self.engine) {
                if let Some(l) = luid_of(&name) {
                    *per_luid.entry(l).or_default() += v;
                }
            }
            let usage = match &self.adapter {
                Some(a) => per_luid.get(&a.luid).copied(),
                None => per_luid.values().copied().reduce(f64::max),
            };
            out.usage = usage.map(|u| u.clamp(0.0, 100.0) as f32);

            if let Some(a) = &self.adapter {
                let used: f64 = read_array(self.memory)
                    .into_iter()
                    .filter(|(n, _)| luid_of(n).as_deref() == Some(a.luid.as_str()))
                    .map(|(_, v)| v)
                    .sum();
                if used > 0.0 {
                    out.vram_used_gb = Some((used / 1024f64.powi(3)) as f32);
                    if a.vram_total > 0 {
                        out.vram_usage = Some((100.0 * used / a.vram_total as f64).min(100.0) as f32);
                    }
                }
            }
            out
        }
    }

    // SAFETY: Die PDH-Handles und das D3DKMT-Handle werden nur vom Sampler-Thread benutzt.
    unsafe impl Send for GpuSampler {}

    fn read_array(counter: isize) -> Vec<(String, f64)> {
        if counter == 0 {
            return Vec::new();
        }
        let (mut size, mut count) = (0u32, 0u32);
        // SAFETY: Erster Aufruf ermittelt nur die Puffergröße.
        let r = unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut()) };
        if r != PDH_MORE_DATA || size == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        // SAFETY: Puffer hat die geforderte Größe und 8-Byte-Ausrichtung.
        if unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) } != 0 {
            return Vec::new();
        }
        // SAFETY: PDH hat `count` Einträge geschrieben; die Namen zeigen in denselben Puffer.
        let slice = unsafe { std::slice::from_raw_parts(items, count as usize) };
        slice
            .iter()
            .filter(|it| it.FmtValue.CStatus <= 1) // VALID_DATA oder NEW_DATA
            .map(|it| {
                let name = unsafe {
                    let mut len = 0;
                    while *it.szName.add(len) != 0 {
                        len += 1;
                    }
                    String::from_utf16_lossy(std::slice::from_raw_parts(it.szName, len))
                };
                (name, unsafe { it.FmtValue.Anonymous.doubleValue })
            })
            .collect()
    }

    fn query<T>(handle: u32, kind: KMTQUERYADAPTERINFOTYPE, data: &mut T) -> bool {
        let mut q = D3DKMT_QUERYADAPTERINFO {
            hAdapter: handle,
            Type: kind,
            pPrivateDriverData: (data as *mut T).cast(),
            PrivateDriverDataSize: std::mem::size_of::<T>() as u32,
        };
        // SAFETY: Datenpuffer passt zum angefragten Typ.
        unsafe { D3DKMTQueryAdapterInfo(&mut q) >= 0 }
    }

    fn temperature(handle: u32) -> Option<f32> {
        // SAFETY: reine Datenstruktur.
        let mut perf: D3DKMT_ADAPTER_PERFDATA = unsafe { std::mem::zeroed() };
        perf.PhysicalAdapterIndex = 0;
        if !query(handle, KMTQAITYPE_ADAPTERPERFDATA, &mut perf) {
            return None;
        }
        // Angabe in Zehntel-Grad; 0 bedeutet „vom Treiber nicht geliefert“.
        (perf.Temperature > 0).then(|| perf.Temperature as f32 / 10.0)
    }

    /// Wählt die Grafikkarte mit dem meisten eigenen Videospeicher (= die Spiele-GPU).
    fn find_main_adapter() -> Option<Adapter> {
        let mut e = D3DKMT_ENUMADAPTERS2 { NumAdapters: 0, pAdapters: std::ptr::null_mut() };
        // SAFETY: Erster Aufruf liefert die Anzahl.
        if unsafe { D3DKMTEnumAdapters2(&mut e) } < 0 || e.NumAdapters == 0 {
            return None;
        }
        // SAFETY: reine Datenstrukturen.
        let mut list: Vec<D3DKMT_ADAPTERINFO> = vec![unsafe { std::mem::zeroed() }; e.NumAdapters as usize];
        e.pAdapters = list.as_mut_ptr();
        if unsafe { D3DKMTEnumAdapters2(&mut e) } < 0 {
            return None;
        }
        list.truncate(e.NumAdapters as usize);

        let mut best: Option<Adapter> = None;
        for info in &list {
            let mut seg: D3DKMT_SEGMENTSIZEINFO = unsafe { std::mem::zeroed() };
            let vram = if query(info.hAdapter, KMTQAITYPE_GETSEGMENTSIZE, &mut seg) { seg.DedicatedVideoMemorySize } else { 0 };
            let mut reg: D3DKMT_ADAPTERREGISTRYINFO = unsafe { std::mem::zeroed() };
            let name = if query(info.hAdapter, KMTQAITYPE_ADAPTERREGISTRYINFO, &mut reg) {
                let len = reg.AdapterString.iter().position(|c| *c == 0).unwrap_or(0);
                String::from_utf16_lossy(&reg.AdapterString[..len])
            } else {
                String::new()
            };
            let candidate = Adapter {
                handle: info.hAdapter,
                luid: format!("0x{:08x}_0x{:08x}", info.AdapterLuid.HighPart as u32, info.AdapterLuid.LowPart),
                name: if name.is_empty() { "Grafikkarte".into() } else { name },
                vram_total: vram,
            };
            let software = candidate.name.contains("Microsoft Basic");
            if !software && best.as_ref().is_none_or(|b| candidate.vram_total > b.vram_total) {
                if let Some(old) = best.replace(candidate) {
                    close(old.handle);
                }
            } else {
                close(candidate.handle);
            }
        }
        best
    }

    fn close(handle: u32) {
        let c = D3DKMT_CLOSEADAPTER { hAdapter: handle };
        // SAFETY: Handle stammt aus D3DKMTEnumAdapters2.
        unsafe { D3DKMTCloseAdapter(&c) };
    }
}

#[cfg(not(windows))]
mod platform {
    use super::GpuStats;
    pub struct GpuSampler;
    impl GpuSampler {
        pub fn new() -> Self {
            Self
        }
        pub fn sample(&mut self) -> GpuStats {
            GpuStats::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::luid_of;

    #[test]
    fn parses_luid() {
        assert_eq!(
            luid_of("pid_1234_luid_0x00000000_0x0000D1C4_phys_0_eng_0_engtype_3D").as_deref(),
            Some("0x00000000_0x0000d1c4")
        );
        assert_eq!(luid_of("luid_0x00000000_0x0000D1C4_phys_0").as_deref(), Some("0x00000000_0x0000d1c4"));
        assert_eq!(luid_of("_Total"), None);
    }
}
