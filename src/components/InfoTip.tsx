import { Info } from "lucide-react";
import { Tooltip } from "@/components/ui/tooltip";

export default function InfoTip({ text }: { text: string }) {
  return (
    <Tooltip content={text}>
      <button className="text-muted-foreground/60 transition hover:text-foreground" aria-label="What is this?">
        <Info className="size-3.5" />
      </button>
    </Tooltip>
  );
}
