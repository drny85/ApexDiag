import { useState } from "react";
import { Activity, AlertTriangle, BatteryMedium, Cable, Cpu, Gauge, History, Power, Wrench, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { useDiag } from "@/lib/store";
import { fmt } from "@/lib/format";
import Connect from "@/pages/Connect";
import Dashboard from "@/pages/Dashboard";
import Codes from "@/pages/Codes";
import Modules from "@/pages/Modules";
import Functions from "@/pages/Functions";
import ActivityPage from "@/pages/Activity";

export type PageId = "connect" | "dashboard" | "codes" | "modules" | "functions" | "activity";

const NAV: { id: PageId; label: string; icon: LucideIcon }[] = [
  { id: "connect", label: "Connect", icon: Cable },
  { id: "dashboard", label: "Live Data", icon: Activity },
  { id: "codes", label: "Fault Codes", icon: AlertTriangle },
  { id: "modules", label: "Modules", icon: Cpu },
  { id: "functions", label: "Special Functions", icon: Wrench },
  { id: "activity", label: "Activity", icon: History },
];

export default function App() {
  const [page, setPage] = useState<PageId>("connect");
  const { connection } = useDiag();

  return (
    <div className="flex h-screen">
      <aside className="flex w-60 shrink-0 flex-col border-r border-white/[0.06] bg-black/20 px-3 pb-3 pt-5">
        <div className="mb-6 flex items-center gap-2.5 px-2">
          <div className="grid size-8 place-items-center rounded-lg bg-gradient-to-br from-cyan-400 to-blue-600 shadow-lg shadow-cyan-500/20">
            <Gauge className="size-4 text-white" />
          </div>
          <div>
            <div className="text-[15px] font-semibold leading-none tracking-tight">ApexDiag</div>
            <div className="mt-1 text-[10px] uppercase tracking-[0.18em] text-muted-foreground">Diagnostics</div>
          </div>
        </div>
        <nav className="flex flex-col gap-0.5">
          {NAV.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setPage(id)}
              className={cn(
                "relative flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition",
                page === id ? "bg-white/[0.07] text-foreground" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground",
              )}
            >
              {page === id && <span className="absolute inset-y-2 left-0 w-0.5 rounded-full bg-cyan-400" />}
              <Icon className={cn("size-4", page === id && "text-cyan-300")} />
              {label}
            </button>
          ))}
        </nav>
        <ConnectionCard onClick={() => setPage("connect")} />
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar />
        <main className="flex-1 overflow-auto px-8 pb-10 pt-6">
          <div className="mx-auto max-w-[1400px]" key={page + (connection ? "c" : "")}>
            {page === "connect" && <Connect go={setPage} />}
            {page === "dashboard" && <Dashboard go={setPage} />}
            {page === "codes" && <Codes go={setPage} />}
            {page === "modules" && <Modules go={setPage} />}
            {page === "functions" && <Functions />}
            {page === "activity" && <ActivityPage />}
          </div>
        </main>
      </div>
    </div>
  );
}

function ConnectionCard({ onClick }: { onClick: () => void }) {
  const { connection } = useDiag();
  return (
    <button
      onClick={onClick}
      className="mt-auto rounded-xl border border-white/[0.07] bg-white/[0.03] p-3 text-left transition hover:border-white/15"
    >
      <div className="flex items-center gap-2 text-xs">
        <span className={cn("relative size-2 rounded-full", connection ? "bg-emerald-400" : "bg-white/25")}>
          {connection && <span className="absolute inset-0 animate-ping rounded-full bg-emerald-400/60" />}
        </span>
        <span className="font-medium">{connection ? "Connected" : "Not connected"}</span>
      </div>
      {connection && (
        <>
          <div className="mt-2 truncate text-xs text-muted-foreground">{connection.device.name}</div>
          <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground/80">{connection.vin}</div>
        </>
      )}
    </button>
  );
}

function StatusBar() {
  const { connection, live, vehicle } = useDiag();
  const rpm = live.get(0x0c);
  const speed = live.get(0x0d);
  const volts = live.get(0x42) ?? vehicle?.state.battery_v ?? undefined;
  const running = rpm !== undefined ? rpm > 300 : undefined;
  return (
    <div className="flex h-12 shrink-0 items-center justify-end gap-2 border-b border-white/[0.06] px-6" data-tauri-drag-region>
      {connection?.simulated && (
        <span className="mr-auto rounded-md bg-amber-400/10 px-2 py-1 text-[11px] font-medium text-amber-300 ring-1 ring-inset ring-amber-400/25">
          DEMO CAR · simulated data
        </span>
      )}
      {connection ? (
        <>
          <Chip icon={Power} label="Engine" value={running === undefined ? "--" : running ? "Running" : "Off"} tone={running ? "info" : "muted"} />
          <Chip icon={Gauge} label="Speed" value={`${fmt(speed)} km/h`} tone={speed === 0 ? "ok" : "warn"} />
          <Chip icon={BatteryMedium} label="Battery" value={`${fmt(volts, 1)} V`} tone={volts === undefined ? "muted" : volts < 12.2 ? "warn" : "ok"} />
        </>
      ) : (
        <span className="text-xs text-muted-foreground">No vehicle</span>
      )}
    </div>
  );
}

function Chip({ icon: Icon, label, value, tone }: { icon: LucideIcon; label: string; value: string; tone: "ok" | "warn" | "info" | "muted" }) {
  return (
    <div className="flex items-center gap-2 rounded-lg bg-white/[0.04] px-2.5 py-1 text-xs ring-1 ring-inset ring-white/[0.06]">
      <Icon
        className={cn(
          "size-3.5",
          tone === "ok" && "text-emerald-400",
          tone === "warn" && "text-amber-300",
          tone === "info" && "text-cyan-300",
          tone === "muted" && "text-muted-foreground",
        )}
      />
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}
