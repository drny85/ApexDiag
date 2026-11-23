/**
 * Small animated diagrams for each Repair Copilot check, so a technician
 * sees what is being measured before reading the numbers.
 */
import type { CheckId, Fix } from "@/lib/tauri";
import { cn } from "@/lib/utils";

const C = { line: "rgba(255,255,255,0.25)", dim: "rgba(255,255,255,0.12)", cyan: "#67e8f9", amber: "#fbbf24", green: "#34d399", rose: "#fb7185" };

export function CheckArt({ check, className }: { check: CheckId; className?: string }) {
  return (
    <svg viewBox="0 0 160 90" className={cn("ill", className)} fill="none" strokeLinecap="round" strokeLinejoin="round">
      {check === "fuel_trims" && <Trims />}
      {check === "intake_vacuum" && <Vacuum />}
      {check === "idle_stability" && <Idle />}
      {check === "catalyst_oxygen_sensors" && <Catalyst />}
      {check === "warm_up" && <WarmUp />}
      {check === "charging" && <Charging />}
    </svg>
  );
}

/** A balance beam: air on one side, fuel on the other, tipping lean. */
function Trims() {
  return (
    <g>
      <path d="M80 70 L 70 84 L 90 84 Z" stroke={C.line} />
      <g style={{ transformOrigin: "80px 62px", animation: "ill-tilt 3s ease-in-out infinite" }}>
        <path d="M24 62 L 136 62" stroke={C.cyan} strokeWidth="2" />
        <circle cx="36" cy="50" r="10" stroke={C.cyan} />
        <text x="36" y="53" textAnchor="middle" fontSize="8" fill={C.cyan}>AIR</text>
        <rect x="114" y="44" width="16" height="16" rx="3" stroke={C.amber} />
        <text x="122" y="55" textAnchor="middle" fontSize="7" fill={C.amber}>FUEL</text>
      </g>
      <text x="80" y="16" textAnchor="middle" fontSize="8" fill={C.line}>+ trim = adding fuel</text>
    </g>
  );
}

function Vacuum() {
  return (
    <g>
      <path d="M30 70 A 50 50 0 0 1 130 70" stroke={C.dim} strokeWidth="6" />
      <path d="M30 70 A 50 50 0 0 1 72 21" stroke={C.green} strokeWidth="6" opacity="0.6" />
      <g style={{ transformOrigin: "80px 70px", ["--from" as string]: "-60deg", ["--to" as string]: "-35deg", animation: "ill-needle 3s ease-in-out infinite alternate" }}>
        <path d="M80 70 L 80 28" stroke="#fff" strokeWidth="2" />
      </g>
      <circle cx="80" cy="70" r="4" fill="#fff" />
      <text x="34" y="86" fontSize="7" fill={C.green}>strong</text>
      <text x="108" y="86" fontSize="7" fill={C.line}>weak</text>
    </g>
  );
}

function Idle() {
  return (
    <g>
      <path d="M10 70 L 150 70" stroke={C.dim} />
      <path d="M10 45 L 150 45" stroke={C.dim} strokeDasharray="3 4" />
      <path
        d="M10 46 Q 20 42 30 46 T 50 45 T 70 47 T 90 44 T 110 46 T 130 45 T 150 46"
        stroke={C.cyan}
        strokeWidth="2"
        strokeDasharray="6 4"
        style={{ animation: "ill-flow 0.8s linear infinite" }}
      />
      <text x="12" y="38" fontSize="7" fill={C.line}>idle rpm, steady = good</text>
    </g>
  );
}

/** Front sensor switching, rear sensor flat behind the converter. */
function Catalyst() {
  return (
    <g>
      <rect x="58" y="34" width="44" height="22" rx="8" stroke={C.line} />
      <path d="M62 40 h36 M62 45 h36 M62 50 h36" stroke={C.dim} />
      <path d="M6 45 H 58 M102 45 H 154" stroke={C.dim} strokeWidth="4" />
      <path d="M8 24 l 6 -10 l 6 20 l 6 -20 l 6 20 l 6 -20 l 6 20 l 6 -10" stroke={C.amber} strokeWidth="1.6" style={{ animation: "ill-breathe 1.2s ease-in-out infinite" }} />
      <path d="M108 20 H 152" stroke={C.green} strokeWidth="1.6" />
      <text x="28" y="72" textAnchor="middle" fontSize="7" fill={C.amber}>front: switching</text>
      <text x="130" y="72" textAnchor="middle" fontSize="7" fill={C.green}>rear: steady</text>
      <text x="80" y="86" textAnchor="middle" fontSize="7" fill={C.line}>converter</text>
    </g>
  );
}

function WarmUp() {
  return (
    <g>
      <rect x="72" y="10" width="16" height="56" rx="8" stroke={C.line} />
      <circle cx="80" cy="74" r="11" stroke={C.line} />
      <circle cx="80" cy="74" r="7" fill={C.rose} />
      <rect x="77" y="30" width="6" height="40" rx="3" fill={C.rose} style={{ transformOrigin: "80px 70px", animation: "ill-grow-y 3s ease-in-out infinite alternate" }} />
      <path d="M92 26 h8" stroke={C.green} />
      <text x="104" y="29" fontSize="7" fill={C.green}>80 °C+</text>
    </g>
  );
}

function Charging() {
  return (
    <g>
      <rect x="44" y="28" width="72" height="44" rx="6" stroke={C.line} />
      <rect x="56" y="22" width="10" height="6" stroke={C.line} />
      <rect x="94" y="22" width="10" height="6" stroke={C.line} />
      <path d="M84 34 L 74 52 H 84 L 78 66 L 94 46 H 84 L 90 34 Z" fill={C.amber} style={{ animation: "ill-breathe 1.6s ease-in-out infinite" }} />
      <text x="80" y="86" textAnchor="middle" fontSize="7" fill={C.line}>12.4+ V resting · 13.5-14.8 V running</text>
    </g>
  );
}

const DIFFICULTY = {
  diy: { label: "DIY", n: 1, tone: "text-emerald-300", bar: "bg-emerald-400" },
  experienced: { label: "Experienced DIY", n: 2, tone: "text-amber-300", bar: "bg-amber-400" },
  shop: { label: "Shop job", n: 3, tone: "text-rose-300", bar: "bg-rose-400" },
} as const;

/** Three bars, filled by how hard the job is. */
export function DifficultyMeter({ difficulty }: { difficulty: Fix["difficulty"] }) {
  const d = DIFFICULTY[difficulty];
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-[11px] font-medium", d.tone)}>
      <span className="flex items-end gap-0.5">
        {[1, 2, 3].map((i) => (
          <span key={i} className={cn("w-1 rounded-sm", i <= d.n ? d.bar : "bg-white/10")} style={{ height: 4 + i * 3 }} />
        ))}
      </span>
      {d.label}
    </span>
  );
}
