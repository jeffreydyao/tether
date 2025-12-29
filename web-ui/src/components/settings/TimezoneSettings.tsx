import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Globe, Loader2, Clock, CheckCircle } from "lucide-react";
import { getConfig, updateTimezone } from "@/generated";
import type { ConfigResponse } from "@/generated";
import { COMMON_TIMEZONES } from "@/types/onboarding";

export function TimezoneSettings(): ReactNode {
  const queryClient = useQueryClient();
  const [selectedTimezone, setSelectedTimezone] = useState("");
  const [currentTime, setCurrentTime] = useState("");

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

  useEffect(() => {
    if (configQuery.data?.timezone) {
      setSelectedTimezone(configQuery.data.timezone);
    }
  }, [configQuery.data?.timezone]);

  useEffect(() => {
    if (selectedTimezone) {
      const updateTime = () => {
        try {
          setCurrentTime(
            new Intl.DateTimeFormat("en-US", {
              timeZone: selectedTimezone,
              hour: "2-digit",
              minute: "2-digit",
              hour12: true,
            }).format(new Date())
          );
        } catch {
          setCurrentTime("--:--");
        }
      };
      updateTime();
      const interval = setInterval(updateTime, 1000);
      return () => clearInterval(interval);
    }
  }, [selectedTimezone]);

  const updateMutation = useMutation({
    mutationFn: async (timezone: string) => {
      const response = await updateTimezone({
        body: { timezone },
      });
      if (response.error || !response.data) {
        throw new Error("Failed to update timezone");
      }
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });

  const hasChanges = configQuery.data?.timezone !== selectedTimezone;
  const selectedTz = COMMON_TIMEZONES.find((tz) => tz.id === selectedTimezone);

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
          <CardTitle className="text-base">Current Time</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-3">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
              <Clock className="h-6 w-6 text-primary" />
            </div>
            <div>
              <div className="font-mono text-2xl font-bold">{currentTime}</div>
              <div className="text-sm text-muted-foreground">
                {selectedTz?.displayName || selectedTimezone}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Select Timezone</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Select value={selectedTimezone} onValueChange={setSelectedTimezone}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Select a timezone">
                {selectedTz && (
                  <div className="flex items-center gap-2">
                    <Globe className="h-4 w-4" />
                    {selectedTz.displayName}
                  </div>
                )}
              </SelectValue>
            </SelectTrigger>
            <SelectContent className="max-h-80">
              <SelectGroup>
                <SelectLabel>Common Timezones</SelectLabel>
                {COMMON_TIMEZONES.map((tz) => (
                  <SelectItem key={tz.id} value={tz.id}>
                    <div className="flex items-center justify-between gap-4">
                      <span>{tz.displayName}</span>
                      <span className="text-xs text-muted-foreground">{tz.utcOffset}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>

          <p className="text-xs text-muted-foreground">
            This timezone is used for determining when monthly passes reset and for displaying times in the dashboard.
          </p>

          <Button
            className="w-full"
            onClick={() => updateMutation.mutate(selectedTimezone)}
            disabled={!hasChanges || updateMutation.isPending}
          >
            {updateMutation.isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : hasChanges ? (
              "Save Timezone"
            ) : (
              "No Changes"
            )}
          </Button>
        </CardContent>
      </Card>

      {updateMutation.isSuccess && (
        <div className="flex items-center gap-2 rounded-lg border border-green-200 bg-green-50 p-3 dark:border-green-800 dark:bg-green-950">
          <CheckCircle className="h-5 w-5 text-green-600" />
          <p className="text-sm text-green-700 dark:text-green-300">Timezone updated successfully!</p>
        </div>
      )}
    </div>
  );
}
