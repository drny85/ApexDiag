import { useEffect, useState } from "react";
import { CheckCircle2, History, ShieldX } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import TierBadge from "@/components/TierBadge";
import { useDiag } from "@/lib/store";
import { api, type LogEntry } from "@/lib/tauri";
import { timeOf } from "@/lib/format";

export default function Activity() {
  const { catalog, revision, connection } = useDiag();
  const [log, setLog] = useState<LogEntry[]>([]);

  useEffect(() => {
    const load = () => api.activityLog().then(setLog).catch(() => {});
    load();
    const id = setInterval(load, 2000);
    return () => clearInterval(id);
  }, [revision, connection]);

  const label = (t: string) => catalog?.tiers.find((x) => x.id === t)?.label ?? t;

  return (
    <div>
      <PageHeader
        title="Activity"
        subtitle="Every action this session, including ones the safety checks blocked. Cleared codes are listed here so nothing is lost."
      />
      {log.length === 0 ? (
        <Empty icon={History} title="Nothing yet">
          Actions appear here as you use the app.
        </Empty>
      ) : (
        <div className="overflow-hidden rounded-2xl border border-white/[0.07]">
          {log.map((e, i) => (
            <div key={i} className="grid grid-cols-[72px_20px_1fr] items-start gap-3 border-b border-white/[0.05] px-4 py-3 text-sm last:border-0 md:grid-cols-[84px_20px_220px_1fr_auto]">
              <span className="font-mono text-xs text-muted-foreground">{timeOf(e.at_ms)}</span>
              {e.ok ? <CheckCircle2 className="mt-0.5 size-4 text-emerald-400" /> : <ShieldX className="mt-0.5 size-4 text-rose-400" />}
              <div>
                <div className="font-medium">{e.action}</div>
                <div className="text-xs text-muted-foreground">{e.target}</div>
              </div>
              <div className="col-span-3 text-muted-foreground md:col-span-1">{e.outcome}</div>
              <div className="col-span-3 md:col-span-1">
                <TierBadge tier={e.tier} label={label(e.tier)} />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
