import { useState } from "react";
import { ChevronRight, ShieldCheck, ShieldX } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import GateDialog from "@/components/GateDialog";
import TierBadge from "@/components/TierBadge";
import { Badge } from "@/components/ui/badge";
import { useDiag } from "@/lib/store";
import { api, type FunctionInfo, type TierId } from "@/lib/tauri";
import { TIER_STYLE } from "@/lib/format";
import { cn } from "@/lib/utils";

export default function Functions() {
  const { catalog, vehicle, connection } = useDiag();
  const [open, setOpen] = useState<FunctionInfo | undefined>();

  const tiers = catalog?.tiers ?? [];
  const functions = (catalog?.functions ?? []).filter((f) => f.tier !== "read");

  return (
    <div>
      <PageHeader
        title="Special Functions"
        subtitle="Service routines, adaptations, coding, keys and programming. Each one explains what it does and what can go wrong, and stays locked until the car is in a safe state."
      />

      <div className="mb-6 grid grid-cols-2 gap-2 md:grid-cols-3 xl:grid-cols-6">
        {tiers.map((t, i) => (
          <div key={t.id} className="rounded-xl border border-white/[0.06] bg-white/[0.02] p-3">
            <div className="flex items-center gap-2">
              <span className="h-1.5 flex-1 rounded-full" style={{ background: `linear-gradient(90deg, ${TIER_STYLE[t.id].hex}, ${TIER_STYLE[t.id].hex}55)` }} />
              <span className="font-mono text-[10px] text-muted-foreground">{i}</span>
            </div>
            <div className={cn("mt-2 text-xs font-medium", TIER_STYLE[t.id].text)}>{t.label}</div>
            <p className="mt-1 text-[11px] leading-snug text-muted-foreground">{t.explain}</p>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        {functions.map((f) => {
          const unmet = vehicle?.functions.find((x) => x.id === f.id)?.unmet;
          const ready = connection && unmet && unmet.length === 0;
          return (
            <button
              key={f.id}
              onClick={() => setOpen(f)}
              className="group relative overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5 text-left transition hover:border-white/15 hover:bg-white/[0.035]"
            >
              <span className="absolute inset-y-0 left-0 w-[3px]" style={{ background: TIER_STYLE[f.tier as TierId].hex }} />
              <div className="flex items-start justify-between gap-3">
                <div>
                  <TierBadge tier={f.tier} label={f.tier_label} />
                  <div className="mt-2.5 text-[15px] font-semibold">{f.name}</div>
                </div>
                <ChevronRight className="mt-1 size-4 text-muted-foreground transition group-hover:translate-x-0.5 group-hover:text-foreground" />
              </div>
              <p className="mt-1.5 text-sm text-muted-foreground">{f.what}</p>
              <div className="mt-4 flex flex-wrap items-center gap-2">
                {!connection ? (
                  <Badge variant="muted">Connect to check readiness</Badge>
                ) : ready ? (
                  <Badge variant="ok">
                    <ShieldCheck /> Ready
                  </Badge>
                ) : (
                  <Badge variant="danger">
                    <ShieldX /> {unmet?.length ?? "?"} condition{unmet?.length === 1 ? "" : "s"} not met
                  </Badge>
                )}
                <Badge variant="muted">
                  {f.confirmations} confirmation{f.confirmations === 1 ? "" : "s"}
                </Badge>
                {!f.verified_on_car && <Badge variant="muted">Real-car commands pending verification</Badge>}
              </div>
            </button>
          );
        })}
      </div>

      <GateDialog
        fn={open}
        open={!!open}
        onOpenChange={(o) => !o && setOpen(undefined)}
        run={async () => {
          const r = await api.runFunction(open!.id);
          return r.steps;
        }}
      />
    </div>
  );
}
