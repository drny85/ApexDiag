/**
 * Drive Mode's centrepiece: a 3D "x-ray" car built from the same
 * brand-neutral side profiles as the 2D drawings. It rolls along a road that
 * scrolls at the car's real speed, pitches when the car speeds up or brakes,
 * and the parts behind each warning (engine, fuel tank, battery) glow.
 *
 * Falls back to the flat side drawing when WebGL is not available.
 */
import { useEffect, useRef, useState, type RefObject } from "react";
import * as THREE from "three";
import { SVGLoader } from "three/examples/jsm/loaders/SVGLoader.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { CarSide, SHAPES, type Body } from "@/components/illustrations/Vehicle";
import { cn } from "@/lib/utils";

export type Level = "ok" | "warn" | "bad" | "off";

/** What the car shows, read every frame (a ref, so React never re-renders the scene). */
export interface CarLive {
  /** km/h */
  speed: number;
  /** Along the car, in g: positive speeding up, negative braking. */
  accel: number;
  /** Across the car, in g: positive turning right. */
  lateral: number;
  /** 0 to 1, if the car reports it. */
  fuel?: number;
  engine: Level;
  tank: Level;
  battery: Level;
  /** Short readouts pinned to each part, e.g. "92 °C". */
  tags: { engine: string; tank: string; battery: string };
}

const LEVEL: Record<Level, string> = { ok: "#22d3ee", warn: "#fbbf24", bad: "#fb7185", off: "#475569" };

/** Body width in drawing units, per body style (the drawings are 296 long). */
const WIDTH: Record<Body, number> = { suv: 118, sedan: 108, offroad: 116 };

/** Windscreen and rear window, as two points on the outline (bottom, top). */
type Pt = [number, number];
const SCREENS: Record<Body, [Pt, Pt][]> = {
  suv: [
    [
      [214, 43.5],
      [196, 27],
    ],
  ],
  sedan: [
    [
      [228, 52.3],
      [209, 35],
    ],
    [
      [76, 52],
      [100, 35],
    ],
  ],
  offroad: [
    [
      [187, 40],
      [178.5, 22.5],
    ],
  ],
};

const loader = new SVGLoader();
const parse = (d: string) => loader.parse(`<svg xmlns="http://www.w3.org/2000/svg"><path d="${d}"/></svg>`).paths;

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
  for (const y of [18, 238 - 10]) g.fillRect(0, y, 300, 10);
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

export default function Car3D({
  body,
  accent,
  live,
  focusY = 0.5,
  className,
  style,
}: {
  body: Body;
  accent: string;
  live: RefObject<CarLive>;
  /** Where the car sits on screen, from the top (0) to the bottom (1). */
  focusY?: number;
  className?: string;
  style?: React.CSSProperties;
}) {
  const host = useRef<HTMLDivElement>(null);
  const tagEls = useRef<Record<string, HTMLDivElement | null>>({});
  const [failed, setFailed] = useState(() => !webglAvailable());
  const focusRef = useRef(focusY);
  const resizeRef = useRef<() => void>(() => {});
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
    el.prepend(renderer.domElement);

    const scene = new THREE.Scene();
    const BG = 0x07090e;
    scene.fog = new THREE.Fog(BG, 520, 1500);
    const pmrem = new THREE.PMREMGenerator(renderer);
    const env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = env;

    const camera = new THREE.PerspectiveCamera(30, 1, 10, 4000);

    // ---- lighting
    scene.add(new THREE.HemisphereLight(0xbfd7ff, 0x05070a, 0.7));
    const key = new THREE.DirectionalLight(0xffffff, 1.6);
    key.position.set(200, 400, 260);
    scene.add(key);
    const rim = new THREE.PointLight(new THREE.Color(accent), 900, 1100, 1);
    rim.position.set(-260, 140, -220);
    scene.add(rim);

    // ---- the car
    const shape = SHAPES[body];
    const W = WIDTH[body];
    const ground = shape.cy + shape.r;
    const BT = 9; // bevel: rounds the roof and shoulders
    const car = new THREE.Group();
    const chassis = new THREE.Group();
    car.add(chassis);
    scene.add(car);

    const toCar = (geo: THREE.BufferGeometry) => {
      // Drawing y grows downwards: flip it (and z, to keep faces outward).
      geo.scale(1, -1, -1);
      geo.translate(-148, ground, 0);
      return geo;
    };

    const outline = parse(shape.outline).flatMap((p) => SVGLoader.createShapes(p));
    const bodyGeo = toCar(
      new THREE.ExtrudeGeometry(outline, {
        depth: W - BT * 2,
        bevelEnabled: true,
        bevelThickness: BT,
        bevelSize: 5,
        bevelSegments: 6,
        curveSegments: 28,
      }),
    );
    bodyGeo.translate(0, 0, W / 2 - BT);
    const paint = new THREE.MeshPhysicalMaterial({
      color: 0x0e1626,
      metalness: 0.75,
      roughness: 0.3,
      clearcoat: 1,
      clearcoatRoughness: 0.12,
      envMapIntensity: 1.2,
      transparent: true,
      opacity: 0.7,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
    const shell = new THREE.Mesh(bodyGeo, paint);
    shell.renderOrder = 2;
    chassis.add(shell);

    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(bodyGeo, 28),
      new THREE.LineBasicMaterial({ color: accent, transparent: true, opacity: 0.55 }),
    );
    edges.renderOrder = 3;
    chassis.add(edges);

    // Side glass and the belt / door lines, on both sides.
    const glass = new THREE.MeshPhysicalMaterial({
      color: 0x050b14,
      metalness: 0.4,
      roughness: 0.08,
      envMapIntensity: 0.7,
      transparent: true,
      opacity: 0.8,
      side: THREE.DoubleSide,
    });
    const glassShapes = parse(shape.glass).flatMap((p) => SVGLoader.createShapes(p));
    const lineMat = new THREE.LineBasicMaterial({ color: accent, transparent: true, opacity: 0.4 });
    for (const side of [1, -1]) {
      const g = new THREE.ShapeGeometry(glassShapes, 16);
      g.scale(1, -1, 1);
      g.translate(-148, ground, side * (W / 2 + 0.6));
      const m = new THREE.Mesh(g, glass);
      m.renderOrder = 4;
      chassis.add(m);
      for (const d of [shape.belt, shape.doors]) {
        for (const path of parse(d)) {
          for (const sub of path.subPaths) {
            const pts = sub.getPoints().map((p) => new THREE.Vector3(p.x - 148, ground - p.y, side * (W / 2 + 0.8)));
            const line = new THREE.Line(new THREE.BufferGeometry().setFromPoints(pts), lineMat);
            line.renderOrder = 4;
            chassis.add(line);
          }
        }
      }
    }
    // Windscreen (and rear window on a saloon).
    for (const [[x1, y1], [x2, y2]] of SCREENS[body]) {
      const a = new THREE.Vector2(x1 - 148, ground - y1);
      const b = new THREE.Vector2(x2 - 148, ground - y2);
      const dir = b.clone().sub(a).normalize();
      let n = new THREE.Vector2(-dir.y, dir.x);
      if (n.y < 0) n = n.negate();
      const off = n.multiplyScalar(5.6);
      const hz = W * 0.42;
      const p0 = a.clone().add(off);
      const p1 = b.clone().add(off);
      const g = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(p0.x, p0.y, -hz),
        new THREE.Vector3(p0.x, p0.y, hz),
        new THREE.Vector3(p1.x, p1.y, hz),
        new THREE.Vector3(p1.x, p1.y, -hz),
      ]);
      g.setIndex([0, 1, 2, 0, 2, 3]);
      g.computeVertexNormals();
      const m = new THREE.Mesh(g, glass);
      m.renderOrder = 4;
      chassis.add(m);
    }

    // Lights: glowing blocks plus soft halos.
    const halo = radialTexture("rgba(255,255,255,1)", "rgba(255,255,255,0)");
    const headPt = parse(shape.head)[0]!.subPaths[0]!.getPoints()[0]!;
    const tailPt = parse(shape.tail)[0]!.subPaths[0]!.getPoints()[0]!;
    const headMat = new THREE.MeshBasicMaterial({ color: new THREE.Color(1.6, 1.8, 2.0), toneMapped: false });
    const tailMat = new THREE.MeshBasicMaterial({ color: new THREE.Color(1.2, 0.08, 0.1), toneMapped: false });
    const haloMat = (c: number) => new THREE.SpriteMaterial({ map: halo, color: c, transparent: true, blending: THREE.AdditiveBlending, depthWrite: false });
    const tailHalo = haloMat(0xff2a3a);
    for (const side of [1, -1]) {
      const h = new THREE.Mesh(new THREE.BoxGeometry(8, 5, 20), headMat);
      h.position.set(headPt.x - 148 + 6, ground - headPt.y, side * W * 0.31);
      chassis.add(h);
      const hs = new THREE.Sprite(haloMat(0xbfe6ff));
      hs.scale.set(46, 30, 1);
      hs.position.copy(h.position).add(new THREE.Vector3(6, 0, 0));
      chassis.add(hs);
      const tl = new THREE.Mesh(new THREE.BoxGeometry(4, 9, 22), tailMat);
      tl.position.set(tailPt.x - 148 - 1, ground - tailPt.y - 5, side * W * 0.32);
      chassis.add(tl);
      const ts = new THREE.Sprite(tailHalo);
      ts.scale.set(40, 30, 1);
      ts.position.copy(tl.position).add(new THREE.Vector3(-5, 0, 0));
      chassis.add(ts);
    }

    // Wheels (outside the chassis so they don't pitch with the body).
    const tireMat = new THREE.MeshStandardMaterial({ color: 0x0a0c10, roughness: 0.85 });
    const rimMat = new THREE.MeshStandardMaterial({ color: 0x9aa7b8, metalness: 1, roughness: 0.22, envMapIntensity: 1.4 });
    const wheels: THREE.Group[] = [];
    const TW = 22;
    for (const wx of shape.wheels) {
      for (const side of [1, -1]) {
        const w = new THREE.Group();
        const tire = new THREE.Mesh(new THREE.CylinderGeometry(shape.r, shape.r, TW, 40), tireMat);
        tire.rotation.x = Math.PI / 2;
        w.add(tire);
        const rimDisc = new THREE.Mesh(new THREE.CylinderGeometry(shape.r * 0.64, shape.r * 0.64, TW + 0.8, 32), rimMat);
        rimDisc.rotation.x = Math.PI / 2;
        w.add(rimDisc);
        for (let i = 0; i < 5; i++) {
          const spoke = new THREE.Mesh(new THREE.BoxGeometry(shape.r * 1.16, 2.6, 2), new THREE.MeshBasicMaterial({ color: 0x1b2230 }));
          spoke.rotation.z = (i / 5) * Math.PI;
          spoke.position.z = side * (TW / 2 + 0.9);
          w.add(spoke);
        }
        const hub = new THREE.Mesh(new THREE.CircleGeometry(shape.r * 0.14, 20), new THREE.MeshBasicMaterial({ color: accent }));
        hub.position.z = side * (TW / 2 + 1.1);
        if (side < 0) hub.rotation.y = Math.PI;
        w.add(hub);
        w.position.set(wx - 148, shape.r, side * (W / 2 - TW / 2 + 4));
        car.add(w);
        wheels.push(w);
      }
    }

    // X-ray parts: engine, fuel tank (with its level), battery.
    const part = (size: [number, number, number], pos: [number, number, number]) => {
      const mat = new THREE.MeshStandardMaterial({ color: LEVEL.ok, emissive: LEVEL.ok, emissiveIntensity: 0.5, roughness: 0.4, metalness: 0.3, transparent: true, opacity: 0.9 });
      const m = new THREE.Mesh(new THREE.BoxGeometry(...size), mat);
      m.position.set(...pos);
      m.renderOrder = 1;
      const wire = new THREE.LineSegments(new THREE.EdgesGeometry(m.geometry), new THREE.LineBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.35 }));
      m.add(wire);
      chassis.add(m);
      return { mesh: m, mat };
    };
    const front = shape.wheels[1] - 148;
    const rear = shape.wheels[0] - 148;
    const bay = ground - shape.cy + shape.r * 1.25;
    const engine = part([46, 28, W * 0.46], [front - 14, bay, 0]);
    const battery = part([12, 12, 18], [front + 16, bay + 6, W * 0.26]);
    const tankY = shape.r * 1.45;
    const tank = part([40, 13, W * 0.58], [rear + 34, tankY, 0]);
    tank.mat.opacity = 0.18;
    tank.mat.emissiveIntensity = 0.15;
    const fillMat = new THREE.MeshStandardMaterial({ color: LEVEL.ok, emissive: LEVEL.ok, emissiveIntensity: 0.7, transparent: true, opacity: 0.9 });
    const fill = new THREE.Mesh(new THREE.BoxGeometry(38, 12, W * 0.56), fillMat);
    fill.position.set(rear + 34, tankY, 0);
    fill.renderOrder = 1;
    chassis.add(fill);
    const glow = new THREE.PointLight(0xfbbf24, 0, 260, 1);
    glow.position.set(front - 14, bay + 40, 0);
    chassis.add(glow);

    // Anchors for the HTML tags.
    const anchors: Record<string, THREE.Vector3> = {
      engine: new THREE.Vector3(front - 30, bay + 46, 0),
      tank: new THREE.Vector3(rear + 20, tankY + 40, -W * 0.2),
      battery: new THREE.Vector3(front + 34, bay + 10, W * 0.3),
    };

    // ---- the road
    const roadTex = roadTexture();
    roadTex.repeat.set(4000 / 220, 1);
    const road = new THREE.Mesh(new THREE.PlaneGeometry(4000, W * 2.6), new THREE.MeshStandardMaterial({ map: roadTex, roughness: 0.92, metalness: 0 }));
    road.rotation.x = -Math.PI / 2;
    road.position.y = 0.1;
    scene.add(road);
    const gridTex = gridTexture("rgba(103,232,249,0.22)");
    gridTex.repeat.set(4000 / 60, 4000 / 60);
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(4000, 4000), new THREE.MeshBasicMaterial({ map: gridTex, transparent: true, opacity: 0.55, depthWrite: false }));
    floor.rotation.x = -Math.PI / 2;
    scene.add(floor);
    const shadow = new THREE.Mesh(
      new THREE.PlaneGeometry(330, W * 1.9),
      new THREE.MeshBasicMaterial({ map: radialTexture("rgba(0,0,0,0.85)", "rgba(0,0,0,0)"), transparent: true, depthWrite: false }),
    );
    shadow.rotation.x = -Math.PI / 2;
    shadow.position.y = 0.3;
    scene.add(shadow);
    const beam = new THREE.Mesh(
      new THREE.PlaneGeometry(260, W * 1.5),
      new THREE.MeshBasicMaterial({ map: radialTexture("rgba(191,230,255,0.28)", "rgba(191,230,255,0)"), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }),
    );
    beam.rotation.x = -Math.PI / 2;
    beam.position.set(250, 0.4, 0);
    scene.add(beam);

    // ---- size and project
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
    const s = { speed: 0, accel: 0, lateral: 0, fuel: 1, dist: 0 };
    const target = new THREE.Vector3(10, 44, 0);
    const v = new THREE.Vector3();
    let raf = 0;
    let last = performance.now();
    const colour = new THREE.Color();
    const setLevel = (mat: THREE.MeshStandardMaterial, level: Level, t: number, base: number) => {
      colour.set(LEVEL[level]);
      mat.color.copy(colour);
      mat.emissive.copy(colour);
      const pulse = level === "warn" || level === "bad" ? 0.55 + 0.45 * Math.sin(t * (level === "bad" ? 7 : 4)) : 0;
      mat.emissiveIntensity = base + pulse * 1.2;
    };

    const frame = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const t = now / 1000;
      const L = live.current;
      const k = 1 - Math.exp(-dt * 6);
      s.speed += ((L?.speed ?? 0) - s.speed) * k;
      s.accel += (Math.max(-1.2, Math.min(1.2, L?.accel ?? 0)) - s.accel) * k;
      s.lateral += (Math.max(-1.2, Math.min(1.2, L?.lateral ?? 0)) - s.lateral) * k;
      s.fuel += ((L?.fuel ?? 1) - s.fuel) * k;

      // Distance in drawing units: the car is ~4.8 m long, drawn 280 long.
      const mps = s.speed / 3.6;
      const step = mps * (280 / 4.8) * dt;
      s.dist += step;
      roadTex.offset.x = (s.dist / 220) % 1;
      gridTex.offset.x = (s.dist / 60) % 1;
      for (const w of wheels) w.rotation.z -= step / shape.r;

      // Body: nose lifts when speeding up, dips when braking, leans in turns.
      chassis.rotation.z += (s.accel * 0.045 - chassis.rotation.z) * k;
      chassis.rotation.x += (-s.lateral * 0.05 - chassis.rotation.x) * k;
      chassis.position.y = Math.min(1, s.speed / 80) * 0.5 * Math.sin(t * 17) * Math.sin(t * 2.3);

      const braking = s.accel < -0.08;
      tailMat.color.setRGB(braking ? 3.2 : 1.1, braking ? 0.1 : 0.06, braking ? 0.12 : 0.08);
      tailHalo.opacity = braking ? 1 : 0.35;

      if (L) {
        setLevel(engine.mat, L.engine, t, 0.35);
        setLevel(battery.mat, L.battery, t, 0.35);
        setLevel(tank.mat, L.tank, t, 0.1);
        setLevel(fillMat, L.tank, t, 0.6);
        glow.color.set(LEVEL[L.engine]);
        glow.intensity = L.engine === "warn" || L.engine === "bad" ? 400 * (0.6 + 0.4 * Math.sin(t * 5)) : 0;
      }
      const f = Math.max(0.03, Math.min(1, s.fuel));
      fill.scale.y = f;
      fill.position.y = tankY - 6 + 6 * f;

      // Camera: three-quarter front view, a slow drift, pulled back a touch
      // under hard acceleration.
      const dist = (camera.userData.dist as number) * (1 + s.accel * 0.06);
      const a = 0.72 + 0.06 * Math.sin(t * 0.13);
      camera.position.set(Math.sin(a) * dist * 0.92, 165 + 12 * Math.sin(t * 0.09), Math.cos(a) * dist);
      camera.lookAt(target);

      renderer.render(scene, camera);

      // Pin the HTML tags to their parts.
      const w = el.clientWidth;
      const h = el.clientHeight;
      for (const [id, p] of Object.entries(anchors)) {
        const tag = tagEls.current[id];
        if (!tag) continue;
        v.copy(p).applyMatrix4(chassis.matrixWorld).project(camera);
        tag.style.transform = `translate(${((v.x + 1) / 2) * w}px, ${((1 - v.y) / 2) * h}px) translate(-50%, -100%)`;
        const text = L?.tags[id as keyof CarLive["tags"]] ?? "";
        const label = tag.lastElementChild as HTMLElement | null;
        if (label && label.textContent !== text) label.textContent = text;
        const level = L?.[id as "engine" | "tank" | "battery"] ?? "off";
        tag.dataset.level = level;
        tag.style.opacity = text ? "1" : "0";
      }
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      scene.traverse((o) => {
        const m = o as THREE.Mesh;
        m.geometry?.dispose();
        const mats = Array.isArray(m.material) ? m.material : m.material ? [m.material] : [];
        for (const mat of mats) mat.dispose();
      });
      for (const tex of [roadTex, gridTex, halo, env]) tex.dispose();
      pmrem.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [body, accent, live, failed]);

  if (failed) {
    return (
      <div className={className}>
        <div className="flex h-full items-center justify-center px-[30%]">
          <CarSide body={body} accent={accent} className="w-full max-w-[520px]" />
        </div>
      </div>
    );
  }

  return (
    <div ref={host} className={cn("relative overflow-hidden", className)} style={style}>
      {(["engine", "tank", "battery"] as const).map((id) => (
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
