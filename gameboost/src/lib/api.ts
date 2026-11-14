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
  boostHotkey: string;
}

export interface PermanentItem {
  id: string;
  title: string;
  summary: string;
  enabled: boolean;
  available: boolean;
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
  getPermanent: () => invoke<PermanentItem[]>("get_permanent"),
  setPermanent: (id: string, on: boolean) => invoke<string>("set_permanent", { id, on }),
  getVision: () => invoke<number>("get_vision"),
  setVision: (level: number) => invoke<void>("set_vision", { level }),
  openSetting: (target: "sound" | "display" | "nvidia" | "amd") => invoke<void>("open_setting", { target }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  applyOverlay: () => invoke<void>("apply_overlay"),
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
  inputLatencyMs?: number | null;
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
  steps: string[];
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

export interface StutterContext {
  cpu: number | null;
  gpu: number | null;
  ram: number | null;
  disk: number | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  wlan: boolean;
  top: { exe: string; cpu: number }[];
}

export interface StutterEvent {
  time: number;
  kind: "stutter" | "ping";
  valueMs: number;
  normalMs: number | null;
  game: string | null;
  cause: string;
  context: StutterContext;
}

export interface LagResult {
  idleMs: number | null;
  downloadLoadMs: number | null;
  uploadLoadMs: number | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  addedMs: number | null;
  grade: string;
  verdict: string;
  tips: string[];
}

export interface InstalledGame {
  name: string;
  exe: string;
  path: string;
  drive: string;
  onHdd: boolean | null;
}

export interface OverlayApp {
  exe: string;
  name: string;
  how: string;
}

export interface ShaderCache {
  label: string;
  path: string;
  sizeMb: number;
}

export interface ConfigGame {
  id: string;
  name: string;
  available: boolean;
  optimized: boolean;
  summary: string;
  changes: string[];
}

export interface FpsAdvice {
  situation: "gpu_bound" | "cpu_bound" | "headroom" | "unknown";
  headline: string;
  detail: string;
  recommendedCap: number | null;
  how: string[];
}

export const detectiveApi = {
  events: () => invoke<StutterEvent[]>("get_stutter_events"),
  clear: () => invoke<void>("clear_stutter_events"),
};

export const labApi = {
  lagTest: () => invoke<LagResult>("lag_test"),
  gamesStorage: () => invoke<InstalledGame[]>("games_storage"),
  overlays: () => invoke<OverlayApp[]>("active_overlays"),
  shaderCaches: () => invoke<ShaderCache[]>("shader_caches"),
  clearShaders: () => invoke<string>("clear_shader_caches"),
  gameConfigs: () => invoke<ConfigGame[]>("game_configs"),
  applyConfig: (id: string) => invoke<string>("apply_game_config", { id }),
  restoreConfig: (id: string) => invoke<string>("restore_game_config", { id }),
  fpsAdvice: () => invoke<FpsAdvice>("fps_advice"),
};
