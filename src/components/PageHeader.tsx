import type { ReactNode } from "react";
import { useT } from "@/lib/i18n";

/** Title and subtitle are English source text, translated here. */
export default function PageHeader({ title, subtitle, actions }: { title: string; subtitle: string; actions?: ReactNode }) {
  const t = useT();
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-[26px] font-semibold tracking-tight">{t(title)}</h1>
        <p className="mt-1 max-w-2xl text-sm text-muted-foreground">{t(subtitle)}</p>
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </header>
  );
}
