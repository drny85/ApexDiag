/**
 * Top-down outline of the connected car with each control module drawn
 * where it sits. Positions come from the vehicle pack and are approximate:
 * they show the area, not the bolt.
 */
import { useState } from "react";
import type { ModuleInfo, ModuleScan, Pack } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { TopDownBody } from "./Vehicle";

export type MapStatus = "unknown" | "absent" | "ok" | "fault" | "scanning" | "skipped";

const COLOR: Record<MapStatus, string> = {
  unknown: "#94a3b8",
  absent: "#475569",
  ok: "#34d399",
  fault: "#fbbf24",
  scanning: "#67e8f9",
  skipped: "#334155",
};

export function statusFromScan(scan: ModuleScan[] | null, address: number): MapStatus {
  const s = scan?.find((x) => x.module.address === address);
  if (!s) return "unknown";
  if (!s.present) return "absent";
  return (s.faults ?? 0) > 0 ? "fault" : "ok";
}

export default function CarMap({
  pack,
  modules,
  status,
  scanning,
  onPick,
  className,
}: {
  pack: Pack;
  modules: ModuleInfo[];
  status: (address: number) => MapStatus;
  scanning?: boolean;
  onPick?: (m: ModuleInfo) => void;
  className?: string;
}) {
  const [hover, setHover] = useState<number | null>(null);
  const hovered = modules.find((m) => m.address === hover);

  return (
    <div className="relative">
      <svg viewBox="0 0 240 470" className={cn("ill mx-auto h-[420px] w-auto", className)} fill="none" strokeLinecap="round" strokeLinejoin="round">
        <TopDownBody body={pack.body} />
        {modules.map((m) => {
          const p = m.spot;
          const st = status(m.address);
          const color = COLOR[st];
          const active = hover === m.address;
          return (
            <g
              key={m.address}
              className={cn(onPick && "cursor-pointer")}
              onMouseEnter={() => setHover(m.address)}
              onMouseLeave={() => setHover(null)}
              onClick={() => {
                setHover(m.address);
                onPick?.(m);
              }}
            >
              {/* Finger-sized hit area around the dot. */}
              <circle cx={p.x} cy={p.y} r="16" fill="transparent" />
              {st === "fault" && <circle cx={p.x} cy={p.y} r="6" fill={color} style={{ animation: "ill-ping 1.8s ease-out infinite" }} />}
              {(st === "scanning" || (scanning && st === "unknown")) && (
                <circle cx={p.x} cy={p.y} r="6" fill="#67e8f9" style={{ animation: "ill-ping 1s ease-out infinite" }} />
              )}
              <circle cx={p.x} cy={p.y} r={active ? 8 : 6} fill={color} stroke="#0b0f14" strokeWidth="2" style={{ transition: "r 150ms, fill 300ms" }} />
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
        {hovered ? (
          <>
            <div className="font-medium text-foreground">
              {hovered.description} <span className="font-mono text-muted-foreground">({hovered.name})</span>
            </div>
            <div className="text-muted-foreground">{hovered.spot.area} (approximate)</div>
          </>
        ) : (
          <div className="text-muted-foreground">Point at a module to see where it lives. Locations are approximate.</div>
        )}
      </div>
    </div>
  );
}
