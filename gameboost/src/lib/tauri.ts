import { invoke } from "@tauri-apps/api/core";

/** True, wenn die Oberfläche innerhalb der Tauri-App läuft (nicht im normalen Browser). */
export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface AppInfo {
  version: string;
  os: string;
  osVersion: string;
  arch: string;
  isElevated: boolean;
}

/**
 * Liest echte Basis-Infos vom Rust-Backend.
 * Im Browser (ohne Tauri) gibt es bewusst KEINE Ersatzwerte – dann `null`.
 */
export async function getAppInfo(): Promise<AppInfo | null> {
  if (!isTauri()) return null;
  return invoke<AppInfo>("get_app_info");
}
