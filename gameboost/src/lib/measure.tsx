import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, measureApi, type Measurement, type Session } from "./api";
import { useAppState } from "./app-state";
import { isTauri } from "./tauri";

export type Step = "idle" | "before" | "between" | "settling" | "after" | "done";

interface MeasureState {
  step: Step;
  before: Measurement | null;
  after: Measurement | null;
  session: Session | null;
  error: string | null;
  startBefore: () => Promise<void>;
  startAfter: () => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

const Ctx = createContext<MeasureState | null>(null);
/** Wartezeit nach dem Boost, bevor gemessen wird (Windows braucht kurz, z. B. für den Energieplan). */
const SETTLE_MS = 5000;

type Finished = { Ok: Measurement } | { Err: string };

/** Ablauf der Vorher-/Nachher-Messung. Bleibt erhalten, wenn man die Seite wechselt. */
export function MeasureProvider({ children }: { children: ReactNode }) {
  const { boost, startBoost } = useAppState();
  const [step, setStep] = useState<Step>("idle");
  const [before, setBefore] = useState<Measurement | null>(null);
  const [after, setAfter] = useState<Measurement | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [error, setError] = useState<string | null>(null);
  const stepRef = useRef(step);
  stepRef.current = step;
  const beforeRef = useRef(before);
  beforeRef.current = before;

  useEffect(() => {
    if (!isTauri()) return;
    const un = listen<Finished>("measurement-finished", async ({ payload }) => {
      if ("Err" in payload) {
        setError(`Messung ungültig: ${payload.Err} Bitte erneut messen.`);
        setStep(stepRef.current === "after" ? "between" : "idle");
        return;
      }
      if (stepRef.current === "before") {
        setBefore(payload.Ok);
        setStep("between");
      } else if (stepRef.current === "after" && beforeRef.current) {
        setAfter(payload.Ok);
        const status = await api.status().catch(() => null);
        const s: Session = {
          id: `${Date.now()}`,
          createdAt: Math.floor(Date.now() / 1000),
          game: payload.Ok.game ?? beforeRef.current.game,
          before: beforeRef.current,
          after: payload.Ok,
          tweaks: (status?.results ?? []).filter((r) => r.status === "applied").map((r) => r.title),
        };
        await measureApi.saveSession(s).catch((e) => setError(String(e)));
        setSession(s);
        setStep("done");
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  const value: MeasureState = {
    step,
    before,
    after,
    session,
    error,
    startBefore: async () => {
      setError(null);
      setBefore(null);
      setAfter(null);
      setSession(null);
      try {
        await measureApi.start("vorher");
        setStep("before");
      } catch (e) {
        setError(String(e));
      }
    },
    startAfter: async () => {
      setError(null);
      setStep("settling");
      try {
        if (!boost?.active) await startBoost();
        await new Promise((r) => setTimeout(r, SETTLE_MS));
        await measureApi.start("nachher");
        setStep("after");
      } catch (e) {
        setError(String(e));
        setStep("between");
      }
    },
    cancel: async () => {
      await measureApi.cancel().catch(() => {});
      setStep(stepRef.current === "after" || stepRef.current === "settling" ? "between" : "idle");
    },
    reset: () => {
      setStep("idle");
      setBefore(null);
      setAfter(null);
      setSession(null);
      setError(null);
    },
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useMeasure(): MeasureState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useMeasure außerhalb von MeasureProvider");
  return v;
}
