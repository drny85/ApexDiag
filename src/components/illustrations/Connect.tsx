/**
 * Illustrations for getting connected: where the OBD port is, and the
 * handshake between the Mac, the adapter and the car.
 */
import { cn } from "@/lib/utils";

const LINE = "rgba(255,255,255,0.22)";
const SOFT = "rgba(255,255,255,0.08)";
const CYAN = "#67e8f9";
const AMBER = "#fcd34d";

/** The X5's driver footwell seen from the driver's door, with the adapter clicking into the port. */
export function ObdPortScene({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 360 222" className={cn("ill", className)} fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <defs>
        <radialGradient id="obd-glow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor={CYAN} stopOpacity="0.35" />
          <stop offset="100%" stopColor={CYAN} stopOpacity="0" />
        </radialGradient>
        <linearGradient id="dash-fill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.06" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0.01" />
        </linearGradient>
      </defs>

      {/* dashboard and lower trim */}
      <path d="M8 34 C 90 18, 250 14, 352 30 L 352 92 C 300 96, 250 100, 214 104 L 120 108 C 70 110, 30 112, 8 116 Z" fill="url(#dash-fill)" stroke={LINE} />
      <path d="M8 116 C 30 112, 70 110, 120 108 L 214 104" stroke={LINE} />
      {/* instrument cluster hood */}
      <path d="M188 30 C 210 10, 290 8, 318 28" stroke={LINE} />
      {/* steering column and wheel rim */}
      <path d="M244 92 L 262 150" stroke={LINE} strokeWidth="10" opacity="0.45" />
      <ellipse cx="254" cy="62" rx="58" ry="20" stroke={LINE} strokeWidth="5" opacity="0.55" />
      {/* hood release */}
      <path d="M38 124 l 18 -2 l 2 10 l -18 2 z" stroke={LINE} />
      <text x="30" y="148" fill="rgba(255,255,255,0.35)" fontSize="8.5">hood release</text>
      {/* pedals */}
      <path d="M200 200 l 10 -46 l 18 2 l -8 46" stroke={LINE} />
      <path d="M262 206 l 4 -40 l 30 2 l -2 40" stroke={LINE} />
      {/* floor */}
      <path d="M170 214 L 352 214" stroke={SOFT} />

      {/* the port, under the trim, left of the column */}
      <circle cx="150" cy="128" r="34" fill="url(#obd-glow)" />
      <path d="M132 118 h 36 l -4 16 h -28 z" fill="#0b0f14" stroke={CYAN} strokeWidth="1.5" />
      <path d="M138 123 h 24 M140 128 h 20" stroke={CYAN} strokeOpacity="0.5" />

      {/* adapter sliding up into the port */}
      <g style={{ animation: "ill-plug 4.2s ease-in-out infinite" }}>
        <path d="M134 136 h 32 l -3 24 h -26 z" fill="#111827" stroke="rgba(255,255,255,0.55)" strokeWidth="1.5" />
        <rect x="140" y="146" width="20" height="8" rx="2" fill="#0b0f14" stroke="rgba(255,255,255,0.2)" />
        <circle cx="150" cy="150" r="2.6" fill={CYAN} style={{ animation: "ill-led 4.2s ease-in-out infinite" }} />
      </g>

      {/* callout */}
      <path d="M128 140 C 110 150, 104 166, 92 174" stroke={AMBER} strokeDasharray="3 4" />
      <circle cx="128" cy="140" r="2.5" fill={AMBER} />
      <text x="12" y="186" fill={AMBER} fontSize="11" fontWeight="600">OBD port</text>
      <text x="12" y="199" fill="rgba(255,255,255,0.5)" fontSize="9.5">under the dash, left of</text>
      <text x="12" y="211" fill="rgba(255,255,255,0.5)" fontSize="9.5">the steering column</text>
    </svg>
  );
}

const STAGES = [
  { label: "Adapter", detail: "Bluetooth link" },
  { label: "Protocol", detail: "CAN 500k" },
  { label: "Car", detail: "VIN read" },
] as const;

/**
 * Mac → adapter → car, lighting up as the handshake runs.
 * `step`: 0 nothing yet, 1 adapter answered, 2 protocol set, 3 car answered (connected).
 */
export function HandshakeFlow({ step, className }: { step: number; className?: string }) {
  const done = step >= 3;
  const nodes = [
    { x: 40, icon: "mac" },
    { x: 180, icon: "adapter" },
    { x: 320, icon: "car" },
  ] as const;
  const lit = (i: number) => (i === 0 ? true : i === 1 ? step >= 1 : step >= 3);
  return (
    <svg viewBox="0 0 360 96" className={cn("ill", className)} fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      {/* links */}
      {[0, 1].map((i) => {
        const on = step >= (i === 0 ? 1 : 2);
        const x1 = nodes[i]!.x + 26;
        const x2 = nodes[i + 1]!.x - 26;
        return (
          <g key={i}>
            <path d={`M${x1} 40 H${x2}`} stroke="rgba(255,255,255,0.1)" strokeWidth="2" />
            <path
              d={`M${x1} 40 H${x2}`}
              stroke={on ? CYAN : "rgba(255,255,255,0.18)"}
              strokeWidth="2"
              strokeDasharray="6 4"
              style={{ animation: on ? `ill-flow ${done ? 1.6 : 0.6}s linear infinite` : undefined, transition: "stroke 300ms" }}
            />
            {/* keep-alive pulse once connected */}
            {done && (
              <circle r="3" cy="40" fill={CYAN}>
                <animate attributeName="cx" from={x1} to={x2} dur="2s" repeatCount="indefinite" begin={`${i * 0.5}s`} />
                <animate attributeName="opacity" values="0;1;1;0" dur="2s" repeatCount="indefinite" begin={`${i * 0.5}s`} />
              </circle>
            )}
          </g>
        );
      })}
      {nodes.map((n, i) => {
        const on = lit(i);
        const color = on ? CYAN : "rgba(255,255,255,0.3)";
        return (
          <g key={n.icon}>
            {on && i > 0 && <circle cx={n.x} cy="40" r="22" fill={CYAN} opacity="0.08" style={{ animation: "ill-pop 400ms ease-out both" }} />}
            <circle cx={n.x} cy="40" r="22" stroke={color} strokeWidth="1.5" fill="#0b0f14" style={{ transition: "stroke 300ms" }} />
            {n.icon === "mac" && (
              <g stroke={color}>
                <rect x={n.x - 11} y="31" width="22" height="14" rx="2" />
                <path d={`M${n.x - 14} 49 h28`} />
              </g>
            )}
            {n.icon === "adapter" && (
              <g stroke={color}>
                <path d={`M${n.x - 9} 30 h18 l-2 18 h-14 z`} />
                <circle cx={n.x} cy="40" r="2" fill={on ? CYAN : "none"} />
              </g>
            )}
            {n.icon === "car" && (
              <g stroke={color}>
                <path d={`M${n.x - 13} 45 v-5 l4 -7 h18 l4 7 v5 z`} />
                <circle cx={n.x - 7} cy="46" r="2.5" />
                <circle cx={n.x + 7} cy="46" r="2.5" />
              </g>
            )}
            <text x={n.x} y="80" textAnchor="middle" fontSize="10" fill={on ? "rgba(255,255,255,0.85)" : "rgba(255,255,255,0.35)"}>
              {i === 0 ? "This Mac" : STAGES[i === 1 ? 0 : 2].label}
            </text>
            <text x={n.x} y="92" textAnchor="middle" fontSize="8.5" fill="rgba(255,255,255,0.35)">
              {i === 0 ? "ApexDiag" : i === 1 ? (step >= 2 ? STAGES[1].detail : STAGES[0].detail) : STAGES[2].detail}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
