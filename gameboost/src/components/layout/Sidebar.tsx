import { NavLink } from "react-router";
import { Zap } from "lucide-react";
import { NAV_ITEMS } from "../../lib/nav";
import { cn } from "../../lib/cn";

export function Sidebar({ version }: { version: string | null }) {
  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-line bg-surface">
      <div className="flex items-center gap-2.5 px-5 py-5">
        <div className="grid size-8 place-items-center rounded-lg bg-accent text-bg">
          <Zap size={18} strokeWidth={2.5} />
        </div>
        <span className="text-[15px] font-semibold tracking-tight">GameBoost</span>
      </div>

      <nav className="flex-1 space-y-0.5 px-3">
        {NAV_ITEMS.map(({ path, label, icon: Icon }) => (
          <NavLink
            key={path}
            to={path}
            end={path === "/"}
            className={({ isActive }) =>
              cn(
                "group relative flex h-10 items-center gap-3 rounded-lg px-3 text-sm font-medium",
                "transition-colors duration-150",
                isActive ? "bg-surface-3 text-text" : "text-muted hover:bg-surface-2 hover:text-text",
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && <span className="absolute left-0 top-2.5 h-5 w-[3px] rounded-r bg-accent" />}
                <Icon size={18} className={isActive ? "text-accent" : ""} />
                {label}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="px-5 py-4 text-[11px] text-faint">
        {version ? `Version ${version}` : "Browser-Vorschau"}
      </div>
    </aside>
  );
}
