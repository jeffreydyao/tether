import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Ticket, AlertCircle } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { cn } from "@/lib/utils";
import { getPasses } from "@/generated";
import type { PassesResponse } from "@/generated";
import { UsePassDialog } from "./UsePassDialog";

interface PassesCardProps {
  className?: string;
}

export function PassesCard({ className }: PassesCardProps): ReactNode {
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const { data: passesInfo, isLoading, isError, error, refetch } = useQuery({
    queryKey: ["passes", "remaining"],
    queryFn: async (): Promise<PassesResponse> => {
      const response = await getPasses();
      if (response.error || !response.data) {
        throw new Error("Failed to fetch passes info");
      }
      return response.data;
    },
    staleTime: 30000,
  });

  const getProgressInfo = (remaining: number, total: number) => {
    const usedPercentage = ((total - remaining) / total) * 100;
    const remainingPercentage = (remaining / total) * 100;

    let colorClass: string;
    if (remainingPercentage > 50) {
      colorClass = "bg-green-500";
    } else if (remainingPercentage > 25) {
      colorClass = "bg-yellow-500";
    } else {
      colorClass = "bg-red-500";
    }

    return { usedPercentage, remainingPercentage, colorClass };
  };

  const getRemainingColor = (remaining: number, total: number): string => {
    const percentage = (remaining / total) * 100;
    if (percentage > 50) return "text-green-600 dark:text-green-400";
    if (percentage > 25) return "text-yellow-600 dark:text-yellow-400";
    return "text-red-600 dark:text-red-400";
  };

  const getCurrentMonthName = (): string => {
    return new Intl.DateTimeFormat("en-US", { month: "long", year: "numeric" }).format(new Date());
  };

  if (isLoading) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <Skeleton className="h-5 w-5 rounded" />
            <Skeleton className="h-6 w-32" />
          </div>
          <Skeleton className="mt-1 h-4 w-40" />
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-baseline justify-center gap-2">
            <Skeleton className="h-16 w-20" />
            <Skeleton className="h-6 w-16" />
          </div>
          <Skeleton className="h-2 w-full rounded-full" />
          <Skeleton className="h-10 w-full rounded-md" />
        </CardContent>
      </Card>
    );
  }

  if (isError) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Ticket className="h-5 w-5" />
            Passes
          </CardTitle>
          <CardDescription>{getCurrentMonthName()}</CardDescription>
        </CardHeader>
        <CardContent>
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error instanceof Error ? error.message : "Failed to load passes"}</AlertDescription>
          </Alert>
          <Button variant="outline" onClick={() => refetch()} className="mt-4 w-full">
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  const remaining = passesInfo?.remaining ?? 0;
  const total = passesInfo?.total_per_month ?? 0;
  const { usedPercentage, colorClass } = getProgressInfo(remaining, total);
  const hasPassesRemaining = remaining > 0;

  return (
    <>
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Ticket className="h-5 w-5" />
            Passes
          </CardTitle>
          <CardDescription>{getCurrentMonthName()}</CardDescription>
        </CardHeader>

        <CardContent className="space-y-4">
          <div className="flex items-baseline justify-center gap-2 py-4">
            <span className={cn("text-5xl font-bold tabular-nums", getRemainingColor(remaining, total))}>
              {remaining}
            </span>
            <span className="text-lg text-muted-foreground">/ {total}</span>
          </div>

          <div className="space-y-1.5">
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>Used this month</span>
              <span>
                {total - remaining} of {total}
              </span>
            </div>
            <div className="relative h-2 w-full overflow-hidden rounded-full bg-secondary">
              <div
                className={cn("h-full transition-all duration-500", colorClass)}
                style={{ width: `${usedPercentage}%` }}
              />
            </div>
          </div>

          <Button
            className="w-full"
            variant={hasPassesRemaining ? "default" : "secondary"}
            disabled={!hasPassesRemaining}
            onClick={() => setIsDialogOpen(true)}
          >
            {hasPassesRemaining ? "Use Pass" : "No Passes Remaining"}
          </Button>

          {hasPassesRemaining && remaining <= 2 && (
            <p className="text-center text-xs text-yellow-600 dark:text-yellow-400">
              {remaining === 1 ? "Last pass for this month!" : `Only ${remaining} passes remaining`}
            </p>
          )}
        </CardContent>
      </Card>

      <UsePassDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        currentRemaining={remaining}
        totalForMonth={total}
      />
    </>
  );
}
