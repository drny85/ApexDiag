import type { TierId } from "./tauri";

export const hex = (n: number, width = 2) => "0x" + n.toString(16).toUpperCase().padStart(width, "0");

export function fmt(value: number | undefined | null, digits = 0) {
  if (value === undefined || value === null || Number.isNaN(value)) return "--";
  return value.toLocaleString(undefined, { minimumFractionDigits: digits, maximumFractionDigits: digits });
}

export function digitsFor(unit: string) {
  if (unit === "V") return 2;
  if (unit === "%" || unit === "g/s" || unit === "°" || unit === "λ") return 1;
  return 0;
}

/** Visual tokens per risk tier: low risk is cool, high risk is hot. */
export const TIER_STYLE: Record<TierId, { text: string; bg: string; ring: string; dot: string; hex: string }> = {
  read: { text: "text-sky-300", bg: "bg-sky-400/10", ring: "ring-sky-400/30", dot: "bg-sky-400", hex: "#38bdf8" },
  service: { text: "text-emerald-300", bg: "bg-emerald-400/10", ring: "ring-emerald-400/30", dot: "bg-emerald-400", hex: "#34d399" },
  adapt: { text: "text-yellow-300", bg: "bg-yellow-400/10", ring: "ring-yellow-400/30", dot: "bg-yellow-400", hex: "#facc15" },
  code: { text: "text-orange-300", bg: "bg-orange-400/10", ring: "ring-orange-400/30", dot: "bg-orange-400", hex: "#fb923c" },
  security: { text: "text-rose-300", bg: "bg-rose-400/10", ring: "ring-rose-400/30", dot: "bg-rose-400", hex: "#fb7185" },
  flash: { text: "text-fuchsia-300", bg: "bg-fuchsia-400/10", ring: "ring-fuchsia-400/30", dot: "bg-fuchsia-400", hex: "#e879f9" },
};

export const SYSTEM_LABEL: Record<string, string> = {
  powertrain: "Powertrain",
  chassis: "Chassis",
  body: "Body",
  comfort: "Comfort",
  infotainment: "Infotainment",
  safety: "Safety",
};

export function timeOf(ms: number) {
  return new Date(ms).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}
