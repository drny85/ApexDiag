/**
 * Drive Mode helpers: units, warning levels, fuel economy, acceleration and
 * the launch timer. Everything the car sends is metric (km/h, °C); it is
 * converted only for display.
 */
import type { Level } from "@/components/Car3D";

export type UnitSystem = "metric" | "imperial";
export type NightSetting = "auto" | "on" | "off";

export interface DriveSettings {
  units: UnitSystem;
  night: NightSetting;
  /** Use the iPad's motion sensor for g-forces. */
  motion: boolean;
  /** Safety notice seen. */
  briefed: boolean;
  /** Best launch time, seconds. */
  best?: number;
}

const KEY = "apexdiag.drive";

export function loadSettings(): DriveSettings {
  const us = typeof navigator !== "undefined" && /^en-(us|lr|mm)$/i.test(navigator.language ?? "");
  const base: DriveSettings = { units: us ? "imperial" : "metric", night: "auto", motion: false, briefed: false };
  try {
    return { ...base, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return base;
  }
}

export function saveSettings(s: DriveSettings) {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    /* storage blocked */
  }
}

export const KMH_PER_MPH = 1.609344;

export function speedIn(kmh: number, u: UnitSystem) {
  return u === "imperial" ? kmh / KMH_PER_MPH : kmh;
}
export function tempIn(c: number, u: UnitSystem) {
  return u === "imperial" ? (c * 9) / 5 + 32 : c;
}
export const speedUnit = (u: UnitSystem) => (u === "imperial" ? "mph" : "km/h");
export const tempUnit = (u: UnitSystem) => (u === "imperial" ? "°F" : "°C");

/** The launch the timer measures: 0–60 mph or 0–100 km/h. */
export const launchTarget = (u: UnitSystem) => (u === "imperial" ? 60 * KMH_PER_MPH : 100);
export const launchLabel = (u: UnitSystem) => (u === "imperial" ? "0–60 mph" : "0–100 km/h");

const worse = (a: Level, b: Level): Level => {
  const rank: Record<Level, number> = { off: 0, ok: 1, warn: 2, bad: 3 };
  return rank[a] >= rank[b] ? a : b;
};
export { worse };

/**
 * General warning points, not specific to any maker. They match the Live
 * Data gauges.
 */
export const LIMITS = {
  coolant: { cold: 60, warn: 112, bad: 118 },
  oil: { warn: 135, bad: 150 },
  fuel: { warn: 15, bad: 8 },
  battery: { running: { low: 12.8, bad: 12.0, high: 15.2 }, off: { low: 12.2 } },
};

export function coolantLevel(c: number | undefined): Level {
  if (c === undefined) return "off";
  return c >= LIMITS.coolant.bad ? "bad" : c >= LIMITS.coolant.warn ? "warn" : "ok";
}
export function oilLevel(c: number | undefined): Level {
  if (c === undefined) return "off";
  return c >= LIMITS.oil.bad ? "bad" : c >= LIMITS.oil.warn ? "warn" : "ok";
}
export function fuelLevel(pct: number | undefined): Level {
  if (pct === undefined) return "off";
  return pct < LIMITS.fuel.bad ? "bad" : pct < LIMITS.fuel.warn ? "warn" : "ok";
}
export function batteryLevel(v: number | undefined, running: boolean): Level {
  if (v === undefined) return "off";
  const r = LIMITS.battery.running;
  if (running) return v < r.bad || v > r.high ? "bad" : v < r.low ? "warn" : "ok";
  return v < LIMITS.battery.off.low ? "warn" : "ok";
}

/**
 * Fuel burned, from the air the engine takes in: petrol burns at about 14.7
 * parts air to 1 part fuel, and a litre of petrol weighs about 745 g.
 */
export function litresPerHour(mafGs: number) {
  return (mafGs * 3600) / (14.7 * 745);
}

export function economyText(lph: number | undefined, kmh: number | undefined, u: UnitSystem): { value: string; unit: string } {
  if (lph === undefined || kmh === undefined) return { value: "--", unit: u === "imperial" ? "mpg" : "L/100 km" };
  if (kmh < 5) {
    return u === "imperial" ? { value: (lph / 3.785).toFixed(2), unit: "gal/h" } : { value: lph.toFixed(1), unit: "L/h" };
  }
  const l100 = (lph / kmh) * 100;
  return u === "imperial" ? { value: (235.215 / Math.max(l100, 0.1)).toFixed(1), unit: "mpg" } : { value: l100.toFixed(1), unit: "L/100 km" };
}

export function avgEconomyText(litres: number, km: number, u: UnitSystem) {
  if (km < 0.2 || litres <= 0) return null;
  const l100 = (litres / km) * 100;
  return u === "imperial" ? `${(235.215 / l100).toFixed(1)} mpg` : `${l100.toFixed(1)} L/100 km`;
}

/** Acceleration along the car, from how fast the speed changes. */
export class AccelFromSpeed {
  private samples: { t: number; v: number }[] = [];
  g = 0;

  push(tMs: number, kmh: number) {
    this.samples.push({ t: tMs / 1000, v: kmh / 3.6 });
    while (this.samples.length && this.samples[0]!.t < tMs / 1000 - 0.9) this.samples.shift();
    const n = this.samples.length;
    if (n < 3) return this.g;
    // Least-squares slope of speed over the last ~0.9 s.
    const mt = this.samples.reduce((a, s) => a + s.t, 0) / n;
    const mv = this.samples.reduce((a, s) => a + s.v, 0) / n;
    let num = 0;
    let den = 0;
    for (const s of this.samples) {
      num += (s.t - mt) * (s.v - mv);
      den += (s.t - mt) ** 2;
    }
    const slope = den > 0 ? num / den : 0;
    const g = slope / 9.81;
    this.g += (g - this.g) * 0.5;
    return this.g;
  }
}

/**
 * Times a launch from a standstill to the target speed. The start and
 * finish are interpolated between readings, which arrive several times a
 * second.
 */
export class LaunchTimer {
  private stillSince: number | null = null;
  private start: number | null = null;
  private prev: { t: number; v: number } | null = null;
  running = false;
  last: number | null = null;

  push(tMs: number, kmh: number, target: number): number | null {
    const prev = this.prev;
    this.prev = { t: tMs, v: kmh };
    if (kmh < 1) {
      this.stillSince ??= tMs;
      this.start = null;
      this.running = false;
      return null;
    }
    if (!this.running && this.stillSince !== null && tMs - this.stillSince > 800 && prev && prev.v < 1) {
      // Moving off: the car started rolling between the two readings.
      const frac = (1 - prev.v) / Math.max(0.001, kmh - prev.v);
      this.start = prev.t + (tMs - prev.t) * Math.min(1, Math.max(0, frac));
      this.running = true;
    }
    this.stillSince = null;
    if (this.running && this.start !== null && prev) {
      if (kmh < prev.v - 3) {
        // Lifted off before the target: not a launch.
        this.running = false;
        this.start = null;
      } else if (kmh >= target) {
        const frac = (target - prev.v) / Math.max(0.001, kmh - prev.v);
        const end = prev.t + (tMs - prev.t) * Math.min(1, Math.max(0, frac));
        this.running = false;
        this.last = (end - this.start) / 1000;
        this.start = null;
        return this.last;
      }
    }
    return null;
  }

  elapsed(nowMs: number) {
    return this.running && this.start !== null ? (nowMs - this.start) / 1000 : null;
  }
}

/** Night dimming: on from 7 pm to 7 am when set to automatic. */
export function isNight(setting: NightSetting, now = new Date()) {
  if (setting !== "auto") return setting === "on";
  const h = now.getHours();
  return h >= 19 || h < 7;
}
