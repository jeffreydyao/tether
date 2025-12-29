import type { ReactNode } from "react";
import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { CheckCircle, Loader2 } from "lucide-react";
import type { WizardStepProps } from "@/types/onboarding";
import { useCompleteOnboarding, useBluetoothConfig, usePassesConfig, useTimezoneConfig } from "@/hooks/useOnboardingApi";

export function CompletionStep({ data, onNext, setCanProceed }: WizardStepProps): ReactNode {
  const bluetoothConfig = useBluetoothConfig();
  const passesConfig = usePassesConfig();
  const timezoneConfig = useTimezoneConfig();
  const completeOnboarding = useCompleteOnboarding();

  const isSubmitting =
    bluetoothConfig.isPending || passesConfig.isPending || timezoneConfig.isPending || completeOnboarding.isPending;

  const isComplete = completeOnboarding.isSuccess;

  useEffect(() => {
    setCanProceed(isComplete);
  }, [isComplete, setCanProceed]);

  const handleComplete = async () => {
    try {
      // Configure Bluetooth
      if (data.bluetooth.selectedDevice) {
        await bluetoothConfig.mutateAsync({
          deviceAddress: data.bluetooth.selectedDevice.id,
          signalThreshold: data.signalThreshold.threshold,
        });
      }

      // Configure passes
      await passesConfig.mutateAsync(data.passes.monthlyCount);

      // Configure timezone
      if (data.timezone.selected) {
        await timezoneConfig.mutateAsync(data.timezone.selected);
      }

      // Mark onboarding complete
      await completeOnboarding.mutateAsync();
    } catch {
      // Error is handled by the mutation state
    }
  };

  if (isComplete) {
    return (
      <div className="space-y-6 py-8 text-center">
        <div className="mx-auto flex h-20 w-20 items-center justify-center rounded-full bg-green-100 dark:bg-green-900">
          <CheckCircle className="h-10 w-10 text-green-600 dark:text-green-400" />
        </div>

        <div>
          <h3 className="text-xl font-bold">All Set!</h3>
          <p className="mt-2 text-muted-foreground">
            Your Tether device is ready. You can now access it from any device on your network.
          </p>
        </div>

        <div className="rounded-lg border bg-card p-4 text-left">
          <h4 className="text-sm font-medium">Quick Summary</h4>
          <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
            <li>Device: {data.bluetooth.selectedDevice?.name || "Unknown"}</li>
            <li>Signal Threshold: {data.signalThreshold.threshold} dBm</li>
            <li>Monthly Passes: {data.passes.monthlyCount}</li>
            <li>Timezone: {data.timezone.selected}</li>
          </ul>
        </div>

        <Button onClick={onNext} className="w-full" size="lg">
          Go to Dashboard
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6 py-8 text-center">
      <div className="mx-auto flex h-20 w-20 items-center justify-center rounded-full bg-primary/10">
        <CheckCircle className="h-10 w-10 text-primary" />
      </div>

      <div>
        <h3 className="text-xl font-bold">Ready to Complete Setup</h3>
        <p className="mt-2 text-muted-foreground">
          Review your settings and tap "Complete Setup" to finish configuration.
        </p>
      </div>

      <div className="rounded-lg border bg-card p-4 text-left">
        <h4 className="text-sm font-medium">Configuration Summary</h4>
        <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
          <li>Device: {data.bluetooth.selectedDevice?.name || "Unknown"}</li>
          <li>Signal Threshold: {data.signalThreshold.threshold} dBm</li>
          <li>Monthly Passes: {data.passes.monthlyCount}</li>
          <li>WiFi: {data.wifi.ssid || "Not configured"}</li>
          <li>Timezone: {data.timezone.selected}</li>
        </ul>
      </div>

      {(bluetoothConfig.isError || passesConfig.isError || timezoneConfig.isError || completeOnboarding.isError) && (
        <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950 dark:text-red-300">
          Failed to save configuration. Please try again.
        </div>
      )}

      <Button onClick={handleComplete} disabled={isSubmitting} className="w-full" size="lg">
        {isSubmitting ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            Saving...
          </>
        ) : (
          "Complete Setup"
        )}
      </Button>
    </div>
  );
}
