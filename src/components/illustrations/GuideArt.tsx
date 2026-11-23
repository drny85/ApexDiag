/**
 * Step illustrations for the fix guides: line drawings with a little
 * motion that shows the action (which way to twist, what to pull, where
 * the smoke comes out). Generic parts, no maker's specific design.
 */
import type { CSSProperties, ReactNode } from "react";
import type { GuideArt as ArtId } from "@/lib/guides";
import { cn } from "@/lib/utils";

const C = {
  line: "rgba(255,255,255,0.55)",
  soft: "rgba(255,255,255,0.28)",
  dim: "rgba(255,255,255,0.12)",
  fill: "rgba(255,255,255,0.04)",
  cyan: "#67e8f9",
  amber: "#fbbf24",
  green: "#34d399",
  rose: "#fb7185",
  violet: "#a78bfa",
  glass: "rgba(186,230,253,0.16)",
};

const anim = (name: string, dur = 3, extra: Record<string, string> = {}): CSSProperties =>
  ({ animation: `${name} ${dur}s ease-in-out infinite`, ...extra }) as CSSProperties;

function Label({ x, y, children, color = C.soft, anchor = "middle" }: { x: number; y: number; children: ReactNode; color?: string; anchor?: "start" | "middle" | "end" }) {
  return (
    <text x={x} y={y} textAnchor={anchor} fontSize="8.5" fill={color} fontWeight={500}>
      {children}
    </text>
  );
}

/** Curved arrow showing an anticlockwise turn round (cx, cy). */
function TurnArrow({ cx, cy, r, color = C.cyan, cw = false }: { cx: number; cy: number; r: number; color?: string; cw?: boolean }) {
  const a0 = cw ? -150 : -30;
  const a1 = cw ? -60 : -120;
  const p = (a: number) => [cx + r * Math.cos((a * Math.PI) / 180), cy + r * Math.sin((a * Math.PI) / 180)] as const;
  const [x0, y0] = p(a0);
  const [x1, y1] = p(a1);
  const head = cw ? `M${x1} ${y1} l 7 -1 l -3 6` : `M${x1} ${y1} l -7 -1 l 3 6`;
  return (
    <g stroke={color} strokeWidth="2" fill="none">
      <path d={`M${x0} ${y0} A ${r} ${r} 0 0 ${cw ? 1 : 0} ${x1} ${y1}`} />
      <path d={head} />
    </g>
  );
}

/** A gloved hand, fingers pointing left, wrist on the right. */
function Hand({ x, y, s = 1, style }: { x: number; y: number; s?: number; style?: CSSProperties }) {
  return (
    <g transform={`translate(${x} ${y}) scale(${s})`} style={style}>
      <path
        d="M0 6 C 0 2, 4 0, 8 0 L 30 0 C 34 0, 36 2, 38 4 L 56 4 L 56 26 L 38 26 C 34 30, 28 30, 24 28 L 8 28 C 4 28, 2 26, 2 23 C 2 20, 5 18, 8 18 L 6 18 C 2 18, 0 15, 0 12 Z"
        fill="#1e293b"
        stroke={C.line}
        strokeWidth="1.4"
      />
      <path d="M8 9 H 30 M8 18 H 28" stroke={C.soft} />
      <rect x="56" y="2" width="12" height="26" rx="3" fill="#0f172a" stroke={C.soft} />
    </g>
  );
}

/** Halogen capsule bulb (H7-style) seen side-on, glass to the left. */
function HalogenBulb({ x, y, glow = false, smudge = false }: { x: number; y: number; glow?: boolean; smudge?: boolean }) {
  return (
    <g transform={`translate(${x} ${y})`}>
      {glow && <ellipse cx="18" cy="12" rx="26" ry="18" fill={C.amber} opacity="0.25" style={anim("g-glow", 2)} />}
      <path d="M0 12 C 0 4, 6 2, 12 2 L 34 2 L 34 22 L 12 22 C 6 22, 0 20, 0 12 Z" fill={C.glass} stroke={C.line} strokeWidth="1.4" />
      <path d="M10 12 H 28" stroke={glow ? C.amber : C.soft} strokeWidth="2" />
      <rect x="34" y="-2" width="6" height="28" rx="1.5" fill="#334155" stroke={C.line} />
      <rect x="40" y="4" width="16" height="16" rx="2" fill="#1e293b" stroke={C.line} />
      <path d="M56 8 h6 M56 16 h6" stroke={C.line} strokeWidth="2" />
      {smudge && (
        <g>
          <ellipse cx="14" cy="9" rx="5" ry="3.5" fill="rgba(251,191,36,0.35)" />
          <path d="M8 2 l 12 18 M20 2 L 8 20" stroke={C.rose} strokeWidth="2.4" />
        </g>
      )}
    </g>
  );
}

/** The back of a headlamp housing: a round dust cover in the middle. */
function LampBack({ children }: { children?: ReactNode }) {
  return (
    <g>
      <path d="M20 30 C 60 14, 180 14, 220 32 L 214 120 C 170 136, 70 136, 26 120 Z" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <path d="M40 40 C 80 28, 160 28, 200 42" stroke={C.dim} />
      {children}
    </g>
  );
}

function Wrap({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <svg viewBox="0 0 240 150" className={cn("ill", className)} fill="none" strokeLinecap="round" strokeLinejoin="round">
      {children}
    </svg>
  );
}

/* ---------- scenes ---------- */

const SCENES: Record<ArtId, () => ReactNode> = {
  lamp_types: () => (
    <g>
      {[
        { x: 40, label: "Halogen", sub: "warm, yellowish", color: "#fcd34d" },
        { x: 120, label: "Xenon (HID)", sub: "white-blue, warms up", color: "#bfdbfe" },
        { x: 200, label: "LED", sub: "instant, crisp", color: "#f8fafc" },
      ].map((l, i) => (
        <g key={l.label}>
          <circle cx={l.x} cy="62" r="30" fill={C.fill} stroke={C.soft} />
          <circle cx={l.x} cy="62" r="22" fill={l.color} opacity="0.2" style={anim("g-glow", 2.4, { animationDelay: `${i * 0.4}s` })} />
          {i === 0 && <path d={`M${l.x - 10} 62 h20`} stroke={l.color} strokeWidth="3" />}
          {i === 1 && <path d={`M${l.x - 8} 62 q 8 -8 16 0`} stroke={l.color} strokeWidth="2.5" />}
          {i === 2 && [-9, -3, 3, 9].map((d) => <rect key={d} x={l.x + d - 2} y="58" width="4" height="8" rx="1" fill={l.color} />)}
          <Label x={l.x} y={112} color={C.line}>{l.label}</Label>
          <Label x={l.x} y={124}>{l.sub}</Label>
        </g>
      ))}
    </g>
  ),
  bulb_codes: () => (
    <g>
      <HalogenBulb x={40} y={50} />
      <circle cx="122" cy="62" r="30" stroke={C.cyan} strokeWidth="2" fill="rgba(103,232,249,0.06)" />
      <path d="M144 84 L 164 104" stroke={C.cyan} strokeWidth="5" />
      <text x="122" y="66" textAnchor="middle" fontSize="13" fontWeight={700} fill={C.cyan} fontFamily="ui-monospace, Menlo, monospace">
        H7
      </text>
      <rect x="168" y="36" width="54" height="72" rx="4" fill={C.fill} stroke={C.soft} />
      <path d="M176 50 h38 M176 60 h30 M176 70 h34 M176 80 h24 M176 90 h32" stroke={C.dim} strokeWidth="2" />
      <Label x={195} y={124}>owner's manual</Label>
      <Label x={70} y={124}>code on the base</Label>
    </g>
  ),
  power_off: () => (
    <g>
      <circle cx="70" cy="70" r="34" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <path d="M70 52 v 16" stroke={C.green} strokeWidth="3" />
      <path d="M58 58 a 17 17 0 1 0 24 0" stroke={C.green} strokeWidth="3" />
      <Label x={70} y={124}>ignition off</Label>
      <rect x="138" y="42" width="64" height="56" rx="10" fill={C.fill} stroke={C.soft} />
      <path d="M154 70 h32" stroke={C.soft} strokeWidth="2" />
      <circle cx="160" cy="70" r="6" stroke={C.green} strokeWidth="2" fill="rgba(52,211,153,0.15)" />
      <Label x={170} y={124}>lights switch off</Label>
      <g style={anim("g-pull", 3.2, { "--dx": "26px", "--dy": "-8px" })}>
        <rect x="96" y="100" width="22" height="12" rx="4" fill="#1e293b" stroke={C.line} />
      </g>
    </g>
  ),
  bonnet_open: () => (
    <g>
      <path d="M20 118 L 40 70 L 200 70 L 220 118 Z" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <g style={anim("g-twist", 4, { "--turn": "-14deg" })} className="o-bottom">
        <path d="M44 66 L 60 20 L 180 20 L 196 66 Z" fill="rgba(103,232,249,0.05)" stroke={C.cyan} strokeWidth="1.4" />
      </g>
      <rect x="52" y="84" width="56" height="26" rx="5" stroke={C.soft} />
      <path d="M64 92 h32 M64 100 h32" stroke={C.dim} />
      <circle cx="160" cy="96" r="14" stroke={C.amber} strokeWidth="2" />
      <circle cx="160" cy="96" r="4" fill={C.amber} />
      <Label x={160} y={134} color={C.amber}>back of the lamp</Label>
      <Label x={80} y={134}>engine</Label>
    </g>
  ),
  access_cover: () => (
    <LampBack>
      <circle cx="120" cy="76" r="34" stroke={C.dim} strokeWidth="6" />
      <g style={anim("g-twist", 3.4, { "--turn": "-90deg" })}>
        <circle cx="120" cy="76" r="28" fill="#111827" stroke={C.cyan} strokeWidth="2" />
        <path d="M100 76 h 40" stroke={C.cyan} strokeWidth="5" />
      </g>
      <TurnArrow cx={120} cy={76} r={44} />
      <Label x={120} y={140} color={C.cyan}>a quarter turn anticlockwise</Label>
    </LampBack>
  ),
  wheel_liner: () => (
    <g>
      <path d="M20 40 L 120 40 C 150 40, 170 60, 176 90 L 180 120 L 20 120 Z" fill={C.fill} stroke={C.soft} />
      <circle cx="120" cy="114" r="34" fill="#0b0f14" stroke={C.line} strokeWidth="3" />
      <circle cx="120" cy="114" r="16" stroke={C.soft} />
      <path d="M78 104 A 46 46 0 0 1 166 100" stroke={C.dim} strokeWidth="8" />
      <g style={anim("g-twist", 3.6, { "--turn": "26deg" })} className="o-left">
        <path d="M150 74 C 162 80, 170 92, 172 106" stroke={C.cyan} strokeWidth="5" />
      </g>
      <rect x="182" y="60" width="26" height="16" rx="4" fill="#c46f12" opacity="0.8" />
      <Label x={196} y={92} color={C.amber}>indicator</Label>
      <Label x={120} y={20} color={C.cyan}>steering on full lock, liner folded back</Label>
    </g>
  ),
  connector: () => (
    <g>
      <HalogenBulb x={30} y={58} />
      <g style={anim("g-pull", 3, { "--dx": "38px" })}>
        <rect x="96" y="56" width="40" height="36" rx="5" fill="#1e293b" stroke={C.cyan} strokeWidth="1.6" />
        <path d="M108 56 v -8 h 12 v 8" stroke={C.cyan} strokeWidth="1.6" />
        <path d="M136 66 C 160 66, 170 50, 210 50 M136 82 C 160 82, 170 96, 210 96" stroke={C.soft} strokeWidth="2.4" />
      </g>
      <path d="M114 36 v 8" stroke={C.amber} strokeWidth="2" />
      <path d="M110 40 l 4 4 l 4 -4" stroke={C.amber} strokeWidth="2" />
      <Label x={114} y={28} color={C.amber}>press tab</Label>
      <Label x={150} y={132}>pull the plug, not the wires</Label>
    </g>
  ),
  spring_clip: () => (
    <LampBack>
      <circle cx="120" cy="76" r="30" stroke={C.soft} strokeWidth="1.4" fill="#0b0f14" />
      <rect x="110" y="66" width="20" height="20" rx="3" fill="#334155" stroke={C.line} />
      <g style={anim("g-twist", 3.2, { "--turn": "-50deg" })} className="o-left">
        <path d="M92 50 L 148 50 L 148 102 L 92 102" stroke={C.cyan} strokeWidth="2.4" />
      </g>
      <path d="M152 60 l 10 -6 M152 60 l 4 8" stroke={C.amber} strokeWidth="2" />
      <Label x={120} y={140} color={C.cyan}>push in and sideways, swing the clip open</Label>
    </LampBack>
  ),
  twist_socket: () => (
    <g>
      <rect x="20" y="30" width="200" height="90" rx="12" fill={C.fill} stroke={C.soft} />
      <circle cx="120" cy="75" r="30" fill="#0b0f14" stroke={C.dim} strokeWidth="5" />
      <g style={anim("g-twist", 3.2, { "--turn": "-90deg" })}>
        <circle cx="120" cy="75" r="22" fill="#1e293b" stroke={C.cyan} strokeWidth="2" />
        <rect x="100" y="70" width="40" height="10" rx="3" fill="#334155" stroke={C.cyan} />
        <path d="M120 53 v-8 M120 97 v8" stroke={C.cyan} strokeWidth="3" />
      </g>
      <TurnArrow cx={120} cy={75} r={40} />
      <Label x={120} y={140} color={C.cyan}>twist anticlockwise, then pull out</Label>
    </g>
  ),
  no_touch: () => (
    <g>
      <HalogenBulb x={70} y={48} />
      <Hand x={122} y={44} s={0.9} />
      <path d="M60 32 L 96 32" stroke={C.green} strokeWidth="1.6" strokeDasharray="3 3" />
      <Label x={80} y={26} color={C.green}>hold the base only</Label>
      <g transform="translate(24 96)">
        <HalogenBulb x={0} y={0} smudge />
      </g>
      <Label x={150} y={112} color={C.rose}>skin oil on the glass</Label>
      <Label x={150} y={124} color={C.rose}>= hot spot, early failure</Label>
    </g>
  ),
  wedge_bulb: () => (
    <g>
      <rect x="40" y="56" width="50" height="40" rx="6" fill="#1e293b" stroke={C.soft} strokeWidth="1.4" />
      <path d="M90 66 H 104 M90 86 H 104" stroke={C.soft} />
      <g style={anim("g-pull", 3, { "--dx": "-34px" })}>
        <g transform="translate(-36 0)">
          <path d="M132 60 C 160 56, 178 64, 178 76 C 178 88, 160 96, 132 92 Z" fill={C.glass} stroke={C.line} strokeWidth="1.4" />
          <rect x="120" y="62" width="14" height="28" rx="2" fill="#475569" stroke={C.line} />
          <path d="M138 72 h 26 M138 80 h 26" stroke={C.amber} strokeWidth="1.4" />
        </g>
      </g>
      <Label x={120} y={126} color={C.cyan}>wedge bulbs pull straight out</Label>
      <Label x={120} y={138}>bayonet bulbs: push in, twist, pull</Label>
    </g>
  ),
  festoon_bulb: () => (
    <g>
      <rect x="30" y="50" width="180" height="50" rx="10" fill={C.fill} stroke={C.soft} />
      <path d="M62 60 v 30 M178 60 v 30" stroke={C.soft} strokeWidth="3" />
      <g style={anim("g-pull", 3, { "--dy": "-16px" })}>
        <rect x="70" y="68" width="100" height="14" rx="7" fill={C.glass} stroke={C.line} />
        <rect x="64" y="68" width="10" height="14" rx="2" fill="#94a3b8" />
        <rect x="166" y="68" width="10" height="14" rx="2" fill="#94a3b8" />
        <path d="M80 75 h 80" stroke={C.amber} strokeWidth="1.2" />
      </g>
      <Label x={120} y={126} color={C.cyan}>lever one end out of its spring</Label>
    </g>
  ),
  plate_lens: () => (
    <g>
      <rect x="30" y="70" width="180" height="50" rx="6" fill="#eef1f4" opacity="0.15" stroke={C.soft} />
      <text x="120" y="102" textAnchor="middle" fontSize="18" fontWeight={700} fill="rgba(255,255,255,0.4)" fontFamily="ui-monospace, Menlo, monospace">
        APX 001
      </text>
      <g style={anim("g-pull", 3, { "--dy": "14px" })}>
        <rect x="90" y="40" width="60" height="16" rx="4" fill={C.glass} stroke={C.cyan} strokeWidth="1.6" />
        <circle cx="98" cy="48" r="2.5" fill={C.cyan} />
        <circle cx="142" cy="48" r="2.5" fill={C.cyan} />
      </g>
      <Label x={120} y={28} color={C.cyan}>two small screws, or a clip</Label>
    </g>
  ),
  boot_trim: () => (
    <g>
      <path d="M30 120 L 30 50 C 30 36, 40 30, 54 30 L 186 30 C 200 30, 210 36, 210 50 L 210 120" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <rect x="150" y="54" width="44" height="54" rx="6" fill="rgba(251,113,133,0.12)" stroke={C.rose} />
      <g style={anim("g-twist", 3.4, { "--turn": "-35deg" })} className="o-left">
        <rect x="156" y="62" width="32" height="38" rx="4" fill="#1e293b" stroke={C.cyan} strokeWidth="1.6" />
      </g>
      <path d="M60 90 h 60" stroke={C.dim} strokeWidth="10" />
      <Label x={172} y={126} color={C.cyan}>trim flap behind the lamp</Label>
      <Label x={90} y={126}>boot floor</Label>
    </g>
  ),
  helper_check: () => (
    <g>
      <rect x="18" y="60" width="120" height="56" rx="12" fill={C.fill} stroke={C.soft} />
      <rect x="22" y="70" width="22" height="14" rx="4" fill={C.rose} style={anim("g-blink", 1.6)} />
      <rect x="112" y="70" width="22" height="14" rx="4" fill={C.rose} style={anim("g-blink", 1.6)} />
      <rect x="64" y="46" width="28" height="6" rx="3" fill={C.rose} style={anim("g-blink", 1.6)} />
      <rect x="50" y="94" width="56" height="14" rx="3" fill="rgba(255,255,255,0.12)" />
      <circle cx="194" cy="52" r="12" stroke={C.line} />
      <path d="M194 64 v 30 M194 72 l -14 12 M194 72 l 14 -8 M194 94 l -10 24 M194 94 l 10 24" stroke={C.line} strokeWidth="2" />
      <path d="M210 60 l 8 -6 M212 66 l 10 -2" stroke={C.green} strokeWidth="2" />
      <Label x={78} y={134}>brake, tail, indicator, reverse</Label>
      <Label x={196} y={134} color={C.green}>helper watches</Label>
    </g>
  ),
  beam_aim: () => (
    <g>
      <rect x="128" y="18" width="98" height="110" rx="4" fill={C.fill} stroke={C.soft} />
      <path d="M134 74 H 176 L 186 64 H 220" stroke={C.amber} strokeWidth="2" />
      <path d="M134 74 H 176 L 186 64 H 220 V 110 H 134 Z" fill={C.amber} opacity="0.14" style={anim("g-glow", 2.6)} />
      <path d="M30 94 L 128 60 M30 94 L 128 116" stroke={C.amber} strokeWidth="1" opacity="0.5" strokeDasharray="4 4" />
      <rect x="14" y="80" width="26" height="22" rx="5" fill="#1e293b" stroke={C.line} />
      <circle cx="30" cy="91" r="6" fill="#fde68a" />
      <Label x={177} y={140} color={C.amber}>level cut-off, same both sides</Label>
    </g>
  ),
  xenon_hv: () => (
    <g>
      <path d="M40 40 h 80 v 70 h -80 Z" fill={C.fill} stroke={C.soft} />
      <circle cx="80" cy="75" r="20" stroke={C.soft} />
      <path d="M80 60 L 72 78 H 84 L 76 94" stroke={C.amber} strokeWidth="2.6" style={anim("g-glow", 1.2)} />
      <path d="M150 30 L 210 130 H 90 Z" transform="translate(30 0) scale(0.8)" fill="rgba(251,191,36,0.08)" stroke={C.amber} strokeWidth="2" />
      <path d="M181 62 L 174 82 H 186 L 179 102" stroke={C.amber} strokeWidth="3" />
      <Label x={80} y={128}>igniter on the bulb</Label>
      <Label x={180} y={130} color={C.amber}>high voltage</Label>
    </g>
  ),
  led_unit: () => (
    <g>
      <path d="M20 40 C 70 24, 170 24, 220 44 L 214 110 C 170 124, 70 124, 26 110 Z" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <path d="M44 56 C 90 46, 150 46, 196 58" stroke="#f8fafc" strokeWidth="4" style={anim("g-glow", 2)} />
      {[70, 92, 114, 136, 158].map((x) => (
        <rect key={x} x={x} y="72" width="12" height="18" rx="2" fill="#e2e8f0" opacity="0.7" />
      ))}
      <rect x="92" y="98" width="56" height="16" rx="3" fill="#1e293b" stroke={C.cyan} />
      <Label x={120} y={109} color={C.cyan}>driver</Label>
      <Label x={120} y={136}>sealed unit: no bulb to change</Label>
    </g>
  ),
  maf_unplug: () => (
    <g>
      <path d="M10 70 H 230 M10 110 H 230" stroke={C.soft} strokeWidth="2" />
      <path d="M10 90 H 230" stroke={C.cyan} strokeWidth="1" strokeDasharray="6 6" style={{ animation: "ill-flow 0.8s linear infinite" }} opacity="0.6" />
      <rect x="96" y="44" width="48" height="30" rx="4" fill="#1e293b" stroke={C.line} />
      <circle cx="104" cy="58" r="3" stroke={C.amber} />
      <circle cx="136" cy="58" r="3" stroke={C.amber} />
      <g style={anim("g-pull", 3, { "--dy": "-18px" })}>
        <rect x="108" y="22" width="24" height="18" rx="3" fill="#334155" stroke={C.cyan} />
        <path d="M120 22 V 8" stroke={C.soft} strokeWidth="2" />
      </g>
      <Label x={120} y={134}>air filter box → sensor → throttle</Label>
      <Label x={180} y={58} color={C.amber}>2 screws</Label>
    </g>
  ),
  maf_spray: () => (
    <g>
      <rect x="120" y="40" width="70" height="30" rx="4" fill="#1e293b" stroke={C.line} />
      <rect x="146" y="70" width="18" height="40" rx="3" fill="#0b0f14" stroke={C.soft} />
      <path d="M152 80 v 22 M158 80 v 22" stroke={C.amber} strokeWidth="1.2" />
      <rect x="20" y="70" width="26" height="54" rx="6" fill="#1e293b" stroke={C.line} />
      <rect x="28" y="60" width="10" height="10" rx="2" fill="#334155" />
      <path d="M38 64 L 60 70" stroke={C.soft} strokeWidth="2" />
      {[0, 1, 2, 3].map((i) => (
        <circle key={i} cx="64" cy="72" r="4" fill={C.cyan} opacity="0.6" style={anim("g-mist", 1.4, { "--dx": "80px", "--dy": `${10 + i * 6}px`, animationDelay: `${i * 0.3}s` })} />
      ))}
      <path d="M190 92 l 20 0 M200 82 l 0 20" stroke={C.rose} strokeWidth="2.4" transform="rotate(45 200 92)" />
      <Label x={196} y={124} color={C.rose}>never touch</Label>
      <Label x={70} y={140}>sensor cleaner only, then air-dry</Label>
    </g>
  ),
  smoke_test: () => (
    <g>
      <rect x="14" y="84" width="46" height="36" rx="6" fill="#1e293b" stroke={C.line} />
      <Label x={37} y={136}>smoke</Label>
      <path d="M60 100 C 90 100, 90 70, 120 70 H 220" stroke={C.soft} strokeWidth="10" />
      <path d="M60 100 C 90 100, 90 70, 120 70 H 220" stroke="#0b0f14" strokeWidth="6" />
      <circle cx="170" cy="66" r="3" fill={C.rose} />
      {[0, 1, 2, 3, 4].map((i) => (
        <circle key={i} cx="170" cy="60" r="6" fill="rgba(226,232,240,0.5)" style={anim("g-mist", 2.2, { "--dx": `${-10 + i * 5}px`, "--dy": "-44px", animationDelay: `${i * 0.4}s` })} />
      ))}
      <Label x={176} y={96} color={C.rose}>the leak</Label>
    </g>
  ),
  coil_swap: () => (
    <g>
      <rect x="20" y="96" width="200" height="30" rx="6" fill={C.fill} stroke={C.soft} />
      {[50, 95, 140, 185].map((x, i) => (
        <g key={x}>
          <circle cx={x} cy="110" r="8" stroke={C.dim} />
          <Label x={x} y={140}>{`${i + 1}`}</Label>
        </g>
      ))}
      <g style={anim("g-pull", 3.4, { "--dx": "45px" })}>
        <rect x="88" y="48" width="14" height="44" rx="3" fill="rgba(251,113,133,0.25)" stroke={C.rose} strokeWidth="1.6" />
        <rect x="84" y="40" width="22" height="12" rx="3" fill="#1e293b" stroke={C.rose} />
      </g>
      <g style={anim("g-pull", 3.4, { "--dx": "-45px" })}>
        <rect x="133" y="48" width="14" height="44" rx="3" fill="#1e293b" stroke={C.soft} />
        <rect x="129" y="40" width="22" height="12" rx="3" fill="#1e293b" stroke={C.soft} />
      </g>
      <Label x={120} y={24} color={C.cyan}>misfire moves with the coil → replace the coil</Label>
    </g>
  ),
  plug_gap: () => (
    <g>
      <rect x="30" y="66" width="70" height="20" rx="3" fill="#e2e8f0" opacity="0.25" stroke={C.line} />
      <rect x="100" y="62" width="30" height="28" rx="2" fill="#475569" stroke={C.line} />
      <path d="M130 72 h 30 M130 80 h 30" stroke={C.soft} strokeWidth="2" />
      <path d="M160 70 v 12 h 8" stroke={C.line} strokeWidth="2.4" />
      <rect x="160" y="82" width="10" height="2" fill={C.cyan} style={anim("g-glow", 1.6)} />
      <path d="M176 76 h 30" stroke={C.cyan} strokeDasharray="3 3" />
      <Label x={196} y={68} color={C.cyan}>gap</Label>
      <Label x={120} y={124}>check the gap; start each plug by hand</Label>
    </g>
  ),
  plug_torque: () => (
    <g>
      <rect x="104" y="40" width="20" height="70" rx="4" fill="#334155" stroke={C.line} />
      <g style={anim("g-twist", 3, { "--turn": "-18deg" })} className="o-left">
        <rect x="114" y="34" width="110" height="12" rx="6" fill="#1e293b" stroke={C.cyan} strokeWidth="1.6" />
        <circle cx="204" cy="40" r="10" fill="#0b0f14" stroke={C.cyan} />
      </g>
      <Label x={168} y={80} color={C.cyan}>torque to the</Label>
      <Label x={168} y={92} color={C.cyan}>maker's figure</Label>
      <Label x={114} y={130}>engine cold</Label>
    </g>
  ),
  battery_neg: () => (
    <g>
      <rect x="40" y="56" width="160" height="70" rx="8" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <rect x="64" y="44" width="22" height="12" rx="2" fill="#334155" stroke={C.line} />
      <rect x="154" y="44" width="22" height="12" rx="2" fill="#334155" stroke={C.line} />
      <Label x={75} y={80} color={C.cyan}>−</Label>
      <Label x={165} y={80} color={C.rose}>+</Label>
      <g style={anim("g-pull", 3, { "--dy": "-20px", "--dx": "-10px" })}>
        <rect x="60" y="36" width="30" height="12" rx="3" fill="#0f172a" stroke={C.cyan} strokeWidth="1.6" />
        <path d="M60 42 C 40 42, 30 30, 16 30" stroke={C.cyan} strokeWidth="3" />
      </g>
      <path d="M150 42 h 30" stroke={C.rose} strokeWidth="3" />
      <Label x={75} y={104} color={C.cyan}>1. negative off first</Label>
      <Label x={165} y={104} color={C.rose}>2. then positive</Label>
      <Label x={120} y={142}>refit: positive first, negative last</Label>
    </g>
  ),
  battery_label: () => (
    <g>
      <rect x="24" y="50" width="130" height="76" rx="8" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <rect x="40" y="40" width="18" height="10" rx="2" fill="#334155" stroke={C.line} />
      <rect x="120" y="40" width="18" height="10" rx="2" fill="#334155" stroke={C.line} />
      <rect x="44" y="68" width="90" height="40" rx="4" fill="rgba(103,232,249,0.06)" stroke={C.dim} />
      <circle cx="172" cy="72" r="40" stroke={C.cyan} strokeWidth="2" fill="#0b0f14" />
      <path d="M200 100 L 222 124" stroke={C.cyan} strokeWidth="5" />
      <text x="172" y="62" textAnchor="middle" fontSize="11" fontWeight={700} fill={C.cyan} fontFamily="ui-monospace, Menlo, monospace">AGM</text>
      <text x="172" y="78" textAnchor="middle" fontSize="10" fill={C.line} fontFamily="ui-monospace, Menlo, monospace">80 Ah</text>
      <text x="172" y="93" textAnchor="middle" fontSize="10" fill={C.line} fontFamily="ui-monospace, Menlo, monospace">800 A</text>
      <Label x={89} y={142}>type, capacity, cold cranking amps</Label>
    </g>
  ),
  terminal_brush: () => (
    <g>
      <rect x="40" y="70" width="160" height="60" rx="8" fill={C.fill} stroke={C.soft} />
      <rect x="100" y="56" width="26" height="16" rx="3" fill="#334155" stroke={C.line} />
      {[0, 1, 2, 3].map((i) => (
        <circle key={i} cx={104 + i * 6} cy={56 - (i % 2) * 3} r="3" fill="rgba(110,231,183,0.6)" />
      ))}
      <g style={anim("g-scrub", 0.6)}>
        <rect x="96" y="20" width="34" height="14" rx="4" fill="#1e293b" stroke={C.cyan} strokeWidth="1.6" />
        <path d="M100 34 v 10 M106 34 v 12 M112 34 v 10 M118 34 v 12 M124 34 v 10" stroke={C.cyan} />
        <path d="M130 27 H 170" stroke={C.cyan} strokeWidth="3" />
      </g>
      <Label x={120} y={146}>bright metal on post and clamp</Label>
    </g>
  ),
  battery_lift: () => (
    <g>
      <rect x="30" y="112" width="180" height="16" rx="3" fill={C.fill} stroke={C.soft} />
      <g style={anim("g-pull", 3.6, { "--dy": "-26px" })}>
        <rect x="70" y="56" width="100" height="56" rx="6" fill="#1e293b" stroke={C.line} strokeWidth="1.4" />
        <rect x="84" y="48" width="14" height="8" fill="#334155" />
        <rect x="142" y="48" width="14" height="8" fill="#334155" />
        <rect x="90" y="74" width="60" height="16" rx="3" fill="rgba(103,232,249,0.1)" stroke={C.cyan} />
        <text x="120" y="85" textAnchor="middle" fontSize="8" fill={C.cyan} fontFamily="ui-monospace, Menlo, monospace">AGM 80Ah</text>
      </g>
      <path d="M60 112 v -12 h 16" stroke={C.amber} strokeWidth="2.4" />
      <Label x={46} y={96} color={C.amber}>hold-down</Label>
      <Label x={120} y={144}>keep it upright, lift with your legs</Label>
    </g>
  ),
  battery_register: () => (
    <g>
      <rect x="20" y="36" width="96" height="72" rx="10" fill={C.fill} stroke={C.soft} />
      <path d="M36 60 h 64 M36 72 h 48 M36 84 h 56" stroke={C.dim} strokeWidth="3" />
      <circle cx="98" cy="92" r="10" fill="rgba(52,211,153,0.15)" stroke={C.green} strokeWidth="2" />
      <path d="M93 92 l 4 4 l 7 -8" stroke={C.green} strokeWidth="2" />
      <path d="M116 72 H 150" stroke={C.cyan} strokeWidth="2" strokeDasharray="5 5" style={{ animation: "ill-flow 0.8s linear infinite" }} />
      <rect x="150" y="50" width="70" height="44" rx="6" fill="#1e293b" stroke={C.line} />
      <Label x={185} y={76} color={C.line}>car</Label>
      <Label x={68} y={128}>scan tool tells the car</Label>
      <Label x={185} y={128}>new part fitted</Label>
    </g>
  ),
  fuel_cap: () => (
    <g>
      <circle cx="84" cy="76" r="44" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <g style={anim("g-twist", 3, { "--turn": "60deg" })}>
        <circle cx="84" cy="76" r="30" fill="#1e293b" stroke={C.cyan} strokeWidth="2" />
        <rect x="64" y="70" width="40" height="12" rx="4" fill="#334155" stroke={C.cyan} />
      </g>
      <circle cx="84" cy="76" r="36" stroke={C.green} strokeDasharray="4 4" />
      <TurnArrow cx={84} cy={76} r={50} cw color={C.green} />
      <Label x={84} y={140} color={C.green}>clockwise until it clicks</Label>
      <rect x="160" y="44" width="58" height="40" rx="8" fill={C.fill} stroke={C.soft} />
      <path d="M172 72 l 8 -16 l 8 16" stroke={C.line} />
      <path d="M198 64 l 10 0 l -4 -4 M208 64 l -4 4" stroke={C.amber} strokeWidth="2" />
      <Label x={189} y={100} color={C.amber}>arrow = filler side</Label>
    </g>
  ),
  clear_codes: () => (
    <g>
      <rect x="40" y="24" width="160" height="104" rx="12" fill={C.fill} stroke={C.soft} strokeWidth="1.4" />
      <rect x="54" y="40" width="132" height="22" rx="5" fill="rgba(52,211,153,0.08)" stroke={C.green} />
      <path d="M64 51 l 4 4 l 8 -8" stroke={C.green} strokeWidth="2" />
      <Label x={128} y={54} color={C.green}>re-test passes</Label>
      <path d="M58 104 L 82 90 L 104 96 L 128 80 L 150 84 L 176 76" stroke={C.cyan} strokeWidth="2" strokeDasharray="200" style={{ animation: "g-draw 3s ease-out infinite", ["--len" as string]: 200 } as CSSProperties} />
      <Label x={120} y={146}>let the app measure it</Label>
    </g>
  ),
};

export default function GuideArtView({ art, className }: { art: ArtId; className?: string }) {
  const Scene = SCENES[art];
  return <Wrap className={className}>{Scene()}</Wrap>;
}
