import { Activity, Pause, Play, Plug } from "lucide-react";
import { useState } from "react";
import Gauge from "@/components/Gauge";
import Sparkline from "@/components/Sparkline";
import InfoTip from "@/components/InfoTip";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { Button } from "@/components/ui/button";
import { useDiag } from "@/lib/store";
import { api } from "@/lib/tauri";
import { digitsFor, fmt } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

const TRENDS: { pid: number; color: string; band?: [number, number] }[] = [
  { pid: 0x06, color: "#22d3ee", band: [-10, 10] },
  { pid: 0x07, color: "#a78bfa", band: [-10, 10] },
  { pid: 0x08, color: "#22d3ee", band: [-10, 10] },
  { pid: 0x09, color: "#a78bfa", band: [-10, 10] },
  { pid: 0x0b, color: "#34d399" },
  { pid: 0x10, color: "#fbbf24" },
  { pid: 0x0e, color: "#f472b6" },
  { pid: 0x14, color: "#60a5fa" },
];

export default function Dashboard({ go }: { go: (p: PageId) => void }) {
  const { connection, live, catalog, vehicle, bump } = useDiag();
  const [toggling, setToggling] = useState(false);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Live Data" subtitle="Real-time engine readings, refreshed several times a second." />
        <Empty icon={Activity} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> Connect
          </Button>
        </Empty>
      </div>
    );
  }

  const info = (pid: number) => catalog?.pids.find((p) => p.pid === pid);
  const running = vehicle?.engine_running ?? (live.get(0x0c) ?? 0) > 300;

  const toggleEngine = async () => {
    setToggling(true);
    await api.simSetEngine(!running).catch(() => {});
    bump();
    setTimeout(() => setToggling(false), 600);
  };

  return (
    <div>
      <PageHeader
        title="Live Data"
        subtitle="Real-time readings from the engine module. Hover the ⓘ on any value to see what it means and what's normal."
        actions={
          connection.simulated && (
            <Button variant="outline" onClick={toggleEngine} disabled={toggling}>
              {running ? <Pause /> : <Play />} {running ? "Stop demo engine" : "Start demo engine"}
            </Button>
          )
        }
      />

      <div className="grid grid-cols-2 gap-4 xl:grid-cols-4">
        <GaugeCard pid={0x0c} explain={info(0x0c)?.explain}>
          <Gauge label="Engine" unit="rpm" value={live.get(0x0c)} min={0} max={7000} redFrom={6500} majorStep={1000} labelScale={1000} />
        </GaugeCard>
        <GaugeCard pid={0x0d} explain={info(0x0d)?.explain}>
          <Gauge label="Speed" unit="km/h" value={live.get(0x0d)} min={0} max={240} majorStep={40} />
        </GaugeCard>
        <GaugeCard pid={0x05} explain={info(0x05)?.explain}>
          <Gauge label="Coolant" unit="°C" value={live.get(0x05)} min={40} max={130} warnFrom={112} redFrom={118} lowBelow={75} majorStep={15} />
        </GaugeCard>
        <GaugeCard pid={0x42} explain={info(0x42)?.explain}>
          <Gauge label="System" unit="volts" value={live.get(0x42)} min={10} max={16} lowBelow={12.2} warnFrom={14.9} redFrom={15.3} majorStep={1} digits={2} />
        </GaugeCard>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
        {TRENDS.map(({ pid, color, band }) => {
          const p = info(pid);
          if (!p) return null;
          const v = live.get(pid);
          const outOfBand = band && v !== undefined && (v < band[0] || v > band[1]);
          return (
            <div key={pid} className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
              <div className="flex items-center justify-between gap-2">
                <div className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate text-xs font-medium text-muted-foreground">{p.name}</span>
                  <InfoTip text={p.explain} />
                </div>
                <span className="size-1.5 shrink-0 rounded-full" style={{ background: color }} />
              </div>
              <div className="mt-1 flex items-baseline gap-1.5">
                <span className={cn("text-2xl font-semibold tabular-nums tracking-tight", outOfBand && "text-amber-300")}>
                  {v !== undefined && v > 0 && p.unit === "%" && band ? "+" : ""}
                  {fmt(v, digitsFor(p.unit))}
                </span>
                <span className="text-xs text-muted-foreground">{p.unit}</span>
              </div>
              <div className="mt-2">
                <Sparkline pid={pid} color={color} band={band} />
              </div>
            </div>
          );
        })}
      </div>

      <AllValues />
    </div>
  );
}

function GaugeCard({ pid, explain, children }: { pid: number; explain?: string; children: React.ReactNode }) {
  return (
    <div className="relative rounded-2xl border border-white/[0.07] bg-[radial-gradient(ellipse_at_top,rgba(34,211,238,0.07),transparent_60%)] p-3" data-pid={pid}>
      {explain && (
        <div className="absolute right-3 top-3 z-10">
          <InfoTip text={explain} />
        </div>
      )}
      {children}
    </div>
  );
}

function AllValues() {
  const { live, catalog } = useDiag();
  const rows = catalog?.pids.filter((p) => live.has(p.pid)) ?? [];
  return (
    <div className="mt-4 overflow-hidden rounded-2xl border border-white/[0.07]">
      <div className="border-b border-white/[0.06] bg-white/[0.02] px-4 py-2.5 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
        All streamed values
      </div>
      <div className="divide-y divide-white/[0.05]">
        {rows.map((p) => (
          <div key={p.pid} className="grid grid-cols-[1fr_auto] items-center gap-4 px-4 py-2.5 text-sm md:grid-cols-[220px_110px_1fr]">
            <div className="font-medium">{p.name}</div>
            <div className="text-right font-mono tabular-nums md:text-left">
              {fmt(live.get(p.pid), digitsFor(p.unit))} <span className="text-muted-foreground">{p.unit}</span>
            </div>
            <div className="col-span-2 text-xs text-muted-foreground md:col-span-1">{p.explain}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
