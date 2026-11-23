import { useState } from "react";
import { AlertTriangle, CheckCircle2, CircleDashed, Cpu, Eraser, Loader2, Plug, ScanSearch } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import GateDialog from "@/components/GateDialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type ModuleDetail, type ModuleInfo, type ModuleScan } from "@/lib/tauri";
import { SYSTEM_LABEL, hex } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

export default function Modules({ go }: { go: (p: PageId) => void }) {
  const { connection, catalog } = useDiag();
  const [scan, setScan] = useState<ModuleScan[] | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<ModuleInfo | null>(null);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Modules" subtitle="Every control unit in the car, what it does, and its fault memory." />
        <Empty icon={Cpu} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> Connect
          </Button>
        </Empty>
      </div>
    );
  }

  const runScan = async () => {
    setScanning(true);
    setError(null);
    try {
      setScan(await api.scanModules());
    } catch (e) {
      setError(asCmdError(e).message);
    } finally {
      setScanning(false);
    }
  };

  const modules = catalog?.modules ?? [];
  const bySystem = Object.keys(SYSTEM_LABEL)
    .map((s) => ({ system: s, list: modules.filter((m) => m.system === s) }))
    .filter((g) => g.list.length);
  const found = scan?.filter((s) => s.present).length ?? 0;
  const withFaults = scan?.filter((s) => (s.faults ?? 0) > 0).length ?? 0;

  return (
    <div>
      <PageHeader
        title="Modules"
        subtitle="The E70 has a control unit for each system. A scan asks every address which modules are fitted and how many faults each has stored."
        actions={
          <Button onClick={runScan} disabled={scanning}>
            {scanning ? <Loader2 className="animate-spin" /> : <ScanSearch />} {scan ? "Re-scan" : "Scan all modules"}
          </Button>
        }
      />

      {scan && (
        <div className="mb-5 grid grid-cols-3 gap-3">
          <Stat label="Modules answered" value={`${found} / ${modules.length}`} />
          <Stat label="With stored faults" value={String(withFaults)} tone={withFaults ? "warn" : "ok"} />
          <Stat label="Total faults" value={String(scan.reduce((n, s) => n + (s.faults ?? 0), 0))} tone={withFaults ? "warn" : "ok"} />
        </div>
      )}
      {error && <p className="mb-4 text-sm text-rose-300">{error}</p>}

      <div className="space-y-6">
        {bySystem.map(({ system, list }) => (
          <section key={system}>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">{SYSTEM_LABEL[system]}</h3>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
              {list.map((m) => {
                const s = scan?.find((x) => x.module.address === m.address);
                const faults = s?.faults ?? 0;
                return (
                  <button
                    key={m.address}
                    onClick={() => setOpen(m)}
                    className={cn(
                      "group flex items-start gap-3 rounded-2xl border p-4 text-left transition",
                      !s && "border-white/[0.07] bg-white/[0.02] hover:border-white/15",
                      s?.present && faults === 0 && "border-emerald-400/15 bg-emerald-400/[0.03] hover:border-emerald-400/30",
                      s?.present && faults > 0 && "border-amber-400/25 bg-amber-400/[0.05] hover:border-amber-400/40",
                      s && !s.present && "border-white/[0.05] opacity-50",
                      scanning && "animate-pulse",
                    )}
                  >
                    <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-white/5 font-mono text-[11px] font-semibold ring-1 ring-white/10">
                      {m.name.split("/")[0]}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{m.description}</span>
                        {s?.present && faults > 0 && (
                          <Badge variant="warn">
                            <AlertTriangle /> {faults}
                          </Badge>
                        )}
                        {s?.present && faults === 0 && <CheckCircle2 className="size-4 text-emerald-400" />}
                        {s && !s.present && <CircleDashed className="size-4 text-muted-foreground" />}
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{m.role}</p>
                      <div className="mt-2 font-mono text-[10px] text-muted-foreground/70">
                        {m.name} · {hex(m.address)} · {s ? (s.present ? "answered" : "not fitted / no reply") : "not scanned"}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </section>
        ))}
      </div>

      <ModuleSheet module={open} onClose={() => setOpen(null)} onChanged={runScan} />
    </div>
  );
}

function Stat({ label, value, tone }: { label: string; value: string; tone?: "ok" | "warn" }) {
  return (
    <div className="rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={cn("mt-1 text-2xl font-semibold tabular-nums", tone === "warn" && "text-amber-300", tone === "ok" && "text-emerald-300")}>
        {value}
      </div>
    </div>
  );
}

function ModuleSheet({ module, onClose, onChanged }: { module: ModuleInfo | null; onClose: () => void; onChanged: () => void }) {
  const { catalog, connection } = useDiag();
  const [detail, setDetail] = useState<ModuleDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadedFor, setLoadedFor] = useState<number | null>(null);
  const [clearing, setClearing] = useState(false);

  if (module && loadedFor !== module.address) {
    setLoadedFor(module.address);
    setDetail(null);
    setError(null);
    api
      .readModule(module.address)
      .then(setDetail)
      .catch((e) => setError(asCmdError(e).message));
  }
  if (!module && loadedFor !== null) setLoadedFor(null);

  const clearFn = catalog?.functions.find((f) => f.id === "clear_codes");

  return (
    <>
      <Dialog open={!!module} onOpenChange={(o) => !o && onClose()}>
        <DialogContent side="right">
          {module && (
            <div className="flex h-full flex-col">
              <div className="border-b border-white/[0.06] px-6 pb-5 pt-6">
                <Badge variant="info">{SYSTEM_LABEL[module.system]}</Badge>
                <DialogTitle className="mt-3 text-xl">
                  {module.description} <span className="font-mono text-base text-muted-foreground">({module.name})</span>
                </DialogTitle>
                <DialogDescription className="mt-1.5 leading-relaxed">{module.role}</DialogDescription>
                <div className="mt-3 font-mono text-[11px] text-muted-foreground">
                  Address {hex(module.address)} · replies on {hex(0x600 + module.address, 3)} · KWP2000 over D-CAN
                </div>
              </div>
              <div className="flex-1 space-y-5 overflow-auto px-6 py-5">
                {error && <p className="text-sm text-rose-300">{error}</p>}
                {!detail && !error && (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="size-4 animate-spin" /> Reading module…
                  </div>
                )}
                {detail && (
                  <>
                    <div>
                      <h4 className="mb-1.5 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">Identification</h4>
                      <div className="rounded-lg bg-black/30 px-3 py-2 font-mono text-xs text-cyan-100/80">{detail.identification ?? "Not available"}</div>
                    </div>
                    <div>
                      <h4 className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                        Fault memory ({detail.faults.length})
                      </h4>
                      {detail.faults.length === 0 ? (
                        <div className="flex items-center gap-2 rounded-xl border border-emerald-400/20 bg-emerald-400/[0.05] p-4 text-sm">
                          <CheckCircle2 className="size-4 text-emerald-400" /> No faults stored.
                        </div>
                      ) : (
                        <div className="space-y-2">
                          {detail.faults.map((f) => (
                            <div key={f.code} className="rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-3">
                              <div className="flex items-center gap-2">
                                <span className="font-mono font-semibold tracking-wider">{f.code}</span>
                                <Badge variant="muted">status {hex(f.status)}</Badge>
                              </div>
                              <p className="mt-1 text-sm text-foreground/85">
                                {f.description ?? "BMW fault text not in the database yet. The code and status are exact."}
                              </p>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                    {connection?.simulated && (
                      <p className="text-xs text-muted-foreground">Demo car: fault codes and descriptions are invented for the demo.</p>
                    )}
                  </>
                )}
              </div>
              <div className="flex justify-end border-t border-white/[0.06] px-6 py-4">
                <Button variant="outline" disabled={!detail || detail.faults.length === 0} onClick={() => setClearing(true)}>
                  <Eraser /> Clear fault memory
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
      {module && (
        <GateDialog
          fn={clearFn}
          open={clearing}
          onOpenChange={(o) => {
            setClearing(o);
            if (!o) {
              setLoadedFor(null);
              onChanged();
            }
          }}
          targetLabel={`${module.description} (${module.name})`}
          run={async () => {
            const codes = detail?.faults.map((f) => f.code).join(", ") || "none";
            await api.clearModuleCodes(module.address);
            return [`Saved ${codes} to the Activity log`, `Sent ClearDiagnosticInformation to ${module.name}`, `${module.name} confirmed`];
          }}
        />
      )}
    </>
  );
}
