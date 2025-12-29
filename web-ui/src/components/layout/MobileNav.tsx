import type { ReactNode } from "react";
import { useMemo } from "react";
import { NavLink, useLocation } from "react-router-dom";
import { CheckCircle, Ticket, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

interface NavTab {
  id: string;
  label: string;
  tabParam: string;
  icon: React.ComponentType<{ className?: string }>;
}

interface MobileNavProps {
  className?: string;
}

const NAV_TABS: NavTab[] = [
  {
    id: "status",
    label: "Status",
    tabParam: "status",
    icon: CheckCircle,
  },
  {
    id: "passes",
    label: "Passes",
    tabParam: "passes",
    icon: Ticket,
  },
  {
    id: "settings",
    label: "Settings",
    tabParam: "settings",
    icon: Settings,
  },
];

export function MobileNav({ className }: MobileNavProps): ReactNode {
  const location = useLocation();

  const currentTab = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get("tab") || "status";
  }, [location.search]);

  return (
    <nav
      className={cn(
        "fixed inset-x-0 bottom-0 z-50",
        "border-t border-border/40 bg-background/80 backdrop-blur-lg",
        "pb-[env(safe-area-inset-bottom,0px)]",
        className
      )}
      role="navigation"
      aria-label="Main navigation"
    >
      <div className="flex h-16 items-stretch justify-around">
        {NAV_TABS.map((tab) => {
          const isActive = currentTab === tab.tabParam;
          const Icon = tab.icon;

          return (
            <NavLink
              key={tab.id}
              to={`/dashboard?tab=${tab.tabParam}`}
              className={cn(
                "flex flex-1 flex-col items-center justify-center gap-1",
                "min-h-[44px] min-w-[44px]",
                "transition-colors duration-150",
                isActive ? "text-primary" : "text-muted-foreground hover:text-foreground"
              )}
              aria-current={isActive ? "page" : undefined}
            >
              <Icon className={cn("h-6 w-6", isActive && "fill-primary/20")} />
              <span
                className={cn(
                  "text-[10px] font-medium leading-none",
                  isActive ? "text-primary" : "text-muted-foreground"
                )}
              >
                {tab.label}
              </span>
            </NavLink>
          );
        })}
      </div>
    </nav>
  );
}

export { NAV_TABS };
export type { NavTab, MobileNavProps };
