import type { SupabaseClient } from "@supabase/supabase-js";
import { api, measureApi, type Session, type Settings } from "./api";

/** Nur geräteunabhängige Einstellungen werden synchronisiert (keine Pfade, kein Wiederherstellungspunkt). */
type Portable = Pick<Settings, "enabledTweaks" | "backgroundApps" | "profiles" | "autoBoost" | "notifications" | "tempWarningC" | "overlay">;

const LAST_PUSH = "gb.sync.lastPush";

function portable(s: Settings): Portable {
  return {
    enabledTweaks: s.enabledTweaks,
    backgroundApps: s.backgroundApps,
    profiles: s.profiles.map((p) => ({ ...p, exePath: null })),
    autoBoost: s.autoBoost,
    notifications: s.notifications,
    tempWarningC: s.tempWarningC,
    overlay: s.overlay,
  };
}

function merge(local: Settings, remote: Portable): Settings {
  return {
    ...local,
    ...remote,
    // Pfade bleiben gerätespezifisch
    profiles: remote.profiles.map((p) => ({ ...p, exePath: local.profiles.find((l) => l.exe.toLowerCase() === p.exe.toLowerCase())?.exePath ?? null })),
  };
}

function lastPush(): number {
  try {
    return Number(localStorage.getItem(LAST_PUSH) ?? 0);
  } catch {
    return 0;
  }
}

function setLastPush(t: number) {
  try {
    localStorage.setItem(LAST_PUSH, String(t));
  } catch {
    /* privater Modus – kein Problem */
  }
}

export async function pushSettings(sb: SupabaseClient, userId: string): Promise<void> {
  const s = await api.getSettings();
  const now = new Date();
  const { error } = await sb.from("user_settings").upsert({ user_id: userId, settings: portable(s), updated_at: now.toISOString() });
  if (error) throw error;
  setLastPush(now.getTime());
}

/** Beim Anmelden: neuere Cloud-Einstellungen übernehmen, sonst lokale hochladen. */
export async function pullOrPushSettings(sb: SupabaseClient, userId: string): Promise<"pulled" | "pushed"> {
  const { data, error } = await sb.from("user_settings").select("settings, updated_at").eq("user_id", userId).maybeSingle();
  if (error) throw error;
  if (data && new Date(data.updated_at).getTime() > lastPush()) {
    const local = await api.getSettings();
    await api.saveSettings(merge(local, data.settings as Portable));
    setLastPush(new Date(data.updated_at).getTime());
    return "pulled";
  }
  await pushSettings(sb, userId);
  return "pushed";
}

/** Messungen in beide Richtungen abgleichen (es geht nichts verloren). */
export async function syncSessions(sb: SupabaseClient, userId: string): Promise<{ up: number; down: number }> {
  const local = await measureApi.listSessions();
  const { data, error } = await sb.from("sessions").select("id, data").eq("user_id", userId);
  if (error) throw error;
  const remote = new Map((data ?? []).map((r) => [r.id as string, r.data as Session]));
  const toUpload = local.filter((s) => !remote.has(s.id));
  if (toUpload.length) {
    const { error: e } = await sb.from("sessions").upsert(
      toUpload.map((s) => ({ user_id: userId, id: s.id, created_at: new Date(s.createdAt * 1000).toISOString(), data: s })),
    );
    if (e) throw e;
  }
  const localIds = new Set(local.map((s) => s.id));
  const toDownload = [...remote.values()].filter((s) => !localIds.has(s.id));
  for (const s of toDownload) await measureApi.saveSession(s);
  return { up: toUpload.length, down: toDownload.length };
}
