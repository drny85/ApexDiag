/**
 * "What will happen" previews for special functions, shown before anything
 * runs. Each is a small looping line drawing of what the routine does.
 */
import type { ReactNode } from "react";

const LINE = "rgba(255,255,255,0.28)";
const DIM = "rgba(255,255,255,0.12)";
const CYAN = "#67e8f9";
const AMBER = "#fcd34d";
const GREEN = "#6ee7b7";
const ORANGE = "#fdba74";

function Frame({ children }: { children: ReactNode }) {
  return (
    <svg viewBox="0 0 320 120" className="ill h-auto w-full" fill="none" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      {children}
    </svg>
  );
}

function CoolantBleed() {
  return (
    <Frame>
      {/* engine block */}
      <rect x="20" y="30" width="80" height="58" rx="8" stroke={LINE} />
      {[34, 52, 70].map((x) => (
        <rect key={x} x={x} y="40" width="14" height="22" rx="3" stroke={DIM} />
      ))}
      <text x="60" y="80" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.4)">engine</text>
      {/* radiator */}
      <rect x="236" y="26" width="62" height="70" rx="4" stroke={LINE} />
      {[0, 1, 2, 3, 4, 5].map((i) => (
        <path key={i} d={`M${244 + i * 9} 32 V90`} stroke={DIM} />
      ))}
      {/* expansion tank */}
      <rect x="150" y="6" width="40" height="22" rx="5" stroke={LINE} />
      <path d="M154 20 h32" stroke={CYAN} strokeOpacity="0.5" />
      {/* hoses with flowing coolant */}
      <path d="M100 44 H236" stroke={CYAN} strokeOpacity="0.25" strokeWidth="5" />
      <path d="M100 44 H236" stroke={CYAN} strokeWidth="2" strokeDasharray="6 6" style={{ animation: "ill-flow 0.7s linear infinite" }} />
      <path d="M236 82 H160 M138 82 H100" stroke={CYAN} strokeOpacity="0.25" strokeWidth="5" />
      <path d="M236 82 H160 M138 82 H100" stroke={CYAN} strokeWidth="2" strokeDasharray="6 6" style={{ animation: "ill-flow 0.7s linear infinite reverse" }} />
      <path d="M170 44 V28" stroke={LINE} />
      {/* electric pump */}
      <circle cx="149" cy="82" r="13" fill="#0b0f14" stroke={AMBER} strokeWidth="1.5" />
      <g style={{ animation: "ill-spin 0.9s linear infinite" }}>
        <path d="M149 74 V90 M141 82 H157 M143.5 76.5 L154.5 87.5 M154.5 76.5 L143.5 87.5" stroke={AMBER} />
      </g>
      <text x="149" y="110" textAnchor="middle" fontSize="8" fill={AMBER}>electric pump</text>
      {/* air bubbles rising to the tank */}
      {[0, 0.6, 1.2, 1.8].map((d, i) => (
        <circle key={i} cx={168 + (i % 2) * 4} cy="44" r={2 + (i % 2)} stroke="#fff" strokeOpacity="0.8" style={{ animation: `ill-rise 2.4s ease-out ${d}s infinite` }} />
      ))}
    </Frame>
  );
}

function BatteryRegistration() {
  return (
    <Frame>
      <rect x="40" y="34" width="96" height="62" rx="6" stroke={LINE} />
      <rect x="56" y="26" width="14" height="8" rx="2" stroke={LINE} />
      <rect x="106" y="26" width="14" height="8" rx="2" stroke={LINE} />
      <text x="63" y="56" textAnchor="middle" fontSize="12" fill={LINE}>−</text>
      <text x="113" y="56" textAnchor="middle" fontSize="12" fill={LINE}>+</text>
      <rect x="50" y="22" width="26" height="8" rx="2" fill="#0b0f14" stroke={CYAN} />
      <text x="63" y="16" textAnchor="middle" fontSize="7.5" fill={CYAN}>battery sensor</text>
      {/* fill level resetting to new */}
      <rect x="48" y="66" width="80" height="20" rx="3" stroke={DIM} />
      <rect x="50" y="68" width="76" height="16" rx="2" fill={GREEN} fillOpacity="0.35" className="o-left" style={{ animation: "ill-grow 3s ease-in-out infinite" }} />
      {/* charge strategy dial */}
      <path d="M200 92 A 50 50 0 0 1 300 92" stroke={DIM} strokeWidth="6" />
      <path d="M200 92 A 50 50 0 0 1 228 50" stroke={AMBER} strokeOpacity="0.6" strokeWidth="6" />
      <path d="M272 50 A 50 50 0 0 1 300 92" stroke={GREEN} strokeOpacity="0.6" strokeWidth="6" />
      <text x="204" y="110" fontSize="8" fill={AMBER}>worn</text>
      <text x="296" y="110" textAnchor="end" fontSize="8" fill={GREEN}>new</text>
      <g style={{ transformOrigin: "250px 92px", transformBox: "view-box", ["--from" as string]: "-60deg", ["--to" as string]: "60deg", animation: "ill-needle 3s ease-in-out infinite" }}>
        <path d="M250 92 V52" stroke="#fff" strokeWidth="2" />
      </g>
      <circle cx="250" cy="92" r="4" fill="#fff" />
      <text x="250" y="22" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.45)">charging strategy</text>
    </Frame>
  );
}

function AdaptationReset({ what }: { what: string }) {
  const wobble = "M30 46 C 50 40, 64 52, 84 44 S 120 38, 140 46 S 176 52, 196 42 S 232 40, 252 46 S 280 50, 296 44";
  return (
    <Frame>
      <path d="M30 70 H296" stroke={DIM} strokeDasharray="3 4" />
      <text x="300" y="73" fontSize="8" fill="rgba(255,255,255,0.4)">0</text>
      <path d={wobble} stroke={AMBER} strokeWidth="2" style={{ animation: "ill-fade-out 4s ease-in-out infinite" }} />
      <text x="30" y="32" fontSize="8" fill={AMBER}>learned {what}</text>
      <path d="M30 70 H296" stroke={CYAN} strokeWidth="2" className="o-left" style={{ animation: "ill-grow 4s ease-in-out infinite" }} />
      <text x="30" y="92" fontSize="8" fill={CYAN}>back to factory value, then relearned while driving</text>
      <g style={{ ["--sweep" as string]: "266px", animation: "ill-sweep 4s ease-in-out infinite" }}>
        <path d="M30 24 V100" stroke="#fff" strokeOpacity="0.5" />
      </g>
    </Frame>
  );
}

function ActuatorTest() {
  return (
    <Frame>
      <circle cx="90" cy="60" r="42" stroke={LINE} />
      <g style={{ animation: "ill-spin 0.6s linear infinite" }}>
        {[0, 72, 144, 216, 288].map((a) => (
          <path key={a} d="M90 60 C 96 44, 110 30, 100 22 C 92 30, 86 44, 90 60" stroke={CYAN} fill={CYAN} fillOpacity="0.12" transform={`rotate(${a} 90 60)`} />
        ))}
      </g>
      <circle cx="90" cy="60" r="6" fill="#0b0f14" stroke={CYAN} />
      {/* control signal */}
      <path d="M160 80 H186 V40 H230 V80 H256 V40 H296" stroke={AMBER} strokeWidth="1.5" strokeDasharray="4 3" style={{ animation: "ill-flow 1s linear infinite" }} />
      <text x="160" y="100" fontSize="8" fill="rgba(255,255,255,0.45)">module switches the part on and off</text>
    </Frame>
  );
}

function Coding() {
  return (
    <Frame>
      {[0, 1, 2].map((i) => {
        const y = 22 + i * 32;
        const moving = i === 1;
        return (
          <g key={i}>
            <rect x="40" y={y} width="120" height="18" rx="4" stroke={DIM} />
            <path d={`M48 ${y + 9} H110`} stroke="rgba(255,255,255,0.25)" strokeWidth="3" />
            <rect x="124" y={y + 2} width="30" height="14" rx="7" stroke={moving ? ORANGE : LINE} />
            <circle
              cx="132"
              cy={y + 9}
              r="5"
              fill={moving ? ORANGE : "rgba(255,255,255,0.5)"}
              style={moving ? { ["--flip" as string]: "14px", animation: "ill-flip 3s ease-in-out infinite alternate" } : undefined}
            />
          </g>
        );
      })}
      <path d="M180 60 H214" stroke={LINE} strokeDasharray="3 4" />
      <circle cx="244" cy="60" r="26" stroke={CYAN} strokeOpacity="0.6" />
      <path d="M244 46 V60 L254 66" stroke={CYAN} />
      <text x="244" y="104" textAnchor="middle" fontSize="8" fill={CYAN}>old settings saved first</text>
    </Frame>
  );
}

function KeyProgramming() {
  return (
    <Frame>
      {/* key fob */}
      <g style={{ animation: "ill-key 3.6s ease-in-out infinite" }}>
        <rect x="36" y="38" width="44" height="26" rx="10" stroke={LINE} />
        <circle cx="50" cy="51" r="4" stroke={LINE} />
        <circle cx="66" cy="51" r="4" stroke={LINE} />
      </g>
      {/* reader slot on the column */}
      <rect x="96" y="40" width="24" height="22" rx="4" stroke={CYAN} />
      <text x="108" y="80" textAnchor="middle" fontSize="8" fill={CYAN}>reader</text>
      <path d="M124 51 H156" stroke={DIM} strokeDasharray="3 4" />
      {/* CAS with 10 key slots */}
      <rect x="160" y="22" width="140" height="60" rx="8" stroke={LINE} />
      <text x="230" y="36" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.45)">Key slots</text>
      {Array.from({ length: 10 }).map((_, i) => {
        const x = 170 + (i % 5) * 26;
        const y = 44 + Math.floor(i / 5) * 18;
        const used = i < 2;
        const adding = i === 2;
        return (
          <rect
            key={i}
            x={x}
            y={y}
            width="20"
            height="12"
            rx="3"
            stroke={used ? GREEN : adding ? AMBER : DIM}
            fill={used ? GREEN : adding ? AMBER : "none"}
            fillOpacity={used || adding ? 0.3 : 0}
            style={adding ? { animation: "ill-slot 3.6s ease-in-out infinite" } : undefined}
          />
        );
      })}
      <text x="230" y="104" textAnchor="middle" fontSize="8" fill="rgba(255,255,255,0.45)">using the car's own security code</text>
    </Frame>
  );
}

function Flash() {
  return (
    <Frame>
      <rect x="40" y="26" width="64" height="64" rx="8" stroke={LINE} />
      <rect x="56" y="42" width="32" height="32" rx="3" stroke={CYAN} />
      {[0, 1, 2, 3].map((i) => (
        <g key={i} stroke={DIM}>
          <path d={`M${50 + i * 14} 26 V18 M${50 + i * 14} 90 V98`} />
        </g>
      ))}
      <rect x="132" y="50" width="160" height="14" rx="7" stroke={DIM} />
      <rect x="134" y="52" width="156" height="10" rx="5" fill={CYAN} fillOpacity="0.6" className="o-left" style={{ animation: "ill-grow 5s linear infinite" }} />
      <text x="132" y="40" fontSize="8.5" fill="rgba(255,255,255,0.6)">writing new software</text>
      <text x="132" y="84" fontSize="8" fill={AMBER}>keep the charger and cable connected until it finishes</text>
    </Frame>
  );
}

function ClearCodes() {
  const codes = ["P0171", "P0174", "2A82"];
  return (
    <Frame>
      {codes.map((c, i) => (
        <g key={c} style={{ animation: `ill-fade-out 3.6s ease-in-out ${i * 0.15}s infinite` }}>
          <rect x={30 + i * 70} y="44" width="60" height="24" rx="6" stroke={AMBER} fill={AMBER} fillOpacity="0.08" />
          <text x={60 + i * 70} y="60" textAnchor="middle" fontSize="10" fontWeight="600" fill={AMBER} className="font-mono">
            {c}
          </text>
        </g>
      ))}
      <g style={{ ["--sweep" as string]: "210px", animation: "ill-sweep 3.6s ease-in-out infinite" }}>
        <rect x="22" y="36" width="6" height="40" rx="3" fill="#fff" fillOpacity="0.6" />
      </g>
      <circle cx="270" cy="56" r="16" stroke={CYAN} strokeOpacity="0.6" />
      <path d="M270 46 V56 L278 60" stroke={CYAN} />
      <text x="270" y="92" textAnchor="middle" fontSize="8" fill={CYAN}>copy kept</text>
    </Frame>
  );
}

function Restore() {
  return (
    <Frame>
      <circle cx="80" cy="60" r="36" stroke={LINE} />
      {Array.from({ length: 12 }).map((_, i) => (
        <path key={i} d="M80 28 V33" stroke={DIM} transform={`rotate(${i * 30} 80 60)`} />
      ))}
      <g style={{ transformOrigin: "80px 60px", transformBox: "view-box", animation: "ill-rewind 2.4s linear infinite" }}>
        <path d="M80 60 V34" stroke={ORANGE} strokeWidth="2" />
      </g>
      <path d="M80 60 L96 68" stroke="#fff" strokeOpacity="0.6" strokeWidth="2" />
      <path d="M40 34 A 46 46 0 0 1 116 24" stroke={ORANGE} strokeOpacity="0.5" />
      <path d="M40 34 l 2 -10 M40 34 l 10 -1" stroke={ORANGE} strokeOpacity="0.8" />
      {["Write", "Read back", "Match ✓"].map((t, i) => (
        <g key={t} style={{ animation: `ill-pop 0.5s ease-out ${0.3 + i * 0.5}s both` }}>
          <rect x={150 + i * 52} y="48" width="46" height="22" rx="6" stroke={i === 2 ? GREEN : LINE} />
          <text x={173 + i * 52} y="62" textAnchor="middle" fontSize="8" fill={i === 2 ? GREEN : "rgba(255,255,255,0.7)"}>
            {t}
          </text>
        </g>
      ))}
      <text x="150" y="96" fontSize="8" fill="rgba(255,255,255,0.45)">for each setting that differs</text>
    </Frame>
  );
}

const PREVIEWS: Record<string, { art: () => ReactNode; caption: string }> = {
  coolant_bleed: {
    art: CoolantBleed,
    caption: "The electric water pump runs in cycles and pushes trapped air up to the expansion tank.",
  },
  battery_registration: {
    art: BatteryRegistration,
    caption: "The car learns a new battery is fitted and charges it as new, instead of overcharging it like the worn one.",
  },
  dme_adaptation_reset: {
    art: () => <AdaptationReset what="fuel and idle corrections" />,
    caption: "Learned fuel and idle corrections go back to zero. The engine relearns them over the next drives.",
  },
  egs_adaptation_reset: {
    art: () => <AdaptationReset what="clutch pressures" />,
    caption: "Learned clutch pressures go back to factory values. Shifts relearn over a few drives.",
  },
  actuator_test: { art: ActuatorTest, caption: "The module switches one part on and off so you can see or hear it work." },
  coding: { art: Coding, caption: "A setting inside the module changes. The old settings are saved to the Time Machine first." },
  key_programming: {
    art: KeyProgramming,
    caption: "The car access system keeps a list of keys allowed to start the car. A new key goes into a free slot.",
  },
  flash: { art: Flash, caption: "The module's software is replaced. An interruption can leave the module unable to start." },
  clear_codes: { art: ClearCodes, caption: "Stored fault codes are erased. A copy stays in the Time Machine and the Activity log." },
  snapshot_restore: {
    art: Restore,
    caption: "Each setting that differs is written back, then read back to check it matches.",
  },
};

export function hasPreview(id: string) {
  return id in PREVIEWS;
}

export default function FunctionPreview({ id }: { id: string }) {
  const p = PREVIEWS[id];
  if (!p) return null;
  const Art = p.art;
  return (
    <figure className="overflow-hidden rounded-xl border border-white/[0.07] bg-black/25">
      <div className="px-4 pt-3">
        <Art />
      </div>
      <figcaption className="border-t border-white/[0.05] px-4 py-2.5 text-xs leading-relaxed text-muted-foreground">
        <span className="mr-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-cyan-300/80">What happens</span>
        {p.caption}
      </figcaption>
    </figure>
  );
}
