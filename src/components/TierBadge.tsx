import { TIER_STYLE } from "@/lib/format";
import type { TierId } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export default function TierBadge({ tier, label, className }: { tier: TierId; label: string; className?: string }) {
  const s = TIER_STYLE[tier];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] font-medium ring-1 ring-inset",
        s.bg,
        s.text,
        s.ring,
        className,
      )}
    >
      <span className={cn("size-1.5 rounded-full", s.dot)} />
      {label}
    </span>
  );
}
