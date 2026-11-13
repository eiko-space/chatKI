import {
  Activity, Gauge, Wifi, ShieldCheck, History, Gamepad2, Settings, UserRound,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  path: string;
  label: string;
  icon: LucideIcon;
}

export const NAV_ITEMS: NavItem[] = [
  { path: "/", label: "Dashboard", icon: Activity },
  { path: "/boost", label: "Boost", icon: Gauge },
  { path: "/netzwerk", label: "Netzwerk", icon: Wifi },
  { path: "/pc-check", label: "PC-Check", icon: ShieldCheck },
  { path: "/verlauf", label: "Verlauf", icon: History },
  { path: "/profile", label: "Profile", icon: Gamepad2 },
  { path: "/einstellungen", label: "Einstellungen", icon: Settings },
  { path: "/konto", label: "Konto", icon: UserRound },
];
