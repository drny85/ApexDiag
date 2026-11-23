/**
 * Live 3D car: drag to turn it, pinch or scroll to zoom. Highlights a part
 * (a lamp, the bonnet, the battery) with a pulsing glow and a pinned label,
 * and flies the camera round to face it. Pauses when off screen.
 */
import { useEffect, useRef } from "react";
import * as THREE from "three";
import type { PartId } from "./build";
import type { CarSpec } from "./specs";
import { environment, makeStage, placeCamera, VIEWS, type Stage, type View } from "./stage";
import { glowTexture, scanTexture } from "./textures";
import { cn } from "@/lib/utils";

export interface Car3DProps {
  spec: CarSpec;
  view?: View;
  autoRotate?: boolean;
  interactive?: boolean;
  highlight?: PartId | null;
  /** Text pinned to the highlighted part. */
  label?: string;
  lights?: boolean;
  scanning?: boolean;
  className?: string;
  onFail?: () => void;
}

const HL = new THREE.Color("#67e8f9");

/** Where the camera goes to look at a part. */
function viewFor(st: Stage, part: PartId): { az: number; el: number; zoom: number } {
  const a = st.car.anchors[part];
  switch (part) {
    case "bonnet":
    case "engine":
      return { az: 0.5, el: 0.55, zoom: 0.68 };
    case "obd":
      return { az: 1.35, el: 0.3, zoom: 0.66 };
    case "battery":
      return a.x < 0 ? { az: Math.PI - 0.9, el: 0.45, zoom: 0.68 } : { az: 0.6, el: 0.55, zoom: 0.68 };
    case "third_brake":
      return { az: Math.PI - 0.45, el: 0.35, zoom: 0.68 };
    default: {
      // face the anchor from outside the car, a little above
      const az = Math.atan2(-a.z, a.x);
      const el = a.y > 1 ? 0.3 : 0.16;
      return { az, el, zoom: 0.64 };
    }
  }
}

export default function Car3DView({ spec, view = "front34", autoRotate = true, interactive = true, highlight = null, label, lights = false, scanning = false, className, onFail }: Car3DProps) {
  const wrap = useRef<HTMLDivElement>(null);
  const tag = useRef<HTMLDivElement>(null);
  const state = useRef({ highlight, lights, scanning, autoRotate, view, label });
  state.current = { highlight, lights, scanning, autoRotate, view, label };
  const kick = useRef<() => void>(() => {});

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "low-power" });
    } catch {
      onFail?.();
      return;
    }
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    const canvas = renderer.domElement;
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    canvas.style.display = "block";
    canvas.style.touchAction = interactive ? "none" : "auto";
    el.prepend(canvas);

    const st = makeStage(renderer, spec, environment(renderer));
    const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

    // Highlight overlays: the part's own geometry, drawn again in additive cyan.
    const hlMat = new THREE.MeshBasicMaterial({ color: HL, transparent: true, opacity: 0, blending: THREE.AdditiveBlending, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2 });
    const overlays = new Map<PartId, THREE.Mesh[]>();
    for (const [id, meshes] of Object.entries(st.car.parts) as [PartId, THREE.Mesh[]][]) {
      overlays.set(
        id,
        meshes.map((src) => {
          const o = new THREE.Mesh(src.geometry, hlMat);
          o.visible = false;
          o.renderOrder = 5;
          src.updateWorldMatrix(true, false);
          o.matrixAutoUpdate = false;
          o.matrix.copy(src.matrixWorld);
          st.scene.add(o);
          return o;
        }),
      );
    }
    const beacon = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTexture(), color: HL, blending: THREE.AdditiveBlending, depthTest: false, depthWrite: false, transparent: true }));
    beacon.renderOrder = 10;
    beacon.visible = false;
    st.scene.add(beacon);

    // Scan band that sweeps front to back: a glow wash seen from the side
    // and a faint sheet seen from the ends.
    const bandH = st.car.size.y + 0.3;
    const band = new THREE.Mesh(
      new THREE.PlaneGeometry(1.4, bandH),
      new THREE.MeshBasicMaterial({ map: scanTexture(), transparent: true, opacity: 0.75, blending: THREE.AdditiveBlending, depthWrite: false, depthTest: false, side: THREE.DoubleSide }),
    );
    band.position.set(0.7, bandH / 2 - 0.05, 0);
    band.rotation.y = Math.PI;
    const sheet = new THREE.Mesh(
      new THREE.PlaneGeometry(st.car.size.z + 0.3, bandH),
      new THREE.MeshBasicMaterial({ color: HL, transparent: true, opacity: 0.12, blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.DoubleSide }),
    );
    sheet.rotation.y = Math.PI / 2;
    sheet.position.set(0, bandH / 2 - 0.05, 0);
    const scanGroup = new THREE.Group();
    scanGroup.add(band, sheet);
    scanGroup.visible = false;
    st.scene.add(scanGroup);

    const v0 = VIEWS[state.current.view];
    const BASE_ZOOM = 0.76;
    const cam = { az: v0.az, el: v0.el, zoom: BASE_ZOOM, target: st.target.clone() };
    const goal = { az: cam.az, el: cam.el, zoom: BASE_ZOOM, target: st.target.clone() };
    let lastHl: PartId | null = null;
    let lastInteract = 0;
    let visible = true;
    let raf = 0;
    let running = false;
    let w = 1;
    let h = 1;
    let t0 = performance.now();
    let disposed = false;

    const resize = () => {
      const r = el.getBoundingClientRect();
      w = Math.max(1, Math.round(r.width));
      h = Math.max(1, Math.round(r.height));
      renderer.setSize(w, h, false);
      frame(performance.now(), true);
    };

    const frame = (now: number, once = false) => {
      if (disposed) return;
      const dt = Math.min(0.05, (now - t0) / 1000);
      t0 = now;
      const s = state.current;
      // highlight target
      if (s.highlight !== lastHl) {
        for (const [, list] of overlays) for (const o of list) o.visible = false;
        if (s.highlight) {
          for (const o of overlays.get(s.highlight) ?? []) o.visible = true;
          const vf = viewFor(st, s.highlight);
          goal.az = vf.az;
          goal.el = vf.el;
          goal.zoom = vf.zoom;
          goal.target.copy(st.target).lerp(st.car.anchors[s.highlight], 0.45);
        } else {
          goal.target.copy(st.target);
          goal.zoom = BASE_ZOOM;
        }
        lastHl = s.highlight;
      }
      const idle = now - lastInteract > 2500;
      if (s.autoRotate && !s.highlight && idle && !reduced) goal.az += dt * 0.22;
      // ease towards the goal (shortest way round)
      let dAz = goal.az - cam.az;
      dAz = Math.atan2(Math.sin(dAz), Math.cos(dAz));
      const k = once ? 1 : 1 - Math.pow(0.002, dt);
      cam.az += dAz * k;
      cam.el += (goal.el - cam.el) * k;
      cam.zoom += (goal.zoom - cam.zoom) * k;
      cam.target.lerp(goal.target, k);
      placeCamera(st, cam.az, cam.el, w / h, cam.zoom, cam.target);

      st.car.setLights(s.lights);
      const pulse = 0.5 + 0.5 * Math.sin(now / 280);
      hlMat.opacity = s.highlight ? 0.25 + 0.45 * pulse : 0;
      if (s.highlight) {
        beacon.visible = true;
        beacon.position.copy(st.car.anchors[s.highlight]);
        beacon.scale.setScalar(0.35 + 0.2 * pulse);
        (beacon.material as THREE.SpriteMaterial).opacity = 0.5 + 0.5 * pulse;
      } else beacon.visible = false;
      scanGroup.visible = s.scanning;
      if (s.scanning) {
        const span = st.car.size.x + 1.2;
        const p = ((now / 1700) % 1) * span;
        scanGroup.position.x = span / 2 - p;
      }
      renderer.render(st.scene, st.camera);

      // pin the label to the highlighted part
      const tg = tag.current;
      if (tg) {
        if (s.highlight && s.label) {
          const p = st.car.anchors[s.highlight].clone().project(st.camera);
          tg.style.opacity = "1";
          tg.style.transform = `translate(${((p.x + 1) / 2) * w}px, ${((1 - p.y) / 2) * h}px) translate(-50%, calc(-100% - 18px))`;
        } else tg.style.opacity = "0";
      }

      const moving = s.autoRotate || s.highlight || s.scanning || Math.abs(dAz) > 0.002 || !idle;
      if (!once && visible && moving && !reduced) raf = requestAnimationFrame((t) => frame(t));
      else if (!once && visible && moving && reduced && (Math.abs(dAz) > 0.002 || !idle)) raf = requestAnimationFrame((t) => frame(t));
      else if (!once) running = false;
    };

    const start = () => {
      if (running || disposed) return;
      running = true;
      t0 = performance.now();
      raf = requestAnimationFrame((t) => frame(t));
    };
    kick.current = start;

    // Pointer controls: drag to orbit, pinch or wheel to zoom.
    const pts = new Map<number, { x: number; y: number }>();
    let pinch = 0;
    const down = (e: PointerEvent) => {
      if (!interactive) return;
      canvas.setPointerCapture(e.pointerId);
      pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
      lastInteract = performance.now();
      start();
    };
    const move = (e: PointerEvent) => {
      const prev = pts.get(e.pointerId);
      if (!prev) return;
      lastInteract = performance.now();
      if (pts.size === 1) {
        goal.az -= ((e.clientX - prev.x) / Math.max(200, w)) * Math.PI;
        goal.el = Math.min(1.1, Math.max(0.02, goal.el + ((e.clientY - prev.y) / Math.max(200, h)) * 1.2));
      }
      pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (pts.size === 2) {
        const [a, b] = [...pts.values()];
        const dist = Math.hypot(a!.x - b!.x, a!.y - b!.y);
        if (pinch) goal.zoom = Math.min(1.4, Math.max(0.55, goal.zoom * (pinch / dist)));
        pinch = dist;
      }
      start();
    };
    const up = (e: PointerEvent) => {
      pts.delete(e.pointerId);
      if (pts.size < 2) pinch = 0;
    };
    const wheel = (e: WheelEvent) => {
      if (!interactive) return;
      e.preventDefault();
      lastInteract = performance.now();
      goal.zoom = Math.min(1.4, Math.max(0.55, goal.zoom * (1 + e.deltaY * 0.0012)));
      start();
    };
    canvas.addEventListener("pointerdown", down);
    canvas.addEventListener("pointermove", move);
    canvas.addEventListener("pointerup", up);
    canvas.addEventListener("pointercancel", up);
    canvas.addEventListener("wheel", wheel, { passive: false });

    const ro = new ResizeObserver(resize);
    ro.observe(el);
    const io = new IntersectionObserver(([entry]) => {
      visible = !!entry?.isIntersecting;
      if (visible) start();
    });
    io.observe(el);
    const vis = () => {
      visible = document.visibilityState === "visible";
      if (visible) start();
    };
    document.addEventListener("visibilitychange", vis);
    canvas.addEventListener("webglcontextlost", () => {
      // our own cleanup forces a context loss: only a real loss counts
      if (!disposed) onFail?.();
    });
    resize();
    start();

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      document.removeEventListener("visibilitychange", vis);
      st.dispose();
      hlMat.dispose();
      (beacon.material as THREE.Material).dispose();
      for (const mesh of [band, sheet]) {
        mesh.geometry.dispose();
        (mesh.material as THREE.Material).dispose();
      }
      renderer.dispose();
      renderer.forceContextLoss();
      canvas.remove();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [spec, interactive]);

  // Restart the loop when a prop that animates changes.
  useEffect(() => {
    kick.current();
  }, [highlight, lights, scanning, autoRotate, label]);

  return (
    <div ref={wrap} className={cn("relative select-none", interactive && "cursor-grab active:cursor-grabbing", className)}>
      <div
        ref={tag}
        className="pointer-events-none absolute left-0 top-0 whitespace-nowrap rounded-lg bg-cyan-300 px-2.5 py-1 text-xs font-semibold text-slate-950 opacity-0 shadow-lg shadow-cyan-500/30 transition-opacity"
      >
        {label}
        <span className="absolute left-1/2 top-full h-3 w-px -translate-x-1/2 bg-cyan-300" />
      </div>
    </div>
  );
}
