import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, CircleDot, Cpu, Pause, Play, Plug, Rewind, Trash2, Zap } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type Replay as ReplayData, type ReplayChange, type ReplayCode, type ReplaySummary } from "@/lib/tauri";
import { digitsFor, fmt } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

const ROW_ORDER = [0x0c, 0x0d, 0x04, 0x06, 0x08, 0x07, 0x09, 0x0b, 0x10, 0x0e, 0x14, 0x05, 0x5c, 0x0f, 0x42, 0x2f];

export default function Replay({ go }: { go: (p: PageId) => void }) {
  const { connection, lastReplay, openReplay, setOpenReplay } = useDiag();
  const [list, setList] = useState<ReplaySummary[] | null>(null);
  const [current, setCurrent] = useState<ReplayData | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setList(await api.listReplays());
    } catch (e) {
      setList([]);
      setError(asCmdError(e).message);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // A replay just saved from the status bar: list it and show it.
  useEffect(() => {
    if (!lastReplay) return;
    refresh();
    setCurrent(lastReplay);
  }, [lastReplay, refresh]);

  const open = useCallback(async (id: string) => {
    try {
      setCurrent(await api.getReplay(id));
      setError(null);
    } catch (e) {
      setError(asCmdError(e).message);
    }
  }, []);

  // Open the replay asked for elsewhere (the status bar), else the newest.
  useEffect(() => {
    if (openReplay) {
      open(openReplay);
      setOpenReplay(null);
    } else if (!current && list && list.length > 0) {
      open(list[0]!.id);
    }
  }, [openReplay, list, current, open, setOpenReplay]);

  const remove = async (id: string) => {
    await api.deleteReplay(id).catch((e) => setError(asCmdError(e).message));
    if (current?.id === id) setCurrent(null);
    refresh();
  };

  return (
    <div>
      <PageHeader
        title="Instant Replay"
        subtitle="For problems that come and go. The app always remembers the last 30 seconds of live data. When the car does something odd, press Mark: it keeps 30 seconds either side, reads every module's fault memory, and shows what changed just before."
      />

      <HowItWorks connected={!!connection} simulated={!!connection?.simulated} go={go} compact={!!list?.length} />

      {error && (
        <div className="mt-4 flex items-start gap-2 rounded-xl border border-amber-400/20 bg-amber-400/[0.06] px-4 py-3 text-sm text-amber-200">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" /> {error}
        </div>
      )}

      {list && list.length > 0 && (
        <div className="mt-6 grid gap-6 xl:grid-cols-[300px_minmax(0,1fr)]">
          <div className="flex gap-3 overflow-x-auto pb-1 xl:flex-col xl:overflow-visible">
            {list.map((r) => (
              <button
                key={r.id}
                onClick={() => open(r.id)}
                className={cn(
                  "w-64 shrink-0 rounded-xl border p-3 text-left transition xl:w-auto",
                  current?.id === r.id ? "border-cyan-400/40 bg-cyan-400/[0.06]" : "border-white/[0.07] bg-white/[0.02] hover:border-white/15",
                )}
              >
                <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
                  <span>{new Date(r.mark_ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" })}</span>
                  {r.simulated && <span className="rounded bg-amber-400/10 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">DEMO</span>}
                </div>
                <div className="mt-1 truncate text-sm font-medium">{r.vehicle}</div>
                <div className="mt-1 line-clamp-2 text-xs text-foreground/80">{r.headline ?? "Nothing stood out in the live data."}</div>
                <div className="mt-2 flex gap-3 text-[11px] text-muted-foreground">
                  <span>{Math.round(r.seconds)} s</span>
                  <span>{r.changes} changes</span>
                  <span>{r.faults} faults</span>
                </div>
              </button>
            ))}
          </div>
          {current ? <Viewer replay={current} onDelete={() => remove(current.id)} /> : <div />}
        </div>
      )}
    </div>
  );
}

// ---- explainer -----------------------------------------------------------

function HowItWorks({ connected, simulated, go, compact }: { connected: boolean; simulated: boolean; go: (p: PageId) => void; compact: boolean }) {
  const { markMoment, recording } = useDiag();
  const [stumbled, setStumbled] = useState(false);
  const steps = [
    { title: "Drive with the app connected", text: "Live data streams in the background. Nothing is saved until you ask." },
    { title: "Press Mark when it happens", text: "The red Mark button is at the top of every page. Press it as soon as you feel the stumble, shudder or warning." },
    { title: "Scrub through the moment", text: "Every reading is lined up on one timeline. The app points out what moved just before you pressed." },
  ];
  return (
    <section className={cn("overflow-hidden rounded-2xl border border-white/[0.07] bg-[radial-gradient(ellipse_at_top_left,rgba(244,63,94,0.08),transparent_55%)]", compact ? "p-4" : "p-6")}>
      <div className={cn("flex flex-col gap-6", !compact && "md:flex-row md:items-center")}>
        {!compact && (
          <div className="md:w-[44%]">
            <TimelineArt />
          </div>
        )}
        <div className="flex-1">
          <ol className={cn("grid gap-3", compact ? "md:grid-cols-3" : "")}>
            {steps.map((s, i) => (
              <li key={s.title} className="flex gap-3">
                <span className="grid size-6 shrink-0 place-items-center rounded-full bg-rose-500/15 text-xs font-semibold text-rose-300 ring-1 ring-rose-400/30">{i + 1}</span>
                <div>
                  <div className="text-sm font-medium">{s.title}</div>
                  <div className="text-xs text-muted-foreground">{s.text}</div>
                </div>
              </li>
            ))}
          </ol>
          <div className="mt-4 flex flex-wrap items-center gap-2">
            {!connected && (
              <Button onClick={() => go("connect")}>
                <Plug /> Connect first
              </Button>
            )}
            {connected && simulated && (
              <Button
                variant="outline"
                onClick={async () => {
                  await api.simStumble().catch(() => {});
                  setStumbled(true);
                  setTimeout(() => setStumbled(false), 6000);
                }}
              >
                <Zap /> Make the demo engine stumble
              </Button>
            )}
            {connected && (
              <Button onClick={markMoment} disabled={!!recording} className="from-rose-400 to-rose-500 text-white shadow-rose-500/25 hover:from-rose-300 hover:to-rose-400">
                <CircleDot /> {recording ? "Recording…" : "Mark this moment"}
              </Button>
            )}
            {stumbled && !recording && <span className="text-xs text-rose-200">The demo engine just stumbled. Press Mark now.</span>}
          </div>
          {connected && simulated && !compact && (
            <p className="mt-2 text-xs text-muted-foreground">
              Demo: make the engine stumble, then press Mark within a few seconds. Start the demo engine on Live Data first if it is off.
            </p>
          )}
        </div>
      </div>
    </section>
  );
}

/** A trace with a dip, the moment marked, and the window kept either side. */
function TimelineArt() {
  const wave = Array.from({ length: 121 }, (_, i) => {
    const x = 20 + i * 3.2;
    const t = i / 120;
    const dip = Math.exp(-(((t - 0.44) / 0.02) ** 2)) * 34;
    const y = 72 + Math.sin(i * 0.55) * 4 + Math.sin(i * 1.7) * 2 + dip;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  return (
    <svg viewBox="0 0 424 182" className="w-full" role="img" aria-label="Thirty seconds before and after the marked moment are kept">
      <defs>
        <linearGradient id="rp-keep" x1="0" x2="1">
          <stop offset="0" stopColor="#22d3ee" stopOpacity="0.02" />
          <stop offset="0.5" stopColor="#22d3ee" stopOpacity="0.12" />
          <stop offset="1" stopColor="#22d3ee" stopOpacity="0.02" />
        </linearGradient>
      </defs>
      <rect x="20" y="30" width="384" height="96" rx="10" fill="url(#rp-keep)" stroke="#ffffff14" />
      <polyline points={wave} fill="none" stroke="#22d3ee" strokeWidth="2" strokeLinejoin="round" />
      <circle cx={20 + 53 * 3.2} cy="105" r="13" fill="none" stroke="#fbbf24" strokeWidth="1.5" strokeDasharray="3 3" />
      <text x={20 + 53 * 3.2} y="142" textAnchor="middle" fill="#fbbf24" fontSize="10">the stumble</text>
      <line x1="212" x2="212" y1="20" y2="132" stroke="#f43f5e" strokeWidth="2" />
      <circle cx="212" cy="20" r="6" fill="#f43f5e" />
      <text x="222" y="24" fill="#fda4af" fontSize="11" fontWeight="600">You press Mark</text>
      <g fill="#94a3b8" fontSize="10">
        <path d="M24 174 H206 M24 170 V178 M206 170 V178" stroke="#64748b" />
        <text x="115" y="166" textAnchor="middle">30 s already in memory</text>
        <path d="M218 174 H400 M218 170 V178 M400 170 V178" stroke="#64748b" />
        <text x="309" y="166" textAnchor="middle">30 s more recorded</text>
      </g>
    </svg>
  );
}

// ---- viewer --------------------------------------------------------------

interface Row {
  pid: number;
  name: string;
  unit: string;
  points: [number, number][];
  lo: number;
  hi: number;
  change?: ReplayChange;
}

function valueAt(points: [number, number][], t: number): number | undefined {
  if (!points.length) return undefined;
  let lo = 0;
  let hi = points.length - 1;
  if (t <= points[0]![0]) return points[0]![1];
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (points[mid]![0] <= t) lo = mid;
    else hi = mid - 1;
  }
  return points[lo]![1];
}

function Viewer({ replay, onDelete }: { replay: ReplayData; onDelete: () => void }) {
  const { catalog } = useDiag();
  const start = replay.frames[0]?.t_ms ?? replay.mark_ms - 30_000;
  const end = Math.max(replay.frames[replay.frames.length - 1]?.t_ms ?? replay.mark_ms, replay.mark_ms + 1000);
  const span = Math.max(1, end - start);
  const [cursor, setCursor] = useState(replay.mark_ms);
  const [playing, setPlaying] = useState(false);
  const [focusPid, setFocusPid] = useState<number | null>(null);
  const plot = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Start just before the first change, so pressing play shows it.
    const first = replay.changes.length ? Math.min(...replay.changes.map((c) => c.offset_ms)) : 0;
    setCursor(Math.max(start, replay.mark_ms + first - 3000));
    setPlaying(false);
    setFocusPid(null);
  }, [replay, start]);

  useEffect(() => {
    if (!playing) return;
    let raf = 0;
    let last = performance.now();
    const step = (now: number) => {
      const dt = now - last;
      last = now;
      setCursor((c) => {
        const next = c + dt;
        if (next >= end) {
          setPlaying(false);
          return end;
        }
        return next;
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [playing, end]);

  const rows = useMemo<Row[]>(() => {
    const byPid = new Map<number, [number, number][]>();
    for (const f of replay.frames) {
      for (const [pid, v] of f.values) {
        const arr = byPid.get(pid) ?? [];
        arr.push([f.t_ms, v]);
        byPid.set(pid, arr);
      }
    }
    const rank = (pid: number) => {
      const i = replay.changes.findIndex((c) => c.pid === pid);
      if (i >= 0) return i;
      const j = ROW_ORDER.indexOf(pid);
      return 100 + (j < 0 ? 99 : j);
    };
    return [...byPid.entries()]
      .sort((a, b) => rank(a[0]) - rank(b[0]))
      .map(([pid, points]) => {
        const info = catalog?.pids.find((p) => p.pid === pid);
        let lo = Infinity;
        let hi = -Infinity;
        for (const [, v] of points) {
          lo = Math.min(lo, v);
          hi = Math.max(hi, v);
        }
        const pad = Math.max((hi - lo) * 0.12, 0.5);
        return {
          pid,
          name: info?.name ?? `PID ${pid.toString(16).toUpperCase()}`,
          unit: info?.unit ?? "",
          points,
          lo: lo - pad,
          hi: hi + pad,
          change: replay.changes.find((c) => c.pid === pid),
        };
      });
  }, [replay, catalog]);

  const x = (t: number) => ((t - start) / span) * 100;
  const seek = (clientX: number) => {
    // Measure the plot area itself, not the labels and values beside it.
    const el = plot.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const f = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
    setCursor(start + f * span);
  };
  const rel = (cursor - replay.mark_ms) / 1000;
  const faults = replay.engine_stored.length + replay.engine_pending.length + replay.modules.reduce((n, m) => n + m.codes.length, 0);

  return (
    <div className="min-w-0 space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-lg font-semibold tracking-tight">{replay.vehicle}</div>
          <div className="text-xs text-muted-foreground">
            Marked {new Date(replay.mark_ms).toLocaleString()} · {replay.vin || "VIN not read"}
            {replay.simulated && " · demo car"}
          </div>
        </div>
        <Button variant="outline" size="sm" onClick={onDelete}>
          <Trash2 /> Delete
        </Button>
      </div>

      <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
        <div className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">What changed just before you pressed</div>
        {replay.changes.length === 0 ? (
          <p className="mt-2 text-sm text-muted-foreground">
            No value moved outside its normal wobble in the 8 seconds before you pressed. The problem may not show in engine live data; check the fault memory below.
          </p>
        ) : (
          <ul className="mt-3 grid gap-2 md:grid-cols-2">
            {replay.changes.slice(0, 6).map((c) => (
              <li key={c.pid}>
                <button
                  onClick={() => {
                    setFocusPid(focusPid === c.pid ? null : c.pid);
                    setCursor(replay.mark_ms + c.offset_ms);
                    setPlaying(false);
                  }}
                  className={cn(
                    "flex w-full items-start gap-2.5 rounded-xl border px-3 py-2.5 text-left text-sm transition",
                    focusPid === c.pid ? "border-amber-400/40 bg-amber-400/[0.08]" : "border-white/[0.06] bg-black/20 hover:border-white/15",
                  )}
                >
                  <span className="mt-1.5 size-2 shrink-0 rounded-full bg-amber-300" />
                  <span>{c.text}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        <p className="mt-3 text-[11px] text-muted-foreground">
          These are facts from the readings, not a diagnosis. Use them with the fault codes and the Repair Copilot to decide what to check.
        </p>
      </section>

      <section className="rounded-2xl border border-white/[0.07] bg-black/20">
        <div className="flex flex-wrap items-center gap-3 border-b border-white/[0.06] px-4 py-2.5">
          <Button size="sm" variant="outline" onClick={() => (cursor >= end ? (setCursor(start), setPlaying(true)) : setPlaying(!playing))} aria-label={playing ? "Pause" : "Play"}>
            {playing ? <Pause /> : <Play />} {playing ? "Pause" : "Play"}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setCursor(replay.mark_ms)}>
            <Rewind /> Jump to mark
          </Button>
          <span className={cn("ml-auto font-mono text-sm tabular-nums", Math.abs(rel) < 0.25 ? "text-rose-300" : "text-foreground")}>
            {rel === 0 ? "at the mark" : `${rel > 0 ? "+" : "−"}${Math.abs(rel).toFixed(1)} s`}
          </span>
        </div>

        <div
          className="relative cursor-ew-resize touch-none select-none"
          onPointerDown={(e) => {
            (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
            setPlaying(false);
            seek(e.clientX);
          }}
          onPointerMove={(e) => e.buttons === 1 && seek(e.clientX)}
        >
          <div ref={plot} className="pointer-events-none absolute inset-y-0 left-[180px] right-[92px] md:left-[220px]">
            <div className="absolute inset-y-0 w-0.5 bg-rose-500/70" style={{ left: `${x(replay.mark_ms)}%` }} />
            <div className="absolute inset-y-0 w-px bg-white/80 shadow-[0_0_8px_rgba(255,255,255,0.6)]" style={{ left: `${x(cursor)}%` }} />
          </div>
          <div className="divide-y divide-white/[0.04]">
            {rows.map((r) => (
              <TraceRow key={r.pid} row={r} start={start} span={span} cursor={cursor} dim={focusPid !== null && focusPid !== r.pid} />
            ))}
          </div>
        </div>
        <div className="flex justify-between px-4 py-2 pl-[180px] pr-[92px] text-[10px] text-muted-foreground md:pl-[220px]">
          <span>−{Math.round((replay.mark_ms - start) / 1000)} s</span>
          <span className="text-rose-300">mark</span>
          <span>+{Math.round((end - replay.mark_ms) / 1000)} s</span>
        </div>
      </section>

      <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
        <div className="flex items-center justify-between gap-2">
          <div className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Fault memory right after</div>
          <span className="text-xs text-muted-foreground">{faults} {faults === 1 ? "fault" : "faults"}</span>
        </div>
        <div className="mt-3 grid gap-3 md:grid-cols-2">
          <CodeBox title="Engine (OBD-II)" stored={replay.engine_stored} pending={replay.engine_pending} />
          {replay.modules
            .filter((m) => m.codes.length > 0)
            .map((m) => (
              <CodeBox key={m.address} title={m.name} stored={m.codes} />
            ))}
        </div>
        {replay.modules.length > 0 && (
          <p className="mt-3 flex items-center gap-1.5 text-xs text-muted-foreground">
            <Cpu className="size-3.5" />
            {replay.modules.filter((m) => m.present).length} of {replay.modules.length} modules answered;{" "}
            {replay.modules.filter((m) => m.present && m.codes.length === 0).length} had no faults stored.
          </p>
        )}
      </section>
    </div>
  );
}

function TraceRow({ row, start, span, cursor, dim }: { row: Row; start: number; span: number; cursor: number; dim: boolean }) {
  const h = 40;
  const pts = row.points
    .map(([t, v]) => `${(((t - start) / span) * 1000).toFixed(1)},${(h - ((v - row.lo) / (row.hi - row.lo)) * h).toFixed(1)}`)
    .join(" ");
  const v = valueAt(row.points, cursor);
  const hot = !!row.change;
  return (
    <div className={cn("grid grid-cols-[180px_minmax(0,1fr)_92px] items-center md:grid-cols-[220px_minmax(0,1fr)_92px]", dim && "opacity-35")}>
      <div className="flex min-w-0 items-center gap-2 px-4 py-2">
        <span className={cn("size-1.5 shrink-0 rounded-full", hot ? "bg-amber-300" : "bg-cyan-400/60")} />
        <span className={cn("truncate text-xs", hot ? "font-medium text-foreground" : "text-muted-foreground")}>{row.name}</span>
      </div>
      <svg viewBox={`0 0 1000 ${h}`} preserveAspectRatio="none" className="h-10 w-full">
        <polyline points={pts} fill="none" stroke={hot ? "#fbbf24" : "#22d3ee"} strokeOpacity={hot ? 1 : 0.75} strokeWidth={1.5} vectorEffect="non-scaling-stroke" strokeLinejoin="round" />
      </svg>
      <div className="px-3 text-right font-mono text-xs tabular-nums">
        {fmt(v, digitsFor(row.unit))} <span className="text-muted-foreground">{row.unit}</span>
      </div>
    </div>
  );
}

function CodeBox({ title, stored, pending = [] }: { title: string; stored: ReplayCode[]; pending?: ReplayCode[] }) {
  const all = [...stored.map((c) => ({ ...c, kind: "stored" })), ...pending.map((c) => ({ ...c, kind: "pending" }))];
  return (
    <div className="rounded-xl border border-white/[0.06] bg-black/20 p-3">
      <div className="text-sm font-medium">{title}</div>
      {all.length === 0 ? (
        <div className="mt-1 text-xs text-emerald-300/90">No faults stored</div>
      ) : (
        <ul className="mt-2 space-y-1.5">
          {all.map((c) => (
            <li key={c.kind + c.code} className="text-xs">
              <span className="font-mono font-medium text-amber-200">{c.code}</span>
              {c.kind === "pending" && <span className="ml-1.5 rounded bg-white/[0.06] px-1 py-px text-[10px] text-muted-foreground">pending</span>}
              <span className="ml-2 text-muted-foreground">{c.description ?? "No description in the app's library yet."}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
