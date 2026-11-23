import { useEffect, useState } from "react";
import { Activity, AlertTriangle, BatteryMedium, Cable, CheckCircle2, CircleDot, ClipboardCheck, ClipboardList, Cpu, Gauge, History, ListChecks, Loader2, Power, Radar, Rewind, Sparkles, Wrench, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { useDiag } from "@/lib/store";
import { LANGS, useI18n, useT } from "@/lib/i18n";
import { fmt } from "@/lib/format";
import Connect from "@/pages/Connect";
import Dashboard from "@/pages/Dashboard";
import Codes from "@/pages/Codes";
import Modules from "@/pages/Modules";
import Functions from "@/pages/Functions";
import ActivityPage from "@/pages/Activity";
import TimeMachine from "@/pages/TimeMachine";
import QuickTest from "@/pages/QuickTest";
import Copilot from "@/pages/Copilot";
import CarTest from "@/pages/CarTest";
import Replay from "@/pages/Replay";
import Inspection from "@/pages/Inspection";

export type PageId = "connect" | "cartest" | "quicktest" | "inspection" | "copilot" | "dashboard" | "replay" | "codes" | "modules" | "functions" | "timemachine" | "activity";

const NAV: { id: PageId; label: string; icon: LucideIcon }[] = [
  { id: "connect", label: "Connect", icon: Cable },
  { id: "cartest", label: "Car Test", icon: ClipboardCheck },
  { id: "quicktest", label: "Quick Test", icon: Radar },
  { id: "inspection", label: "Inspection Report", icon: ClipboardList },
  { id: "dashboard", label: "Live Data", icon: Activity },
  { id: "replay", label: "Instant Replay", icon: Rewind },
  { id: "codes", label: "Fault Codes", icon: AlertTriangle },
  { id: "modules", label: "Modules", icon: Cpu },
  { id: "copilot", label: "Repair Copilot", icon: Sparkles },
  { id: "functions", label: "Special Functions", icon: Wrench },
  { id: "timemachine", label: "Time Machine", icon: History },
  { id: "activity", label: "Activity", icon: ListChecks },
];

/** Labels for the narrow icon rail. */
const SHORT: Partial<Record<PageId, string>> = {
  quicktest: "Quick",
  inspection: "Inspect",
  dashboard: "Live",
  replay: "Replay",
  codes: "Codes",
  functions: "Functions",
  timemachine: "History",
};

export default function App() {
  const [page, setPage] = useState<PageId>("connect");
  const { connection } = useDiag();
  const t = useT();

  return (
    <div className="safe-shell flex h-dvh">
      {/* Full sidebar on desktop and landscape iPad; an icon rail with short
          labels on portrait iPad and in Split View. */}
      <aside className="safe-bottom flex w-[76px] shrink-0 flex-col border-r border-white/[0.06] bg-black/20 px-2 pt-5 lg:w-60 lg:px-3">
        <div className="mb-6 flex items-center justify-center gap-2.5 lg:justify-start lg:px-2">
          <div className="grid size-8 place-items-center rounded-lg bg-gradient-to-br from-cyan-400 to-blue-600 shadow-lg shadow-cyan-500/20">
            <Gauge className="size-4 text-white" />
          </div>
          <div className="hidden lg:block">
            <div className="text-[15px] font-semibold leading-none tracking-tight">ApexDiag</div>
            <div className="mt-1 text-[10px] uppercase tracking-[0.18em] text-muted-foreground">{t("Diagnostics")}</div>
          </div>
        </div>
        <nav className="flex flex-col gap-0.5 overflow-y-auto">
          {NAV.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setPage(id)}
              aria-label={t(label)}
              className={cn(
                "relative flex min-h-11 flex-col items-center justify-center gap-1 rounded-lg px-1 py-2 text-center text-[10px] leading-tight transition lg:flex-row lg:justify-start lg:gap-2.5 lg:px-2.5 lg:text-left lg:text-sm",
                page === id ? "bg-white/[0.07] text-foreground" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground active:bg-white/[0.06]",
              )}
            >
              {page === id && <span className="absolute inset-y-2 left-0 w-0.5 rounded-full bg-cyan-400" />}
              <Icon className={cn("size-5 lg:size-4", page === id && "text-cyan-300")} />
              <span className="lg:hidden">{t(SHORT[id] ?? label)}</span>
              <span className="hidden lg:inline">{t(label)}</span>
            </button>
          ))}
        </nav>
        <LanguageSwitch />
        <ConnectionCard onClick={() => setPage("connect")} />
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar go={setPage} />
        <main className="safe-bottom flex-1 overflow-auto overscroll-contain px-4 pb-10 pt-6 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-[1400px]" key={page + (connection ? "c" : "")}>
            {page === "connect" && <Connect go={setPage} />}
            {page === "cartest" && <CarTest go={setPage} />}
            {page === "quicktest" && <QuickTest go={setPage} />}
            {page === "inspection" && <Inspection go={setPage} />}
            {page === "copilot" && <Copilot go={setPage} />}
            {page === "dashboard" && <Dashboard go={setPage} />}
            {page === "replay" && <Replay go={setPage} />}
            {page === "codes" && <Codes go={setPage} />}
            {page === "modules" && <Modules go={setPage} />}
            {page === "functions" && <Functions />}
            {page === "timemachine" && <TimeMachine go={setPage} />}
            {page === "activity" && <ActivityPage />}
          </div>
        </main>
      </div>
    </div>
  );
}

/** English / Spanish. */
function LanguageSwitch() {
  const { lang, setLang, t } = useI18n();
  return (
    <div className="mb-2 mt-auto flex justify-center gap-1 rounded-lg bg-white/[0.03] p-1 ring-1 ring-inset ring-white/[0.06] lg:justify-start" role="radiogroup" aria-label={t("Language")}>
      {LANGS.map((l) => (
        <button
          key={l.id}
          role="radio"
          aria-checked={lang === l.id}
          title={l.name}
          onClick={() => setLang(l.id)}
          className={cn(
            "min-h-8 flex-1 rounded-md px-2 text-[11px] font-medium transition",
            lang === l.id ? "bg-white/[0.09] text-foreground" : "text-muted-foreground hover:text-foreground",
          )}
        >
          <span className="lg:hidden">{l.label}</span>
          <span className="hidden lg:inline">{l.name}</span>
        </button>
      ))}
    </div>
  );
}

function ConnectionCard({ onClick }: { onClick: () => void }) {
  const { connection } = useDiag();
  const t = useT();
  return (
    <button
      onClick={onClick}
      className="rounded-xl border border-white/[0.07] bg-white/[0.03] p-3 text-left transition hover:border-white/15"
      aria-label={connection ? t("Connected") : t("Not connected")}
    >
      <div className="flex items-center justify-center gap-2 text-xs lg:justify-start">
        <span className={cn("relative size-2 rounded-full", connection ? "bg-emerald-400" : "bg-white/25")}>
          {connection && <span className="absolute inset-0 animate-ping rounded-full bg-emerald-400/60" />}
        </span>
        <span className="hidden font-medium lg:inline">{connection ? t("Connected") : t("Not connected")}</span>
      </div>
      {connection && (
        <div className="hidden lg:block">
          <div className="mt-2 truncate text-xs text-muted-foreground">{connection.device.name}</div>
          <div className="mt-0.5 truncate text-[11px] text-foreground/80">
            {connection.vehicle.pack.brand} {connection.vehicle.pack.model}
          </div>
          <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground/80">{connection.vin}</div>
        </div>
      )}
    </button>
  );
}

function StatusBar({ go }: { go: (p: PageId) => void }) {
  const { connection, live, vehicle } = useDiag();
  const t = useT();
  const rpm = live.get(0x0c);
  const speed = live.get(0x0d);
  const volts = live.get(0x42) ?? vehicle?.state.battery_v ?? undefined;
  const running = rpm !== undefined ? rpm > 300 : undefined;
  return (
    <div className="flex h-12 shrink-0 items-center justify-end gap-2 overflow-x-auto border-b border-white/[0.06] px-4 sm:px-6" data-tauri-drag-region>
      {connection?.simulated && (
        <span className="mr-auto rounded-md bg-amber-400/10 px-2 py-1 text-[11px] font-medium text-amber-300 ring-1 ring-inset ring-amber-400/25">
          {t("DEMO CAR")}<span className="hidden sm:inline"> · {t("simulated data")}</span>
        </span>
      )}
      {connection ? (
        <>
          <MarkButton go={go} />
          <Chip icon={Power} label={t("Engine")} value={running === undefined ? "--" : running ? t("Running") : t("Off")} tone={running ? "info" : "muted"} />
          <Chip icon={Gauge} label={t("Speed")} value={`${fmt(speed)} km/h`} tone={speed === 0 ? "ok" : "warn"} />
          <Chip icon={BatteryMedium} label={t("Battery")} value={`${fmt(volts, 1)} V`} tone={volts === undefined ? "muted" : volts < 12.2 ? "warn" : "ok"} />
        </>
      ) : (
        <span className="text-xs text-muted-foreground">{t("No vehicle")}</span>
      )}
    </div>
  );
}

/** Instant Replay's Mark button: always one tap away while connected. */
function MarkButton({ go }: { go: (p: PageId) => void }) {
  const { recording, savingReplay, lastReplay, replayError, markMoment, finishReplay, setOpenReplay } = useDiag();
  const t = useT();
  const [now, setNow] = useState(Date.now());
  const [showSaved, setShowSaved] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const id = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(id);
  }, [recording]);

  useEffect(() => {
    if (!lastReplay) return;
    setShowSaved(true);
    const id = setTimeout(() => setShowSaved(false), 12_000);
    return () => clearTimeout(id);
  }, [lastReplay]);

  if (savingReplay) {
    return (
      <span className="flex shrink-0 items-center gap-2 rounded-lg bg-rose-500/10 px-2.5 py-1 text-xs text-rose-200 ring-1 ring-inset ring-rose-400/25">
        <Loader2 className="size-3.5 animate-spin" />
        <span>
          {t("Saving replay")}<span className="hidden md:inline">{t(", reading fault memory")}</span>…
        </span>
      </span>
    );
  }
  if (recording) {
    const left = Math.max(0, recording.after_ms - (now - recording.started));
    const done = 1 - left / recording.after_ms;
    const r = 7;
    const c = 2 * Math.PI * r;
    return (
      <span className="flex shrink-0 items-center gap-2 rounded-lg bg-rose-500/10 py-1 pl-2 pr-1 text-xs text-rose-100 ring-1 ring-inset ring-rose-400/30">
        <svg viewBox="0 0 18 18" className="size-4 -rotate-90">
          <circle cx="9" cy="9" r={r} fill="none" stroke="#fb7185" strokeOpacity="0.25" strokeWidth="2.5" />
          <circle cx="9" cy="9" r={r} fill="none" stroke="#fb7185" strokeWidth="2.5" strokeDasharray={c} strokeDashoffset={c * (1 - done)} />
        </svg>
        <span className="tabular-nums">
          {t("Recording")}<span className="hidden md:inline"> {t("after the mark")}</span> · {Math.ceil(left / 1000)} s
        </span>
        <button onClick={finishReplay} className="rounded-md bg-white/10 px-2 py-0.5 font-medium hover:bg-white/15">
          {t("Save now")}
        </button>
      </span>
    );
  }
  if (showSaved && lastReplay) {
    return (
      <button
        onClick={() => {
          setOpenReplay(lastReplay.id);
          setShowSaved(false);
          go("replay");
        }}
        className="flex shrink-0 items-center gap-2 rounded-lg bg-emerald-400/10 px-2.5 py-1 text-xs text-emerald-200 ring-1 ring-inset ring-emerald-400/25 hover:bg-emerald-400/15"
      >
        <CheckCircle2 className="size-3.5" />
        <span>
          {t("Replay saved")} · <span className="font-medium underline-offset-2 hover:underline">{t("View")}</span>
        </span>
      </button>
    );
  }
  return (
    <button
      onClick={markMoment}
      title={replayError ?? t("Instant Replay: keep the 30 seconds before and after this moment")}
      className="group flex shrink-0 items-center gap-2 rounded-lg bg-rose-500/15 px-2.5 py-1 text-xs font-medium text-rose-100 ring-1 ring-inset ring-rose-400/35 transition hover:bg-rose-500/25 active:scale-[0.97]"
    >
      <CircleDot className="size-3.5 text-rose-400 group-hover:animate-pulse" />
      <span>
        {t("Mark")}<span className="hidden sm:inline"> {t("moment")}</span>
      </span>
    </button>
  );
}

function Chip({ icon: Icon, label, value, tone }: { icon: LucideIcon; label: string; value: string; tone: "ok" | "warn" | "info" | "muted" }) {
  return (
    <div className="flex shrink-0 items-center gap-2 rounded-lg bg-white/[0.04] px-2.5 py-1 text-xs ring-1 ring-inset ring-white/[0.06]">
      <Icon
        className={cn(
          "size-3.5",
          tone === "ok" && "text-emerald-400",
          tone === "warn" && "text-amber-300",
          tone === "info" && "text-cyan-300",
          tone === "muted" && "text-muted-foreground",
        )}
      />
      <span className="hidden text-muted-foreground md:inline">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}
