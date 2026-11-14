import { useEffect, useState } from "react";
import { api, type OverlaySettings } from "./lib/api";
import { TelemetryProvider, useTelemetry } from "./lib/telemetry";
import { listen } from "@tauri-apps/api/event";
import { num } from "./lib/format";

function Row({ label, value, unit, bad }: { label: string; value: string | null; unit: string; bad?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-[0.7em] font-semibold uppercase tracking-wider text-white/60">{label}</span>
      <span className={`font-semibold tabular-nums ${bad ? "text-bad" : "text-white"}`}>
        {value ?? "—"}
        {value && <span className="ml-0.5 text-[0.65em] font-medium text-white/60">{unit}</span>}
      </span>
    </div>
  );
}

function Content() {
  const { snapshot } = useTelemetry();
  const [s, setS] = useState<OverlaySettings | null>(null);

  useEffect(() => {
    const load = () => api.getSettings().then((x) => setS(x.overlay)).catch(() => {});
    load();
    const un = listen("settings-changed", load);
    return () => {
      un.then((f) => f());
    };
  }, []);

  if (!s) return null;
  const fps = snapshot?.fps;
  const ping = snapshot?.network.internet;
  const temp = snapshot?.gpu.temperature ?? null;
  return (
    <div className="h-full w-full p-1" style={{ fontSize: `${14 * s.scale}px` }}>
      <div className="flex h-full flex-col justify-center gap-0.5 rounded-xl border border-white/10 bg-black/55 px-3 py-2 backdrop-blur-sm">
        {s.showFps && <Row label="FPS" value={num(fps?.current)} unit="" />}
        {s.showFps && fps?.low1 != null && <Row label="1 % Low" value={num(fps.low1)} unit="" />}
        {s.showPing && <Row label="Ping" value={num(ping?.lastMs)} unit="ms" bad={(ping?.lastMs ?? 0) > 80} />}
        {s.showTemp && <Row label="GPU" value={num(temp)} unit="°C" bad={(temp ?? 0) >= 85} />}
      </div>
    </div>
  );
}

/** Kleines, durchsichtiges Overlay-Fenster (wird nur im Overlay-Fenster gerendert). */
export default function OverlayApp() {
  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  }, []);
  return (
    <TelemetryProvider>
      <Content />
    </TelemetryProvider>
  );
}
