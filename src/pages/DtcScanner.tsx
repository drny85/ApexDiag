import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { api, type ModuleInfo } from "@/lib/tauri";
import PageHeader from "./PageHeader";

const hex = (n: number, w = 2) => "0x" + n.toString(16).toUpperCase().padStart(w, "0");

export default function DtcScanner() {
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  useEffect(() => void api.bmwE70Modules().then(setModules), []);

  return (
    <div>
      <PageHeader title="Trouble Codes" subtitle="Generic OBD-II codes plus BMW module fault memory." />
      <Card className="max-w-3xl">
        <CardHeader>
          <CardTitle>BMW E70 modules</CardTitle>
          <CardDescription>Reading fault memory per module arrives in step 4.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col divide-y">
          {modules.length === 0 && <p className="text-sm text-muted-foreground">Run inside the app to load modules.</p>}
          {modules.map((m) => (
            <div key={m.name} className="flex items-center justify-between py-2">
              <div>
                <div className="text-sm font-medium">{m.name}</div>
                <div className="text-xs text-muted-foreground">
                  {m.description} · address {hex(m.address)} · replies on {hex(m.response_id, 3)}
                </div>
              </div>
              <Button size="sm" variant="outline" disabled>
                Read codes
              </Button>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
