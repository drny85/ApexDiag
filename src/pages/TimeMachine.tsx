import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  Camera,
  CheckCircle2,
  Clock3,
  History,
  Lightbulb,
  Loader2,
  Plug,
  RotateCcw,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import GateDialog from "@/components/GateDialog";
import InfoTip from "@/components/InfoTip";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type Comparison, type ModuleInfo, type RecordChange, type Snapshot } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

const LIVE = "live";

export default function TimeMachine({ go }: { go: (p: PageId) => void }) {
  const { connection, catalog, bump } = useDiag();
  const [snapshots, setSnapshots] = useState<Snapshot[] | null>(null);
  const [address, setAddress] = useState<number | null>(null);
  const [fromId, setFromId] = useState<string | null>(null);
  const [toId, setToId] = useState<string>(LIVE);
  const [cmp, setCmp] = useState<Comparison | null>(null);
  const [loading, setLoading] = useState(false);
  const [taking, setTaking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);

  const modules = useMemo(() => {
    if (!catalog) return [];
    const withRecords = new Set(catalog.records.map((r) => r.module));
    const withSnaps = new Set((snapshots ?? []).map((s) => s.module_address));
    return catalog.modules.filter((m) => withRecords.has(m.address) || withSnaps.has(m.address));
  }, [catalog, snapshots]);

  const reload = useCallback(async () => {
    try {
      setSnapshots(await api.listSnapshots());
    } catch (e) {
      setError(asCmdError(e).message);
    }
  }, []);

  useEffect(() => {
    if (connection) reload();
  }, [connection, reload]);

  // Default to the engine module, the most interesting one.
  useEffect(() => {
    if (address === null && modules.length) setAddress(modules.find((m) => m.name === "DME")?.address ?? modules[0]!.address);
  }, [modules, address]);

  const timeline = useMemo(() => (snapshots ?? []).filter((s) => s.module_address === address), [snapshots, address]);

  // Pick the oldest snapshot as "from" when switching modules.
  useEffect(() => {
    setFromId(timeline.length ? timeline[timeline.length - 1]!.id : null);
    setToId(LIVE);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [address, snapshots === null]);

  const compare = useCallback(async () => {
    if (address === null || !fromId) {
      setCmp(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setCmp(await api.compareSnapshots(address, fromId, toId));
    } catch (e) {
      setError(asCmdError(e).message);
      setCmp(null);
    } finally {
      setLoading(false);
    }
  }, [address, fromId, toId]);

  useEffect(() => {
    compare();
  }, [compare]);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Time Machine" subtitle="Backups of every module, compared over time and restorable." />
        <Empty icon={History} title="No car connected">
          Snapshots are filed by VIN, so connect to see this car's history.
          <div>
            <Button className="mt-4" onClick={() => go("connect")}>
              <Plug /> Connect
            </Button>
          </div>
        </Empty>
      </div>
    );
  }

  const module = modules.find((m) => m.address === address) ?? null;
  const restoreFn = catalog?.functions.find((f) => f.id === "snapshot_restore");
  const from = cmp?.from;
  const canRestore = !!cmp && toId === LIVE && cmp.from.id !== LIVE && cmp.diff.restorable > 0;
  const writes = cmp?.diff.records.filter((r) => r.kind === "coding" && r.changed && r.before !== null) ?? [];

  const take = async () => {
    if (address === null) return;
    setTaking(true);
    setError(null);
    try {
      const s = await api.takeSnapshot(address);
      await reload();
      setFromId(s.id);
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setTaking(false);
    }
  };

  return (
    <div>
      <PageHeader
        title="Time Machine"
        subtitle="ApexDiag backs a module up before anything writes to it. Compare any two moments to see what changed, catch slow drift early, and put coding back if a change goes wrong."
        actions={
          <Button variant="outline" onClick={take} disabled={taking || address === null}>
            {taking ? <Loader2 className="animate-spin" /> : <Camera />} Snapshot {module?.name ?? ""} now
          </Button>
        }
      />

      {!connection.simulated && (
        <div className="mb-5 flex items-start gap-3 rounded-xl border border-sky-400/20 bg-sky-400/[0.05] p-4 text-sm">
          <InfoGlyph />
          <p className="text-sky-50/90">
            On a real car the Time Machine saves identification and fault memory today. Coding and learned values are added module by module as
            each one is verified on your X5, so nothing unverified is ever sent.
          </p>
        </div>
      )}

      <ModulePicker modules={modules} snapshots={snapshots ?? []} active={address} onPick={setAddress} />

      <div className="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-[300px_minmax(0,1fr)]">
        <Timeline
          items={timeline}
          fromId={fromId}
          toId={toId}
          onFrom={(id) => {
            if (id === toId) setToId(LIVE);
            setFromId(id);
          }}
          onTo={(id) => setToId(id)}
          loading={snapshots === null}
        />

        <div className="min-w-0 space-y-4">
          {error && <p className="text-sm text-rose-300">{error}</p>}
          {!fromId && snapshots !== null && (
            <Empty icon={Clock3} title={`No snapshots of ${module?.name ?? "this module"} yet`}>
              One is taken automatically before any write. You can also take one now, to have a baseline to compare against later.
            </Empty>
          )}
          {fromId && !cmp && loading && (
            <div className="flex items-center gap-2 rounded-2xl border border-white/[0.07] p-6 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" /> Reading {module?.name} and comparing…
            </div>
          )}
          {cmp && from && (
            <div className={cn("space-y-4 transition-opacity", loading && "opacity-60")}>
              <CompareHeader cmp={cmp} />
              <Insights cmp={cmp} />
              <RecordSection
                title="Coding"
                hint="Settings the module was told. These can be written back."
                rows={cmp.diff.records.filter((r) => r.kind === "coding")}
                footer={
                  toId === LIVE && cmp.from.id !== LIVE ? (
                    <div className="flex items-center justify-between gap-3 border-t border-white/[0.06] px-4 py-3">
                      <span className="text-xs text-muted-foreground">
                        {canRestore
                          ? `${cmp.diff.restorable} setting${cmp.diff.restorable > 1 ? "s" : ""} would be written back. Everything else is left alone.`
                          : "Coding matches this snapshot. Nothing to restore."}
                      </span>
                      <Button
                        size="sm"
                        disabled={!canRestore}
                        onClick={() => setRestoring(true)}
                        className="bg-gradient-to-r from-orange-300 to-rose-300 text-black hover:opacity-90"
                      >
                        <RotateCcw /> Restore to {shortDate(cmp.from.taken_at_ms)}
                      </Button>
                    </div>
                  ) : null
                }
              />
              <RecordSection
                title="Learned values"
                hint="What the module learned by itself. Compared to spot drift, never written back: the car relearns them."
                rows={cmp.diff.records.filter((r) => r.kind === "adaptation")}
                chart
              />
              <FaultSection cmp={cmp} />
              <IdentSection cmp={cmp} />
            </div>
          )}
        </div>
      </div>

      {module && cmp && (
        <GateDialog
          fn={restoreFn}
          open={restoring}
          onOpenChange={(o) => {
            setRestoring(o);
            if (!o) {
              reload();
              compare();
              bump();
            }
          }}
          targetLabel={`${module.description} (${module.name})`}
          run={async () => (await api.restoreSnapshot(module.address, cmp.from.id)).steps}
        >
          <div>
            <h4 className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Will write {writes.length} setting{writes.length === 1 ? "" : "s"}
            </h4>
            <div className="divide-y divide-white/[0.06] rounded-xl border border-white/10">
              {writes.map((w) => (
                <div key={w.lid} className="flex items-center justify-between gap-3 px-3 py-2.5 text-sm">
                  <span>{w.name}</span>
                  <span className="flex items-center gap-2 font-mono text-xs">
                    <span className="text-muted-foreground line-through decoration-white/30">{w.after}</span>
                    <ArrowRight className="size-3 text-orange-300" />
                    <span className="text-orange-200">{w.before}</span>
                  </span>
                </div>
              ))}
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              Back to “{cmp.from.reason}”, {longDate(cmp.from.taken_at_ms)}. The current coding is snapshotted first.
            </p>
          </div>
        </GateDialog>
      )}
    </div>
  );
}

// ---- module picker -----------------------------------------------------

function ModulePicker({
  modules,
  snapshots,
  active,
  onPick,
}: {
  modules: ModuleInfo[];
  snapshots: Snapshot[];
  active: number | null;
  onPick: (a: number) => void;
}) {
  return (
    <div className="flex gap-2 overflow-x-auto pb-1">
      {modules.map((m) => {
        const n = snapshots.filter((s) => s.module_address === m.address).length;
        const on = m.address === active;
        return (
          <button
            key={m.address}
            onClick={() => onPick(m.address)}
            className={cn(
              "flex min-w-[150px] items-center gap-3 rounded-xl border px-3 py-2.5 text-left transition",
              on ? "border-cyan-400/40 bg-cyan-400/[0.07] shadow-[0_0_0_1px_rgba(34,211,238,0.15)]" : "border-white/[0.07] bg-white/[0.02] hover:border-white/15",
            )}
          >
            <span
              className={cn(
                "grid size-9 shrink-0 place-items-center rounded-lg font-mono text-[10px] font-semibold ring-1",
                on ? "bg-cyan-400/15 text-cyan-100 ring-cyan-300/30" : "bg-white/5 ring-white/10",
              )}
            >
              {m.name.split("/")[0]}
            </span>
            <span className="min-w-0">
              <span className="block truncate text-sm font-medium">{m.description}</span>
              <span className="block text-[11px] text-muted-foreground">
                {n} snapshot{n === 1 ? "" : "s"}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

// ---- timeline ----------------------------------------------------------

function Timeline({
  items,
  fromId,
  toId,
  onFrom,
  onTo,
  loading,
}: {
  items: Snapshot[];
  fromId: string | null;
  toId: string;
  onFrom: (id: string) => void;
  onTo: (id: string) => void;
  loading: boolean;
}) {
  return (
    <div className="self-start rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4 lg:sticky lg:top-0">
      <h3 className="mb-3 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Timeline</h3>
      <ol className="relative">
        <span className="absolute bottom-3 left-[11px] top-3 w-px bg-gradient-to-b from-cyan-400/50 via-white/10 to-white/5" />
        <TimelineNode
          title="Now"
          subtitle="Read live from the car"
          badge={<Badge variant="info">Live</Badge>}
          role={toId === LIVE ? "to" : null}
          live
          onTo={() => onTo(LIVE)}
        />
        {loading && (
          <li className="flex items-center gap-2 py-3 pl-9 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" /> Loading…
          </li>
        )}
        {items.map((s) => (
          <TimelineNode
            key={s.id}
            title={s.reason.replace(" (demo history)", "")}
            subtitle={`${longDate(s.taken_at_ms)} · ${ago(s.taken_at_ms)}`}
            badge={<KindBadge kind={s.kind} />}
            role={s.id === fromId ? "from" : s.id === toId ? "to" : null}
            onFrom={() => onFrom(s.id)}
            onTo={s.id !== fromId ? () => onTo(s.id) : undefined}
          />
        ))}
      </ol>
      <p className="mt-3 text-[11px] leading-relaxed text-muted-foreground">
        Click a snapshot to compare from it. Use <span className="text-foreground/80">Compare to</span> to change the other end.
      </p>
    </div>
  );
}

function TimelineNode({
  title,
  subtitle,
  badge,
  role,
  live,
  onFrom,
  onTo,
}: {
  title: string;
  subtitle: string;
  badge: React.ReactNode;
  role: "from" | "to" | null;
  live?: boolean;
  onFrom?: () => void;
  onTo?: () => void;
}) {
  return (
    <li className="group relative pl-9">
      <span
        className={cn(
          "absolute left-[5px] top-[18px] grid size-[13px] place-items-center rounded-full ring-4 ring-[oklch(0.17_0.008_265)]",
          role === "from" && "bg-amber-300",
          role === "to" && "bg-cyan-300",
          !role && "bg-white/25",
        )}
      >
        {live && <span className="absolute inset-0 animate-ping rounded-full bg-cyan-300/50" />}
      </span>
      <div
        role={onFrom ? "button" : undefined}
        onClick={onFrom}
        className={cn(
          "my-1 rounded-xl border px-3 py-2.5 transition",
          onFrom && "cursor-pointer",
          role === "from" && "border-amber-300/35 bg-amber-300/[0.06]",
          role === "to" && "border-cyan-300/35 bg-cyan-300/[0.06]",
          !role && "border-transparent hover:border-white/10 hover:bg-white/[0.03]",
        )}
      >
        <div className="flex items-start justify-between gap-2">
          <span className="line-clamp-2 text-sm font-medium">{title}</span>
          {role ? (
            <span className={cn("mt-0.5 shrink-0 text-[10px] font-semibold tracking-[0.14em]", role === "from" ? "text-amber-300" : "text-cyan-300")}>
              {role.toUpperCase()}
            </span>
          ) : (
            badge
          )}
        </div>
        <div className="mt-0.5 flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">{subtitle}</span>
          {onTo && role !== "to" && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onTo();
              }}
              className="shrink-0 rounded-md px-1.5 py-0.5 text-[10px] text-cyan-200/80 opacity-0 transition hover:bg-cyan-300/10 group-hover:opacity-100"
            >
              Compare to
            </button>
          )}
        </div>
      </div>
    </li>
  );
}

function KindBadge({ kind }: { kind: Snapshot["kind"] }) {
  if (kind === "automatic")
    return (
      <Badge variant="warn" className="shrink-0 bg-orange-400/10 text-orange-200 ring-orange-400/25">
        Auto backup
      </Badge>
    );
  if (kind === "demo") return <Badge variant="muted" className="shrink-0">Demo history</Badge>;
  return <Badge className="shrink-0">Manual</Badge>;
}

// ---- compare -----------------------------------------------------------

function CompareHeader({ cmp }: { cmp: Comparison }) {
  const { diff, from, to } = cmp;
  const coding = diff.records.filter((r) => r.kind === "coding" && r.changed).length;
  const drift = diff.records.filter((r) => r.kind === "adaptation" && r.changed).length;
  return (
    <div className="overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-white/[0.04] to-transparent">
      <div className="flex flex-wrap items-center gap-3 px-5 py-4">
        <End tone="from" title={from.id === LIVE ? "Now" : from.reason.replace(" (demo history)", "")} when={from} />
        <ArrowRight className="size-4 text-muted-foreground" />
        <End tone="to" title={to.id === LIVE ? "Now" : to.reason.replace(" (demo history)", "")} when={to} />
        <span className="ml-auto text-xs text-muted-foreground">{span(from.taken_at_ms, to.taken_at_ms)}</span>
      </div>
      <div className="grid grid-cols-2 border-t border-white/[0.06] sm:grid-cols-4">
        <Stat label="Coding changed" value={coding} tone={coding ? "orange" : "ok"} />
        <Stat label="Learned values moved" value={drift} tone={drift ? "amber" : "ok"} />
        <Stat label="New faults" value={diff.faults.appeared.length} tone={diff.faults.appeared.length ? "rose" : "ok"} />
        <Stat label="Faults gone" value={diff.faults.gone.length} tone={diff.faults.gone.length ? "emerald" : "muted"} />
      </div>
    </div>
  );
}

function End({ tone, title, when }: { tone: "from" | "to"; title: string; when: Snapshot }) {
  return (
    <div className="min-w-0">
      <div className={cn("text-[10px] font-semibold tracking-[0.16em]", tone === "from" ? "text-amber-300" : "text-cyan-300")}>{tone.toUpperCase()}</div>
      <div className="truncate text-[15px] font-medium">{title}</div>
      <div className="text-[11px] text-muted-foreground">{when.id === LIVE ? "Read just now" : longDate(when.taken_at_ms)}</div>
    </div>
  );
}

function Stat({ label, value, tone }: { label: string; value: number; tone: "ok" | "orange" | "amber" | "rose" | "emerald" | "muted" }) {
  const color = {
    ok: "text-emerald-300",
    orange: "text-orange-300",
    amber: "text-amber-300",
    rose: "text-rose-300",
    emerald: "text-emerald-300",
    muted: "text-foreground/70",
  }[tone];
  return (
    <div className="border-white/[0.06] px-5 py-3 [&:not(:last-child)]:border-r">
      <div className={cn("text-2xl font-semibold tabular-nums", color)}>{value}</div>
      <div className="text-[11px] text-muted-foreground">{label}</div>
    </div>
  );
}

function RecordSection({
  title,
  hint,
  rows,
  chart,
  footer,
}: {
  title: string;
  hint: string;
  rows: RecordChange[];
  chart?: boolean;
  footer?: React.ReactNode;
}) {
  if (!rows.length) return null;
  const changed = rows.filter((r) => r.changed).length;
  return (
    <section className="overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.02]">
      <header className="flex items-baseline justify-between gap-3 px-4 pb-2 pt-4">
        <div>
          <h3 className="text-sm font-semibold">{title}</h3>
          <p className="text-xs text-muted-foreground">{hint}</p>
        </div>
        <span className="shrink-0 text-xs text-muted-foreground">
          {changed} of {rows.length} changed
        </span>
      </header>
      <div className="divide-y divide-white/[0.05]">
        {rows.map((r) => (
          <div
            key={r.lid}
            className={cn(
              "grid grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)] items-center gap-4 px-4 py-3",
              chart && "md:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)_minmax(0,1fr)]",
              !r.changed && "opacity-55",
            )}
          >
            <div className="flex min-w-0 items-center gap-1.5">
              <span className="truncate text-sm">{r.name}</span>
              <InfoTip text={r.explain} />
            </div>
            <div className="flex items-center gap-2 font-mono text-[13px]">
              <span className={cn(r.changed ? "text-amber-200/80" : "text-foreground/70")}>{r.before ?? "—"}</span>
              {r.changed && (
                <>
                  <ArrowRight className="size-3 shrink-0 text-muted-foreground" />
                  <span className="text-cyan-100">{r.after ?? "—"}</span>
                </>
              )}
              {r.changed && r.delta !== null && <DeltaChip change={r} />}
            </div>
            {chart && (
              <div className="hidden md:block">
                <DriftBar change={r} />
              </div>
            )}
          </div>
        ))}
      </div>
      {footer}
    </section>
  );
}

function DeltaChip({ change }: { change: RecordChange }) {
  const d = change.delta ?? 0;
  const digits = (change.after ?? "").split(" ")[0]?.split(".")[1]?.length ?? 0;
  const big = Math.abs(d) > Math.max(Math.abs(change.before_value ?? 0), 1) * 0.5;
  return (
    <span
      className={cn(
        "rounded-md px-1.5 py-0.5 font-sans text-[11px] font-medium tabular-nums ring-1 ring-inset",
        big ? "bg-amber-400/10 text-amber-200 ring-amber-400/25" : "bg-white/5 text-foreground/70 ring-white/10",
      )}
    >
      {d > 0 ? "+" : ""}
      {d.toFixed(digits)}
    </span>
  );
}

/** Before (hollow) and after (solid) on one track, so drift is visible at a glance. */
function DriftBar({ change }: { change: RecordChange }) {
  const a = change.before_value;
  const b = change.after_value;
  if (a === null || b === null) return null;
  const signed = a < 0 || b < 0 || change.before?.startsWith("+") || change.after?.startsWith("+");
  const m = Math.max(Math.abs(a), Math.abs(b), 1e-9) * 1.25;
  const lo = signed ? -m : 0;
  const hi = m;
  const pos = (v: number) => ((v - lo) / (hi - lo)) * 100;
  const pa = pos(a);
  const pb = pos(b);
  const zero = pos(0);
  return (
    <div className="relative h-6">
      <div className="absolute inset-x-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-white/[0.06]" />
      {signed && <div className="absolute top-1 h-4 w-px bg-white/25" style={{ left: `${zero}%` }} />}
      {change.changed && (
        <div
          className="absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-gradient-to-r from-amber-300/40 to-cyan-300/80"
          style={{ left: `${Math.min(pa, pb)}%`, width: `${Math.abs(pb - pa)}%`, transform: pb < pa ? "translateY(-50%) scaleX(-1)" : undefined }}
        />
      )}
      <span className="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-amber-300/80 bg-[oklch(0.17_0.008_265)]" style={{ left: `${pa}%` }} />
      <span className="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-cyan-300 shadow-[0_0_10px_rgba(103,232,249,0.6)]" style={{ left: `${pb}%` }} />
    </div>
  );
}

function FaultSection({ cmp }: { cmp: Comparison }) {
  const { appeared, gone, unchanged } = cmp.diff.faults;
  const none = !appeared.length && !gone.length && !unchanged.length;
  return (
    <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
      <h3 className="text-sm font-semibold">Fault memory</h3>
      <p className="text-xs text-muted-foreground">Kept in every snapshot, so codes cleared later are never lost.</p>
      {none ? (
        <div className="mt-3 flex items-center gap-2 text-sm text-emerald-200">
          <CheckCircle2 className="size-4 text-emerald-400" /> No faults at either point.
        </div>
      ) : (
        <div className="mt-3 flex flex-wrap gap-2">
          {appeared.map((c) => (
            <FaultChip key={c} code={c} label="New" className="border-rose-400/30 bg-rose-400/[0.08] text-rose-100" />
          ))}
          {unchanged.map((c) => (
            <FaultChip key={c} code={c} label="Still there" className="border-amber-400/20 bg-amber-400/[0.05] text-amber-100/90" />
          ))}
          {gone.map((c) => (
            <FaultChip key={c} code={c} label="Gone" className="border-emerald-400/25 bg-emerald-400/[0.06] text-emerald-100" />
          ))}
        </div>
      )}
    </section>
  );
}

function FaultChip({ code, label, className }: { code: string; label: string; className: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2 rounded-lg border px-2.5 py-1.5", className)}>
      <span className="font-mono text-sm font-semibold tracking-wider">{code}</span>
      <span className="text-[11px] opacity-80">{label}</span>
    </span>
  );
}

function IdentSection({ cmp }: { cmp: Comparison }) {
  const { from, to, diff } = cmp;
  return (
    <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4 text-sm">
      <h3 className="font-semibold">Identification</h3>
      {diff.identification_changed ? (
        <div className="mt-2 space-y-1 font-mono text-xs">
          <div className="text-amber-200/80">{from.identification ?? "Not read"}</div>
          <div className="text-cyan-100">{to.identification ?? "Not read"}</div>
          <p className="pt-1 font-sans text-xs text-muted-foreground">
            The hardware or software changed between these points: the module was replaced or reprogrammed.
          </p>
        </div>
      ) : (
        <p className="mt-1 text-xs text-muted-foreground">
          Unchanged: same hardware and software. <span className="font-mono text-foreground/60">{to.identification}</span>
        </p>
      )}
    </section>
  );
}

// ---- insights ----------------------------------------------------------

function Insights({ cmp }: { cmp: Comparison }) {
  const tips = insights(cmp);
  if (!tips.length) return null;
  return (
    <section className="rounded-2xl border border-violet-400/20 bg-gradient-to-br from-violet-400/[0.08] to-transparent p-4">
      <h3 className="flex items-center gap-2 text-sm font-semibold text-violet-100">
        <Sparkles className="size-4 text-violet-300" /> What the changes suggest
      </h3>
      <ul className="mt-2 space-y-2">
        {tips.map((t) => (
          <li key={t} className="flex gap-2.5 text-sm leading-relaxed text-foreground/90">
            <Lightbulb className="mt-0.5 size-4 shrink-0 text-violet-300/80" />
            {t}
          </li>
        ))}
      </ul>
      <p className="mt-3 text-[11px] text-muted-foreground">These point to where to look first. They are not a diagnosis.</p>
    </section>
  );
}

function insights({ diff, from, to }: Comparison): string[] {
  const out: string[] = [];
  const rec = (name: string) => diff.records.find((r) => r.name === name);
  const since = from.id === LIVE ? "" : ` since ${shortDate(from.taken_at_ms)}`;
  const b1 = rec("Mixture adaptation, bank 1");
  const b2 = rec("Mixture adaptation, bank 2");
  if (b1?.delta != null && b2?.delta != null) {
    if (b1.delta > 3 && b2.delta > 3) {
      out.push(
        `Both engine banks have learned to add fuel${since} (${b1.before} to ${b1.after}, and ${b2.before} to ${b2.after}). When both banks climb together it is usually unmetered air: check the crankcase vent valve, intake boots and vacuum lines before anything else.`,
      );
    } else if (Math.abs(b1.delta - b2.delta) > 4) {
      const bank = Math.abs(b1.delta) > Math.abs(b2.delta) ? "1 (cylinders 1-3)" : "2 (cylinders 4-6)";
      out.push(`Only bank ${bank} is drifting. Look at that bank's injectors and intake gasket.`);
    }
  }
  const vanos = rec("Intake VANOS adaptation");
  if (vanos?.delta != null && Math.abs(vanos.delta) >= 1) {
    const fault = diff.faults.appeared.length || diff.faults.unchanged.length;
    out.push(
      `The intake VANOS offset moved from ${vanos.before} to ${vanos.after}.${fault ? " With a fault stored as well, the" : " The"} VANOS solenoid is the first suspect: clean or swap it before looking at the timing chain.`,
    );
  }
  const clutchA = rec("Clutch fill pressure, A");
  if (clutchA?.delta != null && clutchA.delta > 0.015) {
    out.push(
      `Clutch A needs more pressure than before (${clutchA.before} to ${clutchA.after}). Ageing fluid is the usual cause on this gearbox, and a fluid and filter service often brings it back.`,
    );
  }
  const shifts = rec("Shift adaptations learned");
  if (shifts?.delta != null && shifts.delta < 0) {
    out.push("The shift adaptation count went down, so the transmission adaptations were reset between these two points.");
  }
  const coding = diff.records.filter((r) => r.kind === "coding" && r.changed);
  if (coding.length && to.id === LIVE) {
    out.push(
      `${coding.length} setting${coding.length > 1 ? "s" : ""} changed${since}: ${coding.map((c) => c.name.toLowerCase()).join(", ")}. If you didn't ask for ${coding.length > 1 ? "these" : "this"}, you can restore ${coding.length > 1 ? "them" : "it"} below.`,
    );
  }
  if (diff.faults.appeared.length) {
    out.push(`New fault${diff.faults.appeared.length > 1 ? "s" : ""}${since}: ${diff.faults.appeared.join(", ")}. Open Modules to read the details.`);
  }
  return out;
}

// ---- time helpers ------------------------------------------------------

function shortDate(ms: number) {
  return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function longDate(ms: number) {
  return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", year: "numeric", hour: "numeric", minute: "2-digit" });
}

function ago(ms: number) {
  const s = (Date.now() - ms) / 1000;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  const d = Math.round(s / 86400);
  return d === 1 ? "yesterday" : `${d} days ago`;
}

function span(a: number, b: number) {
  const d = Math.round(Math.abs(b - a) / 86400000);
  if (d >= 1) return `${d} day${d > 1 ? "s" : ""} apart`;
  const h = Math.round(Math.abs(b - a) / 3600000);
  return h >= 1 ? `${h} h apart` : "minutes apart";
}

function InfoGlyph() {
  return <TriangleAlert className="mt-0.5 size-4 shrink-0 text-sky-300" />;
}
