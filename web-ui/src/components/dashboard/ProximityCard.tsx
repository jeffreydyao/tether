import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, XCircle, AlertTriangle, RefreshCw, Bluetooth } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { cn } from "@/lib/utils";
import { checkProximity } from "@/generated";
import type { ProximityResponse } from "@/generated";

const PROXIMITY_POLL_INTERVAL = 2000;

const RSSI_THRESHOLDS = {
  EXCELLENT: -50,
  GOOD: -60,
  FAIR: -70,
  WEAK: -80,
} as const;

interface ProximityCardProps {
  className?: string;
}

export function ProximityCard({ className }: ProximityCardProps): ReactNode {
  const { data: proximityStatus, isLoading, isError, error, refetch, isFetching } = useQuery({
    queryKey: ["proximity", "status"],
    queryFn: async (): Promise<ProximityResponse> => {
      const response = await checkProximity();
      if (response.error || !response.data) {
        throw new Error("Failed to fetch proximity status");
      }
      return response.data;
    },
    refetchInterval: PROXIMITY_POLL_INTERVAL,
    refetchIntervalInBackground: false,
    staleTime: PROXIMITY_POLL_INTERVAL - 500,
  });

  const getSignalStrength = (rssi: number): "excellent" | "good" | "fair" | "weak" | "very-weak" => {
    if (rssi > RSSI_THRESHOLDS.EXCELLENT) return "excellent";
    if (rssi > RSSI_THRESHOLDS.GOOD) return "good";
    if (rssi > RSSI_THRESHOLDS.FAIR) return "fair";
    if (rssi > RSSI_THRESHOLDS.WEAK) return "weak";
    return "very-weak";
  };

  const getSignalColor = (rssi: number): string => {
    const strength = getSignalStrength(rssi);
    switch (strength) {
      case "excellent":
      case "good":
        return "text-green-500";
      case "fair":
        return "text-yellow-500";
      case "weak":
      case "very-weak":
        return "text-red-500";
    }
  };

  if (isLoading) {
    return (
      <Card className={cn("relative overflow-hidden", className)}>
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <Skeleton className="h-5 w-5 rounded" />
            <Skeleton className="h-6 w-32" />
          </div>
          <Skeleton className="mt-1 h-4 w-48" />
        </CardHeader>
        <CardContent className="flex flex-col items-center justify-center py-8">
          <Skeleton className="h-24 w-24 rounded-full" />
          <Skeleton className="mt-4 h-6 w-40" />
          <Skeleton className="mt-2 h-4 w-24" />
        </CardContent>
      </Card>
    );
  }

  if (isError) {
    return (
      <Card className={cn("relative overflow-hidden", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Bluetooth className="h-5 w-5" />
            Proximity Status
          </CardTitle>
          <CardDescription>Device tracking</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col items-center justify-center py-6">
          <Alert variant="destructive" className="mb-4">
            <AlertTriangle className="h-4 w-4" />
            <AlertDescription>
              {error instanceof Error ? error.message : "Failed to check proximity status"}
            </AlertDescription>
          </Alert>
          <Button variant="outline" onClick={() => refetch()} disabled={isFetching} className="gap-2">
            <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  const isNearby = proximityStatus?.is_nearby ?? false;
  const deviceName = proximityStatus?.device_name ?? "Unknown Device";
  const rssi = proximityStatus?.rssi_dbm;

  return (
    <Card className={cn("relative overflow-hidden", className)}>
      {isFetching && (
        <div className="absolute top-2 right-2">
          <div className="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
        </div>
      )}

      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-lg">
          <Bluetooth className="h-5 w-5" />
          Proximity Status
        </CardTitle>
        <CardDescription>Real-time device tracking</CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col items-center justify-center py-6">
        <div
          className={cn(
            "flex items-center justify-center rounded-full p-6 transition-all duration-300",
            isNearby ? "bg-green-100 dark:bg-green-900/30" : "bg-red-100 dark:bg-red-900/30"
          )}
        >
          {isNearby ? (
            <CheckCircle2 className="h-16 w-16 text-green-500 dark:text-green-400" />
          ) : (
            <XCircle className="h-16 w-16 text-red-500 dark:text-red-400" />
          )}
        </div>

        <p
          className={cn(
            "mt-4 text-xl font-semibold",
            isNearby ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"
          )}
        >
          {isNearby ? "Device Nearby" : "Device Away"}
        </p>

        <p className="mt-1 text-sm text-muted-foreground">{deviceName}</p>

        {rssi !== undefined && rssi !== null && (
          <div className="mt-3 flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Signal:</span>
            <span className={cn("font-mono text-sm font-medium", getSignalColor(rssi))}>{rssi} dBm</span>
            <span className="text-xs capitalize text-muted-foreground">({getSignalStrength(rssi)})</span>
          </div>
        )}

        <Button
          variant="ghost"
          size="sm"
          onClick={() => refetch()}
          disabled={isFetching}
          className="mt-4 gap-2 text-xs text-muted-foreground"
        >
          <RefreshCw className={cn("h-3 w-3", isFetching && "animate-spin")} />
          Refresh now
        </Button>
      </CardContent>
    </Card>
  );
}
