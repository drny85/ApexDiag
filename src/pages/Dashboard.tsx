import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import PageHeader from "./PageHeader";

const GAUGES = [
  { label: "Engine RPM", unit: "rpm" },
  { label: "Vehicle Speed", unit: "km/h" },
  { label: "Coolant Temp", unit: "°C" },
  { label: "Intake MAP", unit: "kPa" },
  { label: "Short Fuel Trim B1", unit: "%" },
  { label: "Timing Advance", unit: "°" },
];

export default function Dashboard() {
  return (
    <div>
      <PageHeader title="Live Data" subtitle="Real-time OBD-II PIDs. Values appear once an adapter is connected." />
      <div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
        {GAUGES.map((g) => (
          <Card key={g.label}>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground font-medium">{g.label}</CardTitle>
            </CardHeader>
            <CardContent className="flex items-baseline gap-2">
              <span className="text-4xl font-semibold tabular-nums">--</span>
              <span className="text-sm text-muted-foreground">{g.unit}</span>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
