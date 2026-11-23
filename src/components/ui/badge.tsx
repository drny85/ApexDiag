import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-medium tracking-wide whitespace-nowrap ring-1 ring-inset [&_svg]:size-3",
  {
    variants: {
      variant: {
        default: "bg-white/5 text-foreground/80 ring-white/10",
        ok: "bg-emerald-400/10 text-emerald-300 ring-emerald-400/25",
        warn: "bg-amber-400/10 text-amber-300 ring-amber-400/25",
        danger: "bg-rose-500/10 text-rose-300 ring-rose-400/25",
        info: "bg-sky-400/10 text-sky-300 ring-sky-400/25",
        muted: "bg-transparent text-muted-foreground ring-white/10",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export function Badge({ className, variant, ...props }: React.ComponentProps<"span"> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}
