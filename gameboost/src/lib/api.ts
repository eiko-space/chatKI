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
