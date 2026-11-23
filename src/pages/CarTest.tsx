/**
 * First real car test: a guided, read-only run through everything the app
 * needs from an adapter and a car, with a raw capture saved for checking.
 */
import { useEffect, useRef, useState } from "react";
import {
  CheckCircle2,
  ChevronDown,
  CircleDashed,
  ClipboardCheck,
  FileText,
  Loader2,
  Lock,
  MinusCircle,
  Play,
  Save,
  Share2,
  XCircle,
} from "lucide-react";
import PageHeader from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useDiag } from "@/lib/store";
import { api, asCmdError, type CaptureView, type CarTestResult, type CarTestStepInfo, type SavedTest } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

const BEFORE = [
  { title: "Find the OBD port", detail: "Under the dashboard on the driver's side. On a BMW X5 (E70) it is by the bonnet release, behind a small flap." },
  { title: "Plug the adapter in", detail: "Push it fully home. Its light should come on: the car powers it." },
  { title: "Ignition on", detail: "Press Start once without your foot on the brake. Start the engine later for the live-data step." },
  { title: "Keep the iPad close", detail: "Bluetooth LE reaches a few metres. Sit in the car with the doors closed." },
  { title: "Don't pair it in Settings", detail: "BLE adapters are not paired in the Bluetooth settings. ApexDiag finds and connects to it itself." },
  { title: "Connect, then come back here", detail: "On the Connect page tap Scan Bluetooth, pick the Veepeak, and wait for the VIN." },
];

type Run = Record<string, CarTestResult | "running">;

export default function CarTest({ go }: { go: (p: PageId) => void }) {
  const { connection } = useDiag();
  const [plan, setPlan] = useState<CarTestStepInfo[]>([]);
  const [run, setRun] = useState<Run>({});
  const [running, setRunning] = useState(false);
  const [saved, setSaved] = useState<SavedTest | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [capture, setCapture] = useState<CaptureView | null>(null);
  const stop = useRef(false);

  useEffect(() => {
    api.carTestPlan().then(setPlan).catch(() => setPlan([]));
  }, []);

  const refreshCapture = () => api.captureInfo().then(setCapture).catch(() => setCapture(null));
  useEffect(() => {
    if (connection) refreshCapture();
  }, [connection]);

  const start = async () => {
    setRunning(true);
    setSaved(null);
    setError(null);
    setRun({});
    stop.current = false;
    for (const s of plan) {
      if (stop.current) break;
      setRun((r) => ({ ...r, [s.id]: "running" }));
      try {
        const res = await api.carTestStep(s.id);
        setRun((r) => ({ ...r, [s.id]: res }));
      } catch (e) {
        const msg = asCmdError(e).message;
        setRun((r) => ({ ...r, [s.id]: { id: s.id, ok: false, skipped: false, headline: "This step could not run", details: [msg], data: null, elapsed_ms: 0 } }));
      }
      refreshCapture();
    }
    setRunning(false);
  };

  const results = plan.map((s) => run[s.id]).filter((r): r is CarTestResult => !!r && r !== "running");
  const done = results.length === plan.length && plan.length > 0 && !running;
  const passed = results.filter((r) => r.ok && !r.skipped).length;

  const report = () => ({
    app: "ApexDiag",
    kind: "first-real-car-test",
    at: new Date().toISOString(),
    adapter: connection?.device.name,
    adapter_version: connection?.adapter.version,
    vin: connection?.vin,
    pack: connection?.vehicle.pack.id,
    simulated: connection?.simulated,
    steps: results,
  });

  const save = async () => {
    setError(null);
    try {
      setSaved(await api.saveCarTest(report()));
    } catch (e) {
      setError(asCmdError(e).message);
    }
  };

  const summary = () =>
    [
      `ApexDiag car test, ${new Date().toLocaleString()}`,
      `${connection?.device.name ?? "adapter"} · VIN ${connection?.vin ?? "not read"}`,
      ...plan.map((s) => {
        const r = run[s.id];
        if (!r || r === "running") return `- ${s.title}: not run`;
        return `- ${s.title}: ${r.skipped ? "skipped" : r.ok ? "OK" : "FAILED"}. ${r.headline}`;
      }),
    ].join("\n");

  const share = async () => {
    const text = summary();
    try {
      if (navigator.share) {
        await navigator.share({ title: "ApexDiag car test", text });
        return;
      }
    } catch {
      /* fall back to the clipboard */
    }
    try {
      await navigator.clipboard.writeText(text);
      setError("Summary copied. Paste it into a message.");
    } catch {
      setError("Sharing is not available here. Use Save instead.");
    }
  };

  return (
    <div>
      <PageHeader
        title="First real car test"
        subtitle="A guided check of your adapter and car. Every step only reads: nothing on the car is changed, cleared or coded. Everything the adapter says is captured so the results can be checked afterwards."
        actions={
          <Badge variant="ok" className="gap-1">
            <Lock className="size-3" /> Read only
          </Badge>
        }
      />

      <section className="mb-6 flex flex-col items-center gap-6 overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-cyan-400/[0.06] via-transparent to-transparent p-6 md:flex-row">
        <div className="w-full max-w-md shrink-0 md:w-[38%]">
          <SetupArt />
        </div>
        <ol className="grid min-w-0 flex-1 gap-3 sm:grid-cols-2">
          {BEFORE.map((b, i) => (
            <li key={b.title} className="flex gap-3">
              <span className="grid size-6 shrink-0 place-items-center rounded-full bg-cyan-400/10 text-xs font-semibold text-cyan-200 ring-1 ring-cyan-400/25">{i + 1}</span>
              <div>
                <p className="text-sm font-medium">{b.title}</p>
                <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{b.detail}</p>
              </div>
            </li>
          ))}
        </ol>
      </section>

      {!connection ? (
        <div className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
          <p className="text-sm text-muted-foreground">Connect to the adapter first. The test starts from here once the car is connected.</p>
          <Button onClick={() => go("connect")}>Go to Connect</Button>
        </div>
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div className="text-sm">
              <span className="text-muted-foreground">Connected to </span>
              <span className="font-medium">{connection.device.name}</span>
              <span className="text-muted-foreground"> · VIN </span>
              <span className="font-mono text-xs">{connection.vin ?? "not read"}</span>
              {connection.simulated && <Badge variant="warn" className="ml-2">Practice car</Badge>}
            </div>
            <div className="flex items-center gap-2">
              {running ? (
                <Button variant="outline" onClick={() => (stop.current = true)}>
                  Stop after this step
                </Button>
              ) : (
                <Button onClick={start} disabled={!plan.length}>
                  <Play /> {results.length ? "Run the test again" : "Start the test"}
                </Button>
              )}
            </div>
          </div>

          {results.length > 0 && (
            <div className="mb-4 h-1.5 overflow-hidden rounded-full bg-white/5">
              <div className="h-full rounded-full bg-gradient-to-r from-cyan-400 to-emerald-400 transition-all" style={{ width: `${(results.length / Math.max(plan.length, 1)) * 100}%` }} />
            </div>
          )}

          <ol className="space-y-3">
            {plan.map((s, i) => (
              <StepCard key={s.id} n={i + 1} info={s} state={run[s.id]} />
            ))}
          </ol>

          {done && (
            <section className="mt-6 rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 className="flex items-center gap-2 font-semibold">
                    <ClipboardCheck className="size-4 text-cyan-300" /> {passed} of {plan.length} steps worked
                  </h3>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Save the report and the raw capture, then send both files back. They show exactly what this car and adapter said.
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button variant="outline" onClick={share}>
                    <Share2 /> Share summary
                  </Button>
                  <Button onClick={save} disabled={!!saved}>
                    <Save /> {saved ? "Saved" : "Save report and capture"}
                  </Button>
                </div>
              </div>
              {saved && (
                <div className="mt-4 rounded-xl bg-emerald-400/[0.06] p-3 text-xs text-emerald-200 ring-1 ring-emerald-400/20">
                  <p className="font-medium">Saved to {saved.folder}</p>
                  <p className="mt-1 break-all font-mono text-emerald-200/80">{saved.report}</p>
                  {saved.capture && <p className="break-all font-mono text-emerald-200/80">{saved.capture}</p>}
                  <p className="mt-2 text-emerald-200/80">On an iPad, open the Files app, then On My iPad, then ApexDiag.</p>
                </div>
              )}
            </section>
          )}
          {error && <p className="mt-3 text-sm text-amber-200">{error}</p>}

          <CapturePanel capture={capture} onRefresh={refreshCapture} />
        </>
      )}
    </div>
  );
}

function StepCard({ n, info, state }: { n: number; info: CarTestStepInfo; state: CarTestResult | "running" | undefined }) {
  const [open, setOpen] = useState(false);
  const r = state && state !== "running" ? state : null;
  const tone = !r ? "border-white/[0.07]" : r.skipped ? "border-white/[0.07]" : r.ok ? "border-emerald-400/25 bg-emerald-400/[0.03]" : "border-rose-400/30 bg-rose-400/[0.04]";
  return (
    <li className={cn("rounded-2xl border p-4 transition-colors", tone)}>
      <div className="flex items-start gap-3">
        <span className="mt-0.5">
          {state === "running" ? (
            <Loader2 className="size-5 animate-spin text-cyan-300" />
          ) : !r ? (
            <span className="grid size-5 place-items-center rounded-full bg-white/5 text-[11px] text-muted-foreground ring-1 ring-white/10">{n}</span>
          ) : r.skipped ? (
            <MinusCircle className="size-5 text-muted-foreground" />
          ) : r.ok ? (
            <CheckCircle2 className="size-5 text-emerald-300" />
          ) : (
            <XCircle className="size-5 text-rose-300" />
          )}
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <h3 className="font-medium">{info.title}</h3>
            {r && <span className="text-[11px] text-muted-foreground">{(r.elapsed_ms / 1000).toFixed(1)} s</span>}
          </div>
          {r ? (
            <p className={cn("mt-1 text-sm", r.ok || r.skipped ? "text-foreground/90" : "text-rose-200")}>{r.headline}</p>
          ) : (
            <p className="mt-1 text-sm text-muted-foreground">{info.what}</p>
          )}
          <p className="mt-1 text-xs text-muted-foreground/80">
            <span className="text-muted-foreground">Why: </span>
            {info.why}
          </p>
          {r && r.details.length > 0 && (
            <>
              <button onClick={() => setOpen((o) => !o)} className="mt-2 inline-flex min-h-[32px] items-center gap-1 text-xs text-cyan-300 hover:text-cyan-200">
                <ChevronDown className={cn("size-3.5 transition-transform", open && "rotate-180")} /> {open ? "Hide" : "Show"} details ({r.details.length})
              </button>
              {open && (
                <ul className="mt-2 space-y-1 rounded-lg bg-black/25 p-3 font-mono text-[11px] leading-relaxed text-foreground/80">
                  {r.details.map((d, i) => (
                    <li key={i} className="break-words">{d}</li>
                  ))}
                </ul>
              )}
            </>
          )}
        </div>
      </div>
    </li>
  );
}

function CapturePanel({ capture, onRefresh }: { capture: CaptureView | null; onRefresh: () => void }) {
  const [open, setOpen] = useState(false);
  if (!capture?.path) return null;
  return (
    <section className="mt-6 rounded-2xl border border-white/[0.07] bg-white/[0.02] p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            <FileText className="size-4 text-cyan-300" /> Raw capture
          </h3>
          <p className="mt-1 text-xs text-muted-foreground">
            Everything sent to and received from the adapter on this connection ({(capture.bytes / 1024).toFixed(1)} KB).
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={onRefresh}>
            <CircleDashed /> Refresh
          </Button>
          <Button variant="outline" size="sm" onClick={() => setOpen((o) => !o)}>
            {open ? "Hide" : "Show the last lines"}
          </Button>
        </div>
      </div>
      {open && (
        <pre className="mt-3 max-h-80 overflow-auto rounded-lg bg-black/40 p-3 font-mono text-[11px] leading-relaxed text-foreground/80">{capture.tail.join("\n")}</pre>
      )}
    </section>
  );
}

/** Adapter in the OBD port under the dash, talking to a tablet. */
function SetupArt() {
  return (
    <svg viewBox="0 0 360 200" className="ill w-full" fill="none" strokeLinecap="round" strokeLinejoin="round">
      {/* dashboard underside */}
      <path d="M10 40 C 80 24, 200 22, 260 34 L 260 70 C 200 62, 90 64, 10 78 Z" fill="rgba(255,255,255,0.03)" stroke="rgba(255,255,255,0.22)" />
      <text x="14" y="96" fontSize="8" fill="rgba(255,255,255,0.35)">under the dash,</text>
      <text x="14" y="106" fontSize="8" fill="rgba(255,255,255,0.35)">driver's side</text>
      {/* OBD port and adapter */}
      <rect x="96" y="66" width="44" height="16" rx="4" fill="#0b0f14" stroke="rgba(255,255,255,0.35)" />
      <text x="118" y="60" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.45)">OBD port</text>
      <g style={{ animation: "ill-breathe 2.4s ease-in-out infinite" }}>
        <rect x="100" y="82" width="36" height="30" rx="6" fill="#111821" stroke="#67e8f9" strokeOpacity="0.8" />
        <circle cx="118" cy="102" r="3" fill="#34d399" />
      </g>
      <text x="118" y="126" textAnchor="middle" fontSize="8" fill="#67e8f9">adapter</text>
      {/* radio waves */}
      {[16, 28, 40].map((r, i) => (
        <path key={r} d={`M${150 + r * 0.2} ${97 - r} A ${r} ${r} 0 0 1 ${150 + r * 0.2} ${97 + r}`} stroke="#67e8f9" strokeOpacity={0.7 - i * 0.18} style={{ animation: `ill-breathe 1.6s ease-in-out ${i * 0.25}s infinite` }} />
      ))}
      {/* tablet */}
      <rect x="262" y="84" width="84" height="110" rx="10" fill="#0b0f14" stroke="rgba(255,255,255,0.4)" strokeWidth="1.5" />
      <rect x="270" y="94" width="68" height="88" rx="4" fill="rgba(103,232,249,0.06)" />
      <path d="M278 112 h40 M278 124 h52 M278 136 h30" stroke="rgba(255,255,255,0.25)" />
      <path d="M280 156 l 6 6 l 12 -14" stroke="#34d399" strokeWidth="2.5" />
      <text x="304" y="78" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.45)">iPad, a few metres away</text>
    </svg>
  );
}
