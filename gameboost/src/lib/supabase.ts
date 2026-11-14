import type { SupabaseClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

/**
 * Projekt-Adresse bereinigen: Häufig wird versehentlich „…supabase.co/rest/v1/“ eingetragen.
 * Supabase braucht aber nur die Basis-Adresse „https://xyz.supabase.co“.
 */
function cleanUrl(raw: string | undefined): string | undefined {
  if (!raw) return raw;
  let u = raw.trim().replace(/\s+/g, "");
  u = u.replace(/\/(rest|auth|storage|realtime)\/v1\/?.*$/i, "");
  u = u.replace(/\/+$/, "");
  if (u && !/^https?:\/\//i.test(u)) u = `https://${u}`;
  return u;
}

const URL = cleanUrl(import.meta.env.VITE_SUPABASE_URL as string | undefined);
const KEY = (import.meta.env.VITE_SUPABASE_ANON_KEY as string | undefined)?.trim();

/** Ist ein Supabase-Projekt eingetragen? Sonst läuft die App im Gast-Modus. */
export const supabaseConfigured = Boolean(URL && KEY && !URL.includes("DEIN-PROJEKT"));

/**
 * „Nur mit Einladung“: Die App ist nur nach Anmeldung nutzbar, Registrieren ist ausgeblendet.
 * Konten legt nur der Besitzer im Supabase-Dashboard an (Neue Registrierungen dort abschalten!).
 */
export const inviteOnly = supabaseConfigured && import.meta.env.VITE_INVITE_ONLY === "true";

/** Speichert Anmelde-Tokens in der Windows-Anmeldeinformationsverwaltung statt im Browser-Speicher. */
const secureStorage = {
  getItem: async (k: string) => (isTauri() ? invoke<string | null>("secure_get", { k: sanitize(k) }) : null),
  setItem: async (k: string, value: string) => {
    if (isTauri()) await invoke("secure_set", { k: sanitize(k), value });
  },
  removeItem: async (k: string) => {
    if (isTauri()) await invoke("secure_delete", { k: sanitize(k) });
  },
};

function sanitize(k: string) {
  return k.replace(/[^A-Za-z0-9._-]/g, "_").slice(0, 180);
}

let client: Promise<SupabaseClient> | null = null;

/** Lädt die Supabase-Bibliothek erst, wenn sie gebraucht wird (schnellerer App-Start). */
export function getSupabase(): Promise<SupabaseClient> {
  if (!supabaseConfigured) return Promise.reject(new Error("Konto-Funktion ist nicht eingerichtet."));
  client ??= import("@supabase/supabase-js").then(({ createClient }) =>
    createClient(URL!, KEY!, {
      auth: { storage: secureStorage, flowType: "pkce", persistSession: true, autoRefreshToken: true, detectSessionInUrl: false },
    }),
  );
  return client;
}

/** Übersetzt typische Supabase-Fehlermeldungen ins Deutsche. */
export function authError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  const map: [RegExp, string][] = [
    [/invalid login credentials/i, "E-Mail oder Passwort ist falsch."],
    [/email not confirmed/i, "Bitte bestätige zuerst deine E-Mail-Adresse."],
    [/user already registered/i, "Für diese E-Mail gibt es schon ein Konto."],
    [/password should be at least/i, "Das Passwort ist zu kurz (mindestens 8 Zeichen)."],
    [/token has expired|invalid.*otp|otp.*invalid/i, "Der Code ist falsch oder abgelaufen."],
    [/rate limit/i, "Zu viele Versuche. Bitte warte kurz."],
    [/failed to fetch|network/i, "Keine Verbindung zum Server."],
  ];
  return map.find(([re]) => re.test(msg))?.[1] ?? msg;
}
