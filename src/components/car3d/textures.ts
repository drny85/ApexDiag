/**
 * Small canvas-drawn textures for the 3D cars: lamp lenses, grille mesh,
 * number plates and the soft ground shadow. Drawn once and shared.
 */
import * as THREE from "three";
import type { Style } from "./specs";

const cache = new Map<string, THREE.CanvasTexture>();

function make(key: string, w: number, h: number, draw: (g: CanvasRenderingContext2D, w: number, h: number) => void, srgb = true) {
  const hit = cache.get(key);
  if (hit) return hit;
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const g = c.getContext("2d")!;
  draw(g, w, h);
  const t = new THREE.CanvasTexture(c);
  if (srgb) t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  cache.set(key, t);
  return t;
}

function rounded(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  g.beginPath();
  g.roundRect(x, y, w, h, r);
}

/**
 * Headlamp: dark chrome housing, two projector lenses and a daytime
 * running-light strip. `glow` is the emissive mask (what lights up).
 * The texture's u runs from the outer corner (0) to the inboard end (1).
 */
export function headlampTexture(style: Style, glow: boolean) {
  return make(`head-${style}-${glow}`, 256, 96, (g, w, h) => {
    if (!glow) {
      const grd = g.createLinearGradient(0, 0, 0, h);
      grd.addColorStop(0, "#2a3038");
      grd.addColorStop(1, "#0c0f13");
      g.fillStyle = grd;
      g.fillRect(0, 0, w, h);
      // chrome reflector bowls
      for (const [cx, r] of style === "offroad" ? [[0.5, 0.36]] : [[0.32, 0.3], [0.62, 0.26]]) {
        const x = cx! * w;
        const rad = r! * h;
        const b = g.createRadialGradient(x - rad * 0.3, h * 0.45, rad * 0.1, x, h * 0.5, rad * 1.3);
        b.addColorStop(0, "#f5f7fa");
        b.addColorStop(0.35, "#9aa4b0");
        b.addColorStop(1, "#1b2027");
        g.fillStyle = b;
        g.beginPath();
        g.arc(x, h * 0.52, rad * 1.25, 0, Math.PI * 2);
        g.fill();
        g.fillStyle = "#dfe7f1";
        g.beginPath();
        g.arc(x, h * 0.52, rad * 0.55, 0, Math.PI * 2);
        g.fill();
      }
      // amber turn signal at the outer end
      g.fillStyle = "#b86a12";
      rounded(g, w * 0.03, h * 0.3, w * 0.1, h * 0.42, 6);
      g.fill();
    } else {
      g.fillStyle = "#000";
      g.fillRect(0, 0, w, h);
    }
    // running-light strip along the top edge
    g.strokeStyle = glow ? "#ffffff" : "#c9d6e6";
    g.lineWidth = h * 0.09;
    g.lineCap = "round";
    g.beginPath();
    g.moveTo(w * 0.06, h * 0.16);
    g.lineTo(w * 0.92, h * 0.2);
    g.stroke();
    if (glow) {
      for (const cx of style === "offroad" ? [0.5] : [0.32, 0.62]) {
        const b = g.createRadialGradient(cx * w, h * 0.52, 2, cx * w, h * 0.52, h * 0.38);
        b.addColorStop(0, "#ffffff");
        b.addColorStop(1, "rgba(255,255,255,0)");
        g.fillStyle = b;
        g.fillRect(0, 0, w, h);
      }
    }
  });
}

/** Rear lamp: red lens with light bars; `glow` is the lit mask. */
export function taillampTexture(glow: boolean) {
  return make(`tail-${glow}`, 256, 96, (g, w, h) => {
    if (!glow) {
      const grd = g.createLinearGradient(0, 0, 0, h);
      grd.addColorStop(0, "#8a0f18");
      grd.addColorStop(0.5, "#4f060c");
      grd.addColorStop(1, "#2a0306");
      g.fillStyle = grd;
      g.fillRect(0, 0, w, h);
      // clear reverse section and amber indicator
      g.fillStyle = "#d9dde3";
      rounded(g, w * 0.66, h * 0.58, w * 0.18, h * 0.26, 5);
      g.fill();
      g.fillStyle = "#c46f12";
      rounded(g, w * 0.42, h * 0.58, w * 0.2, h * 0.26, 5);
      g.fill();
    } else {
      g.fillStyle = "#000";
      g.fillRect(0, 0, w, h);
    }
    g.strokeStyle = glow ? "#ff2a3a" : "#ff4a55";
    g.lineWidth = h * 0.1;
    g.lineCap = "round";
    for (const y of [0.24, 0.42]) {
      g.beginPath();
      g.moveTo(w * 0.05, h * y);
      g.lineTo(w * 0.95, h * (y + 0.03));
      g.stroke();
    }
  });
}

/** Grille: horizontal bars (rugged) or a fine mesh, with a rounded outline mask. */
export function grilleTexture(kind: "mesh" | "bars") {
  return make(`grille-${kind}`, 256, 128, (g, w, h) => {
    g.fillStyle = "#07090b";
    g.fillRect(0, 0, w, h);
    if (kind === "bars") {
      g.fillStyle = "#3a414a";
      for (let y = 10; y < h; y += 18) g.fillRect(0, y, w, 6);
    } else {
      g.strokeStyle = "#2c333b";
      g.lineWidth = 2;
      for (let x = -h; x < w + h; x += 12) {
        g.beginPath();
        g.moveTo(x, 0);
        g.lineTo(x + h, h);
        g.moveTo(x + h, 0);
        g.lineTo(x, h);
        g.stroke();
      }
    }
  });
}

/** Alpha mask: a rounded rectangle, for lamps and grilles. */
export function roundMask(r = 0.22) {
  return make(
    `mask-${r}`,
    128,
    64,
    (g, w, h) => {
      g.fillStyle = "#000";
      g.fillRect(0, 0, w, h);
      g.fillStyle = "#fff";
      rounded(g, 1, 1, w - 2, h - 2, Math.min(w, h) * r);
      g.fill();
    },
    false,
  );
}

/** A plain, made-up number plate. */
export function plateTexture() {
  return make("plate", 256, 64, (g, w, h) => {
    g.fillStyle = "#eef1f4";
    rounded(g, 0, 0, w, h, 8);
    g.fill();
    g.strokeStyle = "#1c2430";
    g.lineWidth = 4;
    rounded(g, 4, 4, w - 8, h - 8, 6);
    g.stroke();
    g.fillStyle = "#1c2430";
    g.font = "bold 40px ui-monospace, Menlo, monospace";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText("APX 001", w / 2, h / 2 + 2);
  });
}

/** Soft contact shadow under the car. */
export function shadowTexture() {
  return make(
    "shadow",
    256,
    256,
    (g, w, h) => {
      const grd = g.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, w / 2);
      grd.addColorStop(0, "rgba(0,0,0,0.85)");
      grd.addColorStop(0.55, "rgba(0,0,0,0.5)");
      grd.addColorStop(1, "rgba(0,0,0,0)");
      g.fillStyle = grd;
      g.fillRect(0, 0, w, h);
    },
    false,
  );
}

/** Soft radial glow for lit lamps and the highlight beacon. */
export function glowTexture() {
  return make(
    "glow",
    128,
    128,
    (g, w, h) => {
      const grd = g.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, w / 2);
      grd.addColorStop(0, "rgba(255,255,255,1)");
      grd.addColorStop(0.25, "rgba(255,255,255,0.45)");
      grd.addColorStop(1, "rgba(255,255,255,0)");
      g.fillStyle = grd;
      g.fillRect(0, 0, w, h);
    },
    false,
  );
}

/** Vertical scan band that sweeps along the car while it is being scanned. */
export function scanTexture() {
  return make(
    "scan",
    256,
    16,
    (g, w, h) => {
      const grd = g.createLinearGradient(0, 0, w, 0);
      grd.addColorStop(0, "rgba(103,232,249,0)");
      grd.addColorStop(0.8, "rgba(103,232,249,0.35)");
      grd.addColorStop(0.97, "rgba(207,250,254,1)");
      grd.addColorStop(1, "rgba(103,232,249,0)");
      g.fillStyle = grd;
      g.fillRect(0, 0, w, h);
    },
    false,
  );
}
