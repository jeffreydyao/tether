import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, History, FileText } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { cn } from "@/lib/utils";
import { getPassHistory } from "@/generated";
import type { PassHistoryResponse, PassHistoryEntry } from "@/generated";

interface PassHistoryListProps {
  className?: string;
  showNavigation?: boolean;
  maxEntries?: number;
}

export function PassHistoryList({ className, showNavigation = true, maxEntries }: PassHistoryListProps): ReactNode {
  const [selectedMonth, setSelectedMonth] = useState<string>(() => {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
  });

  const { data: historyResponse, isLoading, isError, error, refetch } = useQuery({
    queryKey: ["passes", "history", selectedMonth],
    queryFn: async (): Promise<PassHistoryResponse> => {
      const response = await getPassHistory({ query: { month: selectedMonth } });
      if (response.error || !response.data) {
        throw new Error("Failed to fetch pass history");
      }
      return response.data;
    },
    staleTime: 60000,
  });

  const goToPreviousMonth = () => {
    const [year, month] = selectedMonth.split("-").map(Number);
    const date = new Date(year, month - 2, 1);
    setSelectedMonth(`${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`);
  };

  const goToNextMonth = () => {
    const [year, month] = selectedMonth.split("-").map(Number);
    const date = new Date(year, month, 1);
    setSelectedMonth(`${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`);
  };

  const canGoToNextMonth = (): boolean => {
    const now = new Date();
    const currentMonth = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
    return selectedMonth < currentMonth;
  };

  const formatMonthDisplay = (monthStr: string): string => {
    const [year, month] = monthStr.split("-").map(Number);
    return new Intl.DateTimeFormat("en-US", { month: "long", year: "numeric" }).format(new Date(year, month - 1));
  };

  const formatDateTime = (utcTimestamp: string): { date: string; time: string; relative: string } => {
    const date = new Date(utcTimestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    const dateStr = new Intl.DateTimeFormat("en-US", {
      weekday: "short",
      month: "short",
      day: "numeric",
    }).format(date);

    const timeStr = new Intl.DateTimeFormat("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    }).format(date);

    let relative: string;
    if (diffDays === 0) {
      relative = "Today";
    } else if (diffDays === 1) {
      relative = "Yesterday";
    } else if (diffDays < 7) {
      relative = `${diffDays} days ago`;
    } else if (diffDays < 30) {
      const weeks = Math.floor(diffDays / 7);
      relative = `${weeks} ${weeks === 1 ? "week" : "weeks"} ago`;
    } else {
      relative = dateStr;
    }

    return { date: dateStr, time: timeStr, relative };
  };

  if (isLoading) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Skeleton className="h-5 w-5 rounded" />
              <Skeleton className="h-6 w-24" />
            </div>
            {showNavigation && (
              <div className="flex items-center gap-1">
                <Skeleton className="h-8 w-8 rounded" />
                <Skeleton className="h-5 w-32" />
                <Skeleton className="h-8 w-8 rounded" />
              </div>
            )}
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="flex items-start gap-3 rounded-lg border p-3">
              <Skeleton className="h-10 w-10 rounded-full" />
              <div className="flex-1 space-y-2">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-3 w-full" />
              </div>
            </div>
          ))}
        </CardContent>
      </Card>
    );
  }

  if (isError) {
    return (
      <Card className={cn("relative", className)}>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-lg">
            <History className="h-5 w-5" />
            Pass History
          </CardTitle>
        </CardHeader>
        <CardContent>
          <Alert variant="destructive">
            <AlertDescription>{error instanceof Error ? error.message : "Failed to load history"}</AlertDescription>
          </Alert>
          <Button variant="outline" onClick={() => refetch()} className="mt-4 w-full">
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  const entries = historyResponse?.entries ?? [];
  const displayEntries = maxEntries ? entries.slice(0, maxEntries) : entries;

  return (
    <Card className={cn("relative", className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2 text-lg">
            <History className="h-5 w-5" />
            Pass History
          </CardTitle>

          {showNavigation && (
            <div className="flex items-center gap-1">
              <Button variant="ghost" size="icon" className="h-8 w-8" onClick={goToPreviousMonth}>
                <ChevronLeft className="h-4 w-4" />
                <span className="sr-only">Previous month</span>
              </Button>

              <span className="min-w-[140px] text-center text-sm font-medium">
                {formatMonthDisplay(selectedMonth)}
              </span>

              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={goToNextMonth}
                disabled={!canGoToNextMonth()}
              >
                <ChevronRight className="h-4 w-4" />
                <span className="sr-only">Next month</span>
              </Button>
            </div>
          )}
        </div>
        {!showNavigation && <CardDescription>{formatMonthDisplay(selectedMonth)}</CardDescription>}
      </CardHeader>

      <CardContent>
        {entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <div className="mb-3 rounded-full bg-muted p-3">
              <FileText className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-sm font-medium">No passes used</p>
            <p className="mt-1 text-xs text-muted-foreground">
              You haven&apos;t used any passes in {formatMonthDisplay(selectedMonth)}
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {displayEntries.map((entry: PassHistoryEntry, index: number) => {
              const { date, time, relative } = formatDateTime(entry.used_at_utc);

              return (
                <div
                  key={entry.used_at_utc + index}
                  className="flex items-start gap-3 rounded-lg border bg-card p-3 transition-colors hover:bg-accent/50"
                >
                  <div className="flex min-w-[60px] flex-col items-center justify-center rounded bg-muted px-2 py-1">
                    <span className="text-xs text-muted-foreground">{date}</span>
                    <span className="text-xs font-medium">{time}</span>
                  </div>

                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs text-muted-foreground">{relative}</span>
                    </div>
                    <p className="mt-1 line-clamp-2 text-sm">{entry.reason}</p>
                  </div>
                </div>
              );
            })}

            {maxEntries && entries.length > maxEntries && (
              <p className="pt-2 text-center text-xs text-muted-foreground">
                +{entries.length - maxEntries} more entries
              </p>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
