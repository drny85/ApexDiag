/**
 * Full Scan: one button reads everything the car can tell over the plug,
 * then shows it as a results page anyone can act on.
 *
 * 1. Engine and emissions over OBD-II in one backend call (readiness,
 *    stored / pending / permanent codes, freeze frame when a code is
 *    stored, every supported live value six at a time, battery).
 * 2. Every module, one at a time, so the network map lights up.
 * 3. The backend turns the readings into cards: a good reading says what
 *    that part does, a bad one says what is wrong and how to fix it, with
 *    a link into the Repair Copilot.
 *
 * Everything is read-only.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  ChevronDown,
  CircleDashed,
  CircleHelp,
  Clock,
  Cpu,
  Info,
  Loader2,
  Network,
  Play,
  Plug,
  RotateCw,
  ShieldCheck,
  Sparkles,
  Square,
  Wrench,
  XCircle,
  type LucideIcon,
} from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import CarMap, { type MapStatus } from "@/components/illustrations/CarMap";
import { BrandChip, CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import { AreaArt, HealthRing, STATUS_COLOR, ScanStages } from "@/components/illustrations/ScanArt";
import { useDiag } from "@/lib/store";
import { useI18n, Tr, type T } from "@/lib/i18n";
import { api, asCmdError, type FullScanView, type ModuleFault, type ModuleInfo, type ModuleScan, type Pack, type ScanArea, type ScanItem, type ScanStatus } from "@/lib/tauri";
import { SYSTEM_LABEL } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

type Phase =
  | { step: "idle" }
  | { step: "engine" }
  | { step: "modules"; done: number; total: number; current: ModuleInfo | null }
  | { step: "report" }
  | { step: "done" }
  | { step: "error"; message: string };

interface Run {
  vin: string;
  results: Record<number, ModuleScan>;
  view: FullScanView | null;
  started: number;
  finished: number | null;
}

/** Kept across page switches, per car. */
let lastRun: Run | null = null;

/** Module faults from the last full scan on this car, for the Repair Copilot. */
export function quickTestFaults(vin: string | null): { module: ModuleInfo; fault: ModuleFault }[] {
  if (!lastRun || lastRun.vin !== vin) return [];
  return Object.values(lastRun.results).flatMap((s) => s.codes.map((fault) => ({ module: s.module, fault })));
}

const STATUS_ICON: Record<ScanStatus, LucideIcon> = {
  problem: XCircle,
  attention: AlertTriangle,
  good: CheckCircle2,
  info: Info,
  not_read: CircleHelp,
};

const STATUS_LABEL: Record<ScanStatus, string> = {
  problem: "Problem",
  attention: "Check",
  good: "Good",
  info: "Info",
  not_read: "Not checked",
};

const AREAS: { id: ScanArea; label: string; text: string }[] = [
  { id: "engine", label: "Engine", text: "Check-engine light, engine codes, freeze frame and live values." },
  { id: "emissions", label: "Emissions", text: "Every readiness self-test, and whether codes were cleared recently." },
  { id: "electrical", label: "Battery and charging", text: "Battery charge with the engine off, charging with it running." },
  { id: "modules", label: "Every module", text: "Fault memory of brakes, airbags, body, comfort and more." },
  { id: "vehicle", label: "Vehicle", text: "VIN decoded, odometer where reported, and the adapter." },
];

export default function QuickTest({ go }: { go: (p: PageId) => void }) {
  const { connection, vehicle, pauseLive } = useDiag();
  const { t, lang } = useI18n();
  const vin = connection?.vin ?? "";
  const [run, setRun] = useState<Run | null>(() => (lastRun?.vin === vin ? lastRun : null));
  const [phase, setPhase] = useState<Phase>(() => (lastRun?.vin === vin && lastRun.view ? { step: "done" } : { step: "idle" }));
  const cancelled = useRef(false);

  useEffect(() => {
    lastRun = run;
  }, [run]);
  useEffect(
    () => () => {
      cancelled.current = true;
      pauseLive(false);
    },
    [pauseLive],
  );

  if (!connection) {
    return (
      <div>
        <PageHeader title="Full Scan" subtitle="Every reading the car can give, each explained: what it does when it's good, and what to fix when it isn't." />
        <Empty icon={Cpu} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> <Tr>Connect</Tr>
          </Button>
        </Empty>
      </div>
    );
  }

  const pack = connection.vehicle.pack;
  const order = connection.vehicle.scan_addresses;
  const running = phase.step === "engine" || phase.step === "modules" || phase.step === "report";
  const engineOn = vehicle?.engine_running ?? null;

  const start = async () => {
    cancelled.current = false;
    let r: Run = { vin, results: {}, view: null, started: Date.now(), finished: null };
    setRun(r);
    // The scan gets the adapter to itself: no live polling in between.
    pauseLive(true);
    try {
      setPhase({ step: "engine" });
      await api.fullScanStart();
      for (const [i, address] of order.entries()) {
        if (cancelled.current) return setPhase({ step: "idle" });
        setPhase({ step: "modules", done: i, total: order.length, current: pack.modules.find((m) => m.address === address) ?? null });
        const s = await api.scanModule(address).catch(() => null);
        if (s) {
          r = { ...r, results: { ...r.results, [address]: s } };
          setRun(r);
        }
        // A beat between modules so the sweep is readable on the practice car.
        if (connection.simulated) await new Promise((res) => setTimeout(res, 120));
      }
      setPhase({ step: "report" });
      const scanned = Object.entries(r.results).map(([a, s]) => ({ address: Number(a), present: s.present, codes: s.codes }));
      const view = await api.fullScanFinish(scanned, lang);
      r = { ...r, view, finished: Date.now() };
      setRun(r);
      setPhase({ step: "done" });
    } catch (e) {
      setPhase({ step: "error", message: asCmdError(e).message });
    } finally {
      pauseLive(false);
    }
  };

  const current = phase.step === "modules" ? phase.current?.address : null;
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
        title="Full Scan"
        subtitle="Every reading the car can give, each explained: what it does when it's good, and what to fix when it isn't."
        actions={
          phase.step === "done" && (
            <Button variant="outline" onClick={start}>
              <RotateCw /> {t("Scan again")}
            </Button>
          )
        }
      />

      {phase.step === "done" && run?.view ? (
        <Results run={run} view={run.view} pack={pack} status={status} go={go} />
      ) : (
        <>
          <ScanHero
            pack={pack}
            vin={connection.vin}
            simulated={connection.simulated}
            phase={phase}
            engineOn={engineOn}
            onStart={start}
            onStop={() => (cancelled.current = true)}
            t={t}
          />
          {running && (
            <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_300px]">
              <Topology pack={pack} status={status} results={run?.results ?? {}} running={phase.step === "modules"} t={t} />
              <aside className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
                <h3 className="mb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">{t("Where they live")}</h3>
                <CarMap pack={pack} modules={pack.modules} status={status} scanning={phase.step === "modules"} className="h-[380px]" />
              </aside>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ---- before and during the scan ---------------------------------------------

function ScanHero({
  pack,
  vin,
  simulated,
  phase,
  engineOn,
  onStart,
  onStop,
  t,
}: {
  pack: Pack;
  vin: string | null;
  simulated: boolean;
  phase: Phase;
  engineOn: boolean | null;
  onStart: () => void;
  onStop: () => void;
  t: T;
}) {
  const accent = MAKE_ACCENT[pack.make];
  const running = phase.step === "engine" || phase.step === "modules" || phase.step === "report";
  const stage = phase.step === "engine" ? 0 : phase.step === "modules" ? 1 : phase.step === "report" ? 2 : -1;
  const progress =
    phase.step === "engine" ? 0.12 : phase.step === "modules" ? 0.15 + (0.8 * phase.done) / Math.max(1, phase.total) : phase.step === "report" ? 0.97 : 0;
  const line =
    phase.step === "engine"
      ? t("Reading the engine: codes, readiness, live values and battery…")
      : phase.step === "modules"
        ? t("Asking {name} ({n} of {total})…", { name: phase.current?.name ?? t("module"), n: phase.done + 1, total: phase.total })
        : phase.step === "report"
          ? t("Explaining what was found…")
          : "";
  return (
    <section className="relative overflow-hidden rounded-3xl border border-white/[0.07] bg-gradient-to-br from-white/[0.05] via-white/[0.02] to-transparent p-6 md:p-8">
      <div className="pointer-events-none absolute -right-24 -top-32 size-96 rounded-full blur-3xl" style={{ backgroundColor: `${accent}18` }} />
      <div className="pointer-events-none absolute -bottom-40 -left-20 size-96 rounded-full bg-cyan-400/[0.06] blur-3xl" />
      <div className="relative grid items-center gap-8 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <BrandChip make={pack.make} brand={pack.brand} />
            <Badge variant="muted">{pack.chassis}</Badge>
            {simulated && <Badge variant="warn">{t("Practice car")}</Badge>}
          </div>
          <div className="mt-3 text-2xl font-semibold tracking-tight">
            {pack.brand} {pack.model}
          </div>
          <div className="mt-1 font-mono text-xs tracking-wider text-muted-foreground">VIN {vin ?? t("VIN not read")}</div>
          <CarSide body={pack.body} accent={accent} scanning={running} className="mt-4 w-full" />
        </div>

        <div>
          {!running ? (
            <>
              <ul className="space-y-3">
                {AREAS.map((a) => (
                  <li key={a.id} className="flex items-center gap-3">
                    <AreaArt area={a.id} className="size-10" />
                    <div className="min-w-0">
                      <div className="text-sm font-medium">{t(a.label)}</div>
                      <div className="text-xs text-muted-foreground">{t(a.text)}</div>
                    </div>
                  </li>
                ))}
              </ul>
              <div
                className={cn(
                  "mt-5 flex items-start gap-2 rounded-xl px-3 py-2.5 text-xs ring-1",
                  engineOn ? "bg-emerald-400/[0.06] text-emerald-100/90 ring-emerald-400/20" : "bg-white/[0.03] text-muted-foreground ring-white/[0.08]",
                )}
              >
                <Info className="mt-0.5 size-3.5 shrink-0" />
                {engineOn
                  ? t("Engine running: fuel mixture, idle and charging will be judged too.")
                  : t("Ignition on is enough. Start the engine and let it warm up if you also want fuel mixture, idle and charging judged.")}
              </div>
              {phase.step === "error" && <p className="mt-3 text-sm text-rose-300">{phase.message}</p>}
              <div className="mt-5 flex flex-wrap items-center gap-3">
                <Button size="lg" onClick={onStart} className="h-12 px-6 text-base shadow-[0_0_32px_-8px_#67e8f9]">
                  <Play /> {t("Run full scan")}
                </Button>
                <span className="text-xs text-muted-foreground">{t("Read-only · about a minute on a real car")}</span>
              </div>
            </>
          ) : (
            <>
              <ScanStages stage={stage} labels={[t("Engine"), t("Modules"), t("Report")]} />
              <div className="mt-6 flex items-center justify-between text-xs text-muted-foreground">
                <span className="flex items-center gap-2">
                  <Loader2 className="size-3.5 animate-spin text-cyan-300" /> {line}
                </span>
                <button className="flex items-center gap-1 hover:text-foreground" onClick={onStop}>
                  <Square className="size-3" /> {t("Stop")}
                </button>
              </div>
              <div className="mt-2 h-2 overflow-hidden rounded-full bg-white/[0.06]">
                <div
                  className="h-full rounded-full transition-[width] duration-500"
                  style={{ width: `${progress * 100}%`, background: `linear-gradient(90deg, ${accent}, #67e8f9)` }}
                />
              </div>
              <ul className="mt-6 grid grid-cols-5 gap-2">
                {AREAS.map((a, i) => {
                  const lit = a.id === "modules" ? stage >= 1 : stage >= 0 && i !== 3;
                  return (
                    <li key={a.id} className={cn("flex flex-col items-center gap-1.5 text-center transition-opacity", lit ? "opacity-100" : "opacity-35")}>
                      <AreaArt area={a.id} active={(a.id === "modules" && stage === 1) || (a.id !== "modules" && stage === 0)} className="size-11" />
                      <span className="text-[10px] leading-tight text-muted-foreground">{t(a.label)}</span>
                    </li>
                  );
                })}
              </ul>
            </>
          )}
        </div>
      </div>
    </section>
  );
}

// ---- results ------------------------------------------------------------------

const RANK: Record<ScanStatus, number> = { problem: 4, attention: 3, good: 2, info: 1, not_read: 0 };

function Results({
  run,
  view,
  pack,
  status,
  go,
}: {
  run: Run;
  view: FullScanView;
  pack: Pack;
  status: (a: number) => MapStatus;
  go: (p: PageId) => void;
}) {
  const { t } = useI18n();
  const { setFocus } = useDiag();
  const r = view.report;
  const issues = useMemo(
    () => r.items.filter((i) => i.status === "problem" || i.status === "attention").sort((a, b) => RANK[b.status] - RANK[a.status]),
    [r.items],
  );
  const [area, setArea] = useState<ScanArea | "all">("all");
  const [showMap, setShowMap] = useState(false);
  const seconds = ((run.finished ?? Date.now()) - run.started) / 1000;
  const counts: Record<ScanStatus, number> = r.counts;
  const tone = r.overall === "problem" ? "rose" : r.overall === "attention" ? "amber" : "emerald";
  const toFix = counts.problem + counts.attention;
  const openGuide = (it: ScanItem) => {
    if (!it.guide) return;
    setFocus({ code: it.guide.code, module: it.module });
    go("copilot");
  };
  const list = r.items.filter((i) => area === "all" || i.area === area);

  return (
    <div className="space-y-6">
      {/* Verdict */}
      <section
        className={cn(
          "relative overflow-hidden rounded-3xl border p-6 md:p-8",
          tone === "rose" && "border-rose-400/20 bg-gradient-to-br from-rose-500/[0.09] to-transparent",
          tone === "amber" && "border-amber-400/20 bg-gradient-to-br from-amber-400/[0.08] to-transparent",
          tone === "emerald" && "border-emerald-400/20 bg-gradient-to-br from-emerald-400/[0.08] to-transparent",
        )}
      >
        <div className="pointer-events-none absolute -bottom-6 -right-6 hidden opacity-25 2xl:block">
          <CarSide body={pack.body} accent={STATUS_COLOR[r.overall]} className="w-[340px]" />
        </div>
        <div className="relative flex flex-col items-start gap-6 md:flex-row md:items-center">
          <HealthRing counts={counts} size={176}>
            <div className="text-4xl font-semibold tabular-nums" style={{ color: STATUS_COLOR[r.overall] }}>
              {toFix}
            </div>
            <div className="mx-auto max-w-[96px] text-[10px] uppercase leading-tight tracking-[0.12em] text-muted-foreground">{t("to fix or check")}</div>
          </HealthRing>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              <BrandChip make={pack.make} brand={pack.brand} />
              <span>{view.vehicle}</span>
              {view.vin && <span className="font-mono tracking-wider">· {view.vin}</span>}
              {view.simulated && <Badge variant="warn">{t("Practice car")}</Badge>}
            </div>
            <h2 className="mt-2 text-3xl font-semibold tracking-tight">{r.title}</h2>
            <p className="mt-1 text-sm text-muted-foreground">{r.summary}</p>
            <div className="mt-4 flex flex-wrap gap-2">
              {(["problem", "attention", "good", "info", "not_read"] as ScanStatus[])
                .filter((s) => counts[s] > 0)
                .map((s) => (
                  <span key={s} className="inline-flex items-center gap-1.5 rounded-full bg-black/20 px-2.5 py-1 text-xs ring-1 ring-white/[0.08]">
                    <span className="size-2 rounded-full" style={{ backgroundColor: STATUS_COLOR[s] }} />
                    {counts[s]} {t(STATUS_LABEL[s])}
                  </span>
                ))}
              <span className="inline-flex items-center gap-1.5 rounded-full bg-black/20 px-2.5 py-1 text-xs text-muted-foreground ring-1 ring-white/[0.08]">
                <Clock className="size-3" /> {seconds.toFixed(1)} s
              </span>
            </div>
            {r.tips.length > 0 && (
              <ul className="mt-4 space-y-1">
                {r.tips.map((tip) => (
                  <li key={tip} className="flex items-start gap-2 text-xs text-muted-foreground">
                    <Sparkles className="mt-0.5 size-3.5 shrink-0 text-cyan-300" /> {tip}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </section>

      {/* What to fix first */}
      <section>
        <SectionTitle icon={Wrench} title={t("Fix these first")} note={issues.length ? t("Most urgent at the top.") : undefined} />
        {issues.length === 0 ? (
          <div className="flex items-center gap-4 rounded-2xl border border-emerald-400/20 bg-emerald-400/[0.05] p-5">
            <ShieldCheck className="size-8 shrink-0 text-emerald-300" />
            <div>
              <div className="font-medium">{t("Nothing needs fixing")}</div>
              <p className="text-sm text-muted-foreground">{t("Every reading that could be judged came back healthy. Open any reading below to learn what that part of the car does.")}</p>
            </div>
          </div>
        ) : (
          <div className="grid gap-3 lg:grid-cols-2">
            {issues.map((it) => (
              <IssueCard key={it.id} it={it} onGuide={() => openGuide(it)} onModule={() => go("modules")} t={t} />
            ))}
          </div>
        )}
      </section>

      {/* Every reading */}
      <section>
        <SectionTitle icon={CheckCircle2} title={t("Everything we read")} note={t("Tap a reading to see what it does.")} />
        <div className="mb-3 flex flex-wrap gap-1.5" role="tablist">
          {(["all", ...AREAS.map((a) => a.id)] as (ScanArea | "all")[]).map((a) => {
            const n = a === "all" ? r.items.length : r.items.filter((i) => i.area === a).length;
            if (!n) return null;
            const worst = r.items.filter((i) => a === "all" || i.area === a).reduce<ScanStatus>((w, i) => (RANK[i.status] > RANK[w] ? i.status : w), "not_read");
            return (
              <button
                key={a}
                role="tab"
                aria-selected={area === a}
                onClick={() => setArea(a)}
                className={cn(
                  "inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-xs ring-1 transition",
                  area === a ? "bg-white/10 text-foreground ring-white/20" : "text-muted-foreground ring-white/[0.08] hover:text-foreground",
                )}
              >
                {a !== "all" && <span className="size-1.5 rounded-full" style={{ backgroundColor: STATUS_COLOR[worst] }} />}
                {a === "all" ? t("All") : t(AREAS.find((x) => x.id === a)!.label)}
                <span className="tabular-nums opacity-60">{n}</span>
              </button>
            );
          })}
        </div>
        <div className="space-y-5">
          {AREAS.filter((a) => area === "all" || area === a.id).map((a) => {
            const items = list.filter((i) => i.area === a.id).sort((x, y) => RANK[y.status] - RANK[x.status]);
            if (!items.length) return null;
            return (
              <div key={a.id} className="overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.02]">
                <div className="flex items-center gap-3 border-b border-white/[0.06] px-4 py-3">
                  <AreaArt area={a.id} className="size-9" />
                  <div>
                    <div className="text-sm font-semibold">{t(a.label)}</div>
                    <div className="text-xs text-muted-foreground">{t(a.text)}</div>
                  </div>
                </div>
                <ul className="divide-y divide-white/[0.05]">
                  {items.map((it) => (
                    <ReadingRow key={it.id} it={it} onGuide={() => openGuide(it)} t={t} />
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      </section>

      {/* Network map */}
      <section>
        <button onClick={() => setShowMap((v) => !v)} className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground">
          <Network className="size-4" /> {showMap ? t("Hide the network map") : t("Show the network map")}
          <ChevronDown className={cn("size-4 transition-transform", showMap && "rotate-180")} />
        </button>
        {showMap && (
          <div className="mt-3 grid gap-5 xl:grid-cols-[minmax(0,1fr)_300px]">
            <Topology pack={pack} status={status} results={run.results} running={false} t={t} />
            <aside className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
              <CarMap pack={pack} modules={pack.modules} status={status} scanning={false} className="h-[380px]" />
            </aside>
          </div>
        )}
      </section>
    </div>
  );
}

function SectionTitle({ icon: Icon, title, note }: { icon: LucideIcon; title: string; note?: string }) {
  return (
    <div className="mb-3 flex items-baseline justify-between gap-3">
      <h3 className="flex items-center gap-2 text-base font-semibold">
        <Icon className="size-4 text-muted-foreground" /> {title}
      </h3>
      {note && <span className="text-xs text-muted-foreground">{note}</span>}
    </div>
  );
}

function StatusPill({ status, t }: { status: ScanStatus; t: T }) {
  const Icon = STATUS_ICON[status];
  return (
    <span
      className="inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium ring-1"
      style={{ color: STATUS_COLOR[status], backgroundColor: `${STATUS_COLOR[status]}14`, borderColor: `${STATUS_COLOR[status]}40` }}
    >
      <Icon className="size-3" /> {t(STATUS_LABEL[status])}
    </span>
  );
}

/** A reading that needs work: what's wrong and how to fix it, up front. */
function IssueCard({ it, onGuide, onModule, t }: { it: ScanItem; onGuide: () => void; onModule: () => void; t: T }) {
  const [more, setMore] = useState(false);
  const color = STATUS_COLOR[it.status];
  return (
    <article className="relative flex flex-col overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.025] p-4 pl-5">
      <span className="absolute inset-y-0 left-0 w-1" style={{ background: `linear-gradient(${color}, ${color}55)` }} />
      <div className="flex items-start gap-3">
        <AreaArt area={it.area} className="size-10" />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className={cn("font-semibold", /^[PCBU0-9A-F]{4,}/.test(it.title) && "font-mono tracking-wider")}>{it.title}</span>
            {it.value && <span className="font-mono text-xs text-muted-foreground">{it.value}</span>}
            <span className="ml-auto">
              <StatusPill status={it.status} t={t} />
            </span>
          </div>
          <p className="mt-0.5 text-sm text-foreground/85">{it.headline}</p>
        </div>
      </div>
      {it.problem && (
        <div className="mt-3">
          <div className="text-[11px] font-medium uppercase tracking-[0.14em]" style={{ color }}>
            {t("What's wrong")}
          </div>
          <p className="mt-1 text-sm leading-relaxed text-muted-foreground">{it.problem}</p>
        </div>
      )}
      {it.fix.length > 0 && (
        <div className="mt-3">
          <div className="text-[11px] font-medium uppercase tracking-[0.14em] text-cyan-200">{t("How to fix it")}</div>
          <ol className="mt-1.5 space-y-1.5">
            {it.fix.map((f, i) => (
              <li key={f} className="flex gap-2 text-sm leading-relaxed">
                <span className="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full bg-cyan-400/10 text-[11px] font-semibold text-cyan-200 ring-1 ring-cyan-300/25">{i + 1}</span>
                <span className="text-foreground/85">{f}</span>
              </li>
            ))}
          </ol>
        </div>
      )}
      {it.facts.length > 0 && <Facts facts={it.facts} />}
      {more && <p className="mt-3 rounded-xl bg-black/20 p-3 text-xs leading-relaxed text-muted-foreground">{it.what}</p>}
      <div className="mt-auto flex flex-wrap items-center gap-2 pt-4">
        {it.guide && (
          <Button size="sm" onClick={onGuide}>
            <Sparkles /> {it.guide.playbook ? t("Open fix guide") : t("Plan the repair")}
          </Button>
        )}
        {it.module && (
          <Button size="sm" variant="outline" onClick={onModule}>
            <Cpu /> {t("Open module")}
          </Button>
        )}
        <button className="ml-auto text-xs text-muted-foreground hover:text-foreground" onClick={() => setMore((v) => !v)}>
          {more ? t("Less") : t("What does this do?")}
        </button>
      </div>
    </article>
  );
}

/** One reading in the full list; opens to explain itself. */
function ReadingRow({ it, onGuide, t }: { it: ScanItem; onGuide: () => void; t: T }) {
  const [open, setOpen] = useState(false);
  const Icon = STATUS_ICON[it.status];
  const bad = it.status === "problem" || it.status === "attention";
  // Info rows use the explanation's first sentence as their one-liner; don't
  // repeat it when the row opens.
  const explain = it.what.startsWith(it.headline) ? it.what.slice(it.headline.length).trim() || it.what : it.what;
  return (
    <li>
      <button onClick={() => setOpen((v) => !v)} className="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-white/[0.02]" aria-expanded={open}>
        <Icon className="size-4 shrink-0" style={{ color: STATUS_COLOR[it.status] }} />
        <div className="min-w-0 flex-1">
          <div className={cn("truncate text-sm font-medium", /^[PCBU][0-9A-F]{4}/.test(it.title) && "font-mono tracking-wider")}>{it.title}</div>
          <div className="truncate text-xs text-muted-foreground">{it.headline}</div>
        </div>
        {it.value && <span className="hidden max-w-[40%] truncate font-mono text-sm tabular-nums text-foreground/90 sm:block">{it.value}</span>}
        <ChevronDown className={cn("size-4 shrink-0 text-muted-foreground transition-transform", open && "rotate-180")} />
      </button>
      {open && (
        <div className="space-y-3 px-4 pb-4 pl-11">
          {it.value && <div className="font-mono text-sm sm:hidden">{it.value}</div>}
          <div>
            <div className="text-[11px] font-medium uppercase tracking-[0.14em] text-muted-foreground">{t("What it does")}</div>
            <p className="mt-1 text-sm leading-relaxed text-foreground/85">{explain}</p>
          </div>
          {bad && it.problem && (
            <div>
              <div className="text-[11px] font-medium uppercase tracking-[0.14em]" style={{ color: STATUS_COLOR[it.status] }}>
                {t("What's wrong")}
              </div>
              <p className="mt-1 text-sm leading-relaxed text-muted-foreground">{it.problem}</p>
            </div>
          )}
          {it.fix.length > 0 && (
            <div>
              <div className="text-[11px] font-medium uppercase tracking-[0.14em] text-cyan-200">{bad ? t("How to fix it") : t("Next step")}</div>
              <ul className="mt-1 list-disc space-y-1 pl-4 text-sm text-foreground/85">
                {it.fix.map((f) => (
                  <li key={f}>{f}</li>
                ))}
              </ul>
            </div>
          )}
          {it.facts.length > 0 && <Facts facts={it.facts} />}
          {it.guide && (
            <Button size="sm" variant="outline" onClick={onGuide}>
              <Sparkles /> {it.guide.playbook ? t("Open fix guide") : t("Plan the repair")} <ArrowRight />
            </Button>
          )}
        </div>
      )}
    </li>
  );
}

function Facts({ facts }: { facts: { label: string; value: string }[] }) {
  return (
    <dl className="mt-3 grid gap-x-4 gap-y-1 rounded-xl bg-black/20 p-3 text-xs sm:grid-cols-2">
      {facts.map((f) => (
        <div key={f.label} className="flex min-w-0 justify-between gap-3">
          <dt className="shrink-0 font-mono text-muted-foreground">{f.label}</dt>
          <dd className="truncate text-right text-foreground/90" title={f.value}>
            {f.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

// ---- network map ------------------------------------------------------------------

const SYSTEM_ORDER = ["powertrain", "chassis", "safety", "body", "comfort", "infotainment"];

/** The diagnostic bus as a wiring diagram: the OBD port, the bus, and each
 * system's modules hanging off it, lighting up as they answer. */
function Topology({
  pack,
  status,
  results,
  running,
  t,
}: {
  pack: Pack;
  status: (a: number) => MapStatus;
  results: Record<number, ModuleScan>;
  running: boolean;
  t: T;
}) {
  const groups = SYSTEM_ORDER.map((s) => ({ system: s, list: pack.modules.filter((m) => m.system === s) })).filter((g) => g.list.length);
  const busName = pack.make === "bmw" ? "D-CAN · 500 kbit/s · KWP2000" : "Diagnostic CAN · 500 kbit/s · UDS";
  return (
    <section className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">{t("Network topology")}</h3>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
          <Dot c="bg-emerald-400" l={t("OK")} />
          <Dot c="bg-amber-400" l={t("Faults")} />
          <Dot c="bg-cyan-300" l={t("Asking")} />
          <Dot c="bg-slate-600" l={t("No reply")} />
        </div>
      </div>
      <div className="relative flex items-center gap-3">
        <div className="grid shrink-0 place-items-center rounded-xl bg-cyan-400/10 px-3 py-2 text-center ring-1 ring-cyan-400/30">
          <Plug className="size-4 text-cyan-300" />
          <span className="mt-0.5 text-[10px] font-medium text-cyan-200">{t("OBD port")}</span>
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
            <div className="mb-2 text-center text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground">{t(SYSTEM_LABEL[g.system] ?? g.system)}</div>
            <div className="relative space-y-2 border-l border-white/10 pl-2.5">
              {g.list.map((m) => (
                <Node key={m.address} m={m} st={status(m.address)} scan={results[m.address]} t={t} />
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function Node({ m, st, scan, t }: { m: ModuleInfo; st: MapStatus; scan?: ModuleScan; t: T }) {
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
      <div className="truncate text-[10px] text-muted-foreground">{st === "skipped" ? t("address not verified") : m.description}</div>
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
