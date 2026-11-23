import { useEffect, useState } from "react";
import { Bluetooth, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { api, type DiscoveredDevice } from "@/lib/tauri";
import PageHeader from "./PageHeader";

export default function ConnectionManager() {
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);

  const refresh = () => void api.listDevices().then(setDevices);
  useEffect(refresh, []);

  return (
    <div>
      <PageHeader title="Connection" subtitle="Pick an adapter and protocol to start a session." />
      <Card className="max-w-2xl">
        <CardHeader className="flex-row items-center justify-between">
          <div>
            <CardTitle>Adapters</CardTitle>
            <CardDescription>Bluetooth and serial discovery arrives in step 3.</CardDescription>
          </div>
          <Button variant="outline" size="sm" onClick={refresh}>
            <RefreshCw /> Scan
          </Button>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {devices.map((d) => (
            <div key={d.name} className="flex items-center justify-between rounded-lg border p-3">
              <div className="flex items-center gap-3">
                <Bluetooth className="size-4 text-muted-foreground" />
                <div>
                  <div className="text-sm font-medium">{d.name}</div>
                  <div className="text-xs text-muted-foreground">{d.transport.kind}</div>
                </div>
              </div>
              <Button size="sm" disabled>
                Connect
              </Button>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
