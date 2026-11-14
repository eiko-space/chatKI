import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "./tauri";

interface ProgressEvent {
  mode: "apply" | "revert";
  done: number;
  total: number;
  step: string;
}

/** Mindestdauer der Anzeige, damit man den Fortschritt auch sieht (die Tweaks selbst sind oft sehr schnell). */
const MIN_MS = 1800;

/**
 * Fortschritt des Boosts in Prozent.
 * Folgt den echten Schritten vom Backend, läuft aber sanft (nie schneller als MIN_MS für 100 %).
 */
export function useBoostProgress(working: boolean) {
  const [target, setTarget] = useState(0);
  const [shown, setShown] = useState(0);
  const [step, setStep] = useState("");
  const [mode, setMode] = useState<"apply" | "revert">("apply");
  const [visible, setVisible] = useState(false);
  const start = useRef(0);

  useEffect(() => {
    if (!isTauri()) return;
    const un = listen<ProgressEvent>("boost-progress", ({ payload }) => {
      setMode(payload.mode);
      setStep(payload.step);
      setTarget(payload.total === 0 ? 100 : Math.round((payload.done / payload.total) * 100));
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (working) {
      start.current = performance.now();
      setShown(0);
      setTarget(0);
      setVisible(true);
    }
  }, [working]);

  useEffect(() => {
    if (!visible) return;
    let raf = 0;
    const tick = () => {
      const cap = ((performance.now() - start.current) / MIN_MS) * 100;
      const goal = working ? Math.min(target, 99) : 100; // 100 % erst, wenn wirklich fertig
      setShown((s) => {
        const next = Math.min(goal, cap, s + 2.5);
        return next > s ? next : s;
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [visible, target, working]);

  // Nach 100 % kurz stehen lassen, dann ausblenden
  useEffect(() => {
    if (!working && shown >= 100 && visible) {
      const id = setTimeout(() => setVisible(false), 450);
      return () => clearTimeout(id);
    }
  }, [working, shown, visible]);

  return { active: visible, percent: Math.floor(shown), step, mode };
}
