import type { ReactNode } from "react";
import { useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { cn } from "@/lib/utils";
import { ProximityCard, PassesCard, PassHistoryList, DumbpipeCard } from "@/components/dashboard";

type TabId = "status" | "passes" | "settings";

function StatusTab(): ReactNode {
  return (
    <div className="space-y-4 p-4">
      <h1 className="text-2xl font-bold">Status</h1>
      <p className="text-muted-foreground">Phone proximity and connection status.</p>
      <div className="grid gap-4 md:grid-cols-2">
        <ProximityCard className="md:col-span-1" />
        <DumbpipeCard className="md:col-span-1" />
      </div>
    </div>
  );
}

function PassesTab(): ReactNode {
  return (
    <div className="space-y-4 p-4">
      <h1 className="text-2xl font-bold">Passes</h1>
      <p className="text-muted-foreground">Manage your monthly emergency passes.</p>
      <div className="grid gap-4 md:grid-cols-2">
        <PassesCard className="md:col-span-1" />
        <PassHistoryList className="md:col-span-2" maxEntries={10} />
      </div>
    </div>
  );
}

function SettingsTab(): ReactNode {
  return (
    <div className="space-y-4 p-4">
      <h1 className="text-2xl font-bold">Settings</h1>
      <p className="text-muted-foreground">Configure your Tether device.</p>
      <div className="rounded-lg border bg-card p-6 text-center">
        <p className="text-sm text-muted-foreground">
          Settings are available via the drawer. Tap the Settings icon in the navigation bar.
        </p>
      </div>
    </div>
  );
}

export default function DashboardPage(): ReactNode {
  const [searchParams] = useSearchParams();

  const currentTab = useMemo(() => {
    const tab = searchParams.get("tab");
    if (tab === "passes" || tab === "settings") {
      return tab;
    }
    return "status" as TabId;
  }, [searchParams]);

  return (
    <div className={cn("min-h-full")}>
      {currentTab === "status" && <StatusTab />}
      {currentTab === "passes" && <PassesTab />}
      {currentTab === "settings" && <SettingsTab />}
    </div>
  );
}
