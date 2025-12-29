import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Ticket, Loader2, AlertCircle, CheckCircle } from "lucide-react";
import { getConfig, getPasses, updatePassesPerMonth } from "@/generated";
import type { ConfigResponse, PassesResponse } from "@/generated";

export function PassesSettings(): ReactNode {
  const queryClient = useQueryClient();
  const [passesPerMonth, setPassesPerMonth] = useState(3);

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

  const passesQuery = useQuery({
    queryKey: ["passes", "remaining"],
    queryFn: async (): Promise<PassesResponse> => {
      const response = await getPasses();
      if (response.error || !response.data) {
        throw new Error("Failed to get passes");
      }
      return response.data;
    },
  });

  useEffect(() => {
    if (configQuery.data?.passes_per_month !== undefined) {
      setPassesPerMonth(configQuery.data.passes_per_month);
    }
  }, [configQuery.data?.passes_per_month]);

  const updateMutation = useMutation({
    mutationFn: async (value: number) => {
      const response = await updatePassesPerMonth({
        body: { per_month: value },
      });
      if (response.error || !response.data) {
        throw new Error("Failed to update passes");
      }
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      queryClient.invalidateQueries({ queryKey: ["passes"] });
    },
  });

  const hasChanges = configQuery.data?.passes_per_month !== passesPerMonth;

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
          <CardTitle className="text-base">Current Month</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                <Ticket className="h-6 w-6 text-primary" />
              </div>
              <div>
                <div className="text-2xl font-bold">
                  {passesQuery.isLoading ? (
                    <Loader2 className="h-5 w-5 animate-spin" />
                  ) : (
                    passesQuery.data?.remaining ?? 0
                  )}
                </div>
                <div className="text-sm text-muted-foreground">passes remaining</div>
              </div>
            </div>
            <Badge variant="secondary">{configQuery.data?.passes_per_month ?? 0} / month</Badge>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Monthly Allowance</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="passes-per-month">Passes Per Month</Label>
            <Input
              id="passes-per-month"
              type="number"
              min={0}
              max={31}
              value={passesPerMonth}
              onChange={(e) => setPassesPerMonth(Number.parseInt(e.target.value, 10) || 0)}
            />
            <p className="text-xs text-muted-foreground">
              The number of emergency passes available each month. Passes refresh on the first day of each month.
            </p>
          </div>

          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription className="text-sm">
              Changes will take effect on the first day of next month.
            </AlertDescription>
          </Alert>

          <Button
            className="w-full"
            onClick={() => updateMutation.mutate(passesPerMonth)}
            disabled={!hasChanges || updateMutation.isPending}
          >
            {updateMutation.isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : hasChanges ? (
              "Save Changes"
            ) : (
              "No Changes"
            )}
          </Button>
        </CardContent>
      </Card>

      {updateMutation.isSuccess && (
        <div className="flex items-center gap-2 rounded-lg border border-green-200 bg-green-50 p-3 dark:border-green-800 dark:bg-green-950">
          <CheckCircle className="h-5 w-5 text-green-600" />
          <p className="text-sm text-green-700 dark:text-green-300">
            Passes updated. Changes will apply next month.
          </p>
        </div>
      )}
    </div>
  );
}
