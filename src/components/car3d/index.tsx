/**
 * The 3D car components the pages use. three.js loads on demand, so it
 * stays out of the first paint, and every component falls back to the
 * flat SVG drawing if WebGL is unavailable.
 *
 * - <Car3D>: live, turnable model (one per screen).
 * - <CarStill>: a rendered still image, for lists and cards.
 */
import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import type { Pack } from "@/lib/tauri";
import { CarSide, MAKE_ACCENT } from "@/components/illustrations/Vehicle";
import { cn } from "@/lib/utils";
import type { Car3DProps } from "./Car3DView";
import type { View } from "./stage";
import { specFor, type CarSpec, type Style } from "./specs";

export type { PartId } from "./build";
export type { View } from "./stage";
export { specFor } from "./specs";

const View3D = lazy(() => import("./Car3DView"));

type PackLike = Pick<Pack, "id" | "body" | "make"> | null | undefined;

let glOk: boolean | null = null;
function canGl() {
  if (glOk !== null) return glOk;
  try {
    const c = document.createElement("canvas");
    glOk = !!(c.getContext("webgl2") || c.getContext("webgl"));
  } catch {
    glOk = false;
  }
  return glOk;
}

function Fallback({ pack, body, scanning, className }: { pack: PackLike; body: Style; scanning?: boolean; className?: string }) {
  return <CarSide body={pack?.body ?? body} accent={pack ? MAKE_ACCENT[pack.make] : "#67e8f9"} scanning={scanning} className={className} />;
}

export function Car3D({
  pack,
  body = "sedan",
  className,
  ...rest
}: Omit<Car3DProps, "spec" | "onFail"> & { pack: PackLike; body?: Style }) {
  const spec = useMemo(() => specFor(pack, body), [pack, body]);
  const [failed, setFailed] = useState(() => !canGl());
  if (failed)
    return (
      <div className={cn("grid place-items-center", className)}>
        <Fallback pack={pack} body={body} scanning={rest.scanning} className="w-full max-w-xl" />
      </div>
    );
  return (
    <Suspense fallback={<div className={cn("animate-pulse rounded-2xl bg-white/[0.02]", className)} />}>
      <View3D spec={spec} className={className} onFail={() => setFailed(true)} {...rest} />
    </Suspense>
  );
}

/** A still render, sized by its box. Uses one shared off-screen renderer. */
export function CarStill({
  pack,
  body = "sedan",
  view = "front34",
  lights = false,
  className,
  alt,
  width = 520,
  height = 260,
}: {
  pack: PackLike;
  body?: Style;
  view?: View;
  lights?: boolean;
  className?: string;
  alt?: string;
  width?: number;
  height?: number;
}) {
  const spec: CarSpec = useMemo(() => specFor(pack, body), [pack, body]);
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(() => !canGl());
  useEffect(() => {
    if (failed) return;
    let live = true;
    import("./stage")
      .then(({ renderStill }) => {
        // let the page paint first, then render
        requestAnimationFrame(() => {
          if (!live) return;
          const url = renderStill(spec, view, width, height, { lights });
          if (url) setSrc(url);
          else setFailed(true);
        });
      })
      .catch(() => setFailed(true));
    return () => {
      live = false;
    };
  }, [spec, view, lights, width, height, failed]);
  if (failed) return <Fallback pack={pack} body={body} className={className} />;
  return (
    <div className={cn("relative", className)} style={{ aspectRatio: `${width} / ${height}` }}>
      {src ? (
        <img src={src} alt={alt ?? ""} draggable={false} className="absolute inset-0 h-full w-full object-contain" />
      ) : (
        <div className="absolute inset-[18%_10%] animate-pulse rounded-[40%] bg-white/[0.03] blur-md" />
      )}
    </div>
  );
}
