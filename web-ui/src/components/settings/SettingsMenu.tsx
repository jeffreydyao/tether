import type { ReactNode } from "react";
import { Fragment } from "react";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Wifi, Bluetooth, Ticket, Globe, Settings2, ChevronRight } from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type SettingsSection = "wifi" | "bluetooth" | "passes" | "timezone" | "system";

interface SettingsMenuItem {
  id: SettingsSection;
  label: string;
  description: string;
  icon: LucideIcon;
}

const menuItems: SettingsMenuItem[] = [
  {
    id: "wifi",
    label: "Wi-Fi Networks",
    description: "Manage network connections",
    icon: Wifi,
  },
  {
    id: "bluetooth",
    label: "Bluetooth Device",
    description: "Configure tracked device",
    icon: Bluetooth,
  },
  {
    id: "passes",
    label: "Monthly Passes",
    description: "Set pass allowance",
    icon: Ticket,
  },
  {
    id: "timezone",
    label: "Timezone",
    description: "Set your local time",
    icon: Globe,
  },
  {
    id: "system",
    label: "System",
    description: "Restart and version info",
    icon: Settings2,
  },
];

interface SettingsMenuProps {
  onSelectSection: (section: SettingsSection) => void;
}

export function SettingsMenu({ onSelectSection }: SettingsMenuProps): ReactNode {
  return (
    <div className="space-y-1">
      {menuItems.map((item, index) => (
        <Fragment key={item.id}>
          <Button
            variant="ghost"
            className="h-auto w-full justify-between px-3 py-4"
            onClick={() => onSelectSection(item.id)}
          >
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                <item.icon className="h-5 w-5 text-muted-foreground" />
              </div>
              <div className="text-left">
                <div className="font-medium">{item.label}</div>
                <div className="text-sm text-muted-foreground">{item.description}</div>
              </div>
            </div>
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          </Button>
          {index < menuItems.length - 1 && <Separator className="my-1" />}
        </Fragment>
      ))}
    </div>
  );
}
