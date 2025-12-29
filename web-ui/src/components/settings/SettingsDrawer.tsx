import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import { Drawer, DrawerContent, DrawerHeader, DrawerTitle, DrawerDescription } from "@/components/ui/drawer";
import { Button } from "@/components/ui/button";
import { ChevronLeft } from "lucide-react";
import { SettingsMenu, type SettingsSection } from "./SettingsMenu";
import { WifiSettings } from "./WifiSettings";
import { BluetoothSettings } from "./BluetoothSettings";
import { PassesSettings } from "./PassesSettings";
import { TimezoneSettings } from "./TimezoneSettings";
import { SystemSettings } from "./SystemSettings";

interface SettingsDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDrawer({ open, onOpenChange }: SettingsDrawerProps): ReactNode {
  const [currentSection, setCurrentSection] = useState<SettingsSection | null>(null);

  useEffect(() => {
    if (!open) {
      const timer = setTimeout(() => {
        setCurrentSection(null);
      }, 300);
      return () => clearTimeout(timer);
    }
  }, [open]);

  const handleBack = () => {
    setCurrentSection(null);
  };

  const handleSectionSelect = (section: SettingsSection) => {
    setCurrentSection(section);
  };

  const getSectionTitle = (): string => {
    switch (currentSection) {
      case "wifi":
        return "Wi-Fi Networks";
      case "bluetooth":
        return "Bluetooth Device";
      case "passes":
        return "Monthly Passes";
      case "timezone":
        return "Timezone";
      case "system":
        return "System";
      default:
        return "Settings";
    }
  };

  const getSectionDescription = (): string => {
    switch (currentSection) {
      case "wifi":
        return "Manage your Wi-Fi network connections";
      case "bluetooth":
        return "Configure the device to track";
      case "passes":
        return "Set your monthly pass allowance";
      case "timezone":
        return "Configure your local timezone";
      case "system":
        return "System controls and information";
      default:
        return "Configure your Tether device";
    }
  };

  const renderSectionContent = () => {
    switch (currentSection) {
      case "wifi":
        return <WifiSettings />;
      case "bluetooth":
        return <BluetoothSettings />;
      case "passes":
        return <PassesSettings />;
      case "timezone":
        return <TimezoneSettings />;
      case "system":
        return <SystemSettings />;
      default:
        return <SettingsMenu onSelectSection={handleSectionSelect} />;
    }
  };

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="max-h-[85vh]">
        <div className="mx-auto w-full max-w-md">
          <DrawerHeader className="relative">
            {currentSection && (
              <Button
                variant="ghost"
                size="icon"
                className="absolute left-4 top-1/2 -translate-y-1/2"
                onClick={handleBack}
                aria-label="Back to settings menu"
              >
                <ChevronLeft className="h-5 w-5" />
              </Button>
            )}
            <DrawerTitle className="text-center">{getSectionTitle()}</DrawerTitle>
            <DrawerDescription className="text-center">{getSectionDescription()}</DrawerDescription>
          </DrawerHeader>

          <div className="overflow-y-auto px-4 pb-8">{renderSectionContent()}</div>
        </div>
      </DrawerContent>
    </Drawer>
  );
}
