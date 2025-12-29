import type { ReactNode } from "react";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Wifi, Eye, EyeOff, Loader2, CheckCircle, AlertCircle } from "lucide-react";
import { updateWifi } from "@/generated";

export function WifiSettings(): ReactNode {
  const [ssid, setSsid] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  const wifiMutation = useMutation({
    mutationFn: async () => {
      const response = await updateWifi({
        body: {
          networks: [
            {
              ssid,
              password,
              is_primary: true,
            },
          ],
        },
      });
      if (response.error || !response.data) {
        throw new Error("Failed to update WiFi");
      }
      return response.data;
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (ssid.trim()) {
      wifiMutation.mutate();
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Wifi className="h-5 w-5" />
            Configure WiFi
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="ssid">Network Name (SSID)</Label>
              <Input
                id="ssid"
                type="text"
                placeholder="Enter your WiFi network name"
                value={ssid}
                onChange={(e) => setSsid(e.target.value)}
                autoComplete="off"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="wifi-password">Password</Label>
              <div className="relative">
                <Input
                  id="wifi-password"
                  type={showPassword ? "text" : "password"}
                  placeholder="Enter your WiFi password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete="new-password"
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

            <Button type="submit" className="w-full" disabled={!ssid.trim() || wifiMutation.isPending}>
              {wifiMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Saving...
                </>
              ) : (
                "Save WiFi Configuration"
              )}
            </Button>
          </form>
        </CardContent>
      </Card>

      {wifiMutation.isSuccess && (
        <div className="flex items-center gap-2 rounded-lg border border-green-200 bg-green-50 p-3 dark:border-green-800 dark:bg-green-950">
          <CheckCircle className="h-5 w-5 text-green-600" />
          <p className="text-sm text-green-700 dark:text-green-300">WiFi configuration saved successfully!</p>
        </div>
      )}

      {wifiMutation.isError && (
        <div className="flex items-center gap-2 rounded-lg border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-950">
          <AlertCircle className="h-5 w-5 text-red-600" />
          <p className="text-sm text-red-700 dark:text-red-300">Failed to save WiFi configuration. Please try again.</p>
        </div>
      )}

      <p className="text-center text-xs text-muted-foreground">
        Changes will take effect after the device restarts.
      </p>
    </div>
  );
}
