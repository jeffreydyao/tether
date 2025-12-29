import type { ReactNode } from "react";
import { useState, useCallback, createContext, useContext } from "react";
import { useSearchParams } from "react-router-dom";
import { Bluetooth, Wifi, Ticket, Clock, Link2, Power } from "lucide-react";
import { cn } from "@/lib/utils";
import { MobileNav } from "@/components/layout/MobileNav";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Button } from "@/components/ui/button";

interface AppShellProps {
  children: ReactNode;
  className?: string;
}

interface SettingsDrawerContextType {
  isOpen: boolean;
  open: () => void;
  close: () => void;
  toggle: () => void;
}

const SettingsDrawerContext = createContext<SettingsDrawerContextType | null>(null);

export function useSettingsDrawer(): SettingsDrawerContextType {
  const context = useContext(SettingsDrawerContext);
  if (!context) {
    throw new Error("useSettingsDrawer must be used within an AppShell");
  }
  return context;
}

interface SettingsDrawerContentProps {
  onClose: () => void;
}

function SettingsDrawerContent({ onClose }: SettingsDrawerContentProps): ReactNode {
  const [, setSearchParams] = useSearchParams();

  const handleNavigate = useCallback(
    (section: string) => {
      setSearchParams({ tab: "settings", section });
      onClose();
    },
    [setSearchParams, onClose]
  );

  const settingsSections = [
    {
      id: "bluetooth",
      label: "Bluetooth Device",
      description: "Change the device being tracked",
      icon: Bluetooth,
    },
    {
      id: "wifi",
      label: "Wi-Fi Networks",
      description: "Manage network connections",
      icon: Wifi,
    },
    {
      id: "passes",
      label: "Monthly Passes",
      description: "Configure pass allocation",
      icon: Ticket,
    },
    {
      id: "timezone",
      label: "Timezone",
      description: "Set your local timezone",
      icon: Clock,
    },
    {
      id: "ticket",
      label: "Remote Access",
      description: "View dumbpipe connection ticket",
      icon: Link2,
    },
    {
      id: "system",
      label: "System",
      description: "Restart and advanced options",
      icon: Power,
    },
  ];

  return (
    <>
      <DrawerHeader className="text-left">
        <DrawerTitle>Settings</DrawerTitle>
        <DrawerDescription>Configure your Tether device</DrawerDescription>
      </DrawerHeader>

      <div className="flex flex-col gap-1 px-4">
        {settingsSections.map((section) => {
          const Icon = section.icon;
          return (
            <button
              key={section.id}
              onClick={() => handleNavigate(section.id)}
              className={cn(
                "flex items-center gap-3 rounded-lg p-3 text-left",
                "transition-colors duration-150",
                "hover:bg-accent active:bg-accent/80"
              )}
            >
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
                <Icon className="h-5 w-5" />
              </div>
              <div className="flex flex-col">
                <span className="text-sm font-medium">{section.label}</span>
                <span className="text-xs text-muted-foreground">{section.description}</span>
              </div>
            </button>
          );
        })}
      </div>

      <DrawerFooter className="pt-4">
        <DrawerClose asChild>
          <Button variant="outline" className="w-full">
            Close
          </Button>
        </DrawerClose>
      </DrawerFooter>
    </>
  );
}

export function AppShell({ children, className }: AppShellProps): ReactNode {
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [searchParams] = useSearchParams();

  const currentTab = searchParams.get("tab") || "status";
  const isSettingsTab = currentTab === "settings";

  const openDrawer = useCallback(() => setIsDrawerOpen(true), []);
  const closeDrawer = useCallback(() => setIsDrawerOpen(false), []);
  const toggleDrawer = useCallback(() => setIsDrawerOpen((prev) => !prev), []);

  const contextValue: SettingsDrawerContextType = {
    isOpen: isDrawerOpen,
    open: openDrawer,
    close: closeDrawer,
    toggle: toggleDrawer,
  };

  return (
    <SettingsDrawerContext.Provider value={contextValue}>
      <div
        className={cn(
          "flex h-[100dvh] flex-col",
          "bg-background",
          "pt-[env(safe-area-inset-top,0px)]",
          className
        )}
      >
        <main
          className={cn(
            "flex-1 overflow-y-auto overflow-x-hidden",
            "pb-[calc(64px+env(safe-area-inset-bottom,0px))]",
            "scroll-smooth"
          )}
        >
          {children}
        </main>

        <MobileNav />

        <Drawer
          open={isDrawerOpen || isSettingsTab}
          onOpenChange={(open) => {
            if (!open && isSettingsTab) {
              setIsDrawerOpen(false);
            } else {
              setIsDrawerOpen(open);
            }
          }}
        >
          <DrawerContent className="max-h-[85vh]">
            <SettingsDrawerContent onClose={closeDrawer} />
          </DrawerContent>
        </Drawer>
      </div>
    </SettingsDrawerContext.Provider>
  );
}

export type { AppShellProps, SettingsDrawerContextType };
