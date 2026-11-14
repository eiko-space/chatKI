import { useEffect, useState } from "react";
import { Route, Routes } from "react-router";
import { Sidebar } from "./components/layout/Sidebar";
import { getAppInfo, type AppInfo } from "./lib/tauri";
import { TelemetryProvider } from "./lib/telemetry";
import { AppStateProvider } from "./lib/app-state";
import { MeasureProvider } from "./lib/measure";
import { AuthProvider, useAuth } from "./lib/auth";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { isTauri } from "./lib/tauri";
import Dashboard from "./pages/Dashboard";
import Boost from "./pages/Boost";
import Network from "./pages/Network";
import PcCheck from "./pages/PcCheck";
import HistoryPage from "./pages/History";
import Profiles from "./pages/Profiles";
import SettingsPage from "./pages/Settings";
import Account from "./pages/Account";

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    getAppInfo().then(setInfo).catch((e) => console.error("get_app_info fehlgeschlagen:", e));
  }, []);

  return (
    <TelemetryProvider>
    <AppStateProvider>
    <MeasureProvider>
    <AuthProvider>
    <Gate>
    <div className="flex h-full">
      <Sidebar version={info?.version ?? null} />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-6xl px-8 py-8">
          <Routes>
            <Route path="/" element={<Dashboard info={info} />} />
            <Route path="/boost" element={<Boost info={info} />} />
            <Route path="/netzwerk" element={<Network />} />
            <Route path="/pc-check" element={<PcCheck info={info} />} />
            <Route path="/verlauf" element={<HistoryPage />} />
            <Route path="/profile" element={<Profiles />} />
            <Route path="/einstellungen" element={<SettingsPage info={info} />} />
            <Route path="/konto" element={<Account />} />
          </Routes>
        </div>
      </main>
    </div>
    </Gate>
    </AuthProvider>
    </MeasureProvider>
    </AppStateProvider>
    </TelemetryProvider>
  );
}

/**
 * „Nur mit Einladung“: Ohne Anmeldung ist nur der Login sichtbar.
 * Auch Tray-Boost und Auto-Boost im Hintergrund bleiben bis zur Anmeldung gesperrt.
 */
function Gate({ children }: { children: React.ReactNode }) {
  const auth = useAuth();
  const locked = auth.inviteOnly && !auth.session;

  useEffect(() => {
    if (isTauri()) invoke("set_locked", { locked }).catch(() => {});
  }, [locked]);

  if (!auth.inviteOnly) return <>{children}</>;
  if (!auth.ready)
    return (
      <div className="grid h-full place-items-center text-sm text-muted">
        <span className="inline-flex items-center gap-2"><Loader2 size={14} className="animate-spin" /> Lade …</span>
      </div>
    );
  if (locked)
    return (
      <div className="h-full overflow-y-auto px-8 py-16">
        <Account gate />
      </div>
    );
  return <>{children}</>;
}
