import { useEffect, useRef } from "react";
import { useDiag } from "@/lib/store";

const WINDOW_MS = 30_000;

/** Scrolling 30-second history for one PID, redrawn every frame. */
export default function Sparkline({
  pid,
  color = "#22d3ee",
  height = 64,
  band,
}: {
  pid: number;
  color?: string;
  height?: number;
  /** Shaded "normal" band, e.g. ±10 for fuel trims. */
  band?: [number, number];
}) {
  const { history } = useDiag();
  const canvas = useRef<HTMLCanvasElement>(null);
  const bandLo = band?.[0];
  const bandHi = band?.[1];

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const ctx = el.getContext("2d")!;
    let raf = 0;
    let lo = 0;
    let hi = 1;
    const band = bandLo !== undefined && bandHi !== undefined ? ([bandLo, bandHi] as const) : undefined;

    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const w = el.clientWidth;
      const h = height;
      if (el.width !== Math.round(w * dpr)) {
        el.width = Math.round(w * dpr);
        el.height = Math.round(h * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);

      const pts = history.current?.get(pid) ?? [];
      const now = performance.now();
      if (pts.length > 1) {
        let mn = Infinity;
        let mx = -Infinity;
        for (const p of pts) {
          mn = Math.min(mn, p.v);
          mx = Math.max(mx, p.v);
        }
        if (band) {
          mn = Math.min(mn, band[0]);
          mx = Math.max(mx, band[1]);
        }
        const pad = Math.max((mx - mn) * 0.15, 0.5);
        // Ease the y-range so the line doesn't jump.
        lo += (mn - pad - lo) * 0.1;
        hi += (mx + pad - hi) * 0.1;
        if (!Number.isFinite(lo) || !Number.isFinite(hi) || hi - lo < 1e-6) {
          lo = mn - pad;
          hi = mx + pad;
        }
        const x = (t: number) => w - ((now - t) / WINDOW_MS) * w;
        const y = (v: number) => h - 4 - ((v - lo) / (hi - lo)) * (h - 8);

        if (band) {
          ctx.save();
          ctx.setLineDash([3, 4]);
          ctx.strokeStyle = "rgba(255,255,255,0.12)";
          ctx.lineWidth = 1;
          for (const b of [band[0], 0, band[1]]) {
            ctx.globalAlpha = b === 0 ? 0.6 : 1;
            ctx.beginPath();
            ctx.moveTo(0, Math.round(y(b)) + 0.5);
            ctx.lineTo(w, Math.round(y(b)) + 0.5);
            ctx.stroke();
          }
          ctx.restore();
        }

        ctx.beginPath();
        pts.forEach((p, i) => (i ? ctx.lineTo(x(p.t), y(p.v)) : ctx.moveTo(x(p.t), y(p.v))));
        const lastPt = pts[pts.length - 1]!;
        ctx.lineWidth = 1.75;
        ctx.strokeStyle = color;
        ctx.lineJoin = "round";
        ctx.stroke();

        ctx.lineTo(x(lastPt.t), h);
        ctx.lineTo(x(pts[0]!.t), h);
        ctx.closePath();
        const grad = ctx.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, color + "40");
        grad.addColorStop(1, color + "00");
        ctx.fillStyle = grad;
        ctx.fill();

        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(x(lastPt.t), y(lastPt.v), 2.5, 0, Math.PI * 2);
        ctx.fill();
      }
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [pid, color, height, history, bandLo, bandHi]);

  return <canvas ref={canvas} className="block w-full" style={{ height }} />;
}
