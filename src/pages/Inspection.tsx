import { useRef, useState } from "react";
import { AlertTriangle, BatteryMedium, CheckCircle2, CircleHelp, ClipboardList, Cpu, Download, Eraser, Gauge, Leaf, Plug, RotateCcw, Share2, ShieldAlert, Wrench, XCircle, type LucideIcon } from "lucide-react";
import PageHeader from "@/components/PageHeader";
import Empty from "@/components/Empty";
import { Button } from "@/components/ui/button";
import { CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import { useDiag } from "@/lib/store";
import { useI18n, type Lang, type T, Tr } from "@/lib/i18n";
import { api, asCmdError, type Grade, type InspectionSection, type InspectionView } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { PageId } from "@/App";

/** Count chips under the overall grade: [one, many]. */
const COUNT: Record<Grade, [string, string]> = {
  good: ["{n} good", "{n} good"],
  attention: ["{n} to check", "{n} to check"],
  problem: ["{n} problem", "{n} problems"],
  not_checked: ["{n} not checked", "{n} not checked"],
};

export const GRADE: Record<Grade, { label: string; long: string; color: string; ring: string; chip: string; icon: LucideIcon }> = {
  good: { label: "Good", long: "Looks good", color: "#34d399", ring: "ring-emerald-400/30", chip: "bg-emerald-400/10 text-emerald-300 ring-emerald-400/25", icon: CheckCircle2 },
  attention: { label: "Check", long: "Worth a closer look", color: "#fbbf24", ring: "ring-amber-400/30", chip: "bg-amber-400/10 text-amber-300 ring-amber-400/25", icon: AlertTriangle },
  problem: { label: "Problem", long: "Problems found", color: "#fb7185", ring: "ring-rose-400/30", chip: "bg-rose-500/10 text-rose-300 ring-rose-400/25", icon: XCircle },
  not_checked: { label: "Not checked", long: "Could not be read", color: "#94a3b8", ring: "ring-white/10", chip: "bg-white/[0.05] text-slate-300 ring-white/10", icon: CircleHelp },
};

const AREA_ICON: Record<InspectionSection["id"], LucideIcon> = {
  engine: Gauge,
  modules: Cpu,
  readiness: Leaf,
  cleared: Eraser,
  battery: BatteryMedium,
  mileage: ClipboardList,
};

const CHECKS: { id: InspectionSection["id"]; title: string; text: string }[] = [
  { id: "engine", title: "Engine light and codes", text: "Stored, pending and permanent engine codes." },
  { id: "modules", title: "Every module", text: "Fault memory of brakes, airbags, body, comfort and more." },
  { id: "readiness", title: "Emissions readiness", text: "Whether the car would be ready for an inspection station." },
  { id: "cleared", title: "Codes cleared recently?", text: "Catches the seller's trick of clearing codes before a sale." },
  { id: "battery", title: "Battery and charging", text: "Resting charge with the engine off, charging with it running." },
  { id: "mileage", title: "Mileage", text: "The odometer over OBD-II where the car reports it." },
];

type Phase = { step: "idle" } | { step: "basics" } | { step: "modules"; done: number; total: number; name: string } | { step: "done"; view: InspectionView } | { step: "error"; message: string };

export default function Inspection({ go }: { go: (p: PageId) => void }) {
  const { connection } = useDiag();
  const { t, lang } = useI18n();
  const [phase, setPhase] = useState<Phase>({ step: "idle" });
  const [saved, setSaved] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const cancelled = useRef(false);

  if (!connection) {
    return (
      <div>
        <PageHeader title="Inspection Report" subtitle="A full pre-purchase checkup with a green, amber or red grade for each area." />
        <Empty icon={ClipboardList} title="No car connected">
          <Button className="mt-4" onClick={() => go("connect")}>
            <Plug /> <Tr>Connect</Tr>
          </Button>
        </Empty>
      </div>
    );
  }

  const pack = connection.vehicle.pack;
  const order = connection.vehicle.scan_addresses;

  const start = async () => {
    cancelled.current = false;
    setSaved(null);
    setSaveError(null);
    try {
      setPhase({ step: "basics" });
      await api.inspectionStart();
      const scanned: { address: number; present: boolean; codes: { code: string; description: string | null }[] }[] = [];
      for (const [i, address] of order.entries()) {
        if (cancelled.current) return setPhase({ step: "idle" });
        const name = pack.modules.find((m) => m.address === address)?.name ?? "";
        setPhase({ step: "modules", done: i, total: order.length, name });
        const s = await api.scanModule(address).catch(() => null);
        if (s) scanned.push({ address, present: s.present, codes: s.codes.map((c) => ({ code: c.code, description: c.description })) });
      }
      const view = await api.inspectionFinish(scanned, lang);
      setPhase({ step: "done", view });
    } catch (e) {
      setPhase({ step: "error", message: asCmdError(e).message });
    }
  };

  const running = phase.step === "basics" || phase.step === "modules";
  const progress = phase.step === "basics" ? 0.08 : phase.step === "modules" ? 0.1 + (0.9 * phase.done) / Math.max(1, phase.total) : 1;

  return (
    <div>
      <PageHeader
        title="Inspection Report"
        subtitle="A full checkup in about a minute: every module, emissions readiness, signs of recently cleared codes, battery and mileage. Each area gets a grade and a plain explanation. Everything is read-only."
      />

      {phase.step !== "done" && (
        <section className="overflow-hidden rounded-2xl border border-white/[0.07] bg-[radial-gradient(ellipse_at_top_right,rgba(52,211,153,0.08),transparent_55%)] p-6">
          <div className="flex flex-col gap-6 md:flex-row md:items-center">
            <div className="md:w-[40%]">
              <CarSide body={pack.body} accent={MAKE_ACCENT[pack.make]} scanning={running} className="w-full" />
              <div className="mt-3 text-center text-sm font-medium">
                {pack.brand} {pack.model}
              </div>
              <div className="text-center font-mono text-[11px] text-muted-foreground">{connection.vin ?? t("VIN not read")}</div>
            </div>
            <div className="flex-1">
              <ul className="grid gap-3 sm:grid-cols-2">
                {CHECKS.map((c) => {
                  const Icon = AREA_ICON[c.id];
                  return (
                    <li key={c.id} className="flex gap-3">
                      <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-white/[0.04] ring-1 ring-white/[0.08]">
                        <Icon className="size-4 text-emerald-300" />
                      </span>
                      <div>
                        <div className="text-sm font-medium">{t(c.title)}</div>
                        <div className="text-xs text-muted-foreground">{t(c.text)}</div>
                      </div>
                    </li>
                  );
                })}
              </ul>
              {running ? (
                <div className="mt-5">
                  <div className="flex justify-between text-xs text-muted-foreground">
                    <span>{phase.step === "basics" ? t("Reading engine codes, readiness and battery…") : t("Asking {name} ({n} of {total})…", { name: phase.name || t("module"), n: phase.done + 1, total: phase.total })}</span>
                    <button className="hover:text-foreground" onClick={() => (cancelled.current = true)}>
                      {t("Stop")}
                    </button>
                  </div>
                  <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/[0.06]">
                    <div className="h-full rounded-full bg-gradient-to-r from-emerald-400 to-cyan-400 transition-[width] duration-300" style={{ width: `${progress * 100}%` }} />
                  </div>
                </div>
              ) : (
                <div className="mt-5 flex flex-wrap items-center gap-3">
                  <Button onClick={start} className="from-emerald-300 to-emerald-400 shadow-emerald-500/20 hover:from-emerald-200 hover:to-emerald-300">
                    <ClipboardList /> {t("Start inspection")}
                  </Button>
                  <span className="text-xs text-muted-foreground">{t("Tip: run it with the engine off, then again running, to check both the battery and charging.")}</span>
                </div>
              )}
              {phase.step === "error" && (
                <p className="mt-3 flex items-start gap-2 text-sm text-amber-200">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" /> {phase.message}
                </p>
              )}
            </div>
          </div>
        </section>
      )}

      {phase.step === "done" && (
        <Result
          view={phase.view}
          saved={saved}
          saveError={saveError}
          onAgain={start}
          onSave={async () => {
            const v = phase.view;
            const stamp = new Date(v.at_ms).toISOString().slice(0, 16).replace(/[:T]/g, "-");
            const base = `inspection-${v.vin ?? "car"}-${stamp}`;
            try {
              const path = await api.saveDocument(`${base}.html`, reportHtml(v, t, lang));
              await api.saveDocument(`${base}.json`, JSON.stringify(v, null, 2));
              setSaved(path);
              setSaveError(null);
            } catch (e) {
              setSaveError(asCmdError(e).message);
            }
          }}
        />
      )}
    </div>
  );
}

function Result({ view, saved, saveError, onAgain, onSave }: { view: InspectionView; saved: string | null; saveError: string | null; onAgain: () => void; onSave: () => void }) {
  const { t, lang } = useI18n();
  const g = GRADE[view.report.overall];
  const counts = (["good", "attention", "problem", "not_checked"] as Grade[]).map((k) => ({ k, n: view.report.sections.filter((s) => s.grade === k).length }));
  const share = async () => {
    const text = shareText(view, t, lang);
    try {
      if (navigator.share) await navigator.share({ title: t("Inspection report"), text });
      else await navigator.clipboard.writeText(text);
    } catch {
      /* cancelled */
    }
  };
  return (
    <div className="space-y-5">
      <section className={cn("relative overflow-hidden rounded-2xl border border-white/[0.07] p-6 ring-1 ring-inset", g.ring)} style={{ background: `radial-gradient(ellipse at top left, ${g.color}1f, transparent 60%)` }}>
        <div className="flex flex-col gap-6 md:flex-row md:items-center">
          <GradeRing grade={view.report.overall} sections={view.report.sections} />
          <div className="flex-1">
            <div className="text-xs font-medium uppercase tracking-[0.14em]" style={{ color: g.color }}>
              {t(g.long)}
            </div>
            <div className="mt-1 text-2xl font-semibold tracking-tight">{view.report.summary}</div>
            <div className="mt-2 text-sm text-muted-foreground">
              {view.vehicle} · <span className="font-mono">{view.vin ?? t("VIN not read")}</span> · {new Date(view.at_ms).toLocaleString(lang)}
              {view.simulated && ` · ${t("demo car")}`}
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {counts
                .filter((c) => c.n > 0)
                .map(({ k, n }) => (
                  <span key={k} className={cn("rounded-md px-2 py-0.5 text-xs ring-1 ring-inset", GRADE[k].chip)}>
                    {t(n === 1 ? COUNT[k][0] : COUNT[k][1], { n })}
                  </span>
                ))}
            </div>
            <div className="mt-4 flex flex-wrap gap-2">
              <Button onClick={onSave}>
                <Download /> {t("Save report")}
              </Button>
              <Button variant="outline" onClick={share}>
                <Share2 /> {t("Share summary")}
              </Button>
              <Button variant="ghost" onClick={onAgain}>
                <RotateCcw /> {t("Run again")}
              </Button>
            </div>
            {saved && <p className="mt-2 text-xs text-emerald-300">{t("Saved to {path}. Open it in a browser to print or save as PDF.", { path: saved })}</p>}
            {saveError && <p className="mt-2 text-xs text-amber-200">{saveError}</p>}
          </div>
        </div>
      </section>

      <div className="grid gap-4 lg:grid-cols-2">
        {view.report.sections.map((s) => (
          <SectionCard key={s.id} s={s} />
        ))}
      </div>

      <p className="flex items-start gap-2 text-xs text-muted-foreground">
        <ShieldAlert className="mt-0.5 size-3.5 shrink-0" />
        {t("This report reads the car's electronics only. It doesn't replace a mechanic's physical inspection of tyres, brakes, rust, leaks or accident damage.")}
      </p>
    </div>
  );
}

/** Ring split into one arc per area, coloured by its grade. */
function GradeRing({ grade, sections }: { grade: Grade; sections: InspectionSection[] }) {
  const { t } = useI18n();
  const g = GRADE[grade];
  const Icon = g.icon;
  const r = 52;
  const c = 2 * Math.PI * r;
  const gap = 6;
  const seg = c / sections.length;
  return (
    <div className="relative mx-auto size-40 shrink-0">
      <svg viewBox="0 0 128 128" className="size-full -rotate-90">
        {sections.map((s, i) => (
          <circle
            key={s.id}
            cx="64"
            cy="64"
            r={r}
            fill="none"
            stroke={GRADE[s.grade].color}
            strokeOpacity={s.grade === "not_checked" ? 0.35 : 0.95}
            strokeWidth="10"
            strokeLinecap="round"
            strokeDasharray={`${seg - gap} ${c - seg + gap}`}
            strokeDashoffset={-i * seg}
            className="animate-[ring-in_700ms_ease-out_both]"
            style={{ animationDelay: `${i * 90}ms` }}
          />
        ))}
      </svg>
      <div className="absolute inset-0 grid place-items-center text-center">
        <div>
          <Icon className="mx-auto size-7" style={{ color: g.color }} />
          <div className="mt-1 text-sm font-semibold" style={{ color: g.color }}>
            {t(g.label)}
          </div>
        </div>
      </div>
    </div>
  );
}

function SectionCard({ s }: { s: InspectionSection }) {
  const { t } = useI18n();
  const g = GRADE[s.grade];
  const Icon = AREA_ICON[s.id] ?? Wrench;
  return (
    <section className="relative overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.02] p-4 pl-5">
      <span className="absolute inset-y-0 left-0 w-1" style={{ background: g.color }} />
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <Icon className="size-4 text-muted-foreground" />
          <h3 className="font-medium">{s.title}</h3>
        </div>
        <span className={cn("shrink-0 rounded-md px-2 py-0.5 text-xs font-medium ring-1 ring-inset", g.chip)}>{t(g.label)}</span>
      </div>
      <p className="mt-2 text-sm">{s.headline}</p>
      {s.details.length > 0 && (
        <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
          {s.details.map((d) => (
            <li key={d} className="flex gap-2">
              <span className="mt-1.5 size-1 shrink-0 rounded-full bg-white/30" />
              <span>{d}</span>
            </li>
          ))}
        </ul>
      )}
      <p className="mt-3 border-t border-white/[0.05] pt-2 text-[11px] text-muted-foreground/90">
        <span className="font-medium text-foreground/70">{t("Why it matters:")} </span>
        {s.why}
      </p>
    </section>
  );
}

function shareText(v: InspectionView, t: T, lang: Lang) {
  const lines = [
    `${t("Inspection report")}: ${v.vehicle}${v.vin ? ` (${v.vin})` : ""}`,
    new Date(v.at_ms).toLocaleString(lang),
    "",
    `${t("Overall")}: ${t(GRADE[v.report.overall].long)}. ${v.report.summary}`,
    "",
  ];
  for (const s of v.report.sections) lines.push(`${t(GRADE[s.grade].label).toUpperCase()}  ${s.title}: ${s.headline}`);
  lines.push("", t("Made with ApexDiag. Electronic checks only."));
  return lines.join("\n");
}

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** A standalone, print-friendly page: opens in any browser, prints to PDF. */
export function reportHtml(v: InspectionView, t: T, lang: Lang) {
  const tone: Record<Grade, string> = { good: "#059669", attention: "#d97706", problem: "#e11d48", not_checked: "#64748b" };
  const sections = v.report.sections
    .map(
      (s) => `<section style="border-left:4px solid ${tone[s.grade]}">
  <div class="row"><h3>${esc(s.title)}</h3><span class="chip" style="color:${tone[s.grade]};border-color:${tone[s.grade]}">${esc(t(GRADE[s.grade].label))}</span></div>
  <p class="head">${esc(s.headline)}</p>
  ${s.details.length ? `<ul>${s.details.map((d) => `<li>${esc(d)}</li>`).join("")}</ul>` : ""}
  <p class="why"><b>${esc(t("Why it matters:"))}</b> ${esc(s.why)}</p>
</section>`,
    )
    .join("\n");
  return `<!doctype html>
<html lang="${lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(t("Inspection report"))} · ${esc(v.vehicle)}</title>
<style>
  body{font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;color:#0f172a;background:#fff;max-width:820px;margin:32px auto;padding:0 20px}
  h1{font-size:24px;margin:0} h3{font-size:15px;margin:0}
  .meta{color:#475569;margin:4px 0 20px} .overall{border-radius:12px;padding:16px 18px;margin-bottom:20px;color:#fff}
  .overall b{display:block;font-size:12px;letter-spacing:.12em;text-transform:uppercase;opacity:.9}
  .overall span{font-size:18px;font-weight:600}
  section{padding:10px 14px;margin:0 0 12px;background:#f8fafc;border-radius:8px;break-inside:avoid}
  .row{display:flex;justify-content:space-between;align-items:center;gap:12px}
  .chip{border:1px solid;border-radius:6px;padding:1px 8px;font-size:12px;font-weight:600}
  .head{margin:6px 0} ul{margin:4px 0 6px;padding-left:18px;color:#334155;font-size:13px}
  .why{color:#64748b;font-size:12px;margin:6px 0 0} footer{color:#64748b;font-size:12px;margin-top:24px;border-top:1px solid #e2e8f0;padding-top:10px}
</style></head><body>
<h1>${esc(t("Inspection report"))}</h1>
<div class="meta">${esc(v.vehicle)} · VIN ${esc(v.vin ?? t("not read"))} · ${esc(new Date(v.at_ms).toLocaleString(lang))}${v.simulated ? ` · ${esc(t("demo car (simulated data)"))}` : ""}</div>
<div class="overall" style="background:${tone[v.report.overall]}"><b>${esc(t(GRADE[v.report.overall].long))}</b><span>${esc(v.report.summary)}</span></div>
${sections}
<footer>${esc(t("Made with ApexDiag. These are read-only electronic checks; they don't replace a mechanic's physical inspection of tyres, brakes, rust, leaks or accident damage."))}</footer>
</body></html>`;
}
