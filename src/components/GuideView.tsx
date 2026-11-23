/**
 * A fix guide, step by step: the 3D car shows where the part is, each
 * step has its own animated drawing, and the safety notes stay in view.
 */
import { useEffect, useState } from "react";
import { ArrowLeft, ArrowRight, CheckCircle2, Clock, Info, Lightbulb, MapPin, Package, ShieldAlert, Wrench } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { DifficultyMeter } from "@/components/illustrations/Checks";
import GuideArtView from "@/components/illustrations/GuideArt";
import { Car3D } from "@/components/car3d";
import { CATEGORY_LABEL, type Guide } from "@/lib/guides";
import type { Pack } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export default function GuideView({ guide, pack, onClose }: { guide: Guide | null; pack: Pack | null; onClose: () => void }) {
  const [step, setStep] = useState(0);
  const [lights, setLights] = useState(false);
  useEffect(() => {
    setStep(0);
    setLights(false);
  }, [guide?.id]);
  if (!guide) return null;
  const total = guide.steps.length;
  const done = step >= total;
  const s = guide.steps[Math.min(step, total - 1)]!;
  const lit = lights || (!done && !!s.lights);
  const carNote = pack ? guide.perCar?.[pack.id] : undefined;
  const isLight = guide.category === "lights";

  return (
    <Dialog open={!!guide} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="w-[min(1120px,calc(100vw-1.5rem))] max-h-[calc(100dvh-1.5rem)] overflow-hidden p-0">
        <div className="overflow-y-auto">
          {/* header */}
          <div className="border-b border-white/[0.06] px-6 pb-4 pt-5 pr-14">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="info">{CATEGORY_LABEL[guide.category]}</Badge>
              <Badge variant="muted">
                <Info /> General guidance
              </Badge>
            </div>
            <DialogTitle className="mt-2 text-xl">{guide.title}</DialogTitle>
            <DialogDescription className="mt-1 max-w-3xl">{guide.summary}</DialogDescription>
            <div className="mt-3 flex flex-wrap items-center gap-x-5 gap-y-1.5 text-xs text-muted-foreground">
              <DifficultyMeter difficulty={guide.difficulty} />
              <span className="inline-flex items-center gap-1">
                <Clock className="size-3.5" /> {guide.time}
              </span>
              <span className="inline-flex items-center gap-1">
                <MapPin className="size-3.5 text-cyan-300" /> {guide.partLabel}
              </span>
            </div>
          </div>

          <div className="grid gap-0 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            {/* where it is, what you need */}
            <div className="border-white/[0.06] p-5 lg:border-r">
              <div className="relative overflow-hidden rounded-2xl border border-white/[0.06] bg-[radial-gradient(ellipse_at_50%_30%,rgba(103,232,249,0.08),transparent_60%)]">
                <Car3D pack={pack} body="suv" highlight={guide.part} label={guide.partLabel} lights={lit} autoRotate={false} className="aspect-[16/10] w-full" />
                <div className="absolute bottom-2 left-3 text-[10px] text-muted-foreground/80">Drag to turn · pinch to zoom</div>
                {isLight && (
                  <button
                    onClick={() => setLights((x) => !x)}
                    className={cn(
                      "absolute right-2 top-2 inline-flex min-h-9 items-center gap-1.5 rounded-lg px-2.5 text-xs ring-1 ring-inset transition",
                      lit ? "bg-amber-300/15 text-amber-200 ring-amber-300/30" : "bg-black/40 text-muted-foreground ring-white/10 hover:text-foreground",
                    )}
                  >
                    <Lightbulb className="size-3.5" /> {lit ? "Lights on" : "Lights off"}
                  </button>
                )}
              </div>
              {!pack && <p className="mt-2 text-[11px] text-muted-foreground">Connect a car to see it on your own model. Where the part sits varies between cars.</p>}
              {carNote && (
                <p className="mt-3 rounded-xl bg-cyan-400/[0.06] px-3 py-2 text-xs leading-relaxed text-cyan-100/90 ring-1 ring-inset ring-cyan-400/20">
                  <span className="font-semibold">
                    {pack!.brand} {pack!.model} ({pack!.chassis}):
                  </span>{" "}
                  {carNote}
                </p>
              )}
              <div className="mt-4 grid gap-3 sm:grid-cols-2">
                <ListBox icon={Wrench} title="Tools" items={guide.tools.length ? guide.tools : ["None"]} />
                <ListBox icon={Package} title="Parts" items={guide.parts.length ? guide.parts : ["None"]} />
              </div>
              {guide.safety.length > 0 && (
                <div className="mt-3 space-y-1.5">
                  {guide.safety.map((x) => (
                    <p key={x} className="flex items-start gap-1.5 rounded-lg bg-amber-400/[0.06] px-2.5 py-1.5 text-xs leading-relaxed text-amber-100/90 ring-1 ring-inset ring-amber-400/20">
                      <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-amber-300" /> {x}
                    </p>
                  ))}
                </div>
              )}
              <p className="mt-4 text-[11px] leading-relaxed text-muted-foreground">
                General workshop practice, not the maker's service procedure. Access, part numbers, torques and set-up steps vary by car: check the owner's manual or the maker's service information for yours.
              </p>
            </div>

            {/* steps */}
            <div className="flex min-w-0 flex-col p-5">
              <div className="flex flex-wrap gap-1.5">
                {guide.steps.map((x, i) => (
                  <button
                    key={x.title}
                    onClick={() => setStep(i)}
                    aria-label={`Step ${i + 1}: ${x.title}`}
                    className={cn(
                      "grid size-9 place-items-center rounded-full text-xs font-semibold ring-1 ring-inset transition",
                      i === step ? "bg-cyan-300 text-slate-950 ring-cyan-300" : i < step ? "bg-emerald-400/15 text-emerald-200 ring-emerald-400/30" : "bg-white/5 text-muted-foreground ring-white/10 hover:text-foreground",
                    )}
                  >
                    {i < step ? <CheckCircle2 className="size-4" /> : i + 1}
                  </button>
                ))}
                <button
                  onClick={() => setStep(total)}
                  aria-label="Check it worked"
                  className={cn(
                    "grid h-9 place-items-center rounded-full px-3 text-xs font-semibold ring-1 ring-inset transition",
                    done ? "bg-emerald-300 text-slate-950 ring-emerald-300" : "bg-white/5 text-muted-foreground ring-white/10 hover:text-foreground",
                  )}
                >
                  Check
                </button>
              </div>

              {!done ? (
                <div key={step} className="mt-4 flex flex-1 flex-col">
                  <div className="grid place-items-center rounded-2xl border border-white/[0.06] bg-black/25 p-3">
                    <GuideArtView art={s.art} className="w-full max-w-[460px]" />
                  </div>
                  <div className="mt-4 text-xs font-medium uppercase tracking-[0.16em] text-cyan-300/80">
                    Step {step + 1} of {total}
                  </div>
                  <h3 className="mt-1 text-lg font-semibold tracking-tight">{s.title}</h3>
                  <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">{s.text}</p>
                  {s.caution && (
                    <p className="mt-3 flex items-start gap-1.5 rounded-lg bg-amber-400/[0.06] px-2.5 py-1.5 text-xs text-amber-100/90 ring-1 ring-inset ring-amber-400/20">
                      <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-amber-300" /> {s.caution}
                    </p>
                  )}
                </div>
              ) : (
                <div className="mt-4 flex flex-1 flex-col">
                  <div className="grid place-items-center rounded-2xl border border-emerald-400/15 bg-emerald-400/[0.04] p-3">
                    <GuideArtView art="clear_codes" className="w-full max-w-[460px]" />
                  </div>
                  <h3 className="mt-4 flex items-center gap-2 text-lg font-semibold tracking-tight">
                    <CheckCircle2 className="size-5 text-emerald-300" /> How to know it worked
                  </h3>
                  <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">{guide.verify}</p>
                </div>
              )}

              <div className="mt-5 flex items-center justify-between gap-2">
                <Button variant="ghost" disabled={step === 0} onClick={() => setStep((x) => Math.max(0, x - 1))}>
                  <ArrowLeft /> Back
                </Button>
                {done ? (
                  <Button variant="secondary" onClick={onClose}>
                    Close guide
                  </Button>
                ) : (
                  <Button onClick={() => setStep((x) => x + 1)}>
                    {step === total - 1 ? "Check it worked" : "Next step"} <ArrowRight />
                  </Button>
                )}
              </div>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ListBox({ icon: Icon, title, items }: { icon: typeof Wrench; title: string; items: string[] }) {
  return (
    <div className="rounded-xl bg-white/[0.02] p-3 ring-1 ring-inset ring-white/[0.06]">
      <div className="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-[0.14em] text-muted-foreground">
        <Icon className="size-3.5" /> {title}
      </div>
      <ul className="mt-1.5 space-y-1 text-xs text-foreground/85">
        {items.map((x) => (
          <li key={x}>{x}</li>
        ))}
      </ul>
    </div>
  );
}
