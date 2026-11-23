import * as React from "react";
import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import { cn } from "@/lib/utils";

export const TooltipProvider = TooltipPrimitive.Provider;

export function Tooltip({ content, children, className }: { content: React.ReactNode; children: React.ReactNode; className?: string }) {
  // Radix tooltips open on hover only. On a touch screen a tap opens it and a
  // tap anywhere else closes it.
  const [open, setOpen] = React.useState(false);
  return (
    <TooltipPrimitive.Root delayDuration={150} open={open} onOpenChange={setOpen}>
      <TooltipPrimitive.Trigger
        asChild
        onClick={(e) => {
          if (window.matchMedia("(pointer: coarse)").matches) {
            e.preventDefault();
            setOpen(true);
          }
        }}
      >
        {children}
      </TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          sideOffset={6}
          className={cn(
            "z-50 max-w-72 rounded-lg border border-white/10 bg-[oklch(0.2_0.01_265)] px-3 py-2 text-xs leading-relaxed text-foreground/90 shadow-xl animate-in fade-in-0 zoom-in-95",
            className,
          )}
        >
          {content}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}
