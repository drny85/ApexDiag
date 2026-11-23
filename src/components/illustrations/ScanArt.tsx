/**
 * Art for the Full Scan: a health ring that fills in by status, and a small
 * illustrated tile for each area of the car (engine, emissions, electrical,
 * modules, vehicle). Everything is inline SVG so it stays crisp on Retina
 * and iPad, and animation respects "Reduce motion".
 */
import type { ScanArea, ScanStatus } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export const STATUS_COLOR: Record<ScanStatus, string> = {
  problem: "#fb7185",
  attention: "#fbbf24",
  good: "#34d399",
  info: "#67e8f9",
  not_read: "#64748b",
};

/** Donut showing how the judged readings came out, worst first. Info and
 * not-checked readings aren't verdicts, so they stay out of the ring. */
export function HealthRing({
  counts,
  size = 168,
  children,
  className,
}: {
  counts: Record<ScanStatus, number>;
  size?: number;
  children?: React.ReactNode;
  className?: string;
}) {
  const order: ScanStatus[] = ["problem", "attention", "good"];
  const total = Math.max(1, order.reduce((n, s) => n + counts[s], 0));
  const r = 62;
  const c = 2 * Math.PI * r;
  const gap = total > 1 ? 3 : 0;
  let offset = 0;
  return (
    <div className={cn("relative grid place-items-center", className)} style={{ width: size, height: size }}>
      <svg viewBox="0 0 160 160" className="ill absolute inset-0 -rotate-90">
        <circle cx="80" cy="80" r={r} fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="12" />
        {order.map((s, i) => {
          const len = (counts[s] / total) * c;
          if (len <= 0) return null;
          const seg = (
            <circle
              key={s}
              cx="80"
              cy="80"
              r={r}
              fill="none"
              stroke={STATUS_COLOR[s]}
              strokeWidth="12"
              strokeLinecap="butt"
              strokeDasharray={`${Math.max(0.5, len - gap)} ${c}`}
              strokeDashoffset={-offset}
              style={{ animation: `ring-in 0.9s ${0.12 * i}s cubic-bezier(.2,.7,.2,1) both`, filter: `drop-shadow(0 0 6px ${STATUS_COLOR[s]}55)` }}
            />
          );
          offset += len;
          return seg;
        })}
      </svg>
      <div className="relative text-center">{children}</div>
    </div>
  );
}

const AREA_TINT: Record<ScanArea, string> = {
  engine: "#f97316",
  emissions: "#34d399",
  electrical: "#facc15",
  modules: "#67e8f9",
  vehicle: "#a78bfa",
};

/** A small illustrated tile for an area of the car. */
export function AreaArt({ area, className, active = false }: { area: ScanArea; className?: string; active?: boolean }) {
  const tint = AREA_TINT[area];
  const id = `area-${area}`;
  return (
    <svg viewBox="0 0 56 56" className={cn("ill shrink-0", className)} aria-hidden>
      <defs>
        <linearGradient id={`${id}-bg`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor={tint} stopOpacity="0.28" />
          <stop offset="1" stopColor={tint} stopOpacity="0.04" />
        </linearGradient>
      </defs>
      <rect x="1" y="1" width="54" height="54" rx="14" fill={`url(#${id}-bg)`} stroke={tint} strokeOpacity="0.35" />
      <g fill="none" stroke={tint} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        {area === "engine" && (
          <>
            {/* engine block with a piston moving */}
            <path d="M14 22h6l3-4h10l3 4h6v16H36l-3 4H23l-3-4h-6z" fill={tint} fillOpacity="0.12" />
            <path d="M10 27v6M46 27v6M10 30h4M42 30h4" />
            <rect x="24" y="25" width="8" height="8" rx="1.5" fill={tint} fillOpacity="0.5" stroke="none" style={active ? { animation: "ill-grow-y 0.7s ease-in-out infinite alternate" } : undefined} className="o-bottom" />
          </>
        )}
        {area === "emissions" && (
          <>
            {/* tailpipe with clean puffs and a leaf */}
            <path d="M8 36h18a4 4 0 0 0 0-8H8" fill={tint} fillOpacity="0.12" />
            <circle cx="33" cy="30" r="2.5" style={active ? { animation: "ill-rise 1.6s ease-out infinite" } : undefined} />
            <circle cx="38" cy="26" r="2" style={active ? { animation: "ill-rise 1.6s 0.5s ease-out infinite" } : undefined} />
            <path d="M34 44c0-8 6-12 14-12 0 8-6 12-14 12zM34 44l8-7" fill={tint} fillOpacity="0.25" />
          </>
        )}
        {area === "electrical" && (
          <>
            {/* battery with a bolt */}
            <rect x="11" y="18" width="34" height="24" rx="3" fill={tint} fillOpacity="0.1" />
            <path d="M18 18v-3h6v3M32 18v-3h6v3" />
            <path d="M29 22l-6 9h6l-2 7 7-10h-6z" fill={tint} fillOpacity="0.6" style={active ? { animation: "ill-breathe 1.2s ease-in-out infinite" } : undefined} />
          </>
        )}
        {area === "modules" && (
          <>
            {/* a little network: hub and nodes */}
            <path d="M28 28L14 16M28 28l14-12M28 28L14 40M28 28l14 12" strokeOpacity="0.6" strokeDasharray="3 3" style={active ? { animation: "ill-flow 0.8s linear infinite" } : undefined} />
            <rect x="22" y="22" width="12" height="12" rx="3" fill={tint} fillOpacity="0.35" />
            {[
              [14, 16],
              [42, 16],
              [14, 40],
              [42, 40],
            ].map(([x, y]) => (
              <circle key={`${x}-${y}`} cx={x} cy={y} r="4" fill={tint} fillOpacity="0.15" />
            ))}
          </>
        )}
        {area === "vehicle" && (
          <>
            {/* car front with a VIN tag */}
            <path d="M10 34l4-9c1-2 3-3 5-3h18c2 0 4 1 5 3l4 9v6H10z" fill={tint} fillOpacity="0.12" />
            <path d="M15 34h26" />
            <circle cx="17" cy="40" r="3" fill={tint} fillOpacity="0.4" />
            <circle cx="39" cy="40" r="3" fill={tint} fillOpacity="0.4" />
            <rect x="21" y="12" width="14" height="6" rx="1.5" fill={tint} fillOpacity="0.3" strokeWidth="1.5" />
          </>
        )}
      </g>
    </svg>
  );
}

/** The three stages of a scan, lit up as they run. */
export function ScanStages({
  stage,
  labels,
  className,
}: {
  /** 0 engine, 1 modules, 2 report, 3 done */
  stage: number;
  labels: [string, string, string];
  className?: string;
}) {
  return (
    <ol className={cn("flex items-center gap-2", className)}>
      {labels.map((l, i) => {
        const state = stage > i ? "done" : stage === i ? "now" : "next";
        return (
          <li key={l} className="flex min-w-0 flex-1 items-center gap-2">
            <span
              className={cn(
                "relative grid size-6 shrink-0 place-items-center rounded-full text-[11px] font-semibold ring-1 transition-colors",
                state === "done" && "bg-emerald-400/20 text-emerald-200 ring-emerald-400/40",
                state === "now" && "bg-cyan-400/20 text-cyan-100 ring-cyan-300/60",
                state === "next" && "bg-white/[0.03] text-muted-foreground ring-white/10",
              )}
            >
              {state === "now" && <span className="ill-motion absolute inset-0 rounded-full ring-2 ring-cyan-300/40" style={{ animation: "ill-ping 1.4s ease-out infinite" }} />}
              {state === "done" ? "✓" : i + 1}
            </span>
            <span className={cn("truncate text-xs", state === "next" ? "text-muted-foreground" : "text-foreground")}>{l}</span>
            {i < labels.length - 1 && <span className={cn("h-px flex-1", stage > i ? "bg-emerald-400/40" : "bg-white/10")} />}
          </li>
        );
      })}
    </ol>
  );
}
