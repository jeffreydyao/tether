import type { ReactNode } from "react";
import { useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Bluetooth, RefreshCw, Smartphone, Check, Loader2 } from "lucide-react";
import type { WizardStepProps, BluetoothDevice } from "@/types/onboarding";
import { useBluetoothScan, useBluetoothDevices } from "@/hooks/useOnboardingApi";
import { cn } from "@/lib/utils";

function getSignalInfo(rssi: number): { label: string; colorClass: string } {
  if (rssi >= -50) {
    return { label: "Excellent", colorClass: "text-green-600 dark:text-green-400" };
  }
  if (rssi >= -70) {
    return { label: "Good", colorClass: "text-emerald-600 dark:text-emerald-400" };
  }
  if (rssi >= -85) {
    return { label: "Fair", colorClass: "text-yellow-600 dark:text-yellow-400" };
  }
  return { label: "Weak", colorClass: "text-red-600 dark:text-red-400" };
}

interface DeviceListItemProps {
  device: BluetoothDevice;
  isSelected: boolean;
  onSelect: (device: BluetoothDevice) => void;
}

function DeviceListItem({ device, isSelected, onSelect }: DeviceListItemProps): ReactNode {
  const signalInfo = getSignalInfo(device.rssi);

  return (
    <button
      type="button"
      onClick={() => onSelect(device)}
      className={cn(
        "flex w-full items-center gap-3 rounded-lg border p-3 transition-all",
        "hover:border-accent-foreground/20 hover:bg-accent",
        "focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
        isSelected ? "border-primary bg-primary/5 ring-1 ring-primary" : "border-border bg-card"
      )}
      aria-pressed={isSelected}
    >
      <div
        className={cn(
          "flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full",
          isSelected ? "bg-primary text-primary-foreground" : "bg-muted"
        )}
      >
        <Smartphone className="h-5 w-5" />
      </div>

      <div className="min-w-0 flex-1 text-left">
        <div className="truncate font-medium">{device.name || "Unknown Device"}</div>
        <div className="truncate text-sm text-muted-foreground">{device.id}</div>
      </div>

      <div className="flex flex-shrink-0 flex-col items-end gap-0.5">
        <span className={cn("text-sm font-medium", signalInfo.colorClass)}>{device.rssi} dBm</span>
        <span className="text-xs text-muted-foreground">{signalInfo.label}</span>
      </div>

      {isSelected && (
        <div className="flex-shrink-0">
          <Check className="h-5 w-5 text-primary" />
        </div>
      )}
    </button>
  );
}

export function BluetoothScanStep({ data, onDataChange, setCanProceed }: WizardStepProps): ReactNode {
  const { mutate: startScan, isPending: isScanning } = useBluetoothScan();
  const { data: devicesData, isLoading: isLoadingDevices } = useBluetoothDevices();

  const devices = devicesData?.devices ?? [];
  const selectedDevice = data.bluetooth.selectedDevice;

  useEffect(() => {
    setCanProceed(selectedDevice !== null);
  }, [selectedDevice, setCanProceed]);

  const handleSelectDevice = useCallback(
    (device: BluetoothDevice) => {
      onDataChange("bluetooth", { selectedDevice: device });
    },
    [onDataChange]
  );

  const handleScan = useCallback(() => {
    startScan();
  }, [startScan]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          Make sure Bluetooth is enabled on your phone and it's discoverable.
        </p>
      </div>

      <Button onClick={handleScan} disabled={isScanning} variant="outline" className="w-full">
        {isScanning ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            Scanning...
          </>
        ) : (
          <>
            <RefreshCw className="mr-2 h-4 w-4" />
            Scan for Devices
          </>
        )}
      </Button>

      {isLoadingDevices && (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {!isLoadingDevices && devices.length === 0 && (
        <div className="rounded-lg border border-dashed p-8 text-center">
          <Bluetooth className="mx-auto h-12 w-12 text-muted-foreground/50" />
          <p className="mt-2 text-sm text-muted-foreground">
            No devices found. Tap "Scan for Devices" to search.
          </p>
        </div>
      )}

      {devices.length > 0 && (
        <div className="space-y-2">
          <p className="text-sm font-medium">{devices.length} device(s) found</p>
          <div className="space-y-2">
            {devices.map((device) => (
              <DeviceListItem
                key={device.id}
                device={device}
                isSelected={selectedDevice?.id === device.id}
                onSelect={handleSelectDevice}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
