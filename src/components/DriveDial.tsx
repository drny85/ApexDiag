import { useEffect, useRef, type ReactNode } from "react";

/**
 * Drive Mode's big dial: a 240° arc sized to its box, drawn every frame and
 * eased toward the latest reading so it moves smoothly between the few
 * readings a second the adapter delivers. The number in the middle is the
 * thing to glance at; the arc gives the sense of it.
 */
export default function DriveDial({
  value,
  max,
  majorStep,
  labelScale = 1,
  redFrom,
  shiftFrom,
  unit,
  label,
  color = "#22d3ee",
  children,
}: {
  value: number | undefined;
  max: number;
  majorStep: number;
  labelScale?: number;
  /** Red zone from here up. */
  redFrom?: number;
  /** The outer ring flashes from here up (a shift light). */
  shiftFrom?: number;
  unit: string;
  label: string;
  color?: string;
  children?: ReactNode;
}) {
  const box = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const readout = useRef<HTMLDivElement>(null);
  const target = useRef(value);
  target.current = value;

  useEffect(() => {
    const el = canvas.current;
    const host = box.current;
    if (!el || !host) return;
    const ctx = el.getContext("2d")!;
    let size = 0;
    const fit = () => {
      const s = Math.floor(Math.min(host.clientWidth, host.clientHeight));
      if (!s || s === size) return;
      size = s;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      el.width = s * dpr;
      el.height = s * dpr;
      el.style.width = `${s}px`;
      el.style.height = `${s}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      if (readout.current) readout.current.style.fontSize = `${Math.round(s / 4.1)}px`;
    };
    const ro = new ResizeObserver(fit);
    ro.observe(host);
    fit();

    const START = Math.PI * (150 / 180);
    const SWEEP = Math.PI * (240 / 180);
    let shown = value ?? 0;
    let last = performance.now();
    let raf = 0;
    const angle = (v: number) => START + (Math.min(Math.max(v, 0), max) / max) * SWEEP;

    const draw = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const tv = target.current;
      if (tv !== undefined) shown += (tv - shown) * (1 - Math.exp(-dt * 7));
      const s = size;
      if (s) {
        const c = s / 2;
        const r = s / 2 - s * 0.075;
        const w = Math.max(8, s * 0.045);
        ctx.clearRect(0, 0, s, s);

        // Soft face.
        const face = ctx.createRadialGradient(c, c, r * 0.2, c, c, r * 1.05);
        face.addColorStop(0, "rgba(255,255,255,0.035)");
        face.addColorStop(1, "rgba(255,255,255,0)");
        ctx.fillStyle = face;
        ctx.beginPath();
        ctx.arc(c, c, r * 1.05, 0, Math.PI * 2);
        ctx.fill();

        // Track.
        ctx.lineCap = "round";
        ctx.lineWidth = w;
        ctx.strokeStyle = "rgba(255,255,255,0.06)";
        ctx.beginPath();
        ctx.arc(c, c, r, START, START + SWEEP);
        ctx.stroke();

        // Red zone.
        if (redFrom !== undefined) {
          ctx.lineCap = "butt";
          ctx.lineWidth = w * 0.35;
          ctx.strokeStyle = "rgba(244,63,94,0.8)";
          ctx.beginPath();
          ctx.arc(c, c, r + w * 0.95, angle(redFrom), START + SWEEP);
          ctx.stroke();
        }

        // Ticks and numbers, outside the arc.
        const minor = majorStep / 4;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.font = `600 ${Math.round(s * 0.042)}px -apple-system, "SF Pro Text", Inter, system-ui, sans-serif`;
        for (let x = 0; x <= max + 1e-6; x += minor) {
          const a = angle(x);
          const major = Math.abs(x / majorStep - Math.round(x / majorStep)) < 1e-6;
          const hot = redFrom !== undefined && x >= redFrom;
          const i1 = r - w * (major ? 1.6 : 1.15);
          const i2 = r - w * 0.75;
          ctx.lineWidth = major ? 2 : 1;
          ctx.strokeStyle = hot ? "rgba(251,113,133,0.9)" : major ? "rgba(255,255,255,0.5)" : "rgba(255,255,255,0.16)";
          ctx.beginPath();
          ctx.moveTo(c + Math.cos(a) * i1, c + Math.sin(a) * i1);
          ctx.lineTo(c + Math.cos(a) * i2, c + Math.sin(a) * i2);
          ctx.stroke();
          if (major) {
            const lr = r - w * 2.6;
            ctx.fillStyle = hot ? "rgba(251,113,133,0.95)" : "rgba(255,255,255,0.42)";
            ctx.fillText(String(Math.round(x / labelScale)), c + Math.cos(a) * lr, c + Math.sin(a) * lr);
          }
        }

        if (tv !== undefined) {
          const a = angle(shown);
          const hot = redFrom !== undefined && shown >= redFrom;
          const tone = hot ? "#fb7185" : color;
          const grad = typeof ctx.createConicGradient === "function" ? ctx.createConicGradient(START - 0.05, c, c) : null;
          if (grad) {
            grad.addColorStop(0, "rgba(34,211,238,0.05)");
            grad.addColorStop(Math.max(0.02, Math.min(0.98, ((a - START) / (Math.PI * 2)) * 0.98)), tone);
            grad.addColorStop(1, tone);
          }
          ctx.save();
          ctx.shadowColor = tone;
          ctx.shadowBlur = s * 0.05;
          ctx.lineCap = "round";
          ctx.lineWidth = w;
          ctx.strokeStyle = grad ?? tone;
          ctx.beginPath();
          ctx.arc(c, c, r, START, Math.max(START + 0.001, a));
          ctx.stroke();
          // Bright tip.
          ctx.fillStyle = "#fff";
          ctx.shadowBlur = s * 0.06;
          ctx.beginPath();
          ctx.arc(c + Math.cos(a) * r, c + Math.sin(a) * r, w * 0.42, 0, Math.PI * 2);
          ctx.fill();
          ctx.restore();

          // Shift light: the outer ring flashes.
          if (shiftFrom !== undefined && shown >= shiftFrom && Math.floor(now / 110) % 2 === 0) {
            ctx.save();
            ctx.strokeStyle = "#fb7185";
            ctx.shadowColor = "#fb7185";
            ctx.shadowBlur = s * 0.08;
            ctx.lineWidth = w * 0.5;
            ctx.beginPath();
            ctx.arc(c, c, r + w * 1.6, 0, Math.PI * 2);
            ctx.stroke();
            ctx.restore();
          }
        }
        if (readout.current) {
          const text = tv === undefined ? "--" : String(Math.max(0, Math.round(shown)));
          if (readout.current.textContent !== text) readout.current.textContent = text;
        }
      }
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [max, majorStep, labelScale, redFrom, shiftFrom, color]);

  return (
    <div ref={box} className="relative flex h-full w-full items-center justify-center">
      <canvas ref={canvas} />
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
        <div className="text-[11px] font-semibold uppercase tracking-[0.22em] text-white/40">{label}</div>
        <div ref={readout} className="font-semibold leading-none tabular-nums tracking-tighter text-white" style={{ fontSize: 64 }}>
          --
        </div>
        <div className="mt-1 text-sm font-medium text-white/50">{unit}</div>
        {children && <div className="mt-3">{children}</div>}
      </div>
    </div>
  );
}
