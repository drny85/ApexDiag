/**
 * Brand-neutral car drawings: a side profile for the vehicle cards and a
 * top-down body for the module map, each in an SUV and a sedan shape.
 * No maker logos or trade dress: brands are told apart by name and an
 * accent colour only.
 */
import type { Make, Pack } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export type Body = Pack["body"];

export const MAKE_ACCENT: Record<Make, string> = {
  bmw: "#5eb3ff",
  mercedes_benz: "#d6dbe4",
  toyota: "#ff7a7a",
  honda: "#ff9b62",
  generic: "#67e8f9",
};

/** Side profile, facing right. `scanning` sweeps a light across it. */
export function CarSide({
  body,
  accent = "#67e8f9",
  scanning = false,
  className,
}: {
  body: Body;
  accent?: string;
  scanning?: boolean;
  className?: string;
}) {
  const suv = body === "suv";
  // Outline, glass and wheel arches tuned per body style.
  const outline = suv
    ? "M14 92 C 10 92, 8 90, 8 86 L 8 50 C 8 40, 11 31, 18 25 L 32 18 L 174 16 C 182 16, 188 19, 194 25 L 220 49 L 268 54 C 282 56, 288 62, 288 72 L 288 88 C 288 91, 286 92, 283 92 Z"
    : "M10 80 C 10 70, 16 64, 28 62 L 70 56 L 104 32 C 110 28, 118 26, 128 26 L 186 26 C 196 26, 204 30, 212 38 L 232 56 L 268 62 C 280 64, 288 70, 288 80 L 288 88 C 288 92, 285 94, 281 94 L 14 94 C 10 94, 8 92, 8 88 Z";
  const glass = suv
    ? "M20 46 L 21 33 C 23 27, 27 24, 35 23 L 84 22 L 84 46 Z M90 22 L 136 21 L 136 46 L 90 46 Z M142 21 L 172 21 C 178 21, 182 23, 186 28 L 205 46 L 142 46 Z"
    : "M80 57 L 108 36 C 112 33, 118 32, 126 32 L 154 32 L 154 57 Z M160 32 L 184 32 C 192 32, 198 35, 203 40 L 220 57 L 160 57 Z";
  const wheels = suv ? [62, 238] : [64, 236];
  const r = suv ? 18 : 15;
  return (
    <svg viewBox="0 0 296 112" className={cn("ill", className)} fill="none" strokeLinecap="round" strokeLinejoin="round">
      <defs>
        <linearGradient id={`side-${body}`} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={accent} stopOpacity="0.22" />
          <stop offset="100%" stopColor={accent} stopOpacity="0.02" />
        </linearGradient>
        <clipPath id={`side-clip-${body}`}>
          <path d={outline} />
        </clipPath>
      </defs>
      <ellipse cx="148" cy="104" rx="132" ry="4" fill="#000" opacity="0.35" />
      <path d={outline} fill={`url(#side-${body})`} stroke={accent} strokeOpacity="0.7" strokeWidth="1.5" />
      <path d={glass} fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.25)" />
      {/* belt line and door cut */}
      <path d={suv ? "M12 54 L 272 58" : "M28 66 L 272 66"} stroke="rgba(255,255,255,0.12)" />
      <path d={suv ? "M87 24 L 87 84 M139 22 L 139 84" : "M157 58 L 157 90"} stroke="rgba(255,255,255,0.1)" />
      {/* lights */}
      <path d={suv ? "M274 62 l 12 2" : "M274 68 l 12 2"} stroke={accent} strokeWidth="3" />
      <path d={suv ? "M9 48 l 0 10" : "M12 70 l 8 -1"} stroke="#fb7185" strokeWidth="3" />
      {wheels.map((x) => (
        <g key={x}>
          <circle cx={x} cy={94} r={r + 4} fill="#0b0f14" />
          <circle cx={x} cy={94} r={r} fill="#111821" stroke="rgba(255,255,255,0.35)" strokeWidth="2" />
          <circle cx={x} cy={94} r={r * 0.45} stroke="rgba(255,255,255,0.2)" />
        </g>
      ))}
      {scanning && (
        <g clipPath={`url(#side-clip-${body})`}>
          <rect x="-40" y="0" width="40" height="112" fill={accent} opacity="0.35" style={{ ["--sweep" as string]: "340px", animation: "ill-sweep 1.8s ease-in-out infinite" }} />
        </g>
      )}
    </svg>
  );
}

/** Top-down body (viewBox 240 x 470, front at the top, driver on the left). */
export function TopDownBody({ body }: { body: Body }) {
  const suv = body === "suv";
  const outline = suv
    ? "M64 22 C 90 10, 150 10, 176 22 C 200 32, 212 60, 212 96 L 214 380 C 214 420, 200 452, 170 458 C 140 462, 100 462, 70 458 C 40 452, 26 420, 26 380 L 28 96 C 28 60, 40 32, 64 22 Z"
    : "M70 20 C 94 10, 146 10, 170 20 C 194 30, 206 58, 206 100 L 208 392 C 208 428, 196 452, 168 458 C 140 462, 100 462, 72 458 C 44 452, 32 428, 32 392 L 34 100 C 34 58, 46 30, 70 20 Z";
  const wheels = suv
    ? [
        [20, 88],
        [204, 88],
        [20, 340],
        [204, 340],
      ]
    : [
        [26, 84],
        [198, 84],
        [26, 344],
        [198, 344],
      ];
  return (
    <g>
      <defs>
        <linearGradient id="car-body" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.06" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0.015" />
        </linearGradient>
      </defs>
      {wheels.map(([x, y]) => (
        <rect key={`${x}-${y}`} x={x} y={y} width="16" height="56" rx="6" fill="#0b0f14" stroke="rgba(255,255,255,0.18)" />
      ))}
      <path d={outline} fill="url(#car-body)" stroke="rgba(255,255,255,0.28)" strokeWidth="1.5" />
      {suv ? (
        <>
          <path d="M44 150 C 90 140, 150 140, 196 150" stroke="rgba(255,255,255,0.14)" />
          <path d="M48 164 C 95 152, 145 152, 192 164 L 182 212 C 140 204, 100 204, 58 212 Z" fill="rgba(103,232,249,0.04)" stroke="rgba(255,255,255,0.2)" />
          <path d="M58 212 L 56 380 M182 212 L 184 380" stroke="rgba(255,255,255,0.12)" />
          <path d="M56 380 C 100 388, 140 388, 184 380 L 188 420 C 140 430, 100 430, 52 420 Z" fill="rgba(103,232,249,0.03)" stroke="rgba(255,255,255,0.18)" />
        </>
      ) : (
        <>
          <path d="M48 146 C 92 136, 148 136, 192 146" stroke="rgba(255,255,255,0.14)" />
          <path d="M52 158 C 96 146, 144 146, 188 158 L 178 214 C 140 206, 100 206, 62 214 Z" fill="rgba(103,232,249,0.04)" stroke="rgba(255,255,255,0.2)" />
          <path d="M62 214 L 62 334 M178 214 L 178 334" stroke="rgba(255,255,255,0.12)" />
          <path d="M62 334 C 100 342, 140 342, 178 334 L 186 372 C 140 382, 100 382, 54 372 Z" fill="rgba(103,232,249,0.03)" stroke="rgba(255,255,255,0.18)" />
          {/* boot lid */}
          <path d="M54 386 C 100 394, 140 394, 186 386" stroke="rgba(255,255,255,0.12)" />
        </>
      )}
      <path d={suv ? "M28 176 l -14 4 l 2 12 l 14 -2" : "M34 172 l -14 4 l 2 12 l 14 -2"} stroke="rgba(255,255,255,0.22)" />
      <path d={suv ? "M212 176 l 14 4 l -2 12 l -14 -2" : "M206 172 l 14 4 l -2 12 l -14 -2"} stroke="rgba(255,255,255,0.22)" />
      {[
        [70, 236],
        [140, 236],
      ].map(([x, y]) => (
        <rect key={x} x={x} y={y} width="30" height="38" rx="8" stroke="rgba(255,255,255,0.1)" />
      ))}
      <rect x="66" y="296" width="108" height="34" rx="10" stroke="rgba(255,255,255,0.1)" />
      <circle cx="85" cy="214" r="13" stroke="rgba(255,255,255,0.18)" strokeWidth="3" />
      <path d="M112 240 L 112 320 M128 240 L 128 320" stroke="rgba(255,255,255,0.07)" />
      <text x="120" y="36" textAnchor="middle" fontSize="8" letterSpacing="2" fill="rgba(255,255,255,0.3)">
        FRONT
      </text>
    </g>
  );
}

/** Small brand chip: the maker's name in its accent colour, no logo. */
export function BrandChip({ make, brand, className }: { make: Make; brand: string; className?: string }) {
  const accent = MAKE_ACCENT[make];
  return (
    <span
      className={cn("inline-flex items-center gap-1.5 whitespace-nowrap rounded-md px-2 py-0.5 text-[11px] font-semibold uppercase tracking-[0.14em] ring-1 ring-inset", className)}
      style={{ color: accent, backgroundColor: `${accent}14`, boxShadow: `inset 0 0 0 1px ${accent}40` }}
    >
      <span className="size-1.5 rounded-full" style={{ backgroundColor: accent }} />
      {brand}
    </span>
  );
}
