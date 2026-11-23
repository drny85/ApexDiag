import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, Camera, Sparkles, CheckCircle2, CircleDashed, Clock, Eraser, Leaf, Lock, Plug, RefreshCw, ShieldCheck } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import GateDialog from "@/components/GateDialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useDiag } from "@/lib/store";
import { useT, Tr } from "@/lib/i18n";
import { api, asCmdError, type Dtc, type EmissionsReport, type ObdCodes, type PidInfo } from "@/lib/tauri";
import { digitsFor, fmt } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

const GROUPS = [
  {
    key: "stored" as const,
    title: "Confirmed",
    icon: AlertTriangle,
    tone: "text-amber-300",
    explain: "Faults the engine module has confirmed. These turn the check-engine light on.",
  },
  {
    key: "pending" as const,
    title: "Pending",
    icon: Clock,
    tone: "text-sky-300",
    explain: "Seen once this drive but not yet confirmed. If it happens again it becomes confirmed.",
  },
  {
    key: "permanent" as const,
    title: "Permanent",
    icon: Lock,
    tone: "text-rose-300",
    explain: "Can't be cleared with a scan tool. They clear themselves once the car proves the fault is fixed.",
  },
];

export default function Codes({ go }: { go: (p: PageId) => void }) {
  const { connection, catalog, revision, setFocus } = useDiag();
  const t = useT();
  const openFix = (code: string) => {
    setFocus({ code, module: null });
    go("copilot");
  };
  const [codes, setCodes] = useState<ObdCodes | null>(null);
  const [emissions, setEmissions] = useState<EmissionsReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setCodes(await api.readObdCodes());
      setEmissions(await api.readEmissions().catch(() => null));
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (connection) load();
  }, [connection, load, revision]);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Fault Codes" subtitle="Emissions and engine trouble codes, explained." />
        <Empty icon={AlertTriangle} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> <Tr>Connect</Tr>
          </Button>
        </Empty>
      </div>
    );
  }

  const total = codes ? codes.stored.length + codes.pending.length + codes.permanent.length : 0;
  const clearFn = catalog?.functions.find((f) => f.id === "clear_codes");

  return (
    <div>
      <PageHeader
        title="Fault Codes"
        subtitle={t("Generic OBD-II codes, emissions readiness and freeze frame from the engine module. For {maker} codes from every module (body, brakes, transmission), open Modules or run a Quick Test.", { maker: connection.vehicle.pack.make === "generic" ? t("maker") : connection.vehicle.pack.brand })}
        actions={
          <>
            <Button variant="outline" onClick={load} disabled={loading}>
              <RefreshCw className={cn(loading && "animate-spin")} /> Re-read
            </Button>
            <Button variant="outline" onClick={() => setClearing(true)} disabled={!codes || total === 0}>
              <Eraser /> Clear codes
            </Button>
          </>
        }
      />

      {codes && (
        <div
          className={cn(
            "mb-5 flex items-center gap-4 rounded-2xl border p-5",
            codes.mil?.mil_on ? "border-amber-400/25 bg-gradient-to-r from-amber-400/[0.09] to-transparent" : "border-emerald-400/20 bg-emerald-400/[0.05]",
          )}
        >
          <div className={cn("grid size-12 place-items-center rounded-xl", codes.mil?.mil_on ? "bg-amber-400/15" : "bg-emerald-400/15")}>
            {codes.mil?.mil_on ? <EngineIcon className="size-6 text-amber-300" /> : <ShieldCheck className="size-6 text-emerald-300" />}
          </div>
          <div>
            <div className="font-medium">{codes.mil?.mil_on ? "Check-engine light is ON" : "Check-engine light is off"}</div>
            <div className="text-sm text-muted-foreground">
              {total === 0 ? "No codes stored." : `${codes.stored.length} confirmed, ${codes.pending.length} pending, ${codes.permanent.length} permanent.`}
            </div>
          </div>
        </div>
      )}

      {error && <p className="mb-4 text-sm text-rose-300">{error}</p>}

      <div className="grid gap-4 lg:grid-cols-3">
        {GROUPS.map((g) => {
          const list: Dtc[] = codes?.[g.key] ?? [];
          return (
            <div key={g.key} className="flex flex-col rounded-2xl border border-white/[0.07] bg-white/[0.02]">
              <div className="border-b border-white/[0.06] p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 font-medium">
                    <g.icon className={cn("size-4", g.tone)} /> {g.title}
                  </div>
                  <Badge variant={list.length ? "default" : "muted"}>{list.length}</Badge>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">{g.explain}</p>
              </div>
              <div className="flex-1 space-y-2 p-3">
                {list.length === 0 ? (
                  <div className="flex items-center gap-2 px-2 py-3 text-sm text-muted-foreground">
                    <CheckCircle2 className="size-4 text-emerald-400/70" /> None
                  </div>
                ) : (
                  list.map((d) => <CodeRow key={d.code} dtc={d} onFix={() => openFix(d.code)} />)
                )}
              </div>
            </div>
          );
        })}
      </div>

      {emissions && <Emissions report={emissions} pids={catalog?.pids ?? []} />}

      <GateDialog
        fn={clearFn}
        open={clearing}
        onOpenChange={setClearing}
        targetLabel="Engine (OBD-II)"
        run={async () => {
          const saved = await api.clearObdCodes();
          return [
            `Saved ${saved.length} code${saved.length === 1 ? "" : "s"} to the Activity log: ${saved.map((d) => d.code).join(", ") || "none"}`,
            "Sent Clear Diagnostic Information (OBD-II service 04)",
            "Engine module confirmed",
            "Readiness monitors reset: drive normally to let them complete",
          ];
        }}
      />
    </div>
  );
}

function Emissions({ report, pids }: { report: EmissionsReport; pids: PidInfo[] }) {
  const r = report.readiness;
  const supported = r?.monitors.filter((m) => m.supported) ?? [];
  const incomplete = supported.filter((m) => !m.complete);
  const ff = report.freeze_frame;
  return (
    <div className="mt-5 grid gap-4 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
      <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
        <div className="flex items-center justify-between">
          <h3 className="flex items-center gap-2 font-medium">
            <Leaf className="size-4 text-emerald-300" /> Emissions readiness
          </h3>
          {r &&
            (r.mil_on ? (
              <Badge variant="danger">Would fail: check-engine light on</Badge>
            ) : (
              <Badge variant={incomplete.length ? "warn" : "ok"}>
                {incomplete.length ? `${incomplete.length} not ready` : "Ready for inspection"}
              </Badge>
            ))}
        </div>
        <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
          The engine runs self-tests (monitors) while you drive. An inspection station plugs in and checks that they have all finished. Clearing codes
          or disconnecting the battery resets them.
          {report.km_since_clear !== null && ` ${fmt(report.km_since_clear)} km driven since codes were last cleared.`}
        </p>
        {!r ? (
          <p className="mt-3 text-sm text-muted-foreground">Readiness not available.</p>
        ) : (
          <div className="mt-4 grid gap-2 sm:grid-cols-2">
            {supported.map((m) => (
              <div
                key={m.id}
                className={cn(
                  "rounded-xl border p-3",
                  m.complete ? "border-emerald-400/15 bg-emerald-400/[0.03]" : "border-amber-400/25 bg-amber-400/[0.05]",
                )}
              >
                <div className="flex items-center gap-2 text-sm font-medium">
                  {m.complete ? <CheckCircle2 className="size-4 text-emerald-400" /> : <CircleDashed className="size-4 text-amber-300" />}
                  {m.name}
                  <span className={cn("ml-auto text-[11px]", m.complete ? "text-emerald-300/80" : "text-amber-300")}>
                    {m.complete ? "Complete" : "Not yet run"}
                  </span>
                </div>
                <p className="mt-1 text-[11px] leading-snug text-muted-foreground">{m.explain}</p>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
        <h3 className="flex items-center gap-2 font-medium">
          <Camera className="size-4 text-sky-300" /> Freeze frame
        </h3>
        <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
          A snapshot the engine took the moment a code set: how fast, how hot, how much load. It tells you the conditions to reproduce the fault.
        </p>
        {!ff ? (
          <div className="mt-4 flex items-center gap-2 text-sm text-muted-foreground">
            <CheckCircle2 className="size-4 text-emerald-400/70" /> No freeze frame stored.
          </div>
        ) : (
          <>
            <div className="mt-4 flex items-center gap-2">
              <span className="text-xs text-muted-foreground">Captured when</span>
              <span className="font-mono font-semibold tracking-wider">{ff.dtc}</span>
              <span className="text-xs text-muted-foreground">set</span>
            </div>
            <dl className="mt-3 divide-y divide-white/[0.05] rounded-xl bg-black/20">
              {ff.values.map(([pid, v]) => {
                const info = pids.find((p) => p.pid === pid);
                return (
                  <div key={pid} className="flex items-center justify-between px-3 py-2 text-sm">
                    <dt className="text-muted-foreground">{info?.name ?? `PID ${pid}`}</dt>
                    <dd className="font-medium tabular-nums">
                      {fmt(v, info ? digitsFor(info.unit) : 0)} <span className="text-xs text-muted-foreground">{info?.unit}</span>
                    </dd>
                  </div>
                );
              })}
            </dl>
          </>
        )}
      </section>
    </div>
  );
}

function CodeRow({ dtc, onFix }: { dtc: Dtc; onFix: () => void }) {
  const system = { P: "Powertrain", C: "Chassis", B: "Body", U: "Network" }[dtc.code[0] ?? "P"];
  const generic = dtc.code[1] === "0";
  return (
    <div className="rounded-xl border border-white/[0.06] bg-black/20 p-3">
      <div className="flex items-center gap-2">
        <span className="font-mono text-[15px] font-semibold tracking-wider">{dtc.code}</span>
        <Badge variant="muted">{system}</Badge>
        <Badge variant="muted">{generic ? "Generic" : "Maker-specific"}</Badge>
      </div>
      <p className="mt-1.5 text-sm text-foreground/85">{dtc.description ?? "No description in the database yet."}</p>
      <Button variant="ghost" size="sm" className="-ml-2 mt-1 h-7 text-cyan-200" onClick={onFix}>
        <Sparkles /> Fix this with Repair Copilot
      </Button>
    </div>
  );
}

function EngineIcon({ className }: { className?: string }) {
  // Check-engine symbol, drawn so it isn't tied to any brand icon set.
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className}>
      <path d="M5 9h2l1-2h5l1 2h3v3h2v-2h1v6h-1v-2h-2v3h-3l-2 2H8l-1-2H5z" />
      <path d="M3 11v4M5 13H3" />
    </svg>
  );
}
