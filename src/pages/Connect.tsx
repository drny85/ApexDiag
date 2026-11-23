import { useEffect, useRef, useState } from "react";
import { ArrowRight, Bluetooth, Car, CheckCircle2, CircleAlert, Gauge as GaugeIcon, Loader2, Plug, Power, ScanSearch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import PageHeader from "@/components/PageHeader";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type DiscoveredDevice } from "@/lib/tauri";
import { fmt } from "@/lib/format";
import { cn } from "@/lib/utils";
import { HandshakeFlow, ObdPortScene } from "@/components/illustrations/Connect";
import type { PageId } from "@/App";

export default function Connect({ go }: { go: (p: PageId) => void }) {
  const { connection, setConnection } = useDiag();
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lines, setLines] = useState<string[]>([]);
  const [step, setStep] = useState(0);
  const consoleRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.listDevices().then(setDevices).catch((e) => setError(asCmdError(e).message));
  }, []);

  useEffect(() => {
    consoleRef.current?.scrollTo({ top: consoleRef.current.scrollHeight });
  }, [lines]);

  const connect = async (d: DiscoveredDevice) => {
    setBusy(d.id);
    setError(null);
    setLines([`Opening ${d.name}…`]);
    setStep(0);
    try {
      const report = await api.connect(d.id);
      // Replay the adapter handshake so the user sees what happened, lighting
      // up the diagram as each stage is reached.
      for (const l of report.transcript.slice(0, 24)) {
        setLines((x) => [...x, `→ ${l}`]);
        if (/^AT\s*Z|^ATI/.test(l)) setStep((s) => Math.max(s, 1));
        if (/^AT\s*SP/.test(l)) setStep((s) => Math.max(s, 2));
        await new Promise((r) => setTimeout(r, 70));
      }
      setStep(3);
      setLines((x) => [...x, `✓ ${report.adapter.version} ready · VIN ${report.vin ?? "unknown"}`]);
      setConnection(report);
    } catch (e) {
      const err = asCmdError(e);
      setStep(0);
      setError(err.message);
      setLines((x) => [...x, `✗ ${err.message}`]);
    } finally {
      setBusy(null);
    }
  };

  const disconnect = async () => {
    await api.disconnect().catch(() => {});
    setConnection(null);
    setLines([]);
    setStep(0);
  };

  return (
    <div>
      <PageHeader
        title="Connect"
        subtitle="Pick an adapter. ApexDiag runs the adapter handshake, reads the VIN, and checks what the car supports."
        actions={connection && <Button variant="outline" onClick={disconnect}><Power /> Disconnect</Button>}
      />

      {connection ? (
        <VehicleSummary go={go} />
      ) : (
        <div className="grid gap-3 md:grid-cols-3">
          {devices.map((d) => {
            const sim = d.transport.kind === "simulator";
            const Icon = sim ? Car : Bluetooth;
            return (
              <div
                key={d.id}
                className={cn(
                  "group relative flex flex-col rounded-2xl border border-white/[0.07] bg-gradient-to-b from-white/[0.04] to-transparent p-5 transition",
                  d.available ? "hover:border-cyan-400/30 hover:from-cyan-400/[0.06]" : "opacity-60",
                )}
              >
                <div className="mb-4 flex items-center justify-between">
                  <div className="grid size-10 place-items-center rounded-xl bg-white/5 ring-1 ring-white/10">
                    <Icon className="size-5 text-cyan-300" />
                  </div>
                  {sim ? <Badge variant="warn">Demo data</Badge> : <Badge variant="muted">Next step</Badge>}
                </div>
                <div className="font-medium">{d.name}</div>
                <p className="mt-1 flex-1 text-sm text-muted-foreground">{d.detail}</p>
                <Button className="mt-5" disabled={!d.available || busy !== null} onClick={() => connect(d)}>
                  {busy === d.id ? <Loader2 className="animate-spin" /> : <Plug />}
                  {busy === d.id ? "Connecting" : "Connect"}
                </Button>
              </div>
            );
          })}
        </div>
      )}

      {!connection && !busy && lines.length === 0 && <PlugInGuide />}

      {(lines.length > 0 || error) && (
        <div className="mt-6 overflow-hidden rounded-2xl border border-white/[0.07] bg-black/30">
          <div className="border-b border-white/[0.06] px-6 pb-2 pt-4">
            <HandshakeFlow step={connection ? 3 : step} className="mx-auto h-24 w-full max-w-xl" />
          </div>
          <div className="flex items-center gap-2 border-b border-white/[0.06] px-4 py-2 text-xs text-muted-foreground">
            <span className="size-2 rounded-full bg-cyan-400/80" /> Adapter conversation
          </div>
          <div ref={consoleRef} className="max-h-56 overflow-auto px-4 py-3 font-mono text-[12px] leading-relaxed text-cyan-100/80">
            {lines.map((l, i) => (
              <div key={i} className={cn(l.startsWith("✗") && "text-rose-300", l.startsWith("✓") && "text-emerald-300")}>
                {l}
              </div>
            ))}
          </div>
        </div>
      )}
      {error && !lines.length && <p className="mt-4 text-sm text-rose-300">{error}</p>}
    </div>
  );
}

function PlugInGuide() {
  const steps = [
    ["Ignition on", "Press Start once without your foot on the brake. The engine doesn't need to run."],
    ["Plug in the adapter", "Driver footwell, under the dashboard to the left of the steering column, near the hood release."],
    ["Wait for its light", "The adapter's LED comes on when it has power from the car."],
    ["Pick it above", "ApexDiag finds it over Bluetooth and does the rest."],
  ];
  return (
    <section className="mt-6 grid items-center gap-6 overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-white/[0.03] to-transparent p-6 lg:grid-cols-[1.1fr_1fr]">
      <ObdPortScene className="w-full max-w-lg" />
      <div>
        <h3 className="text-sm font-semibold">Plugging in on the X5</h3>
        <ol className="mt-3 space-y-3">
          {steps.map(([t, d], i) => (
            <li key={t} className="flex gap-3">
              <span className="grid size-6 shrink-0 place-items-center rounded-full bg-cyan-400/10 text-xs font-semibold text-cyan-200 ring-1 ring-cyan-400/25">
                {i + 1}
              </span>
              <div>
                <div className="text-sm font-medium">{t}</div>
                <p className="text-xs leading-relaxed text-muted-foreground">{d}</p>
              </div>
            </li>
          ))}
        </ol>
        <p className="mt-4 text-[11px] text-muted-foreground">The drawing shows the general area, not your exact trim.</p>
      </div>
    </section>
  );
}

function VehicleSummary({ go }: { go: (p: PageId) => void }) {
  const { connection, live } = useDiag();
  if (!connection) return null;
  const c = connection;
  const facts: [string, React.ReactNode][] = [
    ["Adapter", c.adapter.version],
    ["Protocol", "ISO 15765-4 CAN · 11-bit · 500 kbit/s"],
    ["BMW module access", c.adapter.supports_extended_addressing ? "Supported (extended addressing)" : "Not supported by this adapter"],
    ["Live parameters", `${c.supported_pids.length} supported`],
    ["Battery", `${fmt(live.get(0x42) ?? c.battery_v ?? undefined, 2)} V`],
  ];
  return (
    <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
      <div className="relative overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-cyan-500/[0.08] via-transparent to-transparent p-6">
        <div className="absolute -right-24 -top-24 size-72 rounded-full bg-cyan-400/10 blur-3xl" />
        <div className="relative">
          <div className="flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-muted-foreground">
            <CheckCircle2 className="size-3.5 text-emerald-400" /> Connected {c.simulated && <Badge variant="warn" className="ml-1">Demo data</Badge>}
          </div>
          <div className="mt-3 text-2xl font-semibold">{c.simulated ? "BMW X5 (E70) · 2008" : vehicleTitle(c.vin)}</div>
          <div className="mt-1 font-mono text-sm tracking-wider text-cyan-200/80">VIN {c.vin ?? "unavailable"}</div>
          <dl className="mt-6 grid grid-cols-2 gap-x-6 gap-y-4 text-sm">
            {facts.map(([k, v]) => (
              <div key={k}>
                <dt className="text-xs text-muted-foreground">{k}</dt>
                <dd className="mt-0.5 font-medium">{v}</dd>
              </div>
            ))}
          </dl>
        </div>
      </div>
      <div className="flex flex-col gap-4">
        <div
          className={cn(
            "rounded-2xl border p-5",
            c.mil?.mil_on ? "border-amber-400/25 bg-amber-400/[0.06]" : "border-emerald-400/20 bg-emerald-400/[0.05]",
          )}
        >
          <div className="flex items-center gap-2 font-medium">
            <CircleAlert className={cn("size-4", c.mil?.mil_on ? "text-amber-300" : "text-emerald-300")} />
            {c.mil?.mil_on ? "Check-engine light is on" : "Check-engine light is off"}
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            {c.mil?.mil_on
              ? `${c.mil.dtc_count} confirmed emissions code${c.mil.dtc_count === 1 ? "" : "s"} stored in the engine module.`
              : "No confirmed emissions faults."}
          </p>
          <Button variant="secondary" size="sm" className="mt-4" onClick={() => go("codes")}>
            Review fault codes <ArrowRight />
          </Button>
        </div>
        <div className="rounded-2xl border border-white/[0.07] bg-white/[0.02] px-4 pb-2 pt-3">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Link</span>
            <span className="flex items-center gap-1.5">
              <span className="size-1.5 animate-pulse rounded-full bg-cyan-300" /> keep-alive every 2 s
            </span>
          </div>
          <HandshakeFlow step={3} className="mx-auto mt-1 h-20 w-full" />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <QuickAction icon={GaugeIcon} label="Live dashboard" onClick={() => go("dashboard")} />
          <QuickAction icon={ScanSearch} label="Scan all modules" onClick={() => go("modules")} />
        </div>
      </div>
    </div>
  );
}

function QuickAction({ icon: Icon, label, onClick }: { icon: typeof Car; label: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-3 rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4 text-left text-sm font-medium transition hover:border-cyan-400/30 hover:bg-cyan-400/[0.05]"
    >
      <Icon className="size-4 text-cyan-300" />
      {label}
    </button>
  );
}

/** Model year from VIN position 10 (1980-2009 / 2010-2039 cycle, best guess for current cars). */
function vehicleTitle(vin: string | null) {
  const codes = "ABCDEFGHJKLMNPRSTVWXY123456789";
  const ch = vin?.[9];
  const idx = ch ? codes.indexOf(ch) : -1;
  if (idx < 0) return "Connected vehicle";
  const year = 1980 + idx + (1980 + idx + 30 <= new Date().getFullYear() + 1 ? 30 : 0);
  const maker = vin?.startsWith("WBA") || vin?.startsWith("5UX") || vin?.startsWith("WBS") ? "BMW" : "Vehicle";
  return `${maker} · ${year}`;
}
