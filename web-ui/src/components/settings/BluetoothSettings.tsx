import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Bluetooth, BluetoothSearching, RefreshCw, Loader2, Signal, Check } from "lucide-react";
import { getConfig, scanDevices, updateBluetooth } from "@/generated";
import type { ConfigResponse, ScanDevicesResponse, DiscoveredDevice } from "@/generated";
import { cn } from "@/lib/utils";

export function BluetoothSettings(): ReactNode {
  const queryClient = useQueryClient();
  const [isScanning, setIsScanning] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState<DiscoveredDevice | null>(null);
  const [threshold, setThreshold] = useState(-70);

  const configQuery = useQuery({
    queryKey: ["config"],
    queryFn: async (): Promise<ConfigResponse> => {
      const response = await getConfig();
      if (response.error || !response.data) {
        throw new Error("Failed to get config");
      }
      return response.data;
    },
  });

  const scanQuery = useQuery({
    queryKey: ["bluetooth", "scan"],
    queryFn: async (): Promise<ScanDevicesResponse> => {
      const response = await scanDevices();
      if (response.error || !response.data) {
        throw new Error("Failed to scan devices");
      }
      return response.data;
    },
    enabled: isScanning,
    staleTime: 0,
  });

  const updateMutation = useMutation({
    mutationFn: async (device: DiscoveredDevice) => {
      const response = await updateBluetooth({
        body: {
          target_address: device.address,
          target_name: device.name ?? "Phone",
          rssi_threshold: threshold,
        },
      });
      if (response.error || !response.data) {
        throw new Error("Failed to update Bluetooth");
      }
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      setSelectedDevice(null);
      setIsScanning(false);
    },
  });

  const currentDevice = configQuery.data?.bluetooth;

  const getSignalColor = (rssi: number): string => {
    if (rssi >= -50) return "text-green-500";
    if (rssi >= -60) return "text-lime-500";
    if (rssi >= -70) return "text-yellow-500";
    return "text-red-500";
  };

  if (configQuery.isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Current Device</CardTitle>
        </CardHeader>
        <CardContent>
          {currentDevice?.is_configured ? (
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                  <Bluetooth className="h-6 w-6 text-primary" />
                </div>
                <div>
                  <div className="font-medium">{currentDevice.target_name}</div>
                  <div className="text-sm text-muted-foreground">{currentDevice.target_address}</div>
                </div>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Threshold</span>
                <span className="font-medium">{currentDevice.rssi_threshold} dBm</span>
              </div>
            </div>
          ) : (
            <div className="py-4 text-center">
              <Bluetooth className="mx-auto mb-2 h-8 w-8 text-muted-foreground" />
              <p className="text-muted-foreground">No device configured</p>
            </div>
          )}
        </CardContent>
      </Card>

      <Separator />

      {isScanning ? (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {scanQuery.isLoading ? "Scanning..." : `${scanQuery.data?.devices?.length ?? 0} devices found`}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => scanQuery.refetch()}
              disabled={scanQuery.isLoading}
            >
              <RefreshCw className={cn("mr-2 h-4 w-4", scanQuery.isLoading && "animate-spin")} />
              Rescan
            </Button>
          </div>

          <div className="max-h-64 space-y-2 overflow-y-auto">
            {scanQuery.data?.devices?.map((device) => (
              <Card
                key={device.address}
                className={cn(
                  "cursor-pointer transition-colors hover:bg-accent",
                  selectedDevice?.address === device.address && "border-primary"
                )}
                onClick={() => setSelectedDevice(device)}
              >
                <CardContent className="px-4 py-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <Bluetooth className="h-5 w-5 text-muted-foreground" />
                      <div>
                        <div className="font-medium">{device.name || "Unknown Device"}</div>
                        <div className="text-xs text-muted-foreground">{device.address}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {device.rssi_dbm && (
                        <div className={cn("text-sm", getSignalColor(device.rssi_dbm))}>
                          <Signal className="mr-1 inline h-4 w-4" />
                          {device.rssi_dbm} dBm
                        </div>
                      )}
                      {currentDevice?.target_address === device.address && (
                        <Badge variant="secondary">Current</Badge>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>

          {selectedDevice && (
            <Card>
              <CardContent className="py-4">
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium">Signal Threshold</label>
                    <div className="mt-2 flex items-center gap-4">
                      <Slider
                        value={[threshold]}
                        onValueChange={([val]) => setThreshold(val)}
                        min={-100}
                        max={-30}
                        step={5}
                        className="flex-1"
                      />
                      <span className="w-16 text-right text-sm font-medium">{threshold} dBm</span>
                    </div>
                    <div className="mt-1 flex justify-between text-xs text-muted-foreground">
                      <span>Far</span>
                      <span>Close</span>
                    </div>
                  </div>
                  <Button
                    className="w-full"
                    onClick={() => updateMutation.mutate(selectedDevice)}
                    disabled={updateMutation.isPending}
                  >
                    {updateMutation.isPending ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <Check className="mr-2 h-4 w-4" />
                    )}
                    Save Device
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          <Button variant="outline" className="w-full" onClick={() => setIsScanning(false)}>
            Cancel
          </Button>
        </div>
      ) : (
        <Button variant="outline" className="w-full" onClick={() => setIsScanning(true)}>
          <BluetoothSearching className="mr-2 h-4 w-4" />
          Change Device
        </Button>
      )}
    </div>
  );
}
