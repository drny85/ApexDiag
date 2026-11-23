import { useEffect, useRef, useState } from "react";
import { ArrowRight, BadgeCheck, Bluetooth, Car, CheckCircle2, CircleAlert, Gauge as GaugeIcon, Loader2, Plug, Power, Radar, ScanSearch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import PageHeader from "@/components/PageHeader";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type DiscoveredDevice } from "@/lib/tauri";
import { fmt } from "@/lib/format";
import { cn } from "@/lib/utils";
import { HandshakeFlow, ObdPortScene } from "@/components/illustrations/Connect";
import { BrandChip, CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import type { Pack, VinInfo } from "@/lib/tauri";
import type { PageId } from "@/App";

export default function Connect({ go }: { go: (p: PageId) => void }) {
  const { connection, setConnection, catalog } = useDiag();
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lines, setLines] = useState<string[]>([]);
  const [step, setStep] = useState(0);
  const consoleRef = useRef<HTMLDivElement>(null);
  const [scanning, setScanning] = useState(false);
  const [scanned, setScanned] = useState(false);
  const [showAll, setShowAll] = useState(false);

  const scanBle = async (all = showAll) => {
    setScanning(true);
    setError(null);
    try {
      await api.scanBle(all);
      setDevices(await api.listDevices());
      setScanned(true);
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setScanning(false);
    }
  };

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
        <>
          <div className="flex flex-wrap items-end justify-between gap-3">
            <SectionTitle title="Adapters" hint="Your own adapter, plugged into the car with the ignition on." />
            <div className="mb-3 flex items-center gap-2">
              <label className="flex min-h-11 cursor-pointer items-center gap-2 px-2 text-xs text-muted-foreground">
                <input
                  type="checkbox"
                  className="size-4 accent-cyan-400"
                  checked={showAll}
                  onChange={(e) => setShowAll(e.target.checked)}
                />
                Show all Bluetooth devices
              </label>
              <Button variant="outline" disabled={scanning || busy !== null} onClick={() => scanBle()}>
                {scanning ? <Loader2 className="animate-spin" /> : <Radar />}
                {scanning ? "Scanning" : scanned ? "Scan again" : "Scan Bluetooth"}
              </Button>
            </div>
          </div>
          <div className="mb-7 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {devices
              .filter((d) => d.transport.kind !== "simulator")
              .map((d) => (
                <div key={d.id} className={cn("flex flex-col rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5", !d.available && "border-dashed")}>
                  <div className="mb-4 flex items-center justify-between">
                    <div className="grid size-10 place-items-center rounded-xl bg-white/5 ring-1 ring-white/10">
                      <Bluetooth className="size-5 text-cyan-300" />
                    </div>
                    <Badge variant={d.available ? "default" : "muted"}>{d.available ? "Found" : "Not scanned"}</Badge>
                  </div>
                  <div className="font-medium">{d.name}</div>
                  <p className="mt-1 flex-1 text-sm text-muted-foreground">{d.detail}</p>
                  {d.available ? (
                    <Button className="mt-5" disabled={busy !== null || scanning} onClick={() => connect(d)}>
                      {busy === d.id ? <Loader2 className="animate-spin" /> : <Plug />}
                      {busy === d.id ? "Connecting" : "Connect"}
                    </Button>
                  ) : (
                    <Button className="mt-5" variant="secondary" disabled={scanning || busy !== null} onClick={() => scanBle()}>
                      {scanning ? <Loader2 className="animate-spin" /> : <Radar />}
                      {scanning ? "Scanning for 4 seconds" : "Scan Bluetooth"}
                    </Button>
                  )}
                </div>
              ))}
            {scanned && !devices.some((d) => d.transport.kind === "ble" && d.available) && (
              <div className="rounded-2xl border border-amber-400/20 bg-amber-400/[0.04] p-5 text-sm text-amber-100/80">
                No OBD adapter answered. Check that the adapter's light is on and the ignition is on, then scan again.
                If it still does not show, tick "Show all Bluetooth devices". Do not pair it in iPad Settings; ApexDiag connects to it directly.
              </div>
            )}
          </div>
          <SectionTitle title="Practice cars" hint="Simulated cars with invented faults, one per vehicle pack. Nothing here touches a real car." />
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {devices
              .filter((d) => d.transport.kind === "simulator")
              .map((d) => {
                const pack = catalog?.packs.find((p) => p.id === SIM_PACK[d.id]);
                const accent = pack ? MAKE_ACCENT[pack.make] : "#67e8f9";
                return (
                  <div
                    key={d.id}
                    className="group relative flex flex-col overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-b from-white/[0.04] to-transparent p-5 transition hover:border-white/20"
                  >
                    <div
                      className="absolute -right-10 -top-16 size-48 rounded-full opacity-0 blur-3xl transition group-hover:opacity-100"
                      style={{ backgroundColor: `${accent}1f` }}
                    />
                    <div className="relative flex items-center justify-between">
                      {pack ? <BrandChip make={pack.make} brand={pack.brand} /> : <Car className="size-4" />}
                      <Badge variant="warn">Demo</Badge>
                    </div>
                    {pack && <CarSide body={pack.body} accent={accent} scanning={busy === d.id} className="relative my-3 w-full" />}
                    <div className="relative font-medium">{d.name.replace(/^Demo /, "")}</div>
                    {pack && (
                      <div className="relative mt-0.5 text-[11px] text-muted-foreground">
                        {pack.years} · {pack.modules.length} modules · {pack.make === "bmw" ? "KWP2000" : "UDS"}
                      </div>
                    )}
                    <p className="relative mt-2 flex-1 text-sm leading-snug text-muted-foreground">{d.detail.replace(/ Practice car, fake data\.$/, "")}</p>
                    <Button className="relative mt-4" variant="secondary" disabled={busy !== null} onClick={() => connect(d)}>
                      {busy === d.id ? <Loader2 className="animate-spin" /> : <Plug />}
                      {busy === d.id ? "Connecting" : "Connect"}
                    </Button>
                  </div>
                );
              })}
          </div>
        </>
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

/** Which pack each practice car uses. */
const SIM_PACK: Record<string, string> = {
  "sim:bmw_x5": "bmw_e70",
  "sim:mercedes_gle": "mercedes_w167",
  "sim:toyota_camry": "toyota_xv70",
  "sim:honda_accord": "honda_accord",
  "sim:jeep_wrangler": "jeep_jl",
};

function SectionTitle({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="mb-3 flex items-baseline gap-3">
      <h3 className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">{title}</h3>
      <span className="text-xs text-muted-foreground/70">{hint}</span>
    </div>
  );
}

function PlugInGuide() {
  const steps = [
    ["Ignition on", "Press Start once without your foot on the brake. The engine doesn't need to run."],
    ["Plug in the adapter", "Under the dashboard on the driver's side, usually within reach of your left knee. On many BMWs it sits by the hood release."],
    ["Wait for its light", "The adapter's LED comes on when it has power from the car."],
    ["Pick it above", "ApexDiag finds it over Bluetooth and does the rest."],
  ];
  return (
    <section className="mt-6 grid items-center gap-6 overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-white/[0.03] to-transparent p-6 lg:grid-cols-[1.1fr_1fr]">
      <ObdPortScene className="w-full max-w-lg" />
      <div>
        <h3 className="text-sm font-semibold">Plugging in</h3>
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
  const pack = c.vehicle.pack;
  const bmw = pack.make === "bmw";
  const facts: [string, React.ReactNode][] = [
    ["Adapter", c.adapter.version],
    ["Protocol", "ISO 15765-4 CAN · 11-bit · 500 kbit/s"],
    [
      "Module access",
      bmw
        ? c.adapter.supports_extended_addressing
          ? "KWP2000 over D-CAN (extended addressing)"
          : "Not supported by this adapter"
        : pack.make === "generic"
          ? "Generic OBD-II only"
          : "UDS (ISO 14229) on 11-bit CAN",
    ],
    ["Vehicle pack", pack.make === "generic" ? "None yet" : `${pack.brand} ${pack.chassis} · ${pack.layout === "documented" ? "documented addresses" : "demo layout"}`],
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
          <div className="mt-3 flex items-center gap-3">
            <BrandChip make={pack.make} brand={pack.brand} />
            <span className="text-xs text-muted-foreground">Works like {pack.dealer_tool}</span>
          </div>
          <div className="mt-2 text-2xl font-semibold">{vehicleTitle(pack, c.vehicle.vin_info)}</div>
          <div className="mt-1 font-mono text-sm tracking-wider text-cyan-200/80">VIN {c.vin ?? "unavailable"}</div>
          {c.vehicle.demo_story && <p className="mt-2 max-w-lg text-sm text-muted-foreground">{c.vehicle.demo_story}</p>}
          <dl className="mt-6 grid grid-cols-2 gap-x-6 gap-y-4 text-sm">
            {facts.map(([k, v]) => (
              <div key={k}>
                <dt className="text-xs text-muted-foreground">{k}</dt>
                <dd className="mt-0.5 font-medium">{v}</dd>
              </div>
            ))}
          </dl>
          {c.vehicle.vin_info && <VinCard info={c.vehicle.vin_info} />}
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
        <button
          onClick={() => go("quicktest")}
          className="group flex items-center gap-4 rounded-2xl border border-cyan-400/25 bg-gradient-to-r from-cyan-400/[0.12] to-transparent p-4 text-left transition hover:border-cyan-300/50"
        >
          <div className="grid size-10 place-items-center rounded-xl bg-cyan-400/15 ring-1 ring-cyan-300/30">
            <Radar className="size-5 text-cyan-200" />
          </div>
          <div className="flex-1">
            <div className="font-medium">Run a Quick Test</div>
            <div className="text-xs text-muted-foreground">Every module, every fault, emissions readiness. About a minute on a real car.</div>
          </div>
          <ArrowRight className="size-4 text-cyan-200 transition group-hover:translate-x-0.5" />
        </button>
        <div className="grid grid-cols-2 gap-3">
          <QuickAction icon={GaugeIcon} label="Live dashboard" onClick={() => go("dashboard")} />
          <QuickAction icon={ScanSearch} label="Modules" onClick={() => go("modules")} />
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

function vehicleTitle(pack: Pack, vin: VinInfo | null) {
  if (pack.make === "generic") return vin?.manufacturer ? `${vin.manufacturer} vehicle` : "OBD-II vehicle";
  const year = vin?.model_year ? ` · ${vin.model_year}` : "";
  return `${pack.brand} ${pack.model} (${pack.chassis})${year}`;
}

/** What the VIN itself says, decoded without a database. */
function VinCard({ info }: { info: VinInfo }) {
  const rows: [string, string][] = info.demo
    ? [
        ["Manufacturer", "Practice car"],
        ["Model year", "n/a"],
      ]
    : [
        ["Manufacturer", info.manufacturer ?? `Unknown (${info.wmi})`],
        ["Built in", [info.country, info.region].filter(Boolean).join(", ")],
        ["Model year", info.model_year ? String(info.model_year) : "Unknown"],
        ["Plant code", info.plant ?? "-"],
        ["Serial", info.serial || "-"],
      ];
  const check =
    info.demo || info.check_digit_ok === null
      ? null
      : info.check_digit_ok
        ? { ok: true, text: "Check digit valid" }
        : info.check_digit_required
          ? { ok: false, text: "Check digit does not match: the VIN may be misread or altered" }
          : { ok: true, text: "Check digit not used by this maker" };
  return (
    <div className="mt-6 rounded-xl border border-white/[0.07] bg-black/20 p-4">
      <div className="mb-3 flex items-center justify-between">
        <span className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">VIN decode</span>
        {info.demo ? (
          <Badge variant="warn">Demo VIN</Badge>
        ) : (
          check && (
            <Badge variant={check.ok ? "ok" : "danger"}>
              <BadgeCheck /> {check.text}
            </Badge>
          )
        )}
      </div>
      <div className="mb-3 flex font-mono text-[13px] tracking-[0.12em]">
        {info.vin.split("").map((ch, i) => (
          <span
            key={i}
            title={i < 3 ? "World manufacturer" : i < 8 ? "Vehicle description" : i === 8 ? "Check digit" : i === 9 ? "Model year" : i === 10 ? "Plant" : "Serial"}
            className={cn(
              "px-[1px]",
              info.demo
                ? "text-foreground/60"
                : [
                    i < 3 && "text-cyan-300",
                    i >= 3 && i < 8 && "text-foreground/70",
                    i === 8 && "text-violet-300",
                    i === 9 && "text-emerald-300",
                    i === 10 && "text-amber-300",
                    i > 10 && "text-foreground/50",
                  ],
            )}
          >
            {ch}
          </span>
        ))}
      </div>
      <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-xs sm:grid-cols-3">
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt className="text-muted-foreground">{k}</dt>
            <dd className="mt-0.5 font-medium">{v}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
