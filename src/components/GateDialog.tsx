import { useEffect, useState } from "react";
import {
  AlertTriangle,
  BatteryMedium,
  Cable,
  CheckCircle2,
  Circle,
  OctagonPause,
  History,
  Loader2,
  PlugZap,
  Power,
  ShieldAlert,
  ShieldCheck,
  Undo2,
  type LucideIcon,
} from "lucide-react";
import FunctionPreview from "@/components/illustrations/FunctionPreview";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import TierBadge from "@/components/TierBadge";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type FunctionInfo, type Requirement } from "@/lib/tauri";
import { TIER_STYLE } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * The one place every car-changing action passes through: explains the
 * function, shows each safety requirement live, collects the confirmations
 * the risk tier asks for, then runs it and shows the steps.
 */
export default function GateDialog({
  fn,
  open,
  onOpenChange,
  run,
  targetLabel,
  children,
}: {
  fn: FunctionInfo | undefined;
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** Performs the action; returns the steps to show. */
  run: () => Promise<string[]>;
  targetLabel?: string;
  /** Extra detail shown under the risks, e.g. exactly what will be written. */
  children?: React.ReactNode;
}) {
  const { vehicle, connection, refreshVehicle, bump } = useDiag();
  const [acks, setAcks] = useState<boolean[]>([]);
  const [phase, setPhase] = useState<"review" | "running" | "done" | "failed">("review");
  const [steps, setSteps] = useState<string[]>([]);
  const [shown, setShown] = useState(0);
  const [error, setError] = useState<{ message: string; unmet: Requirement[] } | null>(null);
  const [charger, setCharger] = useState(false);

  useEffect(() => {
    if (open) {
      setAcks([]);
      setPhase("review");
      setSteps([]);
      setShown(0);
      setError(null);
      setCharger(vehicle?.state.charger_confirmed ?? false);
      refreshVehicle();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (phase !== "running" || shown >= steps.length) {
      if (phase === "running" && steps.length && shown >= steps.length) setPhase("done");
      return;
    }
    const id = setTimeout(() => setShown((n) => n + 1), 850);
    return () => clearTimeout(id);
  }, [phase, shown, steps]);

  if (!fn) return null;
  const style = TIER_STYLE[fn.tier];
  const unmet = vehicle?.functions.find((f) => f.id === fn.id)?.unmet ?? fn.requirements;
  const unmetKinds = new Set(unmet.map((u) => u.kind + (u.volts ?? "")));
  const needsCharger = fn.requirements.some((r) => r.kind === "charger_connected");
  const prompts = confirmationPrompts(fn);
  const allAcked = prompts.every((_, i) => acks[i]);
  const ready = unmet.length === 0 && allAcked;

  const setAck = (i: number, v: boolean) =>
    setAcks((a) => {
      const n = [...a];
      n[i] = v;
      if (!v) for (let j = i + 1; j < n.length; j++) n[j] = false;
      return n;
    });

  const setDemoEngine = async (running: boolean) => {
    await api.simSetEngine(running).catch(() => {});
    bump();
    setTimeout(refreshVehicle, 300);
  };

  const toggleCharger = async (on: boolean) => {
    setCharger(on);
    await api.setChargerConfirmed(on).catch(() => {});
    setTimeout(refreshVehicle, 250);
  };

  const go = async () => {
    setError(null);
    setPhase("running");
    try {
      const s = await run();
      setSteps(s);
      setShown(0);
      bump();
    } catch (e) {
      const err = asCmdError(e);
      setError({ message: err.message, unmet: err.unmet });
      setPhase("failed");
      refreshVehicle();
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent side="right" className="overflow-hidden">
        <div className={cn("border-b border-white/[0.06] px-6 pb-5 pt-6", style.bg)}>
          <TierBadge tier={fn.tier} label={fn.tier_label} />
          <DialogTitle className="mt-3 text-xl">{fn.name}</DialogTitle>
          <DialogDescription className="mt-1">
            {targetLabel ? `${targetLabel} · ` : ""}
            {fn.what}
          </DialogDescription>
          {connection?.simulated && (
            <Badge variant="warn" className="mt-3">
              Demo car: nothing is sent to a real vehicle
            </Badge>
          )}
        </div>

        <div className="flex-1 space-y-5 overflow-auto px-6 py-5">
          {phase === "review" || phase === "failed" ? (
            <>
              <FunctionPreview id={fn.id} />
              <Section title="Why you'd use it">{fn.why}</Section>
              <Section title="What can go wrong" icon={<AlertTriangle className="size-3.5 text-amber-300" />} tone="warn">
                {fn.risks}
              </Section>
              <Section title="How to undo it" icon={<Undo2 className="size-3.5 text-sky-300" />}>
                {fn.undo}
              </Section>
              {children}

              {fn.requirements.length > 0 && (
                <div>
                  <h4 className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Before it can run</h4>
                  <ul className="space-y-1.5">
                    {fn.requirements.map((r) => {
                      const ok = !unmetKinds.has(r.kind + (r.volts ?? ""));
                      return (
                        <li
                          key={r.kind + (r.volts ?? "")}
                          className={cn(
                            "flex items-start gap-2.5 rounded-lg px-3 py-2 text-sm ring-1 ring-inset transition",
                            ok ? "bg-emerald-400/[0.06] ring-emerald-400/15" : "bg-rose-400/[0.06] ring-rose-400/20",
                          )}
                        >
                          <RequirementIcon kind={r.kind} ok={ok} />
                          <span className={cn("flex-1", ok ? "text-foreground/90" : "text-rose-100")}>{r.text}</span>
                          {!ok && connection?.simulated && (r.kind === "engine_off" || r.kind === "engine_running") && (
                            <button
                              onClick={() => setDemoEngine(r.kind === "engine_running")}
                              className="shrink-0 rounded-md bg-white/10 px-2 py-0.5 text-xs text-foreground transition hover:bg-white/20"
                            >
                              {r.kind === "engine_off" ? "Switch demo engine off" : "Start demo engine"}
                            </button>
                          )}
                        </li>
                      );
                    })}
                  </ul>
                  {needsCharger && (
                    <label
                      className="mt-3 flex cursor-pointer items-center gap-2.5 rounded-lg border border-white/10 px-3 py-2.5 text-sm"
                      onClick={() => toggleCharger(!charger)}
                    >
                      <Checkbox checked={charger} onChange={toggleCharger} />
                      A battery charger or power supply is connected to the car
                    </label>
                  )}
                </div>
              )}

              {prompts.length > 0 && (
                <div>
                  <h4 className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                    Confirm ({prompts.length} step{prompts.length > 1 ? "s" : ""})
                  </h4>
                  <div className="space-y-2">
                    {prompts.map((p, i) => {
                      const enabled = i === 0 || acks[i - 1];
                      return (
                        <label
                          key={p}
                          onClick={() => enabled && setAck(i, !acks[i])}
                          className={cn(
                            "flex cursor-pointer items-start gap-2.5 rounded-lg border px-3 py-2.5 text-sm transition",
                            acks[i] ? "border-cyan-400/30 bg-cyan-400/[0.05]" : "border-white/10",
                            !enabled && "pointer-events-none opacity-40",
                          )}
                        >
                          <Checkbox className="mt-px" checked={!!acks[i]} disabled={!enabled} onChange={(v) => setAck(i, v)} />
                          {p}
                        </label>
                      );
                    })}
                  </div>
                </div>
              )}

              {error && (
                <div className="rounded-xl border border-rose-400/25 bg-rose-400/[0.07] p-4 text-sm">
                  <div className="flex items-center gap-2 font-medium text-rose-200">
                    <ShieldAlert className="size-4" /> {error.message}
                  </div>
                  {error.unmet.length > 0 && (
                    <ul className="mt-2 list-disc space-y-1 pl-6 text-rose-100/80">
                      {error.unmet.map((u) => (
                        <li key={u.kind + (u.volts ?? "")}>{u.text}</li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
            </>
          ) : (
            <div className="space-y-5 py-2">
              <FunctionPreview id={fn.id} />
              <ol className="relative space-y-4 border-l border-white/10 pl-6">
                {steps.length === 0 && (
                  <li className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="size-4 animate-spin" /> Running safety checks…
                  </li>
                )}
                {steps.map((s, i) => (
                  <li key={i} className={cn("relative text-sm transition", i < shown ? "opacity-100" : "opacity-35")}>
                    <span className="absolute -left-[33px] top-0.5 grid size-4 place-items-center rounded-full bg-[oklch(0.17_0.008_265)]">
                      {i < shown ? (
                        <CheckCircle2 className="size-4 text-emerald-400" />
                      ) : i === shown ? (
                        <Loader2 className="size-4 animate-spin text-cyan-300" />
                      ) : (
                        <Circle className="size-4 text-white/25" />
                      )}
                    </span>
                    {s}
                  </li>
                ))}
              </ol>
              {phase === "done" && (
                <div className="mt-6 rounded-xl border border-emerald-400/25 bg-emerald-400/[0.07] p-4 text-sm text-emerald-100">
                  Finished. This was recorded in the Activity log.
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-white/[0.06] px-6 py-4">
          <span className="text-xs text-muted-foreground">
            {phase === "review" && !ready && (unmet.length ? `${unmet.length} condition${unmet.length > 1 ? "s" : ""} not met` : "Confirm to continue")}
          </span>
          {phase === "done" ? (
            <Button onClick={() => onOpenChange(false)}>Close</Button>
          ) : (
            <Button
              disabled={!ready || phase === "running"}
              onClick={go}
              className={cn(fn.tier !== "read" && fn.tier !== "service" && "bg-gradient-to-r from-orange-400 to-rose-400 text-black hover:opacity-90")}
            >
              {phase === "running" ? <Loader2 className="animate-spin" /> : null}
              {phase === "failed" ? "Try again" : `Run ${fn.name.toLowerCase()}`}
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

const REQUIREMENT_ICON: Record<string, LucideIcon> = {
  stationary: OctagonPause,
  engine_off: Power,
  engine_running: Power,
  battery_at_least: BatteryMedium,
  charger_connected: PlugZap,
  wired_adapter: Cable,
  backup_taken: History,
  job_verified: ShieldCheck,
};

/** A picture of the condition, which pops into place when it is met. */
function RequirementIcon({ kind, ok }: { kind: string; ok: boolean }) {
  const Icon = REQUIREMENT_ICON[kind] ?? CheckCircle2;
  return (
    <span
      key={ok ? "ok" : "no"}
      className={cn(
        "ill-motion relative -my-0.5 grid size-[26px] shrink-0 place-items-center rounded-full ring-1",
        ok ? "bg-emerald-400/15 text-emerald-300 ring-emerald-400/40" : "bg-rose-400/10 text-rose-300 ring-rose-400/30",
      )}
      style={{ animation: ok ? "ill-pop 450ms cubic-bezier(.3,1.4,.5,1) both" : undefined }}
    >
      <Icon className="size-3.5" />
      <span
        className={cn(
          "absolute -bottom-0.5 -right-0.5 size-2 rounded-full ring-2 ring-[oklch(0.17_0.008_265)]",
          ok ? "bg-emerald-400" : "bg-rose-400",
        )}
      />
    </span>
  );
}

function Section({ title, icon, tone, children }: { title: string; icon?: React.ReactNode; tone?: "warn"; children: React.ReactNode }) {
  return (
    <div className={cn(tone === "warn" && "rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4")}>
      <h4 className="mb-1.5 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
        {icon}
        {title}
      </h4>
      <p className="text-sm leading-relaxed text-foreground/90">{children}</p>
    </div>
  );
}

function confirmationPrompts(fn: FunctionInfo): string[] {
  const all = [
    "I have read what can go wrong.",
    "The car is parked safely and nobody is near moving parts.",
    `I want to ${fn.name.toLowerCase()} now and accept the risk.`,
  ];
  return all.slice(0, fn.confirmations);
}
