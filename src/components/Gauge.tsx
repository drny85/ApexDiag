import { useEffect, useRef } from "react";
import { fmt } from "@/lib/format";

interface GaugeProps {
  label: string;
  unit: string;
  value: number | undefined;
  min: number;
  max: number;
  /** Values from here up are drawn as a red zone. */
  redFrom?: number;
  /** Values from here up turn the arc amber. */
  warnFrom?: number;
  /** Values below this turn the arc blue (cold / low). */
  lowBelow?: number;
  majorStep: number;
  /** Divide tick labels by this (e.g. 1000 for rpm). */
  labelScale?: number;
  digits?: number;
  size?: number;
}

const START = Math.PI * 0.75;
const SWEEP = Math.PI * 1.5;

/** A 270° dial drawn on canvas at display refresh rate, needle eased toward the latest reading. */
export default function Gauge({
  label,
  unit,
  value,
  min,
  max,
  redFrom,
  warnFrom,
  lowBelow,
  majorStep,
  labelScale = 1,
  digits = 0,
  size = 220,
}: GaugeProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const readout = useRef<HTMLSpanElement>(null);
  const target = useRef<number | undefined>(value);
  const shown = useRef<number>(value ?? min);

  target.current = value;

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const dpr = window.devicePixelRatio || 1;
    el.width = size * dpr;
    el.height = size * dpr;
    const ctx = el.getContext("2d")!;
    ctx.scale(dpr, dpr);

    let raf = 0;
    let last = performance.now();
    const c = size / 2;
    const r = size / 2 - 14;
    const angle = (v: number) => START + (Math.min(Math.max(v, min), max) - min) / (max - min) * SWEEP;

    const draw = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const t = target.current;
      if (t !== undefined) shown.current += (t - shown.current) * (1 - Math.exp(-dt * 9));
      const v = shown.current;

      ctx.clearRect(0, 0, size, size);

      // Track
      ctx.lineCap = "round";
      ctx.lineWidth = 10;
      ctx.strokeStyle = "rgba(255,255,255,0.06)";
      ctx.beginPath();
      ctx.arc(c, c, r, START, START + SWEEP);
      ctx.stroke();

      // Red zone
      if (redFrom !== undefined) {
        ctx.lineCap = "butt";
        ctx.lineWidth = 4;
        ctx.strokeStyle = "rgba(244,63,94,0.75)";
        ctx.beginPath();
        ctx.arc(c, c, r + 9, angle(redFrom), START + SWEEP);
        ctx.stroke();
      }

      // Ticks and labels
      const span = max - min;
      const minor = majorStep / 5;
      for (let x = min; x <= max + 1e-6; x += minor) {
        const a = angle(x);
        const major = Math.abs(((x - min) / majorStep) % 1) < 1e-6 || Math.abs(((x - min) / majorStep) % 1 - 1) < 1e-6;
        const inner = r - (major ? 18 : 13);
        const outer = r - 8;
        ctx.lineCap = "round";
        ctx.lineWidth = major ? 2 : 1;
        const hot = redFrom !== undefined && x >= redFrom;
        ctx.strokeStyle = hot ? "rgba(251,113,133,0.9)" : major ? "rgba(255,255,255,0.55)" : "rgba(255,255,255,0.18)";
        ctx.beginPath();
        ctx.moveTo(c + Math.cos(a) * inner, c + Math.sin(a) * inner);
        ctx.lineTo(c + Math.cos(a) * outer, c + Math.sin(a) * outer);
        ctx.stroke();
        if (major) {
          const lr = r - 32;
          ctx.fillStyle = hot ? "rgba(251,113,133,0.95)" : "rgba(255,255,255,0.5)";
          ctx.font = `500 ${Math.round(size / 20)}px ui-sans-serif, -apple-system, system-ui, sans-serif`;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.fillText(String(Math.round((x / labelScale) * 10) / 10), c + Math.cos(a) * lr, c + Math.sin(a) * lr);
        }
      }

      // Value arc
      const color =
        redFrom !== undefined && v >= redFrom
          ? "#fb7185"
          : warnFrom !== undefined && v >= warnFrom
            ? "#fbbf24"
            : lowBelow !== undefined && v < lowBelow
              ? "#60a5fa"
              : "#22d3ee";
      if (t !== undefined) {
        let grad: CanvasGradient | string = color;
        if (typeof ctx.createConicGradient === "function") {
          grad = ctx.createConicGradient(START, c, c);
          grad.addColorStop(0, "rgba(34,211,238,0.15)");
          grad.addColorStop(Math.min(0.99, Math.max(0.01, ((v - min) / span) * 0.75)), color);
          grad.addColorStop(1, color);
        }
        ctx.save();
        ctx.shadowColor = color;
        ctx.shadowBlur = 14;
        ctx.lineCap = "round";
        ctx.lineWidth = 10;
        ctx.strokeStyle = grad;
        ctx.beginPath();
        ctx.arc(c, c, r, START, Math.max(START + 0.001, angle(v)));
        ctx.stroke();
        ctx.restore();

        // Needle
        const a = angle(v);
        ctx.save();
        ctx.shadowColor = color;
        ctx.shadowBlur = 10;
        ctx.strokeStyle = "rgba(255,255,255,0.95)";
        ctx.lineWidth = 2.5;
        ctx.beginPath();
        ctx.moveTo(c + Math.cos(a) * (r * 0.28), c + Math.sin(a) * (r * 0.28));
        ctx.lineTo(c + Math.cos(a) * (r - 14), c + Math.sin(a) * (r - 14));
        ctx.stroke();
        ctx.restore();
      }

      // Hub
      ctx.fillStyle = "rgba(255,255,255,0.04)";
      ctx.beginPath();
      ctx.arc(c, c, r * 0.26, 0, Math.PI * 2);
      ctx.fill();

      if (readout.current) readout.current.textContent = t === undefined ? "--" : fmt(v, digits);
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [size, min, max, redFrom, warnFrom, lowBelow, majorStep, labelScale, digits]);

  return (
    <div className="relative mx-auto" style={{ width: size, height: size }}>
      <canvas ref={canvas} style={{ width: size, height: size }} />
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
        <span ref={readout} className="text-[34px] font-semibold tabular-nums tracking-tight" style={{ fontSize: size / 6.4 }}>
          --
        </span>
        <span className="-mt-0.5 text-xs text-muted-foreground">{unit}</span>
      </div>
      <div className="absolute inset-x-0 bottom-3 text-center text-[11px] font-medium uppercase tracking-[0.14em] text-muted-foreground">
        {label}
      </div>
    </div>
  );
}
