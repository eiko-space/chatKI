import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "./tauri";

// Typen spiegeln die Rust-Strukturen in src-tauri/src/telemetry wider.
export interface CaptureStatus {
  state: "idle" | "running" | "missingBinary" | "error";
  message: string | null;
}

export interface FpsStats {
  app: string;
  game: string | null;
  pid: number;
  current: number;
  average: number;
  low1: number | null;
  low01: number | null;
  frametimeAvgMs: number;
  frametimeStdDevMs: number;
  frameCount: number;
  windowSecs: number;
  inputLatencyMs: number | null;
  pcLatencyMs: number | null;
}

export interface PingStats {
  label: string;
  target: string | null;
  lastMs: number | null;
  averageMs: number | null;
  jitterMs: number | null;
  lossPercent: number | null;
  spikes: number | null;
  spikeThresholdMs: number | null;
  samples: number;
  windowSecs: number;
}

export interface Snapshot {
  capture: CaptureStatus;
  fps: FpsStats | null;
  frametimes: number[];
  system: { cpuUsage: number | null; ramUsage: number | null; ramUsedGb: number | null; ramTotalGb: number | null };
  gpu: {
    name: string | null;
    usage: number | null;
    vramUsedGb: number | null;
    vramTotalGb: number | null;
    vramUsage: number | null;
    temperature: number | null;
  };
  cpuTemperature: number | null;
  network: {
    internet: PingStats;
    router: PingStats;
    connection: "lan" | "wlan" | "other" | null;
    adapter: string | null;
    downloadMbps: number | null;
    uploadMbps: number | null;
  };
}

/** Wie viele Frametimes der Graph maximal vorhält. */
const MAX_FRAMES = 1200;

interface TelemetryValue {
  snapshot: Snapshot | null;
  /** Ringpuffer der letzten Frametimes (ms) des aktuell gemessenen Programms. */
  frametimes: number[];
}

const TelemetryContext = createContext<TelemetryValue>({ snapshot: null, frametimes: [] });

export function TelemetryProvider({ children }: { children: ReactNode }) {
  const [value, setValue] = useState<TelemetryValue>({ snapshot: null, frametimes: [] });
  const buffer = useRef<number[]>([]);
  const lastPid = useRef<number | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<Snapshot>("telemetry", ({ payload }) => {
      const pid = payload.fps?.pid ?? null;
      if (pid !== lastPid.current) {
        buffer.current = []; // anderes Programm → Graph neu beginnen
        lastPid.current = pid;
      }
      if (payload.frametimes.length) {
        const next = buffer.current.concat(payload.frametimes);
        buffer.current = next.length > MAX_FRAMES ? next.slice(next.length - MAX_FRAMES) : next;
      }
      setValue({ snapshot: payload, frametimes: buffer.current });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return <TelemetryContext.Provider value={value}>{children}</TelemetryContext.Provider>;
}

export const useTelemetry = () => useContext(TelemetryContext);
