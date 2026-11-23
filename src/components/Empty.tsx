import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "@/lib/i18n";

export default function Empty({ icon: Icon, title, children }: { icon: LucideIcon; title: string; children?: ReactNode }) {
  const t = useT();
  return (
    <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-white/10 px-6 py-16 text-center">
      <div className="mb-4 grid size-12 place-items-center rounded-xl bg-white/5 ring-1 ring-white/10">
        <Icon className="size-5 text-muted-foreground" />
      </div>
      <h3 className="font-medium">{t(title)}</h3>
      {children && <div className="mt-1 max-w-sm text-sm text-muted-foreground">{children}</div>}
    </div>
  );
}
