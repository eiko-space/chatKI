import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import type { Session as AuthSession, SupabaseClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { authError, getSupabase, inviteOnly, supabaseConfigured } from "./supabase";
import { pullOrPushSettings, pushSettings, syncSessions } from "./sync";
import { isTauri } from "./tauri";

export type Plan = "free" | "pro";

interface AuthState {
  configured: boolean;
  inviteOnly: boolean;
  ready: boolean;
  session: AuthSession | null;
  plan: Plan;
  syncing: boolean;
  lastSync: Date | null;
  syncError: string | null;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (email: string, password: string) => Promise<"confirm" | "done">;
  signInWithProvider: (provider: "discord" | "google") => Promise<void>;
  verifyCode: (email: string, code: string, type: "signup" | "recovery" | "invite") => Promise<void>;
  sendReset: (email: string) => Promise<void>;
  updatePassword: (password: string) => Promise<void>;
  signOut: () => Promise<void>;
  syncNow: () => Promise<void>;
}

const Ctx = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(!supabaseConfigured);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [plan, setPlan] = useState<Plan>("free");
  const [syncing, setSyncing] = useState(false);
  const [lastSync, setLastSync] = useState<Date | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);
  const sbRef = useRef<SupabaseClient | null>(null);
  const pushTimer = useRef<number | undefined>(undefined);
  const userId = session?.user.id ?? null;

  const fullSync = useCallback(async (uid: string) => {
    const sb = sbRef.current;
    if (!sb) return;
    setSyncing(true);
    setSyncError(null);
    try {
      await pullOrPushSettings(sb, uid);
      await syncSessions(sb, uid);
      const { data } = await sb.from("profiles").select("plan").eq("user_id", uid).maybeSingle();
      setPlan(data?.plan === "pro" ? "pro" : "free");
      setLastSync(new Date());
    } catch (e) {
      setSyncError(authError(e));
    } finally {
      setSyncing(false);
    }
  }, []);

  // Supabase starten und Anmeldestatus beobachten
  useEffect(() => {
    if (!supabaseConfigured || !isTauri()) {
      setReady(true);
      return;
    }
    let unsub: (() => void) | undefined;
    getSupabase()
      .then(async (sb) => {
        sbRef.current = sb;
        const { data } = await sb.auth.getSession();
        setSession(data.session);
        setReady(true);
        unsub = sb.auth.onAuthStateChange((_e, s) => setSession(s)).data.subscription.unsubscribe;
      })
      .catch(() => setReady(true));
    return () => unsub?.();
  }, []);

  // Nach dem Anmelden einmal komplett abgleichen
  useEffect(() => {
    if (userId) fullSync(userId);
  }, [userId, fullSync]);

  // Änderungen automatisch hochladen (Einstellungen gebündelt nach 2 s, Messungen sofort)
  useEffect(() => {
    if (!userId || !isTauri()) return;
    const subs = [
      listen("settings-changed", () => {
        window.clearTimeout(pushTimer.current);
        pushTimer.current = window.setTimeout(() => {
          if (sbRef.current) pushSettings(sbRef.current, userId).then(() => setLastSync(new Date())).catch((e) => setSyncError(authError(e)));
        }, 2000);
      }),
      listen("sessions-changed", () => {
        if (sbRef.current) syncSessions(sbRef.current, userId).then(() => setLastSync(new Date())).catch((e) => setSyncError(authError(e)));
      }),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  }, [userId]);

  const sb = async () => {
    try {
      return sbRef.current ?? (sbRef.current = await getSupabase());
    } catch (e) {
      throw new Error(authError(e));
    }
  };
  const wrap = async <T,>(fn: () => Promise<T>): Promise<T> => {
    try {
      return await fn();
    } catch (e) {
      throw new Error(authError(e));
    }
  };

  const value: AuthState = {
    configured: supabaseConfigured,
    inviteOnly,
    ready,
    session,
    plan,
    syncing,
    lastSync,
    syncError,
    signIn: (email, password) =>
      wrap(async () => {
        const { error } = await (await sb()).auth.signInWithPassword({ email, password });
        if (error) throw error;
      }),
    signUp: (email, password) =>
      wrap(async () => {
        const { data, error } = await (await sb()).auth.signUp({ email, password });
        if (error) throw error;
        return data.session ? "done" : "confirm";
      }),
    signInWithProvider: (provider) =>
      wrap(async () => {
        const client = await sb();
        const redirectTo = await invoke<string>("start_oauth_listener");
        const { data, error } = await client.auth.signInWithOAuth({ provider, options: { redirectTo, skipBrowserRedirect: true } });
        if (error) throw error;
        const un = await listen<string>("oauth-callback", async ({ payload }) => {
          un();
          const code = new URL(payload).searchParams.get("code");
          if (code) await client.auth.exchangeCodeForSession(code);
        });
        await invoke("open_url", { url: data.url });
      }),
    verifyCode: (email, code, type) =>
      wrap(async () => {
        const { error } = await (await sb()).auth.verifyOtp({ email, token: code.trim(), type });
        if (error) throw error;
      }),
    sendReset: (email) =>
      wrap(async () => {
        const { error } = await (await sb()).auth.resetPasswordForEmail(email);
        if (error) throw error;
      }),
    updatePassword: (password) =>
      wrap(async () => {
        const { error } = await (await sb()).auth.updateUser({ password });
        if (error) throw error;
      }),
    signOut: () =>
      wrap(async () => {
        await (await sb()).auth.signOut();
        setPlan("free");
        setLastSync(null);
      }),
    syncNow: async () => {
      if (userId) await fullSync(userId);
    },
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useAuth außerhalb von AuthProvider");
  return v;
}

/** Vorbereitet für Free/Pro: Funktionen später einfach mit `isPro` freischalten. */
export const isPro = (plan: Plan) => plan === "pro";
