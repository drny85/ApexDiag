import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

export function Checkbox({ checked, disabled, onChange, className }: { checked: boolean; disabled?: boolean; onChange: (v: boolean) => void; className?: string }) {
  return (
    <span
      role="checkbox"
      aria-checked={checked}
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (!disabled) onChange(!checked);
      }}
      onKeyDown={(e) => {
        if (!disabled && (e.key === " " || e.key === "Enter")) {
          e.preventDefault();
          onChange(!checked);
        }
      }}
      className={cn(
        "grid size-[18px] shrink-0 place-items-center rounded-[5px] border transition",
        checked ? "border-cyan-300 bg-cyan-300 text-slate-950" : "border-white/25 bg-white/[0.03]",
        className,
      )}
    >
      {checked && <Check className="size-3.5" strokeWidth={3} />}
    </span>
  );
}
