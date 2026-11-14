//! Startet Intel PresentMon (Open Source, MIT) und liest dessen CSV-Ausgabe live ein.
//!
//! PresentMon liest die Bildausgabe über Windows-Ereignisprotokolle (ETW) mit.
//! Es greift NICHT in Spielprozesse ein (keine Injection, kein Overlay) – daher Anti-Cheat-sicher.
//! ETW-Mitschnitte erfordern Administratorrechte.

use super::frames::FrameStore;
use super::CaptureStatus;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const SESSION_NAME: &str = "GameBoost";

/// Programme, deren Bilder uns nicht interessieren (Windows-Oberfläche, unsere eigene App).
const EXCLUDE: &[&str] = &[
    "dwm.exe",
    "msedgewebview2.exe",
    "gameboost.exe",
    "explorer.exe",
    "TextInputHost.exe",
    "ShellExperienceHost.exe",
    "StartMenuExperienceHost.exe",
    "SearchHost.exe",
    "ApplicationFrameHost.exe",
];

/// Sucht PresentMon.exe: im Installationsordner bzw. im Entwicklungsordner `src-tauri/bin`.
pub fn locate(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(r) = resource_dir {
        candidates.push(r.join("bin").join("PresentMon.exe"));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        candidates.push(dir.join("PresentMon.exe"));
    }
    #[cfg(debug_assertions)]
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("bin").join("PresentMon.exe"));
    candidates.into_iter().find(|p| p.is_file())
}

fn command(path: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Startet PresentMon aus einer Kopie im Benutzerordner. So ist die Datei im Installationsordner nie
/// gesperrt und ein Update kann sie jederzeit ersetzen.
fn runtime_copy(src: &Path) -> PathBuf {
    let Some(base) = std::env::var_os("LOCALAPPDATA") else { return src.to_path_buf() };
    let dir = Path::new(&base).join("GameBoost").join("runtime");
    let dst = dir.join("PresentMon.exe");
    let same = std::fs::metadata(&dst).ok().zip(std::fs::metadata(src).ok()).is_some_and(|(a, b)| a.len() == b.len());
    if same {
        return dst;
    }
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(src, &dst)) {
        Ok(_) => dst,
        Err(_) => src.to_path_buf(),
    }
}

/// Bindet PresentMon an GameBoost: Wird GameBoost beendet (auch hart, z. B. durch ein Update),
/// beendet Windows PresentMon automatisch mit.
#[cfg(windows)]
fn tie_to_app(child: &Child) {
    use std::os::windows::io::AsRawHandle;
    use std::sync::OnceLock;
    use windows_sys::Win32::System::JobObjects::*;
    static JOB: OnceLock<usize> = OnceLock::new();
    let job = *JOB.get_or_init(|| {
        // SAFETY: Job-Objekt wird einmal erstellt und bleibt bis zum Prozessende offen (gewollt).
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h.is_null() {
                return 0;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(h, JobObjectExtendedLimitInformation, (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32);
            h as usize
        }
    });
    if job != 0 {
        // SAFETY: beide Handles sind gültig.
        unsafe { AssignProcessToJobObject(job as _, child.as_raw_handle() as _) };
    }
}

#[cfg(not(windows))]
fn tie_to_app(_child: &Child) {}

pub fn spawn(
    path: &Path,
    frames: Arc<Mutex<FrameStore>>,
    status: Arc<Mutex<CaptureStatus>>,
    generation: Arc<AtomicU64>,
) -> std::io::Result<Child> {
    let exe = runtime_copy(path);
    let mut cmd = command(&exe);
    cmd.args(["--output_stdout", "--no_console_stats", "--stop_existing_session", "--session_name", SESSION_NAME, "--track_pc_latency"]);
    for e in EXCLUDE {
        cmd.args(["--exclude", e]);
    }
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    tie_to_app(&child);

    let my_gen = generation.load(Ordering::SeqCst);
    let stderr_tail = Arc::new(Mutex::new(String::new()));

    if let Some(mut err) = child.stderr.take() {
        let tail = stderr_tail.clone();
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            *tail.lock().unwrap() = s.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        });
    }

    let stdout = child.stdout.take().expect("stdout ist gepiped");
    std::thread::Builder::new().name("presentmon".into()).spawn(move || {
        let mut columns: Option<Columns> = None;
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            match &columns {
                None => columns = Columns::from_header(&line),
                Some(c) => {
                    if let Some(r) = c.parse(&line) {
                        frames.lock().unwrap().push(r.pid, r.app, r.ms, r.input_latency, r.pc_latency);
                    }
                }
            }
        }
        // Ausgabe beendet. War das nicht von uns gewollt, Fehler melden.
        std::thread::sleep(std::time::Duration::from_millis(300)); // stderr fertig lesen lassen
        if generation.load(Ordering::SeqCst) == my_gen {
            let detail = stderr_tail.lock().unwrap().clone();
            *status.lock().unwrap() = CaptureStatus::error(if detail.is_empty() {
                "PresentMon wurde unerwartet beendet.".into()
            } else {
                format!("PresentMon wurde beendet: {detail}")
            });
        }
    })?;
    Ok(child)
}

/// Räumt eine evtl. hängengebliebene ETW-Sitzung auf.
pub fn terminate_session(path: &Path) {
    if let Ok(mut c) = command(path)
        .args(["--terminate_existing_session", "--session_name", SESSION_NAME])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        let _ = c.wait();
    }
}

struct Columns {
    app: usize,
    pid: usize,
    ms: usize,
    input: Option<usize>,
    pc: Option<usize>,
    len: usize,
}

pub struct Row<'a> {
    pub app: &'a str,
    pub pid: u32,
    pub ms: f32,
    pub input_latency: Option<f32>,
    pub pc_latency: Option<f32>,
}

impl Columns {
    /// Spalten anhand der Kopfzeile finden – robust gegenüber verschiedenen PresentMon-Versionen.
    fn from_header(line: &str) -> Option<Self> {
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        let find = |names: &[&str]| cols.iter().position(|c| names.contains(c));
        Some(Self {
            app: find(&["Application"])?,
            pid: find(&["ProcessID"])?,
            ms: find(&["MsBetweenPresents", "FrameTime"])?,
            input: find(&["MsAllInputToPhotonLatency", "AllInputToPhotonLatency"]),
            pc: find(&["MsPCLatency"]),
            len: cols.len(),
        })
    }

    fn parse<'a>(&self, line: &'a str) -> Option<Row<'a>> {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < self.len {
            return None;
        }
        let num = |i: usize| f[i].trim().parse::<f32>().ok().filter(|v| v.is_finite() && *v > 0.0);
        Some(Row {
            app: f[self.app].trim(),
            pid: f[self.pid].trim().parse().ok()?,
            ms: num(self.ms)?,
            input_latency: self.input.and_then(num),
            pc_latency: self.pc.and_then(num),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v2_4_default_output() {
        let header = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInSeconds,MsBetweenSimulationStart,MsBetweenPresents,MsBetweenDisplayChange,MsInPresentAPI,MsRenderPresentLatency,MsUntilDisplayed,MsPCLatency,CPUStartTimeInSeconds,MsBetweenAppStart,MsCPUBusy,MsCPUWait,MsGPULatency,MsGPUTime,MsGPUBusy,MsGPUWait,MsAnimationError,AnimationTime,MsFlipDelay,MsAllInputToPhotonLatency,MsClickToPhotonLatency";
        let c = Columns::from_header(header).unwrap();
        let row = "cs2.exe,4242,0x1,DXGI,0,512,1,Hardware: Independent Flip,1.5,NA,4.1667,4.1667,0.1,2.0,3.0,NA,1.49,4.16,2.0,2.1,0.5,3.0,3.0,0.0,0.0,1.4,0.0,18.5,NA";
        let r = c.parse(row).unwrap();
        assert_eq!(r.app, "cs2.exe");
        assert_eq!(r.pid, 4242);
        assert!((r.ms - 4.1667).abs() < 1e-4);
        assert_eq!(r.input_latency, Some(18.5));
        assert_eq!(r.pc_latency, None);
    }

    #[test]
    fn rejects_unknown_header() {
        assert!(Columns::from_header("foo,bar").is_none());
    }
}
