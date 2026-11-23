import { ShieldAlert } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import PageHeader from "./PageHeader";

const FUNCTIONS = [
  { name: "Coolant bleeding", detail: "Runs the electric water pump to purge air from the cooling system." },
  { name: "Battery registration", detail: "Tells the power management module a new battery was fitted." },
  { name: "Adaptation reset", detail: "Clears DME fuel and throttle adaptations and EGS clutch values." },
];

export default function SpecialFunctions() {
  return (
    <div>
      <PageHeader title="Special Functions" subtitle="BMW service routines and actuator tests." />
      <div className="mb-4 flex items-start gap-3 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm max-w-3xl">
        <ShieldAlert className="size-4 mt-0.5 text-destructive" />
        Every routine here will require the vehicle to be stationary and a two-step confirmation before it runs.
      </div>
      <div className="grid gap-4 max-w-3xl">
        {FUNCTIONS.map((f) => (
          <Card key={f.name}>
            <CardHeader>
              <CardTitle>{f.name}</CardTitle>
              <CardDescription>{f.detail}</CardDescription>
            </CardHeader>
            <CardContent className="text-xs text-muted-foreground">Available in step 4.</CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
