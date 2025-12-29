import type { ReactNode } from "react";
import { useState, useCallback, createContext, useContext } from "react";
import { useSearchParams } from "react-router-dom";
import { cn } from "@/lib/utils";
import { MobileNav } from "@/components/layout/MobileNav";
import { SettingsDrawer } from "@/components/settings";

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

        <SettingsDrawer
          open={isDrawerOpen || isSettingsTab}
          onOpenChange={(open) => {
            if (!open && isSettingsTab) {
              setIsDrawerOpen(false);
            } else {
              setIsDrawerOpen(open);
            }
          }}
        />
      </div>
    </SettingsDrawerContext.Provider>
  );
}

export type { AppShellProps, SettingsDrawerContextType };
