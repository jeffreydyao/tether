import type { ReactNode } from "react";
import { useEffect } from "react";
import { Slider } from "@/components/ui/slider";
import { Signal } from "lucide-react";
import type { WizardStepProps } from "@/types/onboarding";
import { useDeviceRssi } from "@/hooks/useOnboardingApi";
import { cn } from "@/lib/utils";

function getProximityLabel(threshold: number): string {
  if (threshold >= -50) return "Very Close (same room)";
  if (threshold >= -65) return "Close (nearby)";
  if (threshold >= -80) return "Medium (same floor)";
  return "Far (anywhere in building)";
}

export function SignalThresholdStep({ data, onDataChange, setCanProceed }: WizardStepProps): ReactNode {
  const deviceId = data.bluetooth.selectedDevice?.id ?? null;
  const threshold = data.signalThreshold.threshold;

  const { data: rssiData } = useDeviceRssi(deviceId, { enabled: !!deviceId });
  const currentRssi = rssiData?.rssi ?? null;

  useEffect(() => {
    setCanProceed(threshold >= -100 && threshold <= -30);
  }, [threshold, setCanProceed]);

  const handleThresholdChange = (value: number[]) => {
    onDataChange("signalThreshold", { threshold: value[0] });
  };

  const isNearby = currentRssi !== null && currentRssi >= threshold;

  return (
    <div className="space-y-6">
      <div className="rounded-lg bg-muted/50 p-4 text-center">
        <p className="text-sm text-muted-foreground">Current Signal Strength</p>
        <div className="mt-2 flex items-center justify-center gap-2">
          <Signal className={cn("h-6 w-6", isNearby ? "text-green-500" : "text-muted-foreground")} />
          <p className="text-3xl font-bold">{currentRssi !== null ? `${currentRssi} dBm` : "---"}</p>
        </div>
        {currentRssi !== null && (
          <p className={cn("mt-1 text-sm font-medium", isNearby ? "text-green-600" : "text-yellow-600")}>
            {isNearby ? "Phone is nearby" : "Phone is too far"}
          </p>
        )}
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">Detection Threshold</label>
          <span className="text-sm font-bold">{threshold} dBm</span>
        </div>

        <Slider
          value={[threshold]}
          onValueChange={handleThresholdChange}
          min={-100}
          max={-30}
          step={5}
          className="w-full"
        />

        <div className="flex justify-between text-xs text-muted-foreground">
          <span>Far away</span>
          <span>Very close</span>
        </div>
      </div>

      <div className="rounded-lg border bg-card p-3">
        <p className="text-sm font-medium">{getProximityLabel(threshold)}</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Your phone will be considered "nearby" when the signal is stronger than {threshold} dBm.
        </p>
      </div>

      <p className="text-center text-xs text-muted-foreground">
        Walk to where you plan to leave your phone at night and adjust the threshold so it shows as "nearby".
      </p>
    </div>
  );
}
