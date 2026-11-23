/**
 * Proportions for the 3D cars. Every model is our own procedural geometry,
 * built from these numbers at run time: no downloaded meshes, no maker
 * logos, grilles or trade dress. The dimensions follow each vehicle's
 * published length, width, height and wheelbase so the shapes read right,
 * while the styling stays generic for the body style.
 *
 * Distances are metres. `d` in a profile is the distance back from the
 * front bumper; `y` is the height above the ground.
 */
import type { Make, Pack } from "@/lib/tauri";

export type Style = Pack["body"];
export type Profile = [d: number, y: number][];

export interface CarSpec {
  id: string;
  style: Style;
  length: number;
  width: number;
  wheelbase: number;
  frontOverhang: number;
  /** Tyre outer radius, rim radius and tyre width. */
  tyre: { r: number; rim: number; width: number };
  /** Plan-view rounding of the nose and tail: zone length and squareness (higher = squarer). */
  nose: { len: number; n: number };
  tail: { len: number; n: number };
  /** Top of the lower body: bonnet, beltline and boot or tailgate. */
  top: Profile;
  /** Underside of the body (the sill line; wheel arches are cut from it). */
  bottom: Profile;
  /** How far the shoulder sits below `top`. */
  shoulder: Profile;
  /** Roof line from the windscreen base to the rear glass base. */
  roof: Profile;
  /** Roof width as a share of the body at the beltline (tumblehome). */
  tumble: number;
  /** Side-glass pillars as `d` positions. */
  pillars: { b: number; c: [number, number] };
  /** Door shut lines as `d` positions. */
  doors: number[];
  bonnet: number;
  paint: string;
  /** Roof and pillars in black (two-tone or a removable hard top). */
  blackRoof?: boolean;
  /** Fuel filler side. Varies by car: the arrow by the fuel gauge shows it. */
  filler: "left" | "right";
  /** Where the 12 V battery sits on this model, or "unknown" when we have not confirmed it. */
  battery: "engine" | "cargo" | "unknown";
  rimStyle: "multi" | "five" | "split" | "steel";
  /** Separate black plastic arches, lower cladding and steel bumpers. */
  rugged?: boolean;
  spare?: boolean;
}

const SUV_E70: CarSpec = {
  id: "bmw_e70",
  style: "suv",
  length: 4.857,
  width: 1.933,
  wheelbase: 2.933,
  frontOverhang: 0.87,
  tyre: { r: 0.37, rim: 0.235, width: 0.255 },
  nose: { len: 0.42, n: 2.6 },
  tail: { len: 0.32, n: 3.2 },
  top: [
    [0, 0.9],
    [0.08, 0.97],
    [0.3, 1.0],
    [0.9, 1.06],
    [1.55, 1.11],
    [3.2, 1.13],
    [4.55, 1.13],
    [4.8, 1.1],
    [4.857, 1.02],
  ],
  bottom: [
    [0, 0.44],
    [0.3, 0.36],
    [0.7, 0.33],
    [4.2, 0.33],
    [4.6, 0.38],
    [4.857, 0.52],
  ],
  shoulder: [
    [0, 0.04],
    [0.3, 0.09],
    [1.2, 0.12],
    [4.4, 0.12],
    [4.857, 0.05],
  ],
  roof: [
    [1.58, 1.11],
    [2.0, 1.47],
    [2.45, 1.72],
    [3.2, 1.766],
    [4.45, 1.735],
    [4.62, 1.6],
    [4.79, 1.13],
  ],
  tumble: 0.8,
  pillars: { b: 2.98, c: [3.98, 4.06] },
  doors: [1.6, 2.98, 3.98],
  bonnet: 1.5,
  paint: "#1f3a63",
  filler: "right",
  battery: "cargo",
  rimStyle: "multi",
};

const SUV_W167: CarSpec = {
  ...SUV_E70,
  id: "mercedes_w167",
  length: 4.924,
  width: 1.947,
  wheelbase: 2.995,
  frontOverhang: 0.93,
  tyre: { r: 0.385, rim: 0.254, width: 0.275 },
  nose: { len: 0.5, n: 2.3 },
  tail: { len: 0.4, n: 2.6 },
  top: [
    [0, 0.88],
    [0.1, 0.95],
    [0.4, 0.99],
    [1.0, 1.04],
    [1.65, 1.1],
    [3.3, 1.12],
    [4.55, 1.12],
    [4.85, 1.07],
    [4.924, 0.98],
  ],
  bottom: [
    [0, 0.42],
    [0.35, 0.34],
    [0.75, 0.32],
    [4.3, 0.32],
    [4.7, 0.38],
    [4.924, 0.5],
  ],
  shoulder: [
    [0, 0.04],
    [0.35, 0.1],
    [1.2, 0.13],
    [4.4, 0.13],
    [4.924, 0.05],
  ],
  roof: [
    [1.68, 1.1],
    [2.1, 1.43],
    [2.6, 1.7],
    [3.3, 1.772],
    [4.3, 1.74],
    [4.6, 1.62],
    [4.86, 1.13],
  ],
  tumble: 0.76,
  pillars: { b: 3.05, c: [4.02, 4.1] },
  doors: [1.7, 3.05, 4.02],
  bonnet: 1.6,
  paint: "#aab1bb",
  filler: "right",
  battery: "unknown",
  rimStyle: "five",
};

const SEDAN_XV70: CarSpec = {
  id: "toyota_xv70",
  style: "sedan",
  length: 4.885,
  width: 1.84,
  wheelbase: 2.825,
  frontOverhang: 0.98,
  tyre: { r: 0.335, rim: 0.229, width: 0.235 },
  nose: { len: 0.5, n: 2.4 },
  tail: { len: 0.28, n: 3.4 },
  top: [
    [0, 0.68],
    [0.1, 0.76],
    [0.45, 0.81],
    [1.1, 0.86],
    [1.78, 0.94],
    [3.4, 0.97],
    [4.1, 1.0],
    [4.6, 1.02],
    [4.83, 0.99],
    [4.885, 0.9],
  ],
  bottom: [
    [0, 0.3],
    [0.4, 0.21],
    [0.8, 0.19],
    [4.3, 0.2],
    [4.7, 0.28],
    [4.885, 0.42],
  ],
  shoulder: [
    [0, 0.03],
    [0.4, 0.07],
    [1.2, 0.09],
    [4.4, 0.09],
    [4.885, 0.05],
  ],
  roof: [
    [1.8, 0.94],
    [2.25, 1.22],
    [2.7, 1.42],
    [3.05, 1.445],
    [3.45, 1.41],
    [3.8, 1.25],
    [4.18, 1.0],
  ],
  tumble: 0.78,
  pillars: { b: 2.98, c: [3.42, 4.18] },
  doors: [1.82, 2.98, 3.72],
  bonnet: 1.7,
  paint: "#7d1720",
  filler: "left",
  battery: "engine",
  rimStyle: "split",
};

const SEDAN_ACCORD: CarSpec = {
  ...SEDAN_XV70,
  id: "honda_accord",
  length: 4.882,
  width: 1.862,
  wheelbase: 2.83,
  frontOverhang: 0.96,
  tyre: { r: 0.335, rim: 0.241, width: 0.235 },
  top: [
    [0, 0.66],
    [0.1, 0.74],
    [0.45, 0.8],
    [1.1, 0.85],
    [1.78, 0.93],
    [3.4, 0.97],
    [4.3, 0.99],
    [4.7, 1.0],
    [4.85, 0.97],
    [4.882, 0.88],
  ],
  roof: [
    [1.8, 0.93],
    [2.25, 1.2],
    [2.65, 1.41],
    [2.95, 1.45],
    [3.3, 1.42],
    [3.9, 1.2],
    [4.42, 0.99],
  ],
  pillars: { b: 2.95, c: [3.35, 4.42] },
  paint: "#4a5059",
  filler: "left",
  rimStyle: "five",
};

const OFFROAD_JL: CarSpec = {
  id: "jeep_jl",
  style: "offroad",
  length: 4.6,
  width: 1.894,
  wheelbase: 3.008,
  frontOverhang: 0.74,
  tyre: { r: 0.41, rim: 0.216, width: 0.27 },
  nose: { len: 0.12, n: 5 },
  tail: { len: 0.1, n: 5 },
  top: [
    [0, 1.02],
    [0.06, 1.1],
    [0.3, 1.12],
    [1.36, 1.15],
    [4.55, 1.16],
    [4.6, 1.12],
  ],
  bottom: [
    [0, 0.56],
    [0.4, 0.5],
    [4.3, 0.5],
    [4.6, 0.56],
  ],
  shoulder: [
    [0, 0.02],
    [0.2, 0.05],
    [4.4, 0.05],
    [4.6, 0.02],
  ],
  roof: [
    [1.38, 1.15],
    [1.52, 1.5],
    [1.66, 1.83],
    [3.0, 1.84],
    [4.5, 1.83],
    [4.56, 1.15],
  ],
  tumble: 0.93,
  pillars: { b: 2.45, c: [3.5, 3.58] },
  doors: [1.42, 2.45, 3.5],
  bonnet: 1.3,
  paint: "#3c5a3a",
  blackRoof: true,
  filler: "left",
  battery: "engine",
  rimStyle: "steel",
  rugged: true,
  spare: true,
};

const GENERIC: CarSpec = { ...SEDAN_XV70, id: "generic", paint: "#2f3640", rimStyle: "multi" };

const SPECS: Record<string, CarSpec> = {
  bmw_e70: SUV_E70,
  mercedes_w167: SUV_W167,
  toyota_xv70: SEDAN_XV70,
  honda_accord: SEDAN_ACCORD,
  jeep_jl: OFFROAD_JL,
  generic: GENERIC,
};

const BY_STYLE: Record<Style, CarSpec> = { suv: SUV_E70, sedan: SEDAN_XV70, offroad: OFFROAD_JL };

const BY_MAKE: Partial<Record<Make, CarSpec>> = {
  bmw: SUV_E70,
  mercedes_benz: SUV_W167,
  toyota: SEDAN_XV70,
  honda: SEDAN_ACCORD,
  jeep: OFFROAD_JL,
};

/** The spec for a vehicle pack, falling back to its make and then its body style. */
export function specFor(pack: Pick<Pack, "id" | "body" | "make"> | null | undefined, body: Style = "sedan"): CarSpec {
  if (!pack) return BY_STYLE[body];
  return SPECS[pack.id] ?? BY_MAKE[pack.make] ?? BY_STYLE[pack.body] ?? GENERIC;
}

export function specForBody(body: Style): CarSpec {
  return BY_STYLE[body];
}
