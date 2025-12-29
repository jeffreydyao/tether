import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Wifi, Eye, EyeOff, Loader2, CheckCircle, AlertCircle } from "lucide-react";
import type { WizardStepProps } from "@/types/onboarding";
import { useWifiConfig } from "@/hooks/useOnboardingApi";
import { cn } from "@/lib/utils";

export function WifiConfigStep({ data, onDataChange, setCanProceed }: WizardStepProps): ReactNode {
  const [showPassword, setShowPassword] = useState(false);
  const { mutate: configureWifi, isPending, isSuccess, isError, error } = useWifiConfig();

  const ssid = data.wifi.ssid;
  const password = data.wifi.password;

  useEffect(() => {
    setCanProceed(ssid.trim().length > 0 && isSuccess);
  }, [ssid, isSuccess, setCanProceed]);

  const handleTest = () => {
    if (ssid.trim()) {
      configureWifi({ ssid, password });
    }
  };

  return (
    <div className="space-y-6">
      <div className="rounded-lg bg-muted/50 p-4 text-center">
        <Wifi className="mx-auto h-10 w-10 text-primary" />
        <p className="mt-2 text-sm text-muted-foreground">Connect to your home WiFi network for remote access</p>
      </div>

      <div className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="ssid">Network Name (SSID)</Label>
          <Input
            id="ssid"
            type="text"
            placeholder="Enter your WiFi network name"
            value={ssid}
            onChange={(e) => onDataChange("wifi", { ssid: e.target.value })}
            autoComplete="off"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="password">Password</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              placeholder="Enter your WiFi password"
              value={password}
              onChange={(e) => onDataChange("wifi", { password: e.target.value })}
              autoComplete="off"
              className="pr-10"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </div>

      <Button onClick={handleTest} disabled={!ssid.trim() || isPending} variant="outline" className="w-full">
        {isPending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            Testing connection...
          </>
        ) : (
          "Test Connection"
        )}
      </Button>

      {isSuccess && (
        <div
          className={cn(
            "flex items-center gap-2 rounded-lg border p-3",
            "border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950"
          )}
        >
          <CheckCircle className="h-5 w-5 text-green-600" />
          <p className="text-sm text-green-700 dark:text-green-300">Connection successful!</p>
        </div>
      )}

      {isError && (
        <div
          className={cn(
            "flex items-center gap-2 rounded-lg border p-3",
            "border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-950"
          )}
        >
          <AlertCircle className="h-5 w-5 text-red-600" />
          <p className="text-sm text-red-700 dark:text-red-300">
            {error?.message || "Failed to connect. Check your credentials."}
          </p>
        </div>
      )}

      <p className="text-center text-xs text-muted-foreground">
        After setup, you'll be able to access Tether from any device on this network.
      </p>
    </div>
  );
}
