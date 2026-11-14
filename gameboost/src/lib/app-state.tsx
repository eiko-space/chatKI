import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type BoostStatus, type Settings, type TweakInfo, type TweakResult } from "./api";
import { isTauri } from "./tauri";

interface AppState {
  catalog: TweakInfo[];
  boost: BoostStatus | null;
  settings: Settings | null;
  busy: boolean;
  lastRevert: TweakResult[] | null;
  error: string | null;
  startBoost: () => Promise<void>;
  stopBoost: () => Promise<void>;
  saveSettings: (s: Settings) => Promise<void>;
  updateSettings: (fn: (s: Settings) => Settings) => Promise<void>;
  refresh: () => Promise<void>;
}

const Ctx = createContext<AppState | null>(null);

/** Gemeinsamer Zustand für Boost und Einstellungen – wird über Backend-Events aktuell gehalten. */
export function AppStateProvider({ children }: { children: ReactNode }) {
  const [catalog, setCatalog] = useState<TweakInfo[]>([]);
  const [boost, setBoost] = useState<BoostStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [busy, setBusy] = useState(false);
  const [lastRevert, setLastRevert] = useState<TweakResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isTauri()) return;
    const [c, b, s] = await Promise.all([api.catalog(), api.status(), api.getSettings()]);
    setCatalog(c);
    setBoost(b);
    setSettings(s);
  }, []);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    if (!isTauri()) return;
    const subs = [
      listen<BoostStatus>("boost-changed", (e) => setBoost(e.payload)),
      listen("settings-changed", () => api.getSettings().then(setSettings)),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  }, [refresh]);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const value: AppState = {
    catalog,
    boost,
    settings,
    busy,
    lastRevert,
    error,
    refresh,
    startBoost: () => run(async () => {
      setLastRevert(null);
      setBoost(await api.applyBoost());
    }),
    stopBoost: () => run(async () => {
      setLastRevert(await api.revertBoost());
      setBoost(await api.status());
    }),
    saveSettings: async (s) => {
      setSettings(s);
      await api.saveSettings(s);
    },
    updateSettings: async (fn) => {
      if (!settings) return;
      const next = fn(settings);
      setSettings(next);
      await api.saveSettings(next);
    },
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAppState(): AppState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useAppState außerhalb von AppStateProvider");
  return v;
}
