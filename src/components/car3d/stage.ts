/**
 * Scene, lighting and camera for the 3D cars, plus a single shared
 * off-screen renderer that turns a car into a still image. Lists of cars
 * use the stills, so a page never holds more than one or two live WebGL
 * contexts (iPad Safari caps how many can exist at once).
 */
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { buildCar, type BuiltCar } from "./build";
import type { CarSpec } from "./specs";

export type View = "front34" | "rear34" | "side" | "front" | "rear" | "top" | "rearLeft34";

/** Azimuth from the car's front (radians, positive towards the driver's side) and elevation. */
export const VIEWS: Record<View, { az: number; el: number }> = {
  front34: { az: 0.72, el: 0.2 },
  rear34: { az: Math.PI - 0.75, el: 0.22 },
  rearLeft34: { az: Math.PI - 0.75, el: 0.22 },
  side: { az: Math.PI / 2, el: 0.08 },
  front: { az: 0.0001, el: 0.12 },
  rear: { az: Math.PI, el: 0.14 },
  top: { az: 0.6, el: 1.05 },
};

export function webglAvailable(): boolean {
  try {
    const c = document.createElement("canvas");
    return !!(c.getContext("webgl2") || c.getContext("webgl"));
  } catch {
    return false;
  }
}

export interface Stage {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  car: BuiltCar;
  target: THREE.Vector3;
  radius: number;
  dispose(): void;
}

let envCache: { renderer: THREE.WebGLRenderer; tex: THREE.Texture } | null = null;

export function environment(renderer: THREE.WebGLRenderer): THREE.Texture {
  if (envCache?.renderer === renderer) return envCache.tex;
  const pm = new THREE.PMREMGenerator(renderer);
  const tex = pm.fromScene(new RoomEnvironment(), 0.04).texture;
  pm.dispose();
  envCache = { renderer, tex };
  return tex;
}

export function makeStage(renderer: THREE.WebGLRenderer, spec: CarSpec, env: THREE.Texture): Stage {
  const scene = new THREE.Scene();
  scene.environment = env;
  scene.environmentIntensity = 0.9;
  const car = buildCar(spec);
  scene.add(car.group);
  // A soft key light from above-front for highlights on the paint.
  const key = new THREE.DirectionalLight("#ffffff", 1.4);
  key.position.set(4, 7, 3);
  scene.add(key);
  const rim = new THREE.DirectionalLight("#9fdcff", 0.8);
  rim.position.set(-5, 3, -4);
  scene.add(rim);
  scene.add(new THREE.HemisphereLight("#cfe7ff", "#0b0e12", 0.35));
  const camera = new THREE.PerspectiveCamera(28, 1, 0.1, 100);
  const target = new THREE.Vector3(0, car.size.y * 0.42, 0);
  const radius = Math.max(car.size.x, car.size.z) * 1.55 + 1.2;
  void renderer;
  return {
    scene,
    camera,
    car,
    target,
    radius,
    dispose() {
      car.dispose();
    },
  };
}

/** Puts the camera on a sphere round the target. Fits the car to the frame aspect. */
export function placeCamera(st: Stage, az: number, el: number, aspect: number, zoom = 1, target?: THREE.Vector3) {
  const t = target ?? st.target;
  // A wide frame needs less distance than a tall one to show the same car.
  const fit = aspect < 1.2 ? 1.2 / aspect : 1;
  const r = st.radius * fit * zoom;
  st.camera.aspect = aspect;
  st.camera.position.set(t.x + r * Math.cos(el) * Math.cos(az), t.y + r * Math.sin(el), t.z - r * Math.cos(el) * Math.sin(az));
  st.camera.lookAt(t);
  st.camera.updateProjectionMatrix();
}

/* ---------- stills ---------- */

let shared: THREE.WebGLRenderer | null = null;
const stills = new Map<string, string>();
const stageCache = new Map<string, Stage>();

function sharedRenderer(): THREE.WebGLRenderer | null {
  if (shared) return shared;
  try {
    const canvas = document.createElement("canvas");
    shared = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
    shared.toneMapping = THREE.ACESFilmicToneMapping;
    shared.toneMappingExposure = 1.05;
    shared.outputColorSpace = THREE.SRGBColorSpace;
    shared.setClearColor(0x000000, 0);
    canvas.addEventListener("webglcontextlost", () => {
      shared = null;
      stageCache.clear();
    });
    return shared;
  } catch {
    return null;
  }
}

/**
 * Renders a still of the car (cached by spec, view, size and lights).
 * Returns null when WebGL is unavailable, so callers can fall back to SVG.
 */
export function renderStill(spec: CarSpec, view: View, w: number, h: number, opts: { lights?: boolean; mirror?: boolean } = {}): string | null {
  const dpr = Math.min(2, typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
  const key = `${spec.id}|${spec.paint}|${view}|${w}x${h}@${dpr}|${opts.lights ? 1 : 0}`;
  const hit = stills.get(key);
  if (hit) return hit;
  const r = sharedRenderer();
  if (!r) return null;
  let st = stageCache.get(spec.id + spec.paint);
  if (!st) {
    st = makeStage(r, spec, environment(r));
    stageCache.set(spec.id + spec.paint, st);
  }
  r.setPixelRatio(dpr);
  r.setSize(w, h, false);
  st.car.setLights(!!opts.lights);
  const v = VIEWS[view];
  const az = view === "rearLeft34" ? Math.PI + 0.75 : v.az;
  placeCamera(st, az, v.el, w / h, view === "side" ? 0.74 : 0.74);
  r.render(st.scene, st.camera);
  const url = r.domElement.toDataURL("image/png");
  stills.set(key, url);
  return url;
}
