/**
 * Builds a 3D car from a CarSpec: a lofted lower body with real wheel
 * arches, a glasshouse with tumblehome, lamps, grille, trim and wheels.
 * Lamps and trim are thin patches that follow the body surface, so they
 * wrap round corners the way real ones do.
 *
 * Axes: +x forward, +y up, +z to the car's right. The driver sits on the
 * left (-z), as on the cars these packs cover.
 */
import * as THREE from "three";
import type { CarSpec, Profile } from "./specs";
import { glowTexture, grilleTexture, headlampTexture, plateTexture, roundMask, shadowTexture, taillampTexture } from "./textures";

export type PartId =
  | "headlights"
  | "taillights"
  | "fogs"
  | "third_brake"
  | "plate_light"
  | "mirrors"
  | "bonnet"
  | "boot"
  | "engine"
  | "battery"
  | "filler"
  | "obd"
  | "wheels";

export interface BuiltCar {
  group: THREE.Group;
  /** Meshes that make up each part, for highlighting. */
  parts: Partial<Record<PartId, THREE.Mesh[]>>;
  /** A point on or in the car for each part, for labels and camera aim. */
  anchors: Record<PartId, THREE.Vector3>;
  setLights(on: boolean): void;
  /** Body size, for framing. */
  size: THREE.Vector3;
  dispose(): void;
}

/* ---------- profiles ---------- */

/** Monotone cubic interpolation through profile points (no overshoot). */
function curve(points: Profile): (d: number) => number {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const n = xs.length;
  const m: number[] = new Array(n).fill(0);
  const del: number[] = [];
  for (let i = 0; i < n - 1; i++) del.push((ys[i + 1]! - ys[i]!) / (xs[i + 1]! - xs[i]!));
  m[0] = del[0]!;
  m[n - 1] = del[n - 2]!;
  for (let i = 1; i < n - 1; i++) {
    const a = del[i - 1]!;
    const b = del[i]!;
    m[i] = a * b <= 0 ? 0 : (a + b) / 2;
  }
  for (let i = 0; i < n - 1; i++) {
    const d = del[i]!;
    if (d === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i]! / d;
    const b = m[i + 1]! / d;
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[i] = t * a * d;
      m[i + 1] = t * b * d;
    }
  }
  return (x: number) => {
    if (x <= xs[0]!) return ys[0]!;
    if (x >= xs[n - 1]!) return ys[n - 1]!;
    let i = 0;
    while (i < n - 2 && x > xs[i + 1]!) i++;
    const h = xs[i + 1]! - xs[i]!;
    const t = (x - xs[i]!) / h;
    const t2 = t * t;
    const t3 = t2 * t;
    return (2 * t3 - 3 * t2 + 1) * ys[i]! + (t3 - 2 * t2 + t) * h * m[i]! + (-2 * t3 + 3 * t2) * ys[i + 1]! + (t3 - t2) * h * m[i + 1]!;
  };
}

const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));

/* ---------- the car's shape as functions ---------- */

interface Station {
  x: number;
  w: number;
  yb: number;
  ys: number;
  yt: number;
  ybRef: number;
}

// Section segments for the lower body: bottom, bottom corner, side, shoulder, top.
const U1 = 0.08;
const U2 = 0.16;
const U3 = 0.62;
const U4 = 0.8;
// Glasshouse: side glass, roof rail, roof.
const G1 = 0.55;
const G2 = 0.72;

class Shape {
  readonly L: number;
  readonly W: number;
  readonly top: (d: number) => number;
  readonly bottom: (d: number) => number;
  readonly shoulder: (d: number) => number;
  readonly roof: (d: number) => number;
  readonly roofStart: number;
  readonly roofEnd: number;
  readonly wheels: { x: number; r: number }[];
  readonly arch: number;
  readonly bulge: number;
  readonly rz: number;
  readonly crown: number;
  readonly roofCrown: number;
  readonly hRef: number;
  readonly beltRef: number;
  readonly sillRef: number;

  constructor(readonly spec: CarSpec) {
    this.L = spec.length;
    this.W = spec.width;
    this.top = curve(spec.top);
    this.bottom = curve(spec.bottom);
    this.shoulder = curve(spec.shoulder);
    this.roof = curve(spec.roof);
    this.roofStart = spec.roof[0]![0];
    this.roofEnd = spec.roof[spec.roof.length - 1]![0];
    const fx = this.L / 2 - spec.frontOverhang;
    this.wheels = [
      { x: fx, r: spec.tyre.r },
      { x: fx - spec.wheelbase, r: spec.tyre.r },
    ];
    this.arch = spec.tyre.r + (spec.rugged ? 0.07 : 0.045);
    const rugged = spec.style === "offroad";
    this.bulge = rugged ? 0.012 : spec.style === "suv" ? 0.04 : 0.05;
    this.rz = rugged ? 0.05 : spec.style === "suv" ? 0.11 : 0.13;
    this.crown = rugged ? 0.012 : 0.04;
    this.roofCrown = rugged ? 0.012 : 0.04;
    let maxRoof = 0;
    for (const [, y] of spec.roof) maxRoof = Math.max(maxRoof, y);
    this.beltRef = this.top(this.roofStart + 0.5);
    this.hRef = maxRoof - this.beltRef;
    this.sillRef = this.bottom(this.L / 2);
  }

  d(x: number) {
    return this.L / 2 - x;
  }
  x(d: number) {
    return this.L / 2 - d;
  }

  /** Station parameter q: 0..1 tail corner, 1..2 straight sides, 2..3 nose corner. */
  xw(q: number): { x: number; w: number } {
    const { L, W } = this;
    const er = this.spec.tail.len;
    const ef = this.spec.nose.len;
    const hw = W / 2;
    if (q <= 1) {
      const th = clamp(q, 0, 1) * (Math.PI / 2);
      const n = this.spec.tail.n;
      return { x: -L / 2 + er - er * Math.pow(Math.max(0, Math.cos(th)), 2 / n), w: hw * Math.pow(Math.max(0, Math.sin(th)), 2 / n) };
    }
    if (q <= 2) return { x: -L / 2 + er + (q - 1) * (L - er - ef), w: hw };
    const th = (3 - clamp(q, 2, 3)) * (Math.PI / 2);
    const n = this.spec.nose.n;
    return { x: L / 2 - ef + ef * Math.pow(Math.max(0, Math.cos(th)), 2 / n), w: hw * Math.pow(Math.max(0, Math.sin(th)), 2 / n) };
  }

  archY(x: number): number {
    let y = -Infinity;
    for (const wh of this.wheels) {
      const dx = x - wh.x;
      if (Math.abs(dx) < this.arch) y = Math.max(y, wh.r + Math.sqrt(this.arch * this.arch - dx * dx));
    }
    return y;
  }

  station(q: number): Station {
    const { x, w } = this.xw(q);
    const d = this.d(x);
    const ybRef = this.bottom(d);
    const yt = this.top(d);
    const ys = yt - this.shoulder(d);
    const yb = Math.min(Math.max(ybRef, this.archY(x)), ys - 0.08);
    return { x, w, yb, ys, yt, ybRef };
  }

  /** Half-width of the body side at height y for this station. */
  zSide(st: Station, y: number) {
    const span = Math.max(0.2, st.ys - st.ybRef);
    const yw = st.ybRef + span * 0.42;
    const k = (y - yw) / span;
    return st.w * (1 - this.bulge * k * k);
  }

  rb(st: Station) {
    return Math.min(0.05, st.w * 0.3, (st.ys - st.yb) * 0.3);
  }
  rzs(st: Station) {
    return Math.min(this.rz, st.w * 0.45);
  }

  /** Point on the lower body section: returns [y, z] with z >= 0. */
  body(st: Station, u: number): [number, number] {
    const rb = this.rb(st);
    const zc = this.zSide(st, st.yb + rb) - rb;
    if (u <= U1) return [st.yb, (u / U1) * Math.max(0, zc)];
    if (u <= U2) {
      const a = -Math.PI / 2 + ((u - U1) / (U2 - U1)) * (Math.PI / 2);
      return [st.yb + rb + rb * Math.sin(a), Math.max(0, zc) + rb * Math.cos(a)];
    }
    if (u <= U3) {
      const y = st.yb + rb + ((u - U2) / (U3 - U2)) * (st.ys - st.yb - rb);
      return [y, this.zSide(st, y)];
    }
    const rz = this.rzs(st);
    const zs = this.zSide(st, st.ys);
    if (u <= U4) {
      const a = ((u - U3) / (U4 - U3)) * (Math.PI / 2);
      return [st.ys + (st.yt - st.ys) * Math.sin(a), zs - rz + rz * Math.cos(a)];
    }
    const z0 = Math.max(0, zs - rz);
    const t = (u - U4) / (1 - U4);
    const z = z0 * (1 - t);
    const crown = this.crown * Math.min(1, st.w / 0.5);
    return [st.yt + (z0 > 0 ? crown * (1 - (z / z0) ** 2) : 0), z];
  }

  /** u on the side segment for a height y (clamped to the side). */
  uForY(st: Station, y: number) {
    const rb = this.rb(st);
    const lo = st.yb + rb;
    return U2 + clamp((y - lo) / Math.max(1e-4, st.ys - lo), 0, 1) * (U3 - U2);
  }

  /** u on the top segment for a half-width z. */
  uForTopZ(st: Station, z: number) {
    const z0 = Math.max(1e-4, this.zSide(st, st.ys) - this.rzs(st));
    return U4 + clamp(1 - z / z0, 0, 1) * (1 - U4);
  }

  /** Glasshouse section at x: returns [y, z]. */
  glass(x: number, u: number): [number, number] {
    const d = this.d(x);
    const yb = this.top(d) - 0.004;
    const yr = Math.max(yb, this.roof(d));
    const h = yr - yb;
    const st = this.station(this.qForX(x));
    const gb = Math.max(0, this.zSide(st, st.ys) - this.rzs(st)) * 0.985;
    const gt = gb * (1 - (1 - this.spec.tumble) * Math.min(1, h / this.hRef));
    const rr = Math.min(0.055, h * 0.3);
    const rrz = Math.min(0.08, gt * 0.3);
    if (u <= G1) {
      const t = u / G1;
      return [yb + t * (h - rr), gb + (gt - gb) * t + 0.012 * Math.sin(Math.PI * t) * Math.min(1, h / 0.3)];
    }
    if (u <= G2) {
      const a = ((u - G1) / (G2 - G1)) * (Math.PI / 2);
      return [yr - rr + rr * Math.sin(a), gt - rrz + rrz * Math.cos(a)];
    }
    const z0 = Math.max(0, gt - rrz);
    const t = (u - G2) / (1 - G2);
    const z = z0 * (1 - t);
    const crown = this.roofCrown * Math.min(1, h / 0.2);
    return [yr + (z0 > 0 ? crown * (1 - (z / z0) ** 2) : 0), z];
  }

  /** Station parameter for an x on the car (bisection; x(q) is increasing). */
  qForX(x: number) {
    let a = 0;
    let b = 3;
    for (let i = 0; i < 40; i++) {
      const m = (a + b) / 2;
      if (this.xw(m).x < x) a = m;
      else b = m;
    }
    return (a + b) / 2;
  }

  /** Station parameter on the nose (front=true) or tail where the half-width is w. */
  qForW(w: number, front: boolean) {
    let a = front ? 2 : 0;
    let b = front ? 3 : 1;
    for (let i = 0; i < 40; i++) {
      const m = (a + b) / 2;
      const mw = this.xw(m).w;
      // width falls as q rises on the nose, and rises with q on the tail
      if (front ? mw > w : mw < w) a = m;
      else b = m;
    }
    return (a + b) / 2;
  }
}

/* ---------- geometry helpers ---------- */

type Pt = [number, number, number];

/** An indexed grid surface. `pt(i, j)` gives the vertex; mirrored grids flip winding. */
function grid(nI: number, nJ: number, pt: (i: number, j: number) => Pt, mirror: boolean, uvx?: (s: number, mirror: boolean) => number): THREE.BufferGeometry {
  const pos = new Float32Array(nI * nJ * 3);
  const uv = new Float32Array(nI * nJ * 2);
  for (let i = 0; i < nI; i++) {
    for (let j = 0; j < nJ; j++) {
      const k = i * nJ + j;
      const [x, y, z] = pt(i, j);
      pos[k * 3] = x;
      pos[k * 3 + 1] = y;
      pos[k * 3 + 2] = mirror ? -z : z;
      // mirrored patches read their texture from the same (outer) end
      uv[k * 2] = uvx ? uvx(i / (nI - 1), mirror) : i / (nI - 1);
      uv[k * 2 + 1] = j / (nJ - 1);
    }
  }
  const idx: number[] = [];
  for (let i = 0; i < nI - 1; i++) {
    for (let j = 0; j < nJ - 1; j++) {
      const a = i * nJ + j;
      const b = (i + 1) * nJ + j;
      const c = i * nJ + j + 1;
      const d = (i + 1) * nJ + j + 1;
      if (mirror) idx.push(a, c, b, b, c, d);
      else idx.push(a, b, c, b, d, c);
    }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  g.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
  g.setIndex(idx);
  g.computeVertexNormals();
  return g;
}

/** Pushes every vertex out along its normal (for patches that sit on the body). */
function inflate(g: THREE.BufferGeometry, by: number) {
  const p = g.getAttribute("position") as THREE.BufferAttribute;
  const n = g.getAttribute("normal") as THREE.BufferAttribute;
  for (let i = 0; i < p.count; i++) {
    p.setXYZ(i, p.getX(i) + n.getX(i) * by, p.getY(i) + n.getY(i) * by, p.getZ(i) + n.getZ(i) * by);
  }
  p.needsUpdate = true;
  g.computeBoundingSphere();
  return g;
}

/* ---------- materials ---------- */

function materials(spec: CarSpec) {
  const paint = new THREE.MeshPhysicalMaterial({
    color: spec.paint,
    metalness: 0.55,
    roughness: 0.32,
    clearcoat: 1,
    clearcoatRoughness: 0.05,
  });
  const glass = new THREE.MeshPhysicalMaterial({ color: "#151d27", metalness: 0.25, roughness: 0.04, clearcoat: 1, clearcoatRoughness: 0.02, envMapIntensity: 1.4 });
  const gloss = new THREE.MeshPhysicalMaterial({ color: "#0b0d10", metalness: 0.3, roughness: 0.22, clearcoat: 0.8 });
  const plastic = new THREE.MeshStandardMaterial({ color: "#16191d", metalness: 0, roughness: 0.82 });
  const seam = new THREE.MeshBasicMaterial({ color: "#030405" });
  const chrome = new THREE.MeshStandardMaterial({ color: "#dfe5ec", metalness: 1, roughness: 0.12 });
  const mirror = new THREE.MeshStandardMaterial({ color: "#7d8996", metalness: 1, roughness: 0.04 });
  const rubber = new THREE.MeshStandardMaterial({ color: "#131518", metalness: 0, roughness: 0.93 });
  const rim =
    spec.rimStyle === "steel"
      ? new THREE.MeshStandardMaterial({ color: "#30363d", metalness: 0.6, roughness: 0.42 })
      : new THREE.MeshStandardMaterial({ color: "#c3c9d0", metalness: 1, roughness: 0.26 });
  const darkMetal = new THREE.MeshStandardMaterial({ color: "#2a2f35", metalness: 0.8, roughness: 0.5 });
  const disc = new THREE.MeshStandardMaterial({ color: "#7a8087", metalness: 1, roughness: 0.42 });
  const caliper = new THREE.MeshStandardMaterial({ color: spec.style === "offroad" ? "#3a3f45" : "#8d1f22", metalness: 0.3, roughness: 0.45 });
  const head = new THREE.MeshPhysicalMaterial({
    map: headlampTexture(spec.style, false),
    emissiveMap: headlampTexture(spec.style, true),
    emissive: new THREE.Color("#eaf4ff"),
    emissiveIntensity: 0.6,
    alphaMap: roundMask(0.35),
    alphaTest: 0.5,
    metalness: 0.6,
    roughness: 0.2,
    clearcoat: 1,
  });
  const tail = new THREE.MeshPhysicalMaterial({
    map: taillampTexture(false),
    emissiveMap: taillampTexture(true),
    emissive: new THREE.Color("#ff1e30"),
    emissiveIntensity: 0.35,
    alphaMap: roundMask(0.35),
    alphaTest: 0.5,
    metalness: 0.2,
    roughness: 0.15,
    clearcoat: 1,
  });
  const grille = new THREE.MeshStandardMaterial({ map: grilleTexture(spec.rugged ? "bars" : "mesh"), metalness: 0.4, roughness: 0.5 });
  const plate = new THREE.MeshStandardMaterial({ map: plateTexture(), roughness: 0.5, metalness: 0.1 });
  const fog = new THREE.MeshStandardMaterial({ color: "#c8d2dc", emissive: new THREE.Color("#fff6e0"), emissiveIntensity: 0, metalness: 0.8, roughness: 0.15, alphaMap: roundMask(0.5), alphaTest: 0.5 });
  const red = new THREE.MeshStandardMaterial({ color: "#5a070d", emissive: new THREE.Color("#ff1e30"), emissiveIntensity: 0.3, roughness: 0.2 });
  const plateLamp = new THREE.MeshStandardMaterial({ color: "#d9dee5", emissive: new THREE.Color("#ffffff"), emissiveIntensity: 0, roughness: 0.2 });
  return { paint, glass, mirror, gloss, plastic, seam, chrome, rubber, rim, darkMetal, disc, caliper, head, tail, grille, plate, fog, red, plateLamp };
}

/* ---------- wheel ---------- */

function wheel(spec: CarSpec, m: ReturnType<typeof materials>): THREE.Group {
  const { r: R, rim: rr, width: w } = spec.tyre;
  const g = new THREE.Group();
  // tyre: a lathed rounded section around the axle (+z axis after rotation)
  const c = Math.min(0.045, w * 0.22);
  const prof: THREE.Vector2[] = [];
  prof.push(new THREE.Vector2(rr * 0.98, -w / 2 + 0.01));
  prof.push(new THREE.Vector2(rr + 0.01, -w / 2));
  for (let k = 0; k <= 6; k++) {
    const a = -Math.PI / 2 + (k / 6) * (Math.PI / 2);
    prof.push(new THREE.Vector2(R - c + c * Math.cos(a), -w / 2 + c + c * Math.sin(a)));
  }
  for (let k = 0; k <= 6; k++) {
    const a = (k / 6) * (Math.PI / 2);
    prof.push(new THREE.Vector2(R - c + c * Math.cos(a), w / 2 - c + c * Math.sin(a)));
  }
  prof.push(new THREE.Vector2(rr + 0.01, w / 2));
  prof.push(new THREE.Vector2(rr * 0.98, w / 2 - 0.01));
  const tyre = new THREE.Mesh(new THREE.LatheGeometry(prof, 56), m.rubber);
  tyre.rotation.x = Math.PI / 2;
  g.add(tyre);
  if (spec.rugged) {
    // chunky tread blocks on an off-road tyre
    const lug = new THREE.BoxGeometry(0.05, 0.022, w * 0.42);
    const n = 34;
    const lugs = new THREE.InstancedMesh(lug, m.rubber, n * 2);
    const o = new THREE.Object3D();
    for (let i = 0; i < n * 2; i++) {
      const a = ((i % n) / n) * Math.PI * 2 + (i >= n ? Math.PI / n : 0);
      o.position.set(Math.cos(a) * (R - 0.004), Math.sin(a) * (R - 0.004), i >= n ? w * 0.24 : -w * 0.24);
      o.rotation.set(0, 0, a + Math.PI / 2);
      o.updateMatrix();
      lugs.setMatrixAt(i, o.matrix);
    }
    g.add(lugs);
  }
  // rim barrel, brake disc and caliper behind the face
  const barrel = new THREE.Mesh(new THREE.CylinderGeometry(rr * 0.98, rr * 0.98, w * 0.9, 40, 1, true), m.darkMetal);
  barrel.rotation.x = Math.PI / 2;
  (barrel.material as THREE.Material).side = THREE.DoubleSide;
  g.add(barrel);
  const discM = new THREE.Mesh(new THREE.CylinderGeometry(rr * 0.8, rr * 0.8, 0.028, 40), m.disc);
  discM.rotation.x = Math.PI / 2;
  discM.position.z = w * 0.05;
  g.add(discM);
  const cal = new THREE.Mesh(new THREE.BoxGeometry(rr * 0.42, rr * 0.22, 0.07), m.caliper);
  cal.position.set(-rr * 0.52, rr * 0.42, w * 0.08);
  cal.rotation.z = -0.65;
  g.add(cal);
  // rim face with spoke windows
  const face = new THREE.Shape();
  face.absarc(0, 0, rr * 0.99, 0, Math.PI * 2, false);
  const hub = rr * 0.26;
  const r2 = rr * 0.86;
  const addWindow = (a0: number, a1: number, sw: number, r1: number, r2b: number) => {
    const h = new THREE.Path();
    const i0 = a0 + sw / (2 * r1);
    const o0 = a0 + sw / (2 * r2b);
    const i1 = a1 - sw / (2 * r1);
    const o1 = a1 - sw / (2 * r2b);
    h.moveTo(Math.cos(i0) * r1, Math.sin(i0) * r1);
    h.lineTo(Math.cos(o0) * r2b, Math.sin(o0) * r2b);
    h.absarc(0, 0, r2b, o0, o1, false);
    h.lineTo(Math.cos(i1) * r1, Math.sin(i1) * r1);
    h.absarc(0, 0, r1, i1, i0, true);
    face.holes.push(h);
  };
  if (spec.rimStyle === "steel") {
    for (let k = 0; k < 8; k++) {
      const a = (k / 8) * Math.PI * 2;
      const h = new THREE.Path();
      h.absarc(Math.cos(a) * rr * 0.62, Math.sin(a) * rr * 0.62, rr * 0.11, 0, Math.PI * 2, true);
      face.holes.push(h);
    }
  } else {
    const n = spec.rimStyle === "multi" ? 10 : 5;
    const sw = spec.rimStyle === "five" ? rr * 0.24 : spec.rimStyle === "split" ? rr * 0.24 : rr * 0.1;
    for (let k = 0; k < n; k++) {
      const a0 = (k / n) * Math.PI * 2;
      const a1 = ((k + 1) / n) * Math.PI * 2;
      if (spec.rimStyle === "split") {
        // each spoke split by a thin slot
        addWindow(a0, a1, sw, hub * 1.15, r2);
        const mid = a0;
        const h = new THREE.Path();
        const s = 0.006;
        h.moveTo(Math.cos(mid) * hub * 1.6 - Math.sin(mid) * s, Math.sin(mid) * hub * 1.6 + Math.cos(mid) * s);
        h.lineTo(Math.cos(mid) * r2 * 0.95 - Math.sin(mid) * s, Math.sin(mid) * r2 * 0.95 + Math.cos(mid) * s);
        h.lineTo(Math.cos(mid) * r2 * 0.95 + Math.sin(mid) * s, Math.sin(mid) * r2 * 0.95 - Math.cos(mid) * s);
        h.lineTo(Math.cos(mid) * hub * 1.6 + Math.sin(mid) * s, Math.sin(mid) * hub * 1.6 - Math.cos(mid) * s);
        h.closePath();
        face.holes.push(h);
      } else addWindow(a0, a1, sw, hub * 1.15, r2);
    }
  }
  const faceGeo = new THREE.ExtrudeGeometry(face, { depth: 0.02, bevelEnabled: true, bevelThickness: 0.012, bevelSize: 0.006, bevelSegments: 2, curveSegments: 40 });
  const faceM = new THREE.Mesh(faceGeo, m.rim);
  faceM.position.z = w / 2 - 0.05;
  g.add(faceM);
  const cap = new THREE.Mesh(new THREE.CylinderGeometry(hub * 0.55, hub * 0.6, 0.02, 24), spec.rimStyle === "steel" ? m.chrome : m.darkMetal);
  cap.rotation.x = Math.PI / 2;
  cap.position.z = w / 2 - 0.012;
  g.add(cap);
  return g;
}

/* ---------- the car ---------- */

export function buildCar(spec: CarSpec): BuiltCar {
  const S = new Shape(spec);
  const m = materials(spec);
  const group = new THREE.Group();
  const parts: Partial<Record<PartId, THREE.Mesh[]>> = {};
  const add = (geo: THREE.BufferGeometry, mat: THREE.Material, part?: PartId) => {
    const mesh = new THREE.Mesh(geo, mat);
    group.add(mesh);
    if (part) (parts[part] ??= []).push(mesh);
    return mesh;
  };
  const both = (make: (mirror: boolean) => THREE.BufferGeometry, mat: THREE.Material, part?: PartId) => {
    add(make(false), mat, part);
    add(make(true), mat, part);
  };

  // Station list: dense round the corners and at the arch edges.
  const qs: number[] = [];
  for (let i = 0; i <= 28; i++) qs.push(i / 28);
  const midLen = S.L - spec.tail.len - spec.nose.len;
  const steps = Math.ceil(midLen / 0.03);
  const extra: number[] = [];
  for (const wh of S.wheels) for (const dx of [-S.arch, S.arch]) for (const e of [-0.004, 0.004]) extra.push(wh.x + dx + e);
  const mids: number[] = [];
  for (let i = 1; i < steps; i++) mids.push(1 + i / steps);
  for (const x of extra) {
    const q = 1 + (x - (-S.L / 2 + spec.tail.len)) / midLen;
    if (q > 1 && q < 2) mids.push(q);
  }
  mids.sort((a, b) => a - b);
  qs.push(...mids);
  for (let i = 0; i <= 28; i++) qs.push(2 + i / 28);
  const stations = qs.map((q) => S.station(q));
  const NU = 46;

  // Lower body
  both(
    (mir) =>
      grid(
        stations.length,
        NU,
        (i, j) => {
          const st = stations[i]!;
          const [y, z] = S.body(st, j / (NU - 1));
          return [st.x, y, z];
        },
        mir,
      ),
    m.paint,
    undefined,
  );

  // Glasshouse
  const gx0 = S.x(S.roofStart);
  const gx1 = S.x(S.roofEnd);
  const gN = Math.ceil((gx0 - gx1) / 0.025) + 1;
  const gxs = Array.from({ length: gN }, (_, i) => gx1 + ((gx0 - gx1) * i) / (gN - 1));
  const GU = 30;
  both(
    (mir) =>
      grid(
        gN,
        GU,
        (i, j) => {
          const x = gxs[i]!;
          const [y, z] = S.glass(x, j / (GU - 1));
          return [x, y, z];
        },
        mir,
      ),
    m.glass,
  );

  /** A patch on the lower body between station params, with u limits per station. */
  // Patches always run front-wards in i so their normals face out.
  const bodyPatch =
    (qa: number, qb: number, uRange: (st: Station) => [number, number], off: number, nI = 24, nJ = 8, uvx?: (s: number, mirror: boolean) => number) =>
    (mir: boolean) => {
      const q0 = Math.min(qa, qb);
      const q1 = Math.max(qa, qb);
      return inflate(
        grid(
          nI,
          nJ,
          (i, j) => {
            const st = S.station(q0 + ((q1 - q0) * i) / (nI - 1));
            const [u0, u1] = uRange(st);
            const [y, z] = S.body(st, u0 + ((u1 - u0) * j) / (nJ - 1));
            return [st.x, y, z];
          },
          mir,
          uvx,
        ),
        off,
      );
    };
  /** A patch on the glasshouse between x positions. */
  const glassPatch = (xa: number, xb: number, uRange: (x: number) => [number, number], off: number, nI = 24, nJ = 10) => (mir: boolean) => {
    const x0 = Math.min(xa, xb);
    const x1 = Math.max(xa, xb);
    return inflate(
      grid(
        nI,
        nJ,
        (i, j) => {
          const x = x0 + ((x1 - x0) * i) / (nI - 1);
          const [u0, u1] = uRange(x);
          const [y, z] = S.glass(x, u0 + ((u1 - u0) * j) / (nJ - 1));
          return [x, y, z];
        },
        mir,
      ),
      off,
    );
  };

  const qx = (d: number) => S.qForX(S.x(d));
  const tip = S.station(3);
  const tail = S.station(0);
  const rugged = !!spec.rugged;

  // Roof, pillars and trim on the glasshouse
  const roofMat = spec.blackRoof ? m.gloss : m.paint;
  const peak = spec.roof.reduce((a, b) => (b[1] > a[1] ? b : a));
  const flatY = peak[1] - 0.05;
  let rf = S.roofStart;
  while (rf < peak[0] && S.roof(rf) < flatY) rf += 0.005;
  let rr = S.roofEnd;
  while (rr > peak[0] && S.roof(rr) < flatY) rr -= 0.005;
  both(glassPatch(S.x(rf), S.x(rr), () => [G1 + 0.02, 1], 0.004, 40, 12), roofMat);
  // A-pillars
  both(glassPatch(S.x(rf), gx0, (x) => [G1 - 0.08 * Math.min(1, (S.roof(S.d(x)) - S.top(S.d(x))) / 0.4), G2 + 0.03], 0.004, 24, 8), roofMat);
  // B-pillar
  const bx = S.x(spec.pillars.b);
  both(glassPatch(bx - 0.04, bx + 0.04, () => [0, G1 + 0.04], 0.004, 4, 10), m.gloss);
  // C/D pillar: from the rear door's glass to the back
  const [c0, c1] = spec.pillars.c;
  both(glassPatch(S.x(c1), S.x(c0), () => [0, G1 + 0.05], 0.005, 10, 10), spec.blackRoof ? m.gloss : m.paint);
  if (spec.style === "suv") {
    // body-colour D-pillar behind the rear quarter glass
    both(glassPatch(gx1, S.x(S.roofEnd - 0.3), () => [0, G1 + 0.05], 0.005, 12, 10), m.paint);
  }
  // chrome or black belt-line trim along the bottom of the glass
  both(glassPatch(gx1 + 0.02, gx0 - 0.02, () => [0, 0.035], 0.003, 60, 2), rugged ? m.gloss : m.chrome);

  // Black window surround along the roof rail, A-pillar to C-pillar
  both(glassPatch(S.x(c1), gx0 - 0.03, (x) => {
    const h = S.roof(S.d(x)) - S.top(S.d(x));
    return h < 0.08 ? [G1, G1] : [G1 - 0.05, G1 + 0.015];
  }, 0.0045, 60, 3), m.gloss);

  // Dark lower valances front and back on sedans
  if (spec.style === "sedan") {
    for (const [qa, qb] of [
      [0, qx(S.L - 0.22)],
      [qx(0.22), 3],
    ] as const)
      both(bodyPatch(qa, qb, (st) => [U1 - 0.02, S.uForY(st, st.yb + 0.07)], 0.003, 30, 4), m.plastic);
  }

  // Door shut lines and handles
  for (const dd of spec.doors) {
    const q = qx(dd);
    both(bodyPatch(q - 0.0012, q + 0.0012, () => [U2 + 0.01, U4 + 0.04], 0.0015, 2, 16), m.seam);
  }
  for (let k = 0; k < spec.doors.length - 1; k++) {
    const dEnd = spec.doors[k + 1]!;
    const q = qx(dEnd - 0.2);
    both(
      bodyPatch(q - 0.003, q + 0.003, (st) => [S.uForY(st, st.ys - 0.12), S.uForY(st, st.ys - 0.09)], 0.008, 4, 3),
      rugged ? m.plastic : m.chrome,
    );
  }
  // Bonnet shut lines along the shoulders
  const qb = qx(spec.bonnet);
  both(bodyPatch(qb, 2.9, () => [U4 + 0.02, U4 + 0.025], 0.0015, 40, 2), m.seam);
  both(bodyPatch(qb - 0.0012, qb + 0.0012, () => [U4 + 0.02, 1], 0.0015, 2, 12), m.seam);

  // Lower cladding (rockers, arches, bumper lips) on SUVs and the off-roader
  if (spec.style !== "sedan") {
    both(
      bodyPatch(0.02, 2.98, (st) => [U1 - 0.02, S.uForY(st, st.yb + (rugged ? 0.06 : 0.11))], 0.003, stations.length, 6),
      m.plastic,
    );
  }

  // Headlamps
  if (rugged) {
    both(bodyPatch(S.qForW(0.86, true), S.qForW(0.56, true), (st) => [S.uForY(st, st.ys - 0.24), S.uForY(st, st.ys - 0.05)], 0.006, 14, 8), m.head, "headlights");
  } else {
    both(
      bodyPatch(qx(spec.style === "suv" ? 0.5 : 0.62), S.qForW(0.5, true), (st) => [S.uForY(st, st.ys - (spec.style === "suv" ? 0.15 : 0.13)), S.uForY(st, st.ys - 0.012)], 0.005, 26, 8),
      m.head,
      "headlights",
    );
  }
  // Grille and lower intake
  if (rugged) {
    both(bodyPatch(S.qForW(0.46, true), 3, (st) => [S.uForY(st, st.ys - 0.3), S.uForY(st, st.ys - 0.04)], 0.004, 14, 10), m.grille);
  } else {
    both(bodyPatch(S.qForW(0.36, true), 3, (st) => [S.uForY(st, st.ys - 0.12), S.uForY(st, st.ys - 0.03)], 0.004, 14, 8), m.grille);
    both(bodyPatch(S.qForW(0.58, true), 3, (st) => [S.uForY(st, st.yb + 0.07), S.uForY(st, st.yb + 0.2)], 0.004, 16, 8), m.grille);
    // fog lamps in the bumper corners
    both(bodyPatch(S.qForW(0.78, true), S.qForW(0.66, true), (st) => [S.uForY(st, st.yb + 0.1), S.uForY(st, st.yb + 0.17)], 0.006, 8, 6), m.fog, "fogs");
  }
  // Front plate
  const plateGeo = (front: boolean) => (mir: boolean) =>
    bodyPatch(front ? S.qForW(0.26, true) : 0, front ? 3 : S.qForW(0.26, false), (st) => {
      const lo = front ? st.yb + (rugged ? 0.02 : 0.08) : st.yb + (st.ys - st.yb) * 0.34;
      return [S.uForY(st, lo), S.uForY(st, lo + 0.12)];
    }, 0.006, 10, 6, front ? (t, mr) => (mr ? 1 - 0.5 * t : 0.5 * t) : (t, mr) => (mr ? 0.5 - 0.5 * t : 0.5 + 0.5 * t))(mir);
  if (!rugged) both(plateGeo(true), m.plate);
  // Rear plate and its lamp
  both(plateGeo(false), m.plate);
  both((mir) => {
    const lo = tail.yb + (tail.ys - tail.yb) * 0.34 + 0.135;
    return bodyPatch(0, S.qForW(0.1, false), (st) => [S.uForY(st, lo), S.uForY(st, lo + 0.015)], 0.008, 4, 2)(mir);
  }, m.plateLamp, "plate_light");

  // Rear lamps
  if (rugged) {
    both(bodyPatch(S.qForW(0.8, false), qx(S.L - 0.05), (st) => [S.uForY(st, st.ys - 0.36), S.uForY(st, st.ys - 0.08)], 0.006, 10, 10), m.tail, "taillights");
  } else {
    both(
      bodyPatch(S.qForW(0.5, false), qx(S.L - (spec.style === "suv" ? 0.3 : 0.4)), (st) => [S.uForY(st, st.ys - (spec.style === "suv" ? 0.17 : 0.15)), S.uForY(st, st.ys - 0.01)], 0.005, 26, 8),
      m.tail,
      "taillights",
    );
  }
  // Boot or tailgate shut lines
  if (spec.style === "sedan") {
    const q = qx(S.roofEnd + 0.03);
    both(bodyPatch(q - 0.0012, q + 0.0012, () => [U3, 1], 0.0015, 2, 12), m.seam);
  } else {
    const q = S.qForW(0.66, false);
    both(bodyPatch(q - 0.002, q + 0.002, (st) => [S.uForY(st, st.yb + 0.16), U4 + 0.05], 0.0015, 2, 12), m.seam);
  }

  // Third brake lamp on the roof edge above the rear glass
  {
    const lamp = new THREE.Mesh(new THREE.BoxGeometry(0.05, 0.03, Math.min(0.32, S.W * 0.18)), m.red);
    lamp.position.set(S.x(rr) - 0.035, S.roof(rr) + S.roofCrown * 0.6 - 0.005, 0);
    group.add(lamp);
    (parts.third_brake ??= []).push(lamp);
  }

  // Fuel filler door on the rear quarter
  {
    const rx = S.wheels[1]!.x;
    const q = S.qForX(rx - (spec.style === "sedan" ? 0.25 : 0.12));
    const g = bodyPatch(q - 0.018, q + 0.018, (st) => [S.uForY(st, st.ys - 0.2), S.uForY(st, st.ys - 0.04)], 0.003, 8, 8);
    const mesh = add(g(spec.filler === "left"), m.paint, "filler");
    mesh.renderOrder = 1;
    both(bodyPatch(q - 0.0195, q - 0.0183, (st) => [S.uForY(st, st.ys - 0.2), S.uForY(st, st.ys - 0.04)], 0.0035, 2, 8), m.seam);
  }

  // Mirrors
  {
    const md = S.roofStart + 0.32;
    const mx = S.x(md);
    const st = S.station(S.qForX(mx));
    const gz = Math.max(0, S.zSide(st, st.ys) - S.rzs(st));
    const my = S.top(md) + 0.09;
    for (const side of [1, -1]) {
      const cap = new THREE.Mesh(new THREE.SphereGeometry(1, 24, 14), rugged ? m.gloss : m.paint);
      cap.scale.set(0.09, 0.075, 0.13);
      cap.position.set(mx - 0.02, my, side * (gz + 0.16));
      group.add(cap);
      const glassM = new THREE.Mesh(new THREE.CircleGeometry(1, 20), m.mirror);
      glassM.scale.set(0.07, 0.06, 1);
      glassM.position.set(mx - 0.105, my, side * (gz + 0.16));
      glassM.rotation.y = -Math.PI / 2;
      group.add(glassM);
      const stalk = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.04, 0.12), m.gloss);
      stalk.position.set(mx, my - 0.04, side * (gz + 0.06));
      group.add(stalk);
      (parts.mirrors ??= []).push(cap);
    }
  }

  // Roof rails (SUVs) and an antenna fin
  if (spec.style === "suv") {
    for (const side of [1, -1]) {
      const pts: THREE.Vector3[] = [];
      for (let k = 0; k <= 20; k++) {
        const d = rf + 0.1 + ((rr - rf - 0.15) * k) / 20;
        const x = S.x(d);
        const [y, z] = S.glass(x, G2 + 0.02);
        pts.push(new THREE.Vector3(x, y + 0.045, side * z * 0.98));
      }
      const tube = new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(pts), 40, 0.014, 8), m.chrome);
      group.add(tube);
    }
  }
  if (!rugged) {
    const fin = new THREE.Shape();
    fin.moveTo(0, 0);
    fin.lineTo(0.16, 0);
    fin.quadraticCurveTo(0.03, 0.02, 0, 0.065);
    fin.closePath();
    const geo = new THREE.ExtrudeGeometry(fin, { depth: 0.04, bevelEnabled: true, bevelThickness: 0.01, bevelSize: 0.01, bevelSegments: 3 });
    geo.translate(-0.08, 0, -0.02);
    const mesh = new THREE.Mesh(geo, m.gloss);
    const fd = rr - 0.1;
    mesh.position.set(S.x(fd) - 0.05, S.roof(fd) + S.roofCrown - 0.005, 0);
    mesh.rotation.y = Math.PI;
    group.add(mesh);
  }

  // Off-road hardware: arch flares, steel bumpers, spare wheel
  if (rugged) {
    for (const wh of S.wheels) {
      const band = new THREE.Shape();
      const r0 = S.arch - 0.005;
      const r1 = S.arch + 0.11;
      band.absarc(0, 0, r1, Math.PI * 0.02, Math.PI * 0.98, false);
      band.lineTo(Math.cos(Math.PI * 0.98) * r0, Math.sin(Math.PI * 0.98) * r0);
      band.absarc(0, 0, r0, Math.PI * 0.98, Math.PI * 0.02, true);
      band.closePath();
      const geo = new THREE.ExtrudeGeometry(band, { depth: 0.22, bevelEnabled: true, bevelThickness: 0.015, bevelSize: 0.012, bevelSegments: 2, curveSegments: 28 });
      for (const side of [1, -1]) {
        const flare = new THREE.Mesh(geo, m.plastic);
        flare.position.set(wh.x, wh.r, side > 0 ? S.W / 2 - 0.1 : -S.W / 2 + 0.1 - 0.22);
        group.add(flare);
      }
    }
    const bumper = (x: number, y: number) => {
      const b = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.16, S.W * 0.98), m.gloss);
      b.position.set(x, y, 0);
      group.add(b);
    };
    bumper(S.L / 2 - 0.02, tip.yb + 0.06);
    bumper(-S.L / 2 + 0.02, tail.yb + 0.04);
  }

  // Wheels and arch liners
  for (const wh of S.wheels) {
    const liner = new THREE.Mesh(new THREE.CylinderGeometry(S.arch - 0.01, S.arch - 0.01, S.W * 0.94, 28, 1, true, Math.PI / 2, Math.PI), m.plastic);
    liner.material = m.plastic.clone();
    (liner.material as THREE.Material).side = THREE.BackSide;
    liner.rotation.x = Math.PI / 2;
    liner.position.set(wh.x, wh.r, 0);
    group.add(liner);
    for (const side of [1, -1]) {
      const wg = wheel(spec, m);
      const track = S.W / 2 - spec.tyre.width / 2 - (rugged ? -0.015 : 0.03);
      wg.position.set(wh.x, wh.r, side * track);
      if (side < 0) wg.rotation.y = Math.PI;
      group.add(wg);
      wg.traverse((o) => {
        if (o instanceof THREE.Mesh) (parts.wheels ??= []).push(o);
      });
    }
  }
  if (spec.spare) {
    const sp = wheel(spec, m);
    sp.rotation.y = -Math.PI / 2;
    sp.position.set(-S.L / 2 - spec.tyre.width / 2 - 0.03, tail.yb + 0.42, 0);
    group.add(sp);
  }

  // Soft contact shadow
  const shadow = new THREE.Mesh(
    new THREE.PlaneGeometry(S.L * 1.3, S.W * 1.7),
    new THREE.MeshBasicMaterial({ map: shadowTexture(), transparent: true, depthWrite: false, opacity: 0.95 }),
  );
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.y = 0.002;
  shadow.renderOrder = -1;
  group.add(shadow);
  for (const wh of S.wheels)
    for (const side of [1, -1]) {
      const s = new THREE.Mesh(new THREE.PlaneGeometry(0.75, 0.5), shadow.material);
      s.rotation.x = -Math.PI / 2;
      s.position.set(wh.x, 0.003, side * (S.W / 2 - spec.tyre.width / 2));
      s.renderOrder = -1;
      group.add(s);
    }

  // Glow sprites for lit lamps
  const glow = glowTexture();
  const sprites: THREE.Sprite[] = [];
  const headSt = S.station(S.qForW(0.72, true));
  const headY = headSt.ys - (rugged ? 0.14 : 0.06);
  const tailSt = S.station(S.qForW(0.78, false));
  for (const side of [1, -1]) {
    const hs = new THREE.Sprite(new THREE.SpriteMaterial({ map: glow, color: "#dff1ff", blending: THREE.AdditiveBlending, depthWrite: false, transparent: true, opacity: 0 }));
    hs.position.set(headSt.x + 0.05, headY, side * (S.W / 2) * 0.72);
    hs.scale.setScalar(0.55);
    group.add(hs);
    sprites.push(hs);
    const ts = new THREE.Sprite(new THREE.SpriteMaterial({ map: glow, color: "#ff3344", blending: THREE.AdditiveBlending, depthWrite: false, transparent: true, opacity: 0 }));
    ts.position.set(tailSt.x - 0.05, tailSt.ys - 0.07, side * (S.W / 2) * 0.78);
    ts.scale.setScalar(0.4);
    group.add(ts);
    sprites.push(ts);
  }

  // Anchors
  const bonD = spec.bonnet * 0.5;
  const anchors: Record<PartId, THREE.Vector3> = {
    headlights: new THREE.Vector3(headSt.x, headY, -(S.W / 2) * 0.74),
    taillights: new THREE.Vector3(tailSt.x, tailSt.ys - 0.07, -(S.W / 2) * 0.8),
    fogs: new THREE.Vector3(tip.x - 0.05, tip.yb + 0.13, -(S.W / 2) * 0.72),
    third_brake: new THREE.Vector3(S.x(rr) - 0.04, S.roof(rr), 0),
    plate_light: new THREE.Vector3(tail.x - 0.02, tail.yb + (tail.ys - tail.yb) * 0.34 + 0.14, 0),
    mirrors: new THREE.Vector3(S.x(S.roofStart + 0.32), S.top(S.roofStart + 0.32) + 0.09, -(S.W / 2 + 0.06)),
    bonnet: new THREE.Vector3(S.x(bonD), S.top(bonD) + 0.04, 0),
    boot: spec.style === "sedan" ? new THREE.Vector3(S.x((S.roofEnd + S.L) / 2), S.top((S.roofEnd + S.L) / 2) + 0.03, 0) : new THREE.Vector3(tail.x - 0.02, (tail.ys + tail.yb) / 2 + 0.15, 0),
    engine: new THREE.Vector3(S.x(bonD), S.top(bonD) - 0.15, 0),
    battery:
      spec.battery === "cargo"
        ? new THREE.Vector3(-S.L / 2 + 0.55, S.bottom(S.L - 0.5) + 0.25, S.W / 2 - 0.45)
        : new THREE.Vector3(S.x(bonD + 0.25), S.top(bonD) - 0.1, -(S.W / 2) + 0.42),
    filler: new THREE.Vector3(S.wheels[1]!.x - (spec.style === "sedan" ? 0.25 : 0.12), S.top(S.d(S.wheels[1]!.x)) - 0.17, (spec.filler === "left" ? -1 : 1) * (S.W / 2 + 0.01)),
    obd: new THREE.Vector3(S.x(S.roofStart + 0.45), S.bottom(S.roofStart) + 0.32, -(S.W / 2) + 0.5),
    wheels: new THREE.Vector3(S.wheels[0]!.x, S.wheels[0]!.r, -(S.W / 2)),
  };

  const setLights = (on: boolean) => {
    m.head.emissiveIntensity = on ? 2.2 : 0.6;
    m.tail.emissiveIntensity = on ? 1.6 : 0.35;
    m.fog.emissiveIntensity = on ? 1.2 : 0;
    m.plateLamp.emissiveIntensity = on ? 1.5 : 0;
    for (const s of sprites) (s.material as THREE.SpriteMaterial).opacity = on ? 0.85 : 0;
  };

  let maxRoof = 0;
  for (const [, y] of spec.roof) maxRoof = Math.max(maxRoof, y);
  return {
    group,
    parts,
    anchors,
    setLights,
    size: new THREE.Vector3(S.L + (spec.spare ? 0.3 : 0), maxRoof, S.W),
    dispose() {
      group.traverse((o) => {
        if (o instanceof THREE.Mesh || o instanceof THREE.Sprite) {
          o.geometry.dispose();
          const mats = Array.isArray(o.material) ? o.material : [o.material];
          for (const mt of mats) mt.dispose();
        }
      });
    },
  };
}
