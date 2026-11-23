/**
 * Top-down outline of the X5 with each control module drawn where it sits
 * in the car. Locations are approximate: they show the area, not the bolt.
 */
import { useState } from "react";
import type { ModuleInfo, ModuleScan } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Approximate position (viewBox 0 0 240 470, front at the top, driver on the left). */
const SPOT: Record<number, { x: number; y: number; where: string }> = {
  0x29: { x: 74, y: 78, where: "Engine bay, driver side, front" },
  0x12: { x: 166, y: 120, where: "Engine bay, passenger side, in the electronics box" },
  0x60: { x: 88, y: 178, where: "Behind the instruments" },
  0x78: { x: 120, y: 182, where: "Centre of the dashboard" },
  0x63: { x: 152, y: 178, where: "Dashboard, behind the centre display" },
  0x40: { x: 100, y: 198, where: "Under the dash, left of the steering column" },
  0x72: { x: 58, y: 204, where: "Driver footwell, left kick panel" },
  0x00: { x: 182, y: 204, where: "Passenger footwell" },
  0x56: { x: 120, y: 226, where: "Roof console, above the mirror" },
  0x18: { x: 120, y: 256, where: "On the transmission, under the centre tunnel" },
  0x01: { x: 148, y: 268, where: "Under the centre console" },
  0x19: { x: 120, y: 290, where: "Transfer case, under the car" },
  0x64: { x: 178, y: 424, where: "Rear, behind the boot trim" },
};

export default function CarMap({
  modules,
  scan,
  scanning,
  onPick,
}: {
  modules: ModuleInfo[];
  scan: ModuleScan[] | null;
  scanning: boolean;
  onPick: (m: ModuleInfo) => void;
}) {
  const [hover, setHover] = useState<number | null>(null);
  const hovered = modules.find((m) => m.address === hover);
  const spot = hover !== null ? SPOT[hover] : undefined;
  const status = (a: number) => {
    const s = scan?.find((x) => x.module.address === a);
    if (!s) return "unknown" as const;
    if (!s.present) return "absent" as const;
    return (s.faults ?? 0) > 0 ? ("fault" as const) : ("ok" as const);
  };

  return (
    <div className="relative">
      <svg viewBox="0 0 240 470" className="ill mx-auto h-[420px] w-auto" fill="none" strokeLinecap="round" strokeLinejoin="round">
        <defs>
          <linearGradient id="car-body" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor="#ffffff" stopOpacity="0.06" />
            <stop offset="100%" stopColor="#ffffff" stopOpacity="0.015" />
          </linearGradient>
        </defs>
        {/* wheels */}
        {[
          [20, 88],
          [204, 88],
          [20, 340],
          [204, 340],
        ].map(([x, y]) => (
          <rect key={`${x}-${y}`} x={x} y={y} width="16" height="56" rx="6" fill="#0b0f14" stroke="rgba(255,255,255,0.18)" />
        ))}
        {/* body */}
        <path
          d="M64 22 C 90 10, 150 10, 176 22 C 200 32, 212 60, 212 96 L 214 380 C 214 420, 200 452, 170 458 C 140 462, 100 462, 70 458 C 40 452, 26 420, 26 380 L 28 96 C 28 60, 40 32, 64 22 Z"
          fill="url(#car-body)"
          stroke="rgba(255,255,255,0.28)"
          strokeWidth="1.5"
        />
        {/* hood seam, windscreen, roof, rear window */}
        <path d="M44 150 C 90 140, 150 140, 196 150" stroke="rgba(255,255,255,0.14)" />
        <path d="M48 164 C 95 152, 145 152, 192 164 L 182 212 C 140 204, 100 204, 58 212 Z" fill="rgba(103,232,249,0.04)" stroke="rgba(255,255,255,0.2)" />
        <path d="M58 212 L 56 380 M182 212 L 184 380" stroke="rgba(255,255,255,0.12)" />
        <path d="M56 380 C 100 388, 140 388, 184 380 L 188 420 C 140 430, 100 430, 52 420 Z" fill="rgba(103,232,249,0.03)" stroke="rgba(255,255,255,0.18)" />
        {/* mirrors */}
        <path d="M28 176 l -14 4 l 2 12 l 14 -2" stroke="rgba(255,255,255,0.22)" />
        <path d="M212 176 l 14 4 l -2 12 l -14 -2" stroke="rgba(255,255,255,0.22)" />
        {/* seats */}
        {[
          [70, 236],
          [140, 236],
        ].map(([x, y]) => (
          <rect key={x} x={x} y={y} width="30" height="38" rx="8" stroke="rgba(255,255,255,0.1)" />
        ))}
        <rect x="66" y="302" width="108" height="36" rx="10" stroke="rgba(255,255,255,0.1)" />
        {/* steering wheel */}
        <circle cx="85" cy="214" r="13" stroke="rgba(255,255,255,0.18)" strokeWidth="3" />
        {/* centre tunnel */}
        <path d="M112 240 L 112 330 M128 240 L 128 330" stroke="rgba(255,255,255,0.07)" />
        <text x="120" y="36" textAnchor="middle" fontSize="8" letterSpacing="2" fill="rgba(255,255,255,0.3)">
          FRONT
        </text>

        {modules.map((m) => {
          const p = SPOT[m.address];
          if (!p) return null;
          const st = status(m.address);
          const color = { unknown: "#94a3b8", absent: "#475569", ok: "#34d399", fault: "#fbbf24" }[st];
          const active = hover === m.address;
          return (
            <g
              key={m.address}
              className="cursor-pointer"
              onMouseEnter={() => setHover(m.address)}
              onMouseLeave={() => setHover(null)}
              onClick={() => onPick(m)}
            >
              {st === "fault" && (
                <circle cx={p.x} cy={p.y} r="6" fill={color} style={{ animation: "ill-ping 1.8s ease-out infinite" }} />
              )}
              {scanning && <circle cx={p.x} cy={p.y} r="6" fill="#67e8f9" style={{ animation: "ill-breathe 1s ease-in-out infinite" }} />}
              <circle cx={p.x} cy={p.y} r={active ? 8 : 6} fill={color} stroke="#0b0f14" strokeWidth="2" style={{ transition: "r 150ms" }} />
              <text
                x={p.x}
                y={p.y - 12}
                textAnchor="middle"
                fontSize="9.5"
                fontWeight={active ? 700 : 500}
                fill={active ? "#fff" : "rgba(255,255,255,0.6)"}
                className="font-mono"
              >
                {m.name.split("/")[0]}
              </text>
            </g>
          );
        })}
      </svg>
      <div className={cn("pointer-events-none mt-2 min-h-[38px] text-center text-xs transition", hovered ? "opacity-100" : "opacity-60")}>
        {hovered && spot ? (
          <>
            <div className="font-medium text-foreground">{hovered.description}</div>
            <div className="text-muted-foreground">{spot.where} (approximate)</div>
          </>
        ) : (
          <div className="text-muted-foreground">Point at a module to see where it lives. Locations are approximate.</div>
        )}
      </div>
    </div>
  );
}
