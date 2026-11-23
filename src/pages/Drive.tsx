/**
 * Drive Mode: a full-screen dashboard for an iPad mounted in the car.
 * Big, glanceable numbers; a 3D car in the middle that shows where any
 * warning is; nothing to tap while moving except Exit.
 */
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { BatteryMedium, Car, CheckCircle2, Droplet, Fuel, Gauge as GaugeIcon, Leaf, Lock, Moon, Play, Plug, Settings2, ShieldCheck, Square, Sun, Thermometer, Timer, TriangleAlert, Wind, X, type LucideIcon } from "lucide-react";
import DriveCar3D, { type CarLive, type Level } from "@/components/DriveCar3D";
import DriveDial from "@/components/DriveDial";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import { Button } from "@/components/ui/button";
import { useDiag } from "@/lib/store";
import { useI18n, Tr, type T } from "@/lib/i18n";
import { api } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import {
  AccelFromSpeed,
  LaunchTimer,
  avgEconomyText,
  batteryLevel,
  coolantLevel,
  economyText,
  fuelLevel,
  isNight,
  launchLabel,
  launchTarget,
  litresPerHour,
  loadSettings,
  oilLevel,
  saveSettings,
  speedIn,
  speedUnit,
  tempIn,
  tempUnit,
  worse,
  type DriveSettings,
} from "@/lib/drive";
import type { PageId } from "@/App";

/** Fades the road out towards the dials. */
const FADE = {
  maskImage: "radial-gradient(ellipse 58% 80% at 50% 48%, black 45%, transparent 100%)",
  WebkitMaskImage: "radial-gradient(ellipse 58% 80% at 50% 48%, black 45%, transparent 100%)",
};

/** Above this the car counts as moving and settings lock. */
const MOVING_KMH = 3;

export default function Drive({ go }: { go: (p: PageId) => void }) {
  const { connection } = useDiag();
  if (!connection) {
    return (
      <div>
        <PageHeader title="Drive Mode" subtitle="A full-screen dashboard for an iPad mounted in the car: speed, rpm, temperatures, fuel and g-forces at a glance." />
        <Empty icon={Car} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> <Tr>Connect</Tr>
          </Button>
        </Empty>
      </div>
    );
  }
  return <DriveScreen onExit={() => go("dashboard")} />;
}

const LEVEL_TEXT: Record<Level, string> = {
  ok: "text-white",
  warn: "text-amber-300",
  bad: "text-rose-300",
  off: "text-white/40",
};
const LEVEL_BAR: Record<Level, string> = {
  ok: "bg-cyan-400",
  warn: "bg-amber-400",
  bad: "bg-rose-400",
  off: "bg-white/20",
};

/** Keeps the screen on while Drive Mode is open, where the system allows it. */
function useWakeLock() {
  const [awake, setAwake] = useState(false);
  useEffect(() => {
    type Sentinel = { release: () => Promise<void>; addEventListener: (e: string, f: () => void) => void };
    const nav = navigator as Navigator & { wakeLock?: { request: (t: "screen") => Promise<Sentinel> } };
    let lock: Sentinel | null = null;
    let gone = false;
    const ask = async () => {
      if (!nav.wakeLock || document.visibilityState !== "visible") return;
      try {
        lock = await nav.wakeLock.request("screen");
        if (gone) {
          lock.release().catch(() => {});
          return;
        }
        setAwake(true);
        lock.addEventListener("release", () => setAwake(false));
      } catch {
        setAwake(false);
      }
    };
    ask();
    document.addEventListener("visibilitychange", ask);
    return () => {
      gone = true;
      document.removeEventListener("visibilitychange", ask);
      lock?.release().catch(() => {});
    };
  }, []);
  return awake;
}

/**
 * The iPad's own motion sensor, for g-forces. Assumes the iPad is mounted
 * upright facing the driver: screen left-right is the car's left-right, and
 * out of the screen is towards the back of the car.
 */
function useMotion(enabled: boolean) {
  const g = useRef<{ lat: number; lon: number; at: number }>({ lat: 0, lon: 0, at: 0 });
  useEffect(() => {
    if (!enabled) return;
    const on = (e: DeviceMotionEvent) => {
      const a = e.acceleration;
      if (!a || a.x == null || a.y == null || a.z == null) return;
      const w = window as Window & { orientation?: number };
      const deg = typeof w.orientation === "number" ? w.orientation : (screen.orientation?.angle ?? 0);
      const r = (deg * Math.PI) / 180;
      const right = a.x * Math.cos(r) - a.y * Math.sin(r);
      const cur = g.current;
      cur.lat += (right / 9.81 - cur.lat) * 0.15;
      cur.lon += (-a.z / 9.81 - cur.lon) * 0.15;
      cur.at = performance.now();
    };
    window.addEventListener("devicemotion", on);
    return () => window.removeEventListener("devicemotion", on);
  }, [enabled]);
  return g;
}

async function askMotionPermission(): Promise<boolean> {
  const DM = (window as Window & { DeviceMotionEvent?: { requestPermission?: () => Promise<string> } }).DeviceMotionEvent;
  if (!DM) return false;
  if (typeof DM.requestPermission === "function") {
    try {
      return (await DM.requestPermission()) === "granted";
    } catch {
      return false;
    }
  }
  return true;
}

function usePortrait() {
  const q = "(orientation: portrait)";
  const [p, setP] = useState(() => window.matchMedia(q).matches);
  useEffect(() => {
    const m = window.matchMedia(q);
    const on = () => setP(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, []);
  return p;
}

function useClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(id);
  }, []);
  return now;
}

function DriveScreen({ onExit }: { onExit: () => void }) {
  const { connection, live, setDriveMode } = useDiag();
  const { t, lang } = useI18n();
  const [settings, setSettingsState] = useState<DriveSettings>(loadSettings);
  const setSettings = (patch: Partial<DriveSettings>) =>
    setSettingsState((s) => {
      const next = { ...s, ...patch };
      saveSettings(next);
      return next;
    });
  const [panel, setPanel] = useState(false);
  const [nudge, setNudge] = useState<string | null>(null);
  const [demo, setDemo] = useState(false);
  const awake = useWakeLock();
  const motion = useMotion(settings.motion);
  const now = useClock();
  const portrait = usePortrait();
  const u = settings.units;

  // Live data favours speed and rpm while this screen is open.
  useEffect(() => {
    setDriveMode(true);
    return () => setDriveMode(false);
  }, [setDriveMode]);

  // Leave the practice car parked when Drive Mode closes.
  const simulated = !!connection?.simulated;
  useEffect(() => {
    return () => {
      if (simulated) api.simDrive(false).catch(() => {});
    };
  }, [simulated]);

  const supported = useMemo(() => new Set(connection?.supported_pids ?? []), [connection]);
  const has = (pid: number) => supported.size === 0 || supported.has(pid);

  const speed = live.get(0x0d);
  const rpm = live.get(0x0c);
  const throttle = live.get(0x11);
  const load = live.get(0x04);
  const coolant = live.get(0x05);
  const oil = live.get(0x5c);
  const fuel = live.get(0x2f);
  const volts = live.get(0x42);
  const intake = live.get(0x0f);
  const maf = live.get(0x10);
  const running = (rpm ?? 0) > 400;
  const moving = (speed ?? 0) > MOVING_KMH;

  // Acceleration, trip economy and the launch timer, updated per reading.
  const accel = useRef(new AccelFromSpeed());
  const launch = useRef(new LaunchTimer());
  const trip = useRef({ litres: 0, km: 0, at: 0 });
  const [launchMsg, setLaunchMsg] = useState<number | null>(null);
  const [, setTick] = useState(0);
  useEffect(() => {
    if (speed === undefined) return;
    const tNow = performance.now();
    accel.current.push(tNow, speed);
    const done = launch.current.push(tNow, speed, launchTarget(u));
    if (done !== null) {
      setLaunchMsg(done);
      if (settings.best === undefined || done < settings.best) setSettings({ best: done });
    }
    const tr = trip.current;
    if (tr.at && maf !== undefined) {
      const h = (tNow - tr.at) / 3_600_000;
      tr.litres += litresPerHour(maf) * h;
      tr.km += speed * h;
    }
    tr.at = tNow;
  }, [live]);

  // Settings close by themselves once the car moves off.
  useEffect(() => {
    if (moving) setPanel(false);
  }, [moving]);

  // A running launch timer redraws ten times a second.
  const timing = launch.current.running;
  useEffect(() => {
    if (!timing) return;
    const id = setInterval(() => setTick((n) => n + 1), 100);
    return () => clearInterval(id);
  }, [timing]);

  const fresh = performance.now() - motion.current.at < 1000;
  const gLon = settings.motion && fresh ? motion.current.lon : accel.current.g;
  const gLat = settings.motion && fresh ? motion.current.lat : 0;

  const levels = {
    coolant: coolantLevel(coolant),
    oil: oilLevel(oil),
    fuel: fuelLevel(fuel),
    battery: batteryLevel(volts, running),
  };
  const cold = coolant !== undefined && coolant < 60 && running;

  const fmtTemp = (c: number | undefined) => (c === undefined ? "--" : String(Math.round(tempIn(c, u))));
  const carLive = useRef<CarLive>({ speed: 0, accel: 0, lateral: 0, engine: "off", tank: "off", battery: "off", tags: { engine: "", tank: "", battery: "" } });
  carLive.current = {
    speed: speed ?? 0,
    accel: gLon,
    lateral: gLat,
    fuel: fuel === undefined ? undefined : fuel / 100,
    engine: worse(levels.coolant, levels.oil),
    tank: levels.fuel,
    battery: levels.battery,
    tags: {
      engine: coolant === undefined ? "" : `${fmtTemp(coolant)} ${tempUnit(u)}`,
      tank: fuel === undefined ? "" : `${Math.round(fuel)} %`,
      battery: volts === undefined ? "" : `${volts.toFixed(1)} V`,
    },
  };

  // The most serious thing to say, in plain words.
  const alerts: { level: Level; text: string }[] = [];
  if (levels.coolant !== "ok" && levels.coolant !== "off") alerts.push({ level: levels.coolant, text: levels.coolant === "bad" ? t("Engine overheating: pull over safely and switch off") : t("Coolant running hot: ease off") });
  if (levels.oil !== "ok" && levels.oil !== "off") alerts.push({ level: levels.oil, text: t("Oil temperature high: ease off") });
  if (levels.battery !== "ok" && levels.battery !== "off")
    alerts.push({ level: levels.battery, text: running ? (volts! > 15 ? t("Charging voltage too high") : t("Battery not charging properly")) : t("Battery low") });
  if (levels.fuel !== "ok" && levels.fuel !== "off") alerts.push({ level: levels.fuel, text: levels.fuel === "bad" ? t("Fuel very low") : t("Fuel low") });
  alerts.sort((a, b) => (a.level === "bad" ? -1 : 0) - (b.level === "bad" ? -1 : 0));

  const night = isNight(settings.night, now);
  const accent = connection ? MAKE_ACCENT[connection.vehicle.pack.make] : "#67e8f9";
  const pack = connection?.vehicle.pack;

  const locked = (what: () => void) => () => {
    if (moving) {
      setNudge(t("Settings unlock when the car is stopped."));
      setTimeout(() => setNudge(null), 2600);
      return;
    }
    what();
  };

  const toggleDemo = async () => {
    const next = !demo;
    await api.simDrive(next).catch(() => {});
    setDemo(next);
  };

  const economy = economyText(maf === undefined ? undefined : litresPerHour(maf), speed, u);
  const avg = avgEconomyText(trip.current.litres, trip.current.km, u);
  const elapsed = launch.current.elapsed(performance.now());

  const tiles: { id: string; show: boolean; node: ReactNode }[] = [
    {
      id: "coolant",
      show: has(0x05),
      node: (
        <Tile icon={Thermometer} label={t("Coolant")} value={fmtTemp(coolant)} unit={tempUnit(u)} level={cold ? "off" : levels.coolant} bar={coolant === undefined ? undefined : (coolant - 40) / 90} note={cold ? t("Warming up") : undefined} />
      ),
    },
    {
      id: "oil",
      show: has(0x5c),
      node: <Tile icon={Droplet} label={t("Oil")} value={fmtTemp(oil)} unit={tempUnit(u)} level={levels.oil} bar={oil === undefined ? undefined : (oil - 40) / 120} />,
    },
    {
      id: "fuel",
      show: has(0x2f),
      node: <Tile icon={Fuel} label={t("Fuel")} value={fuel === undefined ? "--" : String(Math.round(fuel))} unit="%" level={levels.fuel} bar={fuel === undefined ? undefined : fuel / 100} />,
    },
    {
      id: "battery",
      show: has(0x42),
      node: (
        <Tile
          icon={BatteryMedium}
          label={t("Battery")}
          value={volts === undefined ? "--" : volts.toFixed(1)}
          unit="V"
          level={levels.battery}
          note={volts === undefined ? undefined : running ? (levels.battery === "ok" ? t("Charging") : undefined) : t("Engine off")}
        />
      ),
    },
    {
      id: "economy",
      show: has(0x10),
      node: <Tile icon={Leaf} label={t("Economy")} value={economy.value} unit={economy.unit} level="ok" note={avg ? t("Trip {avg}", { avg }) : t("Estimated from airflow")} />,
    },
    {
      id: "intake",
      show: has(0x0f),
      node: <Tile icon={Wind} label={t("Intake air")} value={fmtTemp(intake)} unit={tempUnit(u)} level={intake === undefined ? "off" : "ok"} />,
    },
  ];

  return (
    <div
      className="safe-shell fixed inset-0 z-50 flex select-none flex-col overflow-hidden bg-[#07090e] text-white"
      style={{ filter: night ? "brightness(0.72) saturate(0.85)" : undefined }}
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_35%,rgba(34,211,238,0.08),transparent_60%)]" />

      {/* Top bar */}
      <div className="relative z-10 flex items-center gap-3 px-4 pt-3 sm:px-6">
        <button onClick={onExit} className="flex min-h-12 items-center gap-2 rounded-xl bg-white/[0.06] px-4 text-sm font-medium ring-1 ring-inset ring-white/10 active:bg-white/[0.12]">
          <X className="size-5" /> {t("Exit")}
        </button>
        <div className="hidden min-w-0 md:block">
          <div className="truncate text-sm font-semibold">{pack ? `${pack.brand} ${pack.model}` : t("Your car")}</div>
          <div className="text-xs text-white/45">{simulated ? t("Demo car · simulated data") : t("Live from the car")}</div>
        </div>
        <div className="flex min-w-0 flex-1 justify-center">
          {alerts.length ? (
            <div
              className={cn(
                "flex min-w-0 items-center gap-2 rounded-full px-4 py-2 text-sm font-semibold ring-1 ring-inset",
                alerts[0]!.level === "bad" ? "animate-pulse bg-rose-500/20 text-rose-200 ring-rose-400/50" : "bg-amber-400/15 text-amber-200 ring-amber-400/40",
              )}
            >
              <TriangleAlert className="size-4 shrink-0" />
              <span className="truncate">{alerts[0]!.text}</span>
              {alerts.length > 1 && <span className="shrink-0 rounded-full bg-black/30 px-1.5 text-xs">+{alerts.length - 1}</span>}
            </div>
          ) : (
            <div className="flex items-center gap-2 rounded-full bg-emerald-400/10 px-4 py-2 text-sm font-medium text-emerald-200 ring-1 ring-inset ring-emerald-400/25">
              <CheckCircle2 className="size-4" /> {cold ? t("All normal · engine warming up") : t("All normal")}
            </div>
          )}
        </div>
        <div className="hidden text-xl font-semibold tabular-nums text-white/80 sm:block">{now.toLocaleTimeString(lang === "es" ? "es" : undefined, { hour: "numeric", minute: "2-digit" })}</div>
        {simulated && (
          <button
            onClick={toggleDemo}
            className={cn(
              "flex min-h-12 items-center gap-2 rounded-xl px-4 text-sm font-medium ring-1 ring-inset",
              demo ? "bg-amber-400/15 text-amber-200 ring-amber-400/40" : "bg-white/[0.06] ring-white/10",
            )}
          >
            {demo ? <Square className="size-4" /> : <Play className="size-4" />} {demo ? t("Stop demo drive") : t("Demo drive")}
          </button>
        )}
        <button
          onClick={locked(() => setPanel((p) => !p))}
          aria-label={t("Drive Mode settings")}
          className={cn("grid size-12 place-items-center rounded-xl bg-white/[0.06] ring-1 ring-inset ring-white/10", moving && "opacity-40")}
        >
          {moving ? <Lock className="size-5" /> : <Settings2 className="size-5" />}
        </button>
      </div>

      {/* Dials and car */}
      <div className="relative grid min-h-0 flex-1 grid-cols-2 grid-rows-[1.1fr_1fr] gap-x-2 px-3 landscape:grid-cols-[minmax(0,1fr)_minmax(0,1.3fr)_minmax(0,1fr)] landscape:grid-rows-1 sm:px-6">
        {/* The road runs the full width behind the dials, fading out at the edges. */}
        <DriveCar3D
          body={pack?.body ?? "sedan"}
          accent={accent}
          live={carLive}
          focusY={portrait ? 0.3 : 0.47}
          className="absolute inset-0"
          style={FADE}
        />
        <div className="relative col-span-2 min-h-0 landscape:order-2 landscape:col-span-1">
          <GBall lon={gLon} lat={gLat} t={t} />
        </div>
        <div className="flex min-h-0 flex-col landscape:order-1">
          <div className="min-h-0 flex-1">
            <DriveDial value={speed === undefined ? undefined : speedIn(speed, u)} max={u === "imperial" ? 160 : 260} majorStep={u === "imperial" ? 20 : 40} unit={speedUnit(u)} label={t("Speed")} color={accent}>
              <Launch label={launchLabel(u)} running={elapsed} last={launchMsg} best={settings.best} />
            </DriveDial>
          </div>
          {has(0x11) && <Bar label={t("Throttle")} value={throttle} />}
        </div>
        <div className="flex min-h-0 flex-col landscape:order-3">
          <div className="min-h-0 flex-1">
            <DriveDial value={rpm} max={8000} majorStep={1000} labelScale={1000} redFrom={6500} shiftFrom={6000} unit={t("rpm")} label={t("Engine")} color={accent}>
              <div className="text-xs text-white/40">× 1000</div>
            </DriveDial>
          </div>
          {has(0x04) && <Bar label={t("Engine load")} value={load} />}
        </div>
      </div>

      {/* Readings */}
      <div className="safe-bottom relative z-10 grid grid-cols-3 gap-2 px-3 pt-2 sm:px-6 lg:grid-cols-6">
        {tiles.filter((x) => x.show).map((x) => (
          <div key={x.id}>{x.node}</div>
        ))}
      </div>

      {nudge && (
        <div className="absolute left-1/2 top-20 z-30 -translate-x-1/2 rounded-full bg-black/80 px-4 py-2 text-sm ring-1 ring-white/15">
          <Lock className="mr-1.5 inline size-4" />
          {nudge}
        </div>
      )}

      {panel && !moving && (
        <SettingsPanel
          settings={settings}
          setSettings={setSettings}
          awake={awake}
          onClose={() => setPanel(false)}
          onMotion={async (on) => {
            if (on && !(await askMotionPermission())) {
              setNudge(t("Motion access was not allowed."));
              setTimeout(() => setNudge(null), 2600);
              return;
            }
            setSettings({ motion: on });
          }}
          t={t}
        />
      )}

      {!settings.briefed && <Briefing onOk={() => setSettings({ briefed: true })} t={t} />}
    </div>
  );
}

function Tile({ icon: Icon, label, value, unit, level, bar, note }: { icon: LucideIcon; label: string; value: string; unit: string; level: Level; bar?: number; note?: string }) {
  return (
    <div
      className={cn(
        "relative h-full overflow-hidden rounded-2xl bg-white/[0.035] px-4 py-3 ring-1 ring-inset transition-colors",
        level === "bad" ? "bg-rose-500/10 ring-rose-400/50" : level === "warn" ? "bg-amber-400/[0.08] ring-amber-400/40" : "ring-white/[0.07]",
      )}
    >
      <div className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] text-white/45">
        <Icon className="size-3.5 shrink-0" />
        <span className="min-w-0 truncate">{label}</span>
        {bar !== undefined && note && <span className="ml-auto shrink-0 normal-case tracking-normal text-sky-300">{note}</span>}
      </div>
      <div className="mt-1 flex items-baseline gap-1">
        <span className={cn("text-[clamp(26px,3.2vw,40px)] font-semibold leading-none tabular-nums tracking-tight", LEVEL_TEXT[level])}>{value}</span>
        <span className="text-sm text-white/45">{unit}</span>
      </div>
      {bar !== undefined ? (
        <div className="mt-2.5 h-1.5 overflow-hidden rounded-full bg-white/[0.07]">
          <div className={cn("h-full rounded-full transition-[width] duration-500", LEVEL_BAR[level])} style={{ width: `${Math.max(2, Math.min(100, bar * 100))}%` }} />
        </div>
      ) : (
        <div className="mt-1.5 truncate text-xs text-white/45">{note ?? " "}</div>
      )}
    </div>
  );
}

function Bar({ label, value }: { label: string; value: number | undefined }) {
  return (
    <div className="mx-auto mb-1 w-[70%] max-w-[260px]">
      <div className="mb-1 flex justify-between text-[11px] font-semibold uppercase tracking-[0.16em] text-white/40">
        <span>{label}</span>
        <span className="tabular-nums text-white/70">{value === undefined ? "--" : `${Math.round(value)} %`}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-white/[0.07]">
        <div className="h-full rounded-full bg-gradient-to-r from-cyan-500 to-cyan-300 transition-[width] duration-200" style={{ width: `${Math.max(0, Math.min(100, value ?? 0))}%` }} />
      </div>
    </div>
  );
}

/** Acceleration and braking (and cornering, with the motion sensor). */
function GBall({ lon, lat, t }: { lon: number; lat: number; t: T }) {
  const R = 44;
  const clamp = (v: number) => Math.max(-1, Math.min(1, v));
  // Braking moves the dot forward (up), speeding up pulls it back (down),
  // like the push you feel.
  const x = clamp(-lat) * R;
  const y = clamp(lon) * R;
  const mag = Math.hypot(lon, lat);
  const what = lon < -0.08 ? t("Braking") : lon > 0.08 ? t("Accelerating") : Math.abs(lat) > 0.08 ? t("Cornering") : t("Steady");
  return (
    <div className="absolute bottom-2 left-1/2 flex -translate-x-1/2 items-center gap-3 rounded-2xl bg-black/40 px-3 py-2 ring-1 ring-inset ring-white/10 backdrop-blur-md">
      <svg width={R * 2 + 8} height={R * 2 + 8} viewBox={`${-R - 4} ${-R - 4} ${R * 2 + 8} ${R * 2 + 8}`}>
        <circle r={R} fill="rgba(255,255,255,0.03)" stroke="rgba(255,255,255,0.15)" />
        <circle r={R / 2} fill="none" stroke="rgba(255,255,255,0.1)" strokeDasharray="2 3" />
        <line x1={-R} x2={R} stroke="rgba(255,255,255,0.08)" />
        <line y1={-R} y2={R} stroke="rgba(255,255,255,0.08)" />
        <circle cx={x} cy={y} r={7} fill={lon < -0.3 ? "#fb7185" : "#22d3ee"} style={{ filter: "drop-shadow(0 0 6px currentColor)", transition: "cx 120ms linear, cy 120ms linear" }} />
      </svg>
      <div>
        <div className="text-2xl font-semibold tabular-nums leading-none">
          {mag.toFixed(2)}
          <span className="ml-1 text-sm text-white/45">g</span>
        </div>
        <div className="mt-1 text-xs text-white/50">{what}</div>
      </div>
    </div>
  );
}

function Launch({ label, running, last, best }: { label: string; running: number | null; last: number | null; best?: number }) {
  if (running === null && last === null && best === undefined) return null;
  return (
    <div className="flex items-center gap-1.5 rounded-full bg-white/[0.05] px-3 py-1 text-xs tabular-nums text-white/60 ring-1 ring-inset ring-white/10">
      <Timer className={cn("size-3.5", running !== null && "text-amber-300")} />
      <span>{label}</span>
      <span className="font-semibold text-white">{(running ?? last ?? best)!.toFixed(1)} s</span>
    </div>
  );
}

function Segmented<V extends string>({ value, options, onChange }: { value: V; options: { id: V; label: string; icon?: LucideIcon }[]; onChange: (v: V) => void }) {
  return (
    <div className="flex gap-1 rounded-xl bg-white/[0.04] p-1 ring-1 ring-inset ring-white/[0.08]">
      {options.map(({ id, label, icon: Icon }) => (
        <button
          key={id}
          onClick={() => onChange(id)}
          className={cn("flex min-h-10 flex-1 items-center justify-center gap-1.5 rounded-lg px-3 text-sm font-medium", value === id ? "bg-white/[0.12] text-white" : "text-white/55")}
        >
          {Icon && <Icon className="size-4" />}
          {label}
        </button>
      ))}
    </div>
  );
}

function SettingsPanel({
  settings,
  setSettings,
  awake,
  onClose,
  onMotion,
  t,
}: {
  settings: DriveSettings;
  setSettings: (p: Partial<DriveSettings>) => void;
  awake: boolean;
  onClose: () => void;
  onMotion: (on: boolean) => void;
  t: T;
}) {
  return (
    <div className="absolute inset-0 z-40 flex items-start justify-end bg-black/40 p-4 pt-20" onClick={onClose}>
      <div className="w-full max-w-sm rounded-2xl bg-[#0d121b] p-5 shadow-2xl ring-1 ring-white/10" onClick={(e) => e.stopPropagation()}>
        <div className="mb-4 flex items-center justify-between">
          <div className="text-base font-semibold">{t("Drive Mode settings")}</div>
          <button onClick={onClose} className="grid size-10 place-items-center rounded-lg bg-white/[0.06]" aria-label={t("Close")}>
            <X className="size-5" />
          </button>
        </div>
        <div className="space-y-4">
          <div>
            <div className="mb-1.5 text-xs font-medium text-white/50">{t("Units")}</div>
            <Segmented
              value={settings.units}
              onChange={(units) => setSettings({ units })}
              options={[
                { id: "imperial", label: "mph · °F" },
                { id: "metric", label: "km/h · °C" },
              ]}
            />
          </div>
          <div>
            <div className="mb-1.5 text-xs font-medium text-white/50">{t("Night dimming")}</div>
            <Segmented
              value={settings.night}
              onChange={(night) => setSettings({ night })}
              options={[
                { id: "auto", label: t("Auto") },
                { id: "on", label: t("On"), icon: Moon },
                { id: "off", label: t("Off"), icon: Sun },
              ]}
            />
            <div className="mt-1 text-xs text-white/40">{t("Auto dims the screen from 7 pm to 7 am.")}</div>
          </div>
          <div>
            <div className="mb-1.5 text-xs font-medium text-white/50">{t("Cornering g-force")}</div>
            <Segmented
              value={settings.motion ? "on" : "off"}
              onChange={(v) => onMotion(v === "on")}
              options={[
                { id: "off", label: t("From speed only") },
                { id: "on", label: t("Use iPad sensor"), icon: GaugeIcon },
              ]}
            />
            <div className="mt-1 text-xs text-white/40">{t("The sensor adds cornering. Mount the iPad upright, facing you.")}</div>
          </div>
          <div className="flex items-start gap-2 rounded-xl bg-white/[0.04] p-3 text-xs text-white/60">
            <ShieldCheck className="mt-0.5 size-4 shrink-0 text-emerald-300" />
            <div>
              {awake ? t("The screen stays on while Drive Mode is open.") : t("This iPad may still dim. Set Auto-Lock to Never in Settings > Display & Brightness for long drives.")}
              {settings.best !== undefined && (
                <button className="mt-2 block text-cyan-300" onClick={() => setSettings({ best: undefined })}>
                  {t("Reset best launch time ({s} s)", { s: settings.best.toFixed(1) })}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function Briefing({ onOk, t }: { onOk: () => void; t: T }) {
  const points = [
    t("Mount the iPad where it doesn't block your view of the road or any airbag, and check local rules on screens in view of the driver."),
    t("Glance, don't read. Settings lock while the car is moving, and nothing here can change the car."),
    t("Warnings are general guides from the readings, not your maker's limits. If the engine overheats, pull over safely and switch off."),
  ];
  return (
    <div className="absolute inset-0 z-50 grid place-items-center bg-black/70 p-6 backdrop-blur-sm">
      <div className="w-full max-w-lg rounded-3xl bg-[#0d121b] p-7 shadow-2xl ring-1 ring-white/10">
        <div className="grid size-12 place-items-center rounded-2xl bg-cyan-400/10 text-cyan-300 ring-1 ring-cyan-400/30">
          <Car className="size-6" />
        </div>
        <div className="mt-4 text-xl font-semibold">{t("Before you drive")}</div>
        <ul className="mt-3 space-y-3 text-sm leading-relaxed text-white/70">
          {points.map((p) => (
            <li key={p} className="flex gap-2.5">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-cyan-300" />
              {p}
            </li>
          ))}
        </ul>
        <Button className="mt-6 h-12 w-full text-base" onClick={onOk}>
          {t("Got it")}
        </Button>
      </div>
    </div>
  );
}
