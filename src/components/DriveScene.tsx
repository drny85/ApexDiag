/**
 * Drive Mode's centrepiece: the same 3D model of the car the rest of the
 * app uses (see car3d/), driving on a road that scrolls at the car's real
 * speed. The body pitches when the car speeds up or brakes, the wheels turn,
 * the brake lights brighten, and the engine, battery and fuel filler carry
 * a glowing marker that turns amber or red with its warning.
 *
 * Falls back to the flat side drawing when WebGL is not available.
 */
import { useEffect, useMemo, useRef, useState, type CSSProperties, type RefObject } from "react";
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { buildCar } from "@/components/car3d/build";
import { specFor } from "@/components/car3d/specs";
import { CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import type { Pack } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export type Level = "ok" | "warn" | "bad" | "off";

/** What the scene shows, read every frame (a ref, so React never re-renders it). */
export interface CarLive {
  /** km/h */
  speed: number;
  /** Along the car, in g: positive speeding up, negative braking. */
  accel: number;
  /** Across the car, in g: positive turning right. */
  lateral: number;
  engine: Level;
  tank: Level;
  battery: Level;
  /** Short readouts pinned to each part, e.g. "92 °C". */
  tags: { engine: string; tank: string; battery: string };
}

const LEVEL: Record<Level, string> = { ok: "#22d3ee", warn: "#fbbf24", bad: "#fb7185", off: "#475569" };

/** Scene units per metre: the road and camera were laid out for a ~4.8 m car 280 units long. */
const K = 280 / 4.8;

type PackLike = Pick<Pack, "id" | "body" | "make"> | null | undefined;

function radialTexture(inner: string, outer: string) {
  const c = document.createElement("canvas");
  c.width = c.height = 128;
  const g = c.getContext("2d")!;
  const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64);
  grad.addColorStop(0, inner);
  grad.addColorStop(1, outer);
  g.fillStyle = grad;
  g.fillRect(0, 0, 128, 128);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

/** Asphalt with dashed lane lines, tiled along the road. */
function roadTexture() {
  const c = document.createElement("canvas");
  c.width = 512;
  c.height = 256;
  const g = c.getContext("2d")!;
  g.fillStyle = "#0b0f16";
  g.fillRect(0, 0, 512, 256);
  for (let i = 0; i < 900; i++) {
    g.fillStyle = `rgba(255,255,255,${Math.random() * 0.025})`;
    g.fillRect(Math.random() * 512, Math.random() * 256, 2, 2);
  }
  g.fillStyle = "rgba(226,232,240,0.55)";
  for (const y of [18, 228]) g.fillRect(0, y, 300, 10);
  g.fillStyle = "rgba(226,232,240,0.18)";
  g.fillRect(0, 2, 512, 3);
  g.fillRect(0, 251, 512, 3);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  tex.wrapS = THREE.RepeatWrapping;
  tex.anisotropy = 4;
  return tex;
}

function gridTexture(color: string) {
  const c = document.createElement("canvas");
  c.width = c.height = 256;
  const g = c.getContext("2d")!;
  g.strokeStyle = color;
  g.lineWidth = 2;
  g.strokeRect(0, 0, 256, 256);
  const tex = new THREE.CanvasTexture(c);
  tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
  tex.anisotropy = 4;
  return tex;
}

function webglAvailable() {
  try {
    const c = document.createElement("canvas");
    return !!(c.getContext("webgl2") || c.getContext("webgl"));
  } catch {
    return false;
  }
}

const TAGS = ["engine", "tank", "battery"] as const;

export default function DriveScene({
  pack,
  live,
  focusY = 0.5,
  className,
  style,
}: {
  pack: PackLike;
  live: RefObject<CarLive>;
  /** Where the car sits on screen, from the top (0) to the bottom (1). */
  focusY?: number;
  className?: string;
  style?: CSSProperties;
}) {
  const host = useRef<HTMLDivElement>(null);
  const tagEls = useRef<Record<string, HTMLDivElement | null>>({});
  const [failed, setFailed] = useState(() => !webglAvailable());
  const focusRef = useRef(focusY);
  const resizeRef = useRef<() => void>(() => {});
  const spec = useMemo(() => specFor(pack, pack?.body ?? "sedan"), [pack]);
  const accent = pack ? MAKE_ACCENT[pack.make] : "#67e8f9";

  useEffect(() => {
    focusRef.current = focusY;
    resizeRef.current();
  }, [focusY]);

  useEffect(() => {
    const el = host.current;
    if (!el || failed) return;
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    } catch {
      setFailed(true);
      return;
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.domElement.style.display = "block";
    const lost = (e: Event) => {
      e.preventDefault();
      setFailed(true);
    };
    renderer.domElement.addEventListener("webglcontextlost", lost);
    el.prepend(renderer.domElement);

    const scene = new THREE.Scene();
    scene.fog = new THREE.Fog(0x07090e, 520, 1500);
    const pmrem = new THREE.PMREMGenerator(renderer);
    const env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = env;
    scene.environmentIntensity = 0.9;
    const camera = new THREE.PerspectiveCamera(30, 1, 10, 4000);

    // ---- lighting
    scene.add(new THREE.HemisphereLight(0xcfe7ff, 0x05070a, 0.45));
    const key = new THREE.DirectionalLight(0xffffff, 1.5);
    key.position.set(240, 420, 180);
    scene.add(key);
    const rim = new THREE.DirectionalLight(new THREE.Color(accent), 0.9);
    rim.position.set(-300, 180, -240);
    scene.add(rim);

    // ---- the car (shared model, built in metres, scaled into the scene)
    const built = buildCar(spec);
    built.setLights(true);
    const chassis = new THREE.Group();
    built.group.scale.setScalar(K);
    chassis.add(built.group);
    scene.add(chassis);

    // Wheels: each side's wheel group, spun about its own axle.
    const wheelGroups = new Set<THREE.Object3D>();
    for (const m of built.parts.wheels ?? []) {
      let o: THREE.Object3D | null = m;
      while (o && o.parent && o.parent !== built.group) o = o.parent;
      if (o && o.parent === built.group) wheelGroups.add(o);
    }
    const wheels = [...wheelGroups].map((g) => ({ g, sign: Math.abs(g.rotation.y) > 1 ? 1 : -1 }));
    const tyreR = spec.tyre.r * K;

    // Brake lights.
    const tails = new Set<THREE.MeshStandardMaterial>();
    for (const m of built.parts.taillights ?? []) {
      const mats = Array.isArray(m.material) ? m.material : [m.material];
      for (const mt of mats) if (mt instanceof THREE.MeshStandardMaterial) tails.add(mt);
    }
    const tailBase = [...tails].map((mt) => mt.emissiveIntensity);

    // Warning markers at the engine, battery and fuel filler.
    const halo = radialTexture("rgba(255,255,255,1)", "rgba(255,255,255,0)");
    const anchorOf = { engine: built.anchors.engine, battery: built.anchors.battery, tank: built.anchors.filler };
    const markers = Object.fromEntries(
      TAGS.map((id) => {
        const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: halo, color: LEVEL.ok, transparent: true, depthTest: false, depthWrite: false, blending: THREE.AdditiveBlending }));
        sp.position.copy(anchorOf[id]);
        sp.scale.setScalar(0.35);
        sp.renderOrder = 10;
        built.group.add(sp);
        return [id, sp];
      }),
    ) as Record<(typeof TAGS)[number], THREE.Sprite>;
    const glow = new THREE.PointLight(0xfbbf24, 0, 260, 1);
    glow.position.copy(built.anchors.engine).multiplyScalar(K).add(new THREE.Vector3(0, 40, 0));
    chassis.add(glow);

    // ---- the road
    const W = spec.width * K;
    const roadTex = roadTexture();
    roadTex.repeat.set(4000 / 220, 1);
    const road = new THREE.Mesh(new THREE.PlaneGeometry(4000, W * 2.6), new THREE.MeshStandardMaterial({ map: roadTex, roughness: 0.92, metalness: 0 }));
    road.rotation.x = -Math.PI / 2;
    road.position.y = -0.2;
    scene.add(road);
    const gridTex = gridTexture("rgba(103,232,249,0.22)");
    gridTex.repeat.set(4000 / 60, 4000 / 60);
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(4000, 4000), new THREE.MeshBasicMaterial({ map: gridTex, transparent: true, opacity: 0.55, depthWrite: false }));
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -0.4;
    scene.add(floor);
    const beam = new THREE.Mesh(
      new THREE.PlaneGeometry(260, W * 1.5),
      new THREE.MeshBasicMaterial({ map: radialTexture("rgba(191,230,255,0.28)", "rgba(191,230,255,0)"), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }),
    );
    beam.rotation.x = -Math.PI / 2;
    beam.position.set((spec.length / 2) * K + 120, 0.2, 0);
    scene.add(beam);

    // ---- size and frame
    const resize = () => {
      const w = el.clientWidth;
      const h = el.clientHeight;
      if (!w || !h) return;
      renderer.setSize(w, h, false);
      renderer.domElement.style.width = `${w}px`;
      renderer.domElement.style.height = `${h}px`;
      camera.aspect = w / h;
      // Pull back on narrow frames so the whole car fits.
      camera.userData.dist = 760 * Math.max(1, 1.9 / camera.aspect);
      camera.setViewOffset(w, h, 0, (0.5 - focusRef.current) * h, w, h);
      camera.updateProjectionMatrix();
    };
    resizeRef.current = resize;
    const ro = new ResizeObserver(resize);
    ro.observe(el);
    resize();

    // ---- animate
    const s = { speed: 0, accel: 0, lateral: 0, dist: 0 };
    const target = new THREE.Vector3(10, built.size.y * K * 0.45, 0);
    const v = new THREE.Vector3();
    let raf = 0;
    let last = performance.now();
    const pulse = (level: Level, t: number) => (level === "warn" ? 0.55 + 0.45 * Math.sin(t * 4) : level === "bad" ? 0.55 + 0.45 * Math.sin(t * 7) : 0);

    const frame = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const t = now / 1000;
      const L = live.current;
      const k = 1 - Math.exp(-dt * 6);
      s.speed += ((L?.speed ?? 0) - s.speed) * k;
      s.accel += (Math.max(-1.2, Math.min(1.2, L?.accel ?? 0)) - s.accel) * k;
      s.lateral += (Math.max(-1.2, Math.min(1.2, L?.lateral ?? 0)) - s.lateral) * k;

      const step = (s.speed / 3.6) * K * dt;
      s.dist += step;
      roadTex.offset.x = (s.dist / 220) % 1;
      gridTex.offset.x = (s.dist / 60) % 1;
      for (const w of wheels) w.g.rotation.z += (w.sign * step) / tyreR;

      // Nose lifts when speeding up, dips when braking; leans in turns.
      chassis.rotation.z += (s.accel * 0.045 - chassis.rotation.z) * k;
      chassis.rotation.x += (-s.lateral * 0.05 - chassis.rotation.x) * k;
      chassis.position.y = Math.min(1, s.speed / 80) * 0.5 * Math.sin(t * 17) * Math.sin(t * 2.3);

      const braking = s.accel < -0.08;
      [...tails].forEach((mt, i) => (mt.emissiveIntensity = braking ? 4 : (tailBase[i] ?? 1.6)));

      if (L) {
        for (const id of TAGS) {
          const level = L[id];
          const mat = markers[id].material;
          mat.color.set(LEVEL[level]);
          const p = pulse(level, t);
          mat.opacity = level === "off" ? 0 : level === "ok" ? 0.5 : 0.6 + 0.4 * p;
          markers[id].scale.setScalar(level === "ok" ? 0.3 : 0.55 + 0.35 * p);
        }
        glow.color.set(LEVEL[L.engine]);
        glow.intensity = L.engine === "warn" || L.engine === "bad" ? 400 * (0.6 + 0.4 * Math.sin(t * 5)) : 0;
      }

      // Camera: three-quarter front view with a slow drift, pulled back a
      // touch under hard acceleration.
      const dist = (camera.userData.dist as number) * (1 + s.accel * 0.06);
      const a = 0.72 + 0.06 * Math.sin(t * 0.13);
      camera.position.set(Math.sin(a) * dist * 0.92, 165 + 12 * Math.sin(t * 0.09), Math.cos(a) * dist);
      camera.lookAt(target);
      renderer.render(scene, camera);

      // Pin the HTML tags to their parts.
      const w = el.clientWidth;
      const h = el.clientHeight;
      for (const id of TAGS) {
        const tag = tagEls.current[id];
        if (!tag) continue;
        v.copy(anchorOf[id]).applyMatrix4(built.group.matrixWorld).project(camera);
        // The battery's tag hangs below its marker so it never sits on the fuel tag.
        const below = id === "battery";
        tag.style.transform = `translate(${((v.x + 1) / 2) * w}px, ${((1 - v.y) / 2) * h + (below ? 14 : -14)}px) translate(-50%, ${below ? "0" : "-100%"})`;
        const text = L?.tags[id] ?? "";
        const label = tag.lastElementChild as HTMLElement | null;
        if (label && label.textContent !== text) label.textContent = text;
        tag.dataset.level = L?.[id] ?? "off";
        tag.style.opacity = text ? "1" : "0";
      }
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      renderer.domElement.removeEventListener("webglcontextlost", lost);
      built.dispose();
      for (const o of [road, floor, beam]) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
      }
      for (const tex of [roadTex, gridTex, halo, env]) tex.dispose();
      pmrem.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
    };
  }, [spec, accent, live, failed]);

  if (failed) {
    return (
      <div className={className} style={style}>
        <div className="flex h-full items-center justify-center px-[30%]">
          <CarSide body={pack?.body ?? "sedan"} accent={accent} className="w-full max-w-[520px]" />
        </div>
      </div>
    );
  }

  return (
    <div ref={host} className={cn("relative overflow-hidden", className)} style={style}>
      {TAGS.map((id) => (
        <div
          key={id}
          ref={(n) => {
            tagEls.current[id] = n;
          }}
          className="car-tag pointer-events-none absolute left-0 top-0 flex items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-1 text-xs font-medium tabular-nums opacity-0 backdrop-blur-md transition-opacity"
        >
          <span className="size-1.5 rounded-full bg-current" />
          <span />
        </div>
      ))}
    </div>
  );
}
