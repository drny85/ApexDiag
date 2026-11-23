import { useState } from "react";
import { Activity, AlertTriangle, Cable, Wrench, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import ConnectionManager from "@/pages/ConnectionManager";
import Dashboard from "@/pages/Dashboard";
import DtcScanner from "@/pages/DtcScanner";
import SpecialFunctions from "@/pages/SpecialFunctions";

type PageId = "connection" | "dashboard" | "dtc" | "special";

const PAGES: { id: PageId; label: string; icon: LucideIcon; component: () => React.JSX.Element }[] = [
  { id: "connection", label: "Connection", icon: Cable, component: ConnectionManager },
  { id: "dashboard", label: "Live Data", icon: Activity, component: Dashboard },
  { id: "dtc", label: "Trouble Codes", icon: AlertTriangle, component: DtcScanner },
  { id: "special", label: "Special Functions", icon: Wrench, component: SpecialFunctions },
];

export default function App() {
  const [active, setActive] = useState<PageId>("connection");
  const Page = PAGES.find((p) => p.id === active)!.component;

  return (
    <div className="flex h-screen">
      <aside className="w-56 shrink-0 border-r bg-card/40 p-3 flex flex-col gap-1">
        <div className="px-2 py-3 text-lg font-semibold tracking-tight">ApexDiag</div>
        {PAGES.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActive(id)}
            className={cn(
              "flex items-center gap-2 rounded-md px-2 py-2 text-sm text-left transition-colors",
              active === id ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent/60",
            )}
          >
            <Icon className="size-4" />
            {label}
          </button>
        ))}
        <div className="mt-auto px-2 text-xs text-muted-foreground">Not connected</div>
      </aside>
      <main className="flex-1 overflow-auto p-6">
        <Page />
      </main>
    </div>
  );
}
