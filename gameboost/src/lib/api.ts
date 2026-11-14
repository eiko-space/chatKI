import { invoke } from "@tauri-apps/api/core";

// Typen spiegeln die Rust-Strukturen (src-tauri/src/boost, settings.rs) wider.

export interface TweakInfo {
  id: string;
  title: string;
  summary: string;
  details: string;
  category: "fps" | "network";
  needsAdmin: boolean;
  defaultOn: boolean;
  oneShot: boolean;
}

export type TweakStatus = "applied" | "unchanged" | "skipped" | "failed" | "reverted";

export interface TweakResult {
  id: string;
  title: string;
  status: TweakStatus;
  message: string;
}

export interface BoostStatus {
  active: boolean;
  startedAt: number | null;
  game: string | null;
  results: TweakResult[];
}

export interface LogEntry {
  time: number;
  action: string;
  title: string;
  status: string;
  message: string;
}

export interface BackgroundApp {
  exe: string;
  enabled: boolean;
}

export interface GameProfile {
  name: string;
  exe: string;
  tweaks: string[] | null;
  disableFullscreenOptimizations: boolean;
  exePath: string | null;
  custom: boolean;
}

export interface OverlaySettings {
  enabled: boolean;
  position: "top-left" | "top-right" | "bottom-left" | "bottom-right";
  scale: number;
  hotkey: string;
  showFps: boolean;
  showPing: boolean;
  showTemp: boolean;
}

export interface Settings {
  enabledTweaks: string[];
  backgroundApps: BackgroundApp[];
  profiles: GameProfile[];
  autoBoost: boolean;
  notifications: boolean;
  tempWarningC: number;
  overlay: OverlaySettings;
  restorePointCreatedAt: number | null;
  firstBoostConfirmed: boolean;
}

export interface ProtectedApp {
  exe: string;
  reason: string;
}

export interface RunningApp {
  exe: string;
  memoryMb: number | null;
}

export const api = {
  catalog: () => invoke<TweakInfo[]>("get_boost_catalog"),
  status: () => invoke<BoostStatus>("get_boost_status"),
  applyBoost: () => invoke<BoostStatus>("apply_boost"),
  revertBoost: () => invoke<TweakResult[]>("revert_boost"),
  createRestorePoint: () => invoke<string>("create_restore_point"),
  changeLog: () => invoke<LogEntry[]>("get_change_log"),
  protectedRunning: () => invoke<ProtectedApp[]>("get_protected_running"),
  runningApps: () => invoke<RunningApp[]>("list_running_apps"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  setFullscreenOptimizations: (exePath: string, disable: boolean) =>
    invoke<void>("set_fullscreen_optimizations", { exePath, disable }),
};

export interface Measurement {
  game: string | null;
  durationSecs: number;
  frames: number;
  fpsAvg: number | null;
  low1: number | null;
  low01: number | null;
  frametimeStdDevMs: number | null;
  stuttersPerMin: number | null;
  pingAvgMs: number | null;
  jitterMs: number | null;
  lossPercent: number | null;
  spikesPerMin: number | null;
  cpuAvg: number | null;
  gpuAvg: number | null;
}

export interface Session {
  id: string;
  createdAt: number;
  game: string | null;
  before: Measurement;
  after: Measurement;
  tweaks: string[];
}

export const measureApi = {
  start: (label: string) => invoke<void>("start_measurement", { label }),
  cancel: () => invoke<void>("cancel_measurement"),
  listSessions: () => invoke<Session[]>("list_sessions"),
  saveSession: (session: Session) => invoke<void>("save_session", { session }),
  deleteSession: (id: string) => invoke<void>("delete_session", { id }),
};

export interface RegionResult {
  id: string;
  city: string;
  area: string;
  host: string;
  minMs: number | null;
  medianMs: number | null;
  jitterMs: number | null;
  failed: number;
  attempts: number;
}

export interface Hop {
  hop: number;
  address: string | null;
  kind: "home" | "isp" | "internet" | "target" | "unknown";
  bestMs: number | null;
  avgMs: number | null;
  worstMs: number | null;
  lossPercent: number;
}

export interface TraceResult {
  target: string;
  resolved: string | null;
  hops: Hop[];
  reached: boolean;
  analysis: string[];
}

export interface NetApp {
  exe: string;
  connections: number;
}

export type CheckLevel = "good" | "warn" | "bad" | "info";

export interface CheckItem {
  id: string;
  title: string;
  level: CheckLevel;
  value: string;
  explanation: string;
  details: string | null;
  fix: string | null;
  fixLabel: string | null;
  fixNeedsAdmin: boolean;
}

export interface CheckReport {
  score: number;
  items: CheckItem[];
}

export const toolsApi = {
  regionPing: () => invoke<RegionResult[]>("region_ping"),
  traceroute: (target: string) => invoke<TraceResult>("traceroute", { target }),
  netApps: () => invoke<NetApp[]>("net_apps"),
  pcCheck: () => invoke<CheckReport>("run_pc_check"),
  pcFix: (id: string) => invoke<string>("run_pc_fix", { id }),
};
