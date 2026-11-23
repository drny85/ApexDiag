/**
 * Repair Copilot: pick a fault, see the likely causes with what each fix
 * costs in time and money, work through a test plan where the app measures
 * what it can, then prove the fix and save it to the car's service history.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  BookOpen,
  CheckCircle2,
  ChevronRight,
  CircleDashed,
  ClipboardCheck,
  Clock,
  Gauge,
  Hammer,
  Loader2,
  Package,
  Play,
  Plug,
  Save,
  Search,
  ShieldAlert,
  SkipForward,
  Sparkles,
  Wrench,
  XCircle,
} from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CheckArt, DifficultyMeter } from "@/components/illustrations/Checks";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type CheckId, type CheckResult, type Playbook, type RepairRecord, type StepRecord, type Verdict } from "@/lib/tauri";
import { quickTestFaults } from "@/pages/QuickTest";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

interface Fault {
  code: string;
  module: string | null;
  description: string | null;
  source: string;
}

const VERDICT: Record<Verdict, { label: string; icon: typeof CheckCircle2; tone: string; ring: string }> = {
  pass: { label: "Pass", icon: CheckCircle2, tone: "text-emerald-300", ring: "border-emerald-400/25 bg-emerald-400/[0.06]" },
  warn: { label: "Borderline", icon: AlertTriangle, tone: "text-amber-300", ring: "border-amber-400/25 bg-amber-400/[0.06]" },
  fail: { label: "Fail", icon: XCircle, tone: "text-rose-300", ring: "border-rose-400/25 bg-rose-400/[0.06]" },
  unknown: { label: "Can't tell yet", icon: CircleDashed, tone: "text-sky-300", ring: "border-sky-400/20 bg-sky-400/[0.05]" },
};

export default function Copilot({ go }: { go: (p: PageId) => void }) {
  const { connection, focus, setFocus } = useDiag();
  const [faults, setFaults] = useState<Fault[]>([]);
  const [selected, setSelected] = useState<Fault | null>(null);
  const [lookup, setLookup] = useState("");
  const [history, setHistory] = useState<RepairRecord[]>([]);

  const loadHistory = useCallback(() => {
    api.listRepairs().then(setHistory).catch(() => setHistory([]));
  }, []);

  useEffect(() => {
    if (!connection) return;
    (async () => {
      const list: Fault[] = [];
      try {
        const c = await api.readObdCodes();
        for (const [group, label] of [
          ["stored", "Engine · confirmed"],
          ["pending", "Engine · pending"],
          ["permanent", "Engine · permanent"],
        ] as const) {
          for (const d of c[group]) {
            if (!list.some((f) => f.code === d.code && f.module === null)) list.push({ code: d.code, module: null, description: d.description, source: label });
          }
        }
      } catch {
        /* no OBD codes available */
      }
      for (const { module, fault } of quickTestFaults(connection.vin)) {
        list.push({ code: fault.code, module: module.name, description: fault.description, source: `${module.name} · Quick Test` });
      }
      setFaults(list);
      setSelected((cur) => cur ?? (focus ? { ...focus, description: null, source: "Picked" } : (list[0] ?? null)));
      setFocus(null);
    })();
    loadHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connection]);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Repair Copilot" subtitle="From fault code to proven fix: likely causes, what they cost, guided tests and a service record." />
        <Empty icon={Wrench} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> Connect
          </Button>
        </Empty>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Repair Copilot"
        subtitle="From fault code to proven fix. See what usually causes it and what each fix costs, let the app measure what it can, then prove the repair and keep the record."
      />
      <div className="grid gap-5 xl:grid-cols-[280px_minmax(0,1fr)]">
        <aside className="space-y-4 self-start">
          <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-3">
            <h3 className="px-1 pb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Faults on this car</h3>
            {faults.length === 0 && <p className="px-1 py-2 text-sm text-muted-foreground">No engine codes stored. Run a Quick Test to include every module.</p>}
            <div className="space-y-1">
              {faults.map((f) => {
                const active = selected?.code === f.code && selected?.module === f.module;
                return (
                  <button
                    key={`${f.module}-${f.code}`}
                    onClick={() => setSelected(f)}
                    className={cn(
                      "flex w-full items-start gap-2 rounded-xl px-2.5 py-2 text-left transition",
                      active ? "bg-cyan-400/10 ring-1 ring-inset ring-cyan-400/30" : "hover:bg-white/[0.04]",
                    )}
                  >
                    <span className="mt-0.5 font-mono text-[13px] font-semibold tracking-wider">{f.code}</span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-xs text-foreground/80">{f.description ?? "No description"}</span>
                      <span className="block text-[10px] text-muted-foreground">{f.source}</span>
                    </span>
                    {active && <ChevronRight className="mt-0.5 size-3.5 text-cyan-300" />}
                  </button>
                );
              })}
            </div>
            <Button variant="ghost" size="sm" className="mt-2 w-full" onClick={() => go("quicktest")}>
              Find module faults with a Quick Test
            </Button>
          </section>
          <form
            className="flex gap-2 rounded-2xl border border-white/[0.07] bg-white/[0.02] p-3"
            onSubmit={(e) => {
              e.preventDefault();
              const code = lookup.trim().toUpperCase();
              if (code) setSelected({ code, module: null, description: null, source: "Looked up" });
            }}
          >
            <div className="relative flex-1">
              <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <input
                value={lookup}
                onChange={(e) => setLookup(e.target.value)}
                placeholder="Look up any code"
                className="h-8 w-full rounded-lg bg-black/30 pl-8 pr-2 font-mono text-sm uppercase outline-none ring-1 ring-white/10 placeholder:font-sans placeholder:normal-case placeholder:text-muted-foreground focus:ring-cyan-400/40"
              />
            </div>
            <Button size="sm" type="submit" variant="secondary">
              Open
            </Button>
          </form>
          <History records={history} />
        </aside>

        {selected ? (
          <PlanView key={`${selected.module}-${selected.code}`} fault={selected} simulated={connection.simulated} onSaved={loadHistory} />
        ) : (
          <Empty icon={Sparkles} title="Pick a fault">
            Choose a fault on the left, or type any code to see its plan.
          </Empty>
        )}
      </div>
    </div>
  );
}

function PlanView({ fault, simulated, onSaved }: { fault: Fault; simulated: boolean; onSaved: () => void }) {
  const [plan, setPlan] = useState<Playbook | null>(null);
  const [records, setRecords] = useState<Record<string, StepRecord>>({});
  const [running, setRunning] = useState<string | null>(null);
  const [verification, setVerification] = useState<CheckResult[]>([]);
  const [verifying, setVerifying] = useState(false);
  const [fix, setFix] = useState("");
  const [parts, setParts] = useState("");
  const [saved, setSaved] = useState<RepairRecord | null>(null);
  // Guards against a double click writing the record twice before state updates.
  const saving = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [demoFixed, setDemoFixed] = useState<boolean | null>(null);
  const started = useMemo(() => Date.now(), []);

  useEffect(() => {
    api.copilotPlan(fault.code).then(setPlan).catch((e) => setError(asCmdError(e).message));
  }, [fault.code]);

  if (!plan) return <div className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2 className="size-4 animate-spin" /> Building the plan…</div>;

  const setStep = (id: string, r: Partial<StepRecord>) =>
    setRecords((x) => {
      const step = plan.steps.find((s) => s.id === id)!;
      return { ...x, [id]: { step_id: id, title: step.title, outcome: "done", note: "", result: null, ...x[id], ...r } };
    });

  const runCheck = async (id: string, check: CheckId) => {
    setRunning(id);
    setError(null);
    try {
      const result = await api.copilotRunCheck(check);
      setStep(id, { result, outcome: result.verdict });
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setRunning(null);
    }
  };

  const verify = async () => {
    setVerifying(true);
    setError(null);
    try {
      const out: CheckResult[] = [];
      for (const c of plan.verify_checks) out.push(await api.copilotRunCheck(c));
      setVerification(out);
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setVerifying(false);
    }
  };

  const done = plan.steps.filter((s) => records[s.id]).length;
  const verified = verification.length > 0 && verification.every((v) => v.verdict === "pass");
  const before = (check: CheckId) => Object.values(records).find((r) => r.result?.check === check)?.result ?? null;

  const save = async () => {
    if (saving.current || saved) return;
    saving.current = true;
    setError(null);
    try {
      const rec = await api.saveRepair({
        id: "",
        vin: "",
        code: fault.code,
        module: fault.module,
        title: plan.title,
        started_ms: started,
        finished_ms: Date.now(),
        steps: plan.steps.map((s) => records[s.id] ?? { step_id: s.id, title: s.title, outcome: "skipped", note: "", result: null }),
        fix,
        parts,
        verification,
        verified,
        simulated,
      });
      setSaved(rec);
      onSaved();
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      saving.current = false;
    }
  };

  return (
    <div className="min-w-0 space-y-5">
      {/* header */}
      <section className="relative overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-violet-500/[0.08] via-transparent to-transparent p-6">
        <div className="absolute -right-24 -top-24 size-72 rounded-full bg-violet-400/10 blur-3xl" />
        <div className="relative">
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded-lg bg-black/30 px-2.5 py-1 font-mono text-lg font-semibold tracking-wider ring-1 ring-white/10">{fault.code}</span>
            {fault.module && <Badge variant="muted">{fault.module}</Badge>}
            {plan.id === "general" ? <Badge variant="info">General plan</Badge> : <Badge variant="ok">Guided plan</Badge>}
          </div>
          <h2 className="mt-3 text-2xl font-semibold tracking-tight">{plan.title}</h2>
          {fault.description && <p className="mt-0.5 text-sm text-foreground/80">{fault.description}</p>}
          <p className="mt-3 max-w-3xl text-sm leading-relaxed text-muted-foreground">{plan.meaning}</p>
          {plan.id === "general" && (
            <p className="mt-3 max-w-3xl text-xs text-sky-200/80">
              There is no guided plan for this exact code yet. These steps rule out the usual suspects first; the maker's own test plan has the rest.
            </p>
          )}
          <div className="mt-5 flex items-center gap-3 text-xs text-muted-foreground">
            <span>
              {done} of {plan.steps.length} steps done
            </span>
            <div className="h-1.5 w-48 overflow-hidden rounded-full bg-white/[0.06]">
              <div className="h-full rounded-full bg-gradient-to-r from-violet-400 to-cyan-300 transition-[width]" style={{ width: `${(done / plan.steps.length) * 100}%` }} />
            </div>
          </div>
        </div>
      </section>

      {error && <p className="text-sm text-rose-300">{error}</p>}

      {/* causes */}
      {plan.causes.length > 0 && (
        <section>
          <SectionTitle icon={BookOpen} title="Common fixes, most likely first" hint="Costs are rough US estimates for parts and shop labour. They vary a lot by car and shop." />
          <div className="grid items-start gap-3 lg:grid-cols-2">
            {plan.causes.map((c, i) => (
              <div key={c.name} className="flex flex-col rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
                <div className="flex items-start gap-3">
                  <span className="grid size-7 shrink-0 place-items-center rounded-full bg-white/5 text-xs font-semibold ring-1 ring-white/10">{i + 1}</span>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{c.name}</span>
                      <Badge variant={c.likelihood === "common" ? "warn" : "muted"}>{c.likelihood === "common" ? "Common" : c.likelihood === "possible" ? "Possible" : "Rare"}</Badge>
                    </div>
                    <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{c.why}</p>
                  </div>
                </div>
                <div className="mt-3 rounded-xl bg-black/25 p-3">
                  <div className="flex items-start gap-2 text-sm">
                    <Hammer className="mt-0.5 size-3.5 shrink-0 text-cyan-300" />
                    <span>{c.fix.action}</span>
                  </div>
                  <div className="mt-2.5 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[11px] text-muted-foreground">
                    <DifficultyMeter difficulty={c.fix.difficulty} />
                    <span className="font-medium text-foreground/85">
                      ${c.fix.cost_usd[0].toLocaleString()}-{c.fix.cost_usd[1].toLocaleString()}
                    </span>
                    <span className="inline-flex items-center gap-1">
                      <Clock className="size-3" /> {c.fix.time}
                    </span>
                  </div>
                  <div className="mt-2 grid gap-1 text-[11px] text-muted-foreground sm:grid-cols-2">
                    <span className="inline-flex items-start gap-1">
                      <Package className="mt-0.5 size-3 shrink-0" /> {c.fix.parts}
                    </span>
                    <span className="inline-flex items-start gap-1">
                      <Wrench className="mt-0.5 size-3 shrink-0" /> {c.fix.tools}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* steps */}
      <section>
        <SectionTitle icon={ClipboardCheck} title="Test plan" hint="Confirm the cause before buying parts. The app measures the steps marked Auto." />
        <ol className="space-y-3">
          {plan.steps.map((s, i) => {
            const rec = records[s.id];
            const auto = s.kind.kind === "auto" ? s.kind.check : null;
            return (
              <li key={s.id} className={cn("relative rounded-2xl border bg-white/[0.02] p-4", rec ? "border-white/[0.1]" : "border-white/[0.07]")}>
                <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_170px]">
                  <div>
                    <div className="flex items-center gap-2.5">
                      <span
                        className={cn(
                          "grid size-6 place-items-center rounded-full text-[11px] font-semibold ring-1",
                          rec ? "bg-emerald-400/15 text-emerald-200 ring-emerald-400/30" : "bg-white/5 ring-white/10",
                        )}
                      >
                        {rec ? <CheckCircle2 className="size-3.5" /> : i + 1}
                      </span>
                      <span className="font-medium">{s.title}</span>
                      {auto ? <Badge variant="info"><Gauge /> Auto</Badge> : <Badge variant="muted"><Hammer /> Hands-on</Badge>}
                    </div>
                    <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{s.detail}</p>
                    {s.tools && (
                      <p className="mt-2 flex items-center gap-1.5 text-xs text-muted-foreground">
                        <Wrench className="size-3" /> {s.tools}
                      </p>
                    )}
                    {s.caution && (
                      <p className="mt-2 flex items-start gap-1.5 rounded-lg bg-amber-400/[0.06] px-2.5 py-1.5 text-xs text-amber-100/90 ring-1 ring-inset ring-amber-400/20">
                        <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-amber-300" /> {s.caution}
                      </p>
                    )}
                    {rec?.result && <ResultCard r={rec.result} />}
                    <div className="mt-3 flex flex-wrap items-center gap-2">
                      {auto ? (
                        <Button size="sm" onClick={() => runCheck(s.id, auto)} disabled={running !== null}>
                          {running === s.id ? <Loader2 className="animate-spin" /> : <Play />}
                          {running === s.id ? "Measuring" : rec ? "Measure again" : "Run check"}
                        </Button>
                      ) : (
                        <>
                          <Button size="sm" variant={rec?.outcome === "done" ? "secondary" : "default"} onClick={() => setStep(s.id, { outcome: "done" })}>
                            <CheckCircle2 /> Done
                          </Button>
                          <Button size="sm" variant="ghost" onClick={() => setStep(s.id, { outcome: "skipped" })}>
                            <SkipForward /> Skip
                          </Button>
                        </>
                      )}
                      {rec && (
                        <input
                          value={rec.note}
                          onChange={(e) => setStep(s.id, { note: e.target.value })}
                          placeholder="What did you find?"
                          className="h-8 min-w-[200px] flex-1 rounded-lg bg-black/30 px-2.5 text-xs outline-none ring-1 ring-white/10 placeholder:text-muted-foreground focus:ring-cyan-400/40"
                        />
                      )}
                    </div>
                  </div>
                  {auto && (
                    <div className="hidden place-items-center rounded-xl bg-black/25 p-2 md:grid">
                      <CheckArt check={auto} className="w-full" />
                    </div>
                  )}
                </div>
              </li>
            );
          })}
        </ol>
      </section>

      {/* verify and save */}
      <section className="rounded-2xl border border-emerald-400/15 bg-gradient-to-br from-emerald-400/[0.05] to-transparent p-5">
        <SectionTitle icon={CheckCircle2} title="Prove the fix" hint="" />
        <p className="-mt-1 text-sm leading-relaxed text-muted-foreground">{plan.verify}</p>
        {simulated && (
          <div className="mt-3 flex flex-wrap items-center gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-3 text-xs">
            <span className="text-amber-100/90">Practice car: pretend the repair was done, so the re-test has something to show.</span>
            <Button
              size="sm"
              variant="outline"
              onClick={async () => {
                setDemoFixed(await api.simApplyRepair(fault.code).catch(() => false));
              }}
            >
              Demo: apply the repair
            </Button>
            {demoFixed === true && <span className="text-emerald-300">Repaired on the practice car.</span>}
            {demoFixed === false && <span className="text-muted-foreground">Nothing to repair for this code on this car.</span>}
          </div>
        )}
        {plan.verify_checks.length > 0 && (
          <div className="mt-4">
            <Button onClick={verify} disabled={verifying}>
              {verifying ? <Loader2 className="animate-spin" /> : <Play />} Re-test now
            </Button>
            {verification.length > 0 && (
              <div className="mt-3 space-y-2">
                {verification.map((v) => {
                  const b = before(v.check);
                  return (
                    <div key={v.check} className="grid gap-2 md:grid-cols-2">
                      {b ? <ResultCard r={b} label="Before" /> : <div className="rounded-xl border border-dashed border-white/10 p-3 text-xs text-muted-foreground">No measurement before the repair.</div>}
                      <ResultCard r={v} label="After" />
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
        <div className="mt-5 grid gap-3 md:grid-cols-2">
          <label className="text-xs text-muted-foreground">
            What was found and done
            <textarea
              value={fix}
              onChange={(e) => setFix(e.target.value)}
              rows={3}
              placeholder="e.g. Crankcase vent valve diaphragm split; replaced valve and hoses."
              className="mt-1 w-full resize-none rounded-lg bg-black/30 p-2.5 text-sm text-foreground outline-none ring-1 ring-white/10 placeholder:text-muted-foreground/70 focus:ring-cyan-400/40"
            />
          </label>
          <label className="text-xs text-muted-foreground">
            Parts used
            <textarea
              value={parts}
              onChange={(e) => setParts(e.target.value)}
              rows={3}
              placeholder="Part names and numbers"
              className="mt-1 w-full resize-none rounded-lg bg-black/30 p-2.5 text-sm text-foreground outline-none ring-1 ring-white/10 placeholder:text-muted-foreground/70 focus:ring-cyan-400/40"
            />
          </label>
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-3">
          <Button onClick={save} disabled={!!saved}>
            <Save /> {saved ? "Saved to service history" : "Save to service history"}
          </Button>
          {verification.length > 0 &&
            (verified ? (
              <Badge variant="ok"><CheckCircle2 /> Fix verified by re-test</Badge>
            ) : (
              <Badge variant="warn"><AlertTriangle /> Re-test not passing yet</Badge>
            ))}
        </div>
      </section>
    </div>
  );
}

function ResultCard({ r, label }: { r: CheckResult; label?: string }) {
  const v = VERDICT[r.verdict];
  return (
    <div className={cn("mt-3 rounded-xl border p-3", v.ring)}>
      <div className="flex items-center gap-2 text-sm font-medium">
        <v.icon className={cn("size-4", v.tone)} />
        {label && <span className="text-xs uppercase tracking-wider text-muted-foreground">{label}</span>}
        <span>{r.headline}</span>
      </div>
      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{r.detail}</p>
      {r.readings.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {r.readings.map((x) => (
            <span key={x.name} className="rounded-md bg-black/30 px-2 py-0.5 text-[11px] ring-1 ring-inset ring-white/10">
              <span className="text-muted-foreground">{x.name}</span>{" "}
              <span className="font-medium tabular-nums">
                {x.value.toLocaleString(undefined, { maximumFractionDigits: 2 })} {x.unit}
              </span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function History({ records }: { records: RepairRecord[] }) {
  return (
    <section className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-3">
      <h3 className="px-1 pb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Service history</h3>
      {records.length === 0 ? (
        <p className="px-1 text-xs text-muted-foreground">Repairs you save appear here, kept with the car's VIN.</p>
      ) : (
        <ul className="space-y-1.5">
          {records.map((r) => (
            <li key={r.id} className="rounded-xl bg-black/20 px-2.5 py-2">
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs font-semibold">{r.code}</span>
                {r.verified ? <Badge variant="ok">Verified</Badge> : <Badge variant="muted">Not verified</Badge>}
              </div>
              <div className="mt-0.5 truncate text-xs text-foreground/80">{r.fix || r.title}</div>
              <div className="text-[10px] text-muted-foreground">{new Date(r.finished_ms).toLocaleString()}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function SectionTitle({ icon: Icon, title, hint }: { icon: typeof BookOpen; title: string; hint: string }) {
  return (
    <div className="mb-3">
      <h3 className="flex items-center gap-2 text-sm font-semibold">
        <Icon className="size-4 text-cyan-300" /> {title}
      </h3>
      {hint && <p className="mt-0.5 text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}
