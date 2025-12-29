import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { usePass } from "@/generated";
import type { PassesResponse, PassHistoryResponse, PassHistoryEntry } from "@/generated";

interface UsePassDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentRemaining: number;
  totalForMonth: number;
}

const MIN_REASON_LENGTH = 10;
const MAX_REASON_LENGTH = 500;

export function UsePassDialog({
  open,
  onOpenChange,
  currentRemaining,
  totalForMonth: _totalForMonth,
}: UsePassDialogProps): ReactNode {
  const [reason, setReason] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);

  const queryClient = useQueryClient();

  useEffect(() => {
    if (!open) {
      const timer = setTimeout(() => {
        setReason("");
        setValidationError(null);
      }, 200);
      return () => clearTimeout(timer);
    }
  }, [open]);

  const usePassMutation = useMutation({
    mutationFn: async (passReason: string) => {
      const response = await usePass({ body: { reason: passReason } });
      if (response.error || !response.data) {
        throw new Error("Failed to use pass");
      }
      return response.data;
    },
    onMutate: async (newReason) => {
      await queryClient.cancelQueries({ queryKey: ["passes"] });

      const previousPasses = queryClient.getQueryData<PassesResponse>(["passes", "remaining"]);

      if (previousPasses) {
        queryClient.setQueryData<PassesResponse>(["passes", "remaining"], {
          ...previousPasses,
          remaining: Math.max(0, previousPasses.remaining - 1),
          used_this_month: previousPasses.used_this_month + 1,
        });
      }

      const currentMonth = new Date().toISOString().slice(0, 7);
      const historyKey = ["passes", "history", currentMonth];
      const currentHistory = queryClient.getQueryData<PassHistoryResponse>(historyKey);

      if (currentHistory) {
        const newEntry: PassHistoryEntry = {
          used_at_utc: new Date().toISOString(),
          reason: newReason,
        };
        queryClient.setQueryData<PassHistoryResponse>(historyKey, {
          ...currentHistory,
          entries: [newEntry, ...currentHistory.entries],
          total_used: currentHistory.total_used + 1,
        });
      }

      return { previousPasses, currentHistory, historyKey };
    },
    onError: (_err, _newReason, context) => {
      if (context?.previousPasses) {
        queryClient.setQueryData(["passes", "remaining"], context.previousPasses);
      }
      if (context?.currentHistory && context?.historyKey) {
        queryClient.setQueryData(context.historyKey, context.currentHistory);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["passes"] });
    },
    onSuccess: () => {
      onOpenChange(false);
    },
  });

  const validateReason = (value: string): string | null => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return "Please provide a reason for using this pass";
    }
    if (trimmed.length < MIN_REASON_LENGTH) {
      return `Reason must be at least ${MIN_REASON_LENGTH} characters`;
    }
    if (trimmed.length > MAX_REASON_LENGTH) {
      return `Reason cannot exceed ${MAX_REASON_LENGTH} characters`;
    }
    return null;
  };

  const handleSubmit = () => {
    const error = validateReason(reason);
    if (error) {
      setValidationError(error);
      return;
    }

    setValidationError(null);
    usePassMutation.mutate(reason.trim());
  };

  const handleReasonChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    setReason(value);
    if (validationError) {
      setValidationError(null);
    }
  };

  const isSubmitting = usePassMutation.isPending;
  const mutationError = usePassMutation.error;
  const remainingAfterUse = currentRemaining - 1;
  const isLastPass = currentRemaining === 1;
  const characterCount = reason.trim().length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Use Emergency Pass</DialogTitle>
          <DialogDescription>
            You have {currentRemaining} {currentRemaining === 1 ? "pass" : "passes"} remaining this month.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {isLastPass && (
            <Alert variant="destructive" className="border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
              <AlertTriangle className="h-4 w-4 text-yellow-600" />
              <AlertDescription className="text-yellow-700 dark:text-yellow-400">
                This is your last pass for the month! Use it wisely.
              </AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label htmlFor="reason" className="text-sm font-medium">
              Reason <span className="text-destructive">*</span>
            </Label>
            <Textarea
              id="reason"
              placeholder="Why do you need to use an emergency pass? (e.g., early morning flight, medical emergency...)"
              value={reason}
              onChange={handleReasonChange}
              disabled={isSubmitting}
              className={cn(
                "min-h-[100px] resize-none",
                validationError && "border-destructive focus-visible:ring-destructive"
              )}
              maxLength={MAX_REASON_LENGTH}
            />
            <div className="flex justify-between text-xs">
              {validationError ? (
                <span className="text-destructive">{validationError}</span>
              ) : (
                <span className="text-muted-foreground">Minimum {MIN_REASON_LENGTH} characters required</span>
              )}
              <span
                className={cn(
                  "tabular-nums text-muted-foreground",
                  characterCount > MAX_REASON_LENGTH * 0.9 && "text-yellow-600",
                  characterCount >= MAX_REASON_LENGTH && "text-destructive"
                )}
              >
                {characterCount}/{MAX_REASON_LENGTH}
              </span>
            </div>
          </div>

          {mutationError && (
            <Alert variant="destructive">
              <AlertDescription>
                {mutationError instanceof Error ? mutationError.message : "Failed to use pass. Please try again."}
              </AlertDescription>
            </Alert>
          )}

          <p className="text-sm text-muted-foreground">
            After using this pass, you will have{" "}
            <span className="font-semibold text-foreground">
              {remainingAfterUse} {remainingAfterUse === 1 ? "pass" : "passes"}
            </span>{" "}
            remaining for the month.
          </p>
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isSubmitting}>
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSubmit}
            disabled={isSubmitting || reason.trim().length < MIN_REASON_LENGTH}
            className="gap-2"
          >
            {isSubmitting ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Using Pass...
              </>
            ) : (
              "Confirm Use Pass"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
