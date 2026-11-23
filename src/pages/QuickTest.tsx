/**
 * Quick Test: the dealer-style full vehicle scan. Every module is asked in
 * turn whether it is fitted and what it has stored, the bus diagram and the
 * car map light up as answers come in, and emissions readiness is read at
 * the end. The result is one page a technician can act on.
 */
import { useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  CircleDashed,
  Clock,
  Cpu,
  Leaf,
  Loader2,
  Play,
  Plug,
  RotateCw,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import CarMap, { type MapStatus } from "@/components/illustrations/CarMap";
import { BrandChip, CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type EmissionsReport, type ModuleFault, type ModuleInfo, type ModuleScan, type Pack } from "@/lib/tauri";
import { SYSTEM_LABEL, hex } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

type Phase = "idle" | "running" | "done";

interface Run {
  vin: string;
  results: Record<number, ModuleScan>;
  emissions: EmissionsReport | null;
  started: number;
  finished: number | null;
  error: string | null;
}

/** Kept across page switches, per car. */
let lastRun: Run | null = null;

/** Module faults from the last Quick Test on this car, for the Repair Copilot. */
export function quickTestFaults(vin: string | null): { module: ModuleInfo; fault: ModuleFault }[] {
  if (!lastRun || lastRun.vin !== vin) return [];
  return Object.values(lastRun.results).flatMap((s) => s.codes.map((fault) => ({ module: s.module, fault })));
}

export default function QuickTest({ go }: { go: (p: PageId) => void }) {
  const { connection } = useDiag();
  const vin = connection?.vin ?? "";
  const [run, setRun] = useState<Run | null>(() => (lastRun?.vin === vin ? lastRun : null));
  const [current, setCurrent] = useState<number | null>(null);
  const cancelled = useRef(false);

  useEffect(() => {
    lastRun = run;
  }, [run]);
  useEffect(
    () => () => {
      cancelled.current = true;
    },
    [],
  );

  if (!connection) {
    return (
      <div>
        <PageHeader title="Quick Test" subtitle="One pass over the whole car: every module, its fault memory and emissions readiness." />
        <Empty icon={Cpu} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> Connect
          </Button>
        </Empty>
      </div>
    );
  }

  const pack = connection.vehicle.pack;
  const order = connection.vehicle.scan_addresses;
  const modules = pack.modules;
  const phase: Phase = !run ? "idle" : run.finished ? "done" : "running";
  const done = run ? Object.keys(run.results).length : 0;

  const start = async () => {
    cancelled.current = false;
    let r: Run = { vin, results: {}, emissions: null, started: Date.now(), finished: null, error: null };
    setRun(r);
    for (const address of order) {
      if (cancelled.current) return;
      setCurrent(address);
      try {
        const s = await api.scanModule(address);
        r = { ...r, results: { ...r.results, [address]: s } };
        setRun(r);
      } catch (e) {
        r = { ...r, error: asCmdError(e).message };
        setRun(r);
      }
      // A beat between modules so the sweep is readable on the demo car.
      if (connection.simulated) await new Promise((res) => setTimeout(res, 140));
    }
    setCurrent(null);
    const emissions = await api.readEmissions().catch(() => null);
    const list = Object.values(r.results);
    const faults = list.reduce((n, s) => n + s.codes.length, 0);
    await api.logQuickTest(list.filter((s) => s.present).length, faults).catch(() => {});
    r = { ...r, emissions, finished: Date.now() };
    setRun(r);
  };

  const status = (address: number): MapStatus => {
    if (!order.includes(address)) return "skipped";
    if (current === address) return "scanning";
    const s = run?.results[address];
    if (!s) return "unknown";
    if (!s.present) return "absent";
    return s.codes.length ? "fault" : "ok";
  };

  return (
    <div>
      <PageHeader
        title="Quick Test"
        subtitle={`A full-vehicle scan the way ${pack.dealer_tool} runs one: every module is asked what it is and what it has stored, then emissions readiness is checked.`}
        actions={
          <Button onClick={start} disabled={phase === "running"}>
            {phase === "running" ? <Loader2 className="animate-spin" /> : phase === "done" ? <RotateCw /> : <Play />}
            {phase === "running" ? "Scanning" : phase === "done" ? "Run again" : "Start Quick Test"}
          </Button>
        }
      />

      <Hero pack={pack} vin={connection.vin} simulated={connection.simulated} phase={phase} done={done} total={order.length} current={modules.find((m) => m.address === current)} />

      {run?.error && <p className="mt-4 text-sm text-rose-300">{run.error}</p>}

      <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_300px]">
        <Topology pack={pack} status={status} results={run?.results ?? {}} running={phase === "running"} />
        <aside className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
          <h3 className="mb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Where they live</h3>
          <CarMap pack={pack} modules={modules} status={status} scanning={phase === "running"} className="h-[380px]" />
        </aside>
      </div>

      {phase === "done" && run && <Report run={run} pack={pack} go={go} />}
    </div>
  );
}

function Hero({
  pack,
  vin,
  simulated,
  phase,
  done,
  total,
  current,
}: {
  pack: Pack;
  vin: string | null;
  simulated: boolean;
  phase: Phase;
  done: number;
  total: number;
  current?: ModuleInfo;
}) {
  const accent = MAKE_ACCENT[pack.make];
  const pct = total ? Math.round((done / total) * 100) : 0;
  return (
    <div className="relative overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-white/[0.04] to-transparent p-6">
      <div className="absolute -right-20 -top-28 size-80 rounded-full blur-3xl" style={{ backgroundColor: `${accent}14` }} />
      <div className="relative grid items-center gap-6 md:grid-cols-[1fr_minmax(0,360px)]">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <BrandChip make={pack.make} brand={pack.brand} />
            <Badge variant="muted">{pack.chassis}</Badge>
            {simulated && <Badge variant="warn">Demo car</Badge>}
          </div>
          <div className="mt-3 text-2xl font-semibold tracking-tight">
            {pack.brand} {pack.model}
          </div>
          <div className="mt-1 font-mono text-xs tracking-wider text-muted-foreground">VIN {vin ?? "unavailable"}</div>
          <div className="mt-5">
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>
                {phase === "idle" && `${total} modules to ask`}
                {phase === "running" && (current ? `Asking ${current.name} · ${current.description}` : "Reading emissions readiness")}
                {phase === "done" && "Scan complete"}
              </span>
              <span className="tabular-nums">
                {done} / {total}
              </span>
            </div>
            <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/[0.06]">
              <div
                className="h-full rounded-full transition-[width] duration-300"
                style={{ width: `${phase === "done" ? 100 : pct}%`, background: `linear-gradient(90deg, ${accent}, #67e8f9)` }}
              />
            </div>
          </div>
        </div>
        <CarSide body={pack.body} accent={accent} scanning={phase === "running"} className="w-full" />
      </div>
    </div>
  );
}

const SYSTEM_ORDER = ["powertrain", "chassis", "safety", "body", "comfort", "infotainment"];

/** The diagnostic bus as a wiring diagram: the OBD port, the bus, and each
 * system's modules hanging off it, lighting up as they answer. */
function Topology({
  pack,
  status,
  results,
  running,
}: {
  pack: Pack;
  status: (a: number) => MapStatus;
  results: Record<number, ModuleScan>;
  running: boolean;
}) {
  const groups = SYSTEM_ORDER.map((s) => ({ system: s, list: pack.modules.filter((m) => m.system === s) })).filter((g) => g.list.length);
  const busName = pack.make === "bmw" ? "D-CAN · 500 kbit/s · KWP2000" : "Diagnostic CAN · 500 kbit/s · UDS";
  return (
    <section className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Network topology</h3>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
          <Dot c="bg-emerald-400" l="OK" />
          <Dot c="bg-amber-400" l="Faults" />
          <Dot c="bg-cyan-300" l="Asking" />
          <Dot c="bg-slate-600" l="No reply" />
        </div>
      </div>
      {/* port and bus */}
      <div className="relative flex items-center gap-3">
        <div className="grid shrink-0 place-items-center rounded-xl bg-cyan-400/10 px-3 py-2 text-center ring-1 ring-cyan-400/30">
          <Plug className="size-4 text-cyan-300" />
          <span className="mt-0.5 text-[10px] font-medium text-cyan-200">OBD port</span>
        </div>
        <div className="relative h-[3px] flex-1 overflow-hidden rounded-full bg-gradient-to-r from-cyan-400/60 via-cyan-400/25 to-white/10">
          {running && (
            <span
              className="ill-motion absolute top-1/2 size-2 -translate-y-1/2 rounded-full bg-cyan-200 shadow-[0_0_12px_#67e8f9]"
              style={{ animation: "bus-packet 1.2s linear infinite" }}
            />
          )}
        </div>
        <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{busName}</span>
      </div>
      <div className="mt-1 grid gap-3" style={{ gridTemplateColumns: `repeat(${groups.length}, minmax(0, 1fr))` }}>
        {groups.map((g) => (
          <div key={g.system} className="relative pt-4">
            <span className="absolute left-1/2 top-0 h-4 w-px bg-white/15" />
            <div className="mb-2 text-center text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground">{SYSTEM_LABEL[g.system]}</div>
            <div className="relative space-y-2 border-l border-white/10 pl-2.5">
              {g.list.map((m) => (
                <Node key={m.address} m={m} st={status(m.address)} scan={results[m.address]} />
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function Node({ m, st, scan }: { m: ModuleInfo; st: MapStatus; scan?: ModuleScan }) {
  const n = scan?.codes.length ?? 0;
  return (
    <div
      title={`${m.description} · ${m.spot.area}`}
      className={cn(
        "relative rounded-lg px-2 py-1.5 ring-1 ring-inset transition-all duration-300",
        "before:absolute before:-left-2.5 before:top-1/2 before:h-px before:w-2.5 before:bg-white/10",
        st === "unknown" && "bg-white/[0.02] ring-white/[0.07]",
        st === "scanning" && "scale-[1.03] bg-cyan-400/15 ring-cyan-300/60 shadow-[0_0_18px_-4px_#67e8f9]",
        st === "ok" && "bg-emerald-400/[0.07] ring-emerald-400/30",
        st === "fault" && "bg-amber-400/[0.09] ring-amber-400/40",
        st === "absent" && "bg-transparent opacity-45 ring-white/[0.06] [border-style:dashed]",
        st === "skipped" && "opacity-35 ring-white/[0.05]",
      )}
    >
      <div className="flex items-center justify-between gap-1">
        <span className="truncate font-mono text-[11px] font-semibold">{m.name}</span>
        {st === "scanning" && <Loader2 className="size-3 shrink-0 animate-spin text-cyan-200" />}
        {st === "ok" && <CheckCircle2 className="size-3 shrink-0 text-emerald-400" />}
        {st === "fault" && <span className="rounded bg-amber-400/20 px-1 text-[10px] font-semibold text-amber-200">{n}</span>}
        {st === "absent" && <CircleDashed className="size-3 shrink-0 text-muted-foreground" />}
      </div>
      <div className="truncate text-[10px] text-muted-foreground">{st === "skipped" ? "address not verified" : m.description}</div>
    </div>
  );
}

function Dot({ c, l }: { c: string; l: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={cn("size-2 rounded-full", c)} />
      {l}
    </span>
  );
}

/** UDS status bit 0 (testFailed): the fault is present right now. */
function isActive(f: ModuleFault) {
  return (f.status & 0x01) !== 0;
}

function Report({ run, pack, go }: { run: Run; pack: Pack; go: (p: PageId) => void }) {
  const { setFocus } = useDiag();
  const list = Object.values(run.results);
  const answered = list.filter((s) => s.present);
  const withFaults = answered.filter((s) => s.codes.length);
  const faults = withFaults.flatMap((s) => s.codes.map((c) => ({ module: s.module, fault: c })));
  const active = faults.filter((f) => isActive(f.fault)).length;
  const r = run.emissions?.readiness;
  const incomplete = r ? r.monitors.filter((m) => m.supported && !m.complete) : [];
  const recentlyCleared = !!run.emissions && incomplete.length >= 2 && (run.emissions.km_since_clear ?? 9999) < 200;
  const seconds = ((run.finished ?? Date.now()) - run.started) / 1000;

  const verdict =
    faults.length === 0 && !r?.mil_on
      ? { tone: "ok" as const, icon: ShieldCheck, title: "All clear", text: `${answered.length} modules answered with nothing stored.` }
      : r?.mil_on || active > 0
        ? {
            tone: "bad" as const,
            icon: ShieldAlert,
            title: r?.mil_on ? "Check-engine light on" : `${active} active fault${active === 1 ? "" : "s"}`,
            text: `${faults.length} fault${faults.length === 1 ? "" : "s"} in ${withFaults.length} module${withFaults.length === 1 ? "" : "s"}. Start with the active ones.`,
          }
        : {
            tone: "warn" as const,
            icon: AlertTriangle,
            title: `${faults.length} stored fault${faults.length === 1 ? "" : "s"}`,
            text: "Nothing is failing right now. These are history entries worth reading before clearing.",
          };

  return (
    <div className="mt-5 space-y-5">
      <div className="grid gap-3 md:grid-cols-4">
        <div
          className={cn(
            "flex items-start gap-3 rounded-2xl border p-4 md:col-span-2",
            verdict.tone === "ok" && "border-emerald-400/25 bg-emerald-400/[0.06]",
            verdict.tone === "warn" && "border-amber-400/25 bg-amber-400/[0.06]",
            verdict.tone === "bad" && "border-rose-400/25 bg-rose-400/[0.06]",
          )}
        >
          <verdict.icon
            className={cn(
              "mt-0.5 size-5 shrink-0",
              verdict.tone === "ok" && "text-emerald-300",
              verdict.tone === "warn" && "text-amber-300",
              verdict.tone === "bad" && "text-rose-300",
            )}
          />
          <div>
            <div className="font-semibold">{verdict.title}</div>
            <p className="text-sm text-muted-foreground">{verdict.text}</p>
          </div>
        </div>
        <Stat label="Modules answered" value={`${answered.length} / ${list.length}`} />
        <Stat label="Scan time" value={`${seconds.toFixed(1)} s`} icon={Clock} />
      </div>

      <div className="grid gap-5 lg:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
        <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02]">
          <div className="flex items-center justify-between border-b border-white/[0.06] px-5 py-3">
            <h3 className="text-sm font-semibold">Findings</h3>
            <Button variant="ghost" size="sm" onClick={() => go("modules")}>
              Open modules <ArrowRight />
            </Button>
          </div>
          {faults.length === 0 ? (
            <div className="flex items-center gap-2 p-5 text-sm text-muted-foreground">
              <CheckCircle2 className="size-4 text-emerald-400" /> No module has anything stored.
            </div>
          ) : (
            <ul className="divide-y divide-white/[0.05]">
              {faults
                .sort((a, b) => Number(isActive(b.fault)) - Number(isActive(a.fault)))
                .map(({ module, fault }) => (
                  <li key={`${module.address}-${fault.code}`} className="flex gap-3 px-5 py-3">
                    <span className="mt-0.5 w-20 shrink-0 font-mono text-[11px] font-semibold text-muted-foreground">{module.name}</span>
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-mono text-sm font-semibold tracking-wider">{fault.code}</span>
                        {module.protocol === "uds" ? (
                          <UdsStatus status={fault.status} />
                        ) : (
                          <Badge variant="muted">Stored · {hex(fault.status)}</Badge>
                        )}
                      </div>
                      <p className="mt-0.5 text-sm text-foreground/80">{fault.description ?? `${pack.brand} fault text not in the database yet.`}</p>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="shrink-0 self-center text-cyan-200"
                      onClick={() => {
                        setFocus({ code: fault.code, module: module.name });
                        go("copilot");
                      }}
                    >
                      <Sparkles /> Fix
                    </Button>
                  </li>
                ))}
            </ul>
          )}
        </section>

        <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
          <div className="flex items-center justify-between">
            <h3 className="flex items-center gap-2 text-sm font-semibold">
              <Leaf className="size-4 text-emerald-300" /> Emissions readiness
            </h3>
            <Button variant="ghost" size="sm" onClick={() => go("codes")}>
              Details <ArrowRight />
            </Button>
          </div>
          {!r ? (
            <p className="mt-3 text-sm text-muted-foreground">Readiness could not be read.</p>
          ) : (
            <>
              <p className="mt-2 text-sm text-muted-foreground">
                {r.mil_on
                  ? "The check-engine light is on, so a plug-in emissions check would fail until the fault is fixed."
                  : incomplete.length === 0
                  ? "Every supported monitor has completed. The car would pass a plug-in emissions check."
                  : `${incomplete.length} monitor${incomplete.length === 1 ? " has" : "s have"} not finished: ${incomplete.map((m) => m.name).join(", ")}.`}
              </p>
              {recentlyCleared && (
                <div className="mt-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.06] p-3 text-sm">
                  <div className="flex items-center gap-2 font-medium text-amber-200">
                    <AlertTriangle className="size-4" /> Codes were cleared recently
                  </div>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                    Only {Math.round(run.emissions?.km_since_clear ?? 0)} km since the last clear and several monitors still incomplete. On a car
                    you are buying, ask why: clearing codes before a sale hides faults.
                  </p>
                </div>
              )}
              <div className="mt-4 grid grid-cols-2 gap-1.5">
                {r.monitors
                  .filter((m) => m.supported)
                  .map((m) => (
                    <div key={m.id} className="flex items-center gap-2 rounded-lg bg-black/20 px-2.5 py-1.5 text-xs">
                      {m.complete ? <CheckCircle2 className="size-3.5 text-emerald-400" /> : <CircleDashed className="size-3.5 text-amber-300" />}
                      <span className="truncate">{m.name}</span>
                    </div>
                  ))}
              </div>
            </>
          )}
        </section>
      </div>
    </div>
  );
}

/** UDS DTC status byte in words: failing now, pending, or stored history. */
function UdsStatus({ status }: { status: number }) {
  if (status & 0x01) return <Badge variant="danger">Active</Badge>;
  if (status & 0x04 && !(status & 0x08)) return <Badge variant="info">Pending</Badge>;
  return <Badge variant="muted">History</Badge>;
}

function Stat({ label, value, icon: Icon }: { label: string; value: string; icon?: typeof Clock }) {
  return (
    <div className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {Icon && <Icon className="size-3.5" />} {label}
      </div>
      <div className="mt-1 text-2xl font-semibold tabular-nums">{value}</div>
    </div>
  );
}
