import type { ReactNode } from "react";
import { useEffect } from "react";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Clock } from "lucide-react";
import type { WizardStepProps } from "@/types/onboarding";
import { COMMON_TIMEZONES } from "@/types/onboarding";

export function TimezoneStep({ data, onDataChange, setCanProceed }: WizardStepProps): ReactNode {
  const selected = data.timezone.selected;

  useEffect(() => {
    // Auto-detect timezone on mount
    if (!selected) {
      const detected = Intl.DateTimeFormat().resolvedOptions().timeZone;
      const match = COMMON_TIMEZONES.find((tz) => tz.id === detected);
      if (match) {
        onDataChange("timezone", { selected: match.id });
      }
    }
  }, [selected, onDataChange]);

  useEffect(() => {
    setCanProceed(selected.length > 0);
  }, [selected, setCanProceed]);

  const handleChange = (value: string) => {
    onDataChange("timezone", { selected: value });
  };

  const selectedTimezone = COMMON_TIMEZONES.find((tz) => tz.id === selected);

  return (
    <div className="space-y-6">
      <div className="rounded-lg bg-muted/50 p-4 text-center">
        <Clock className="mx-auto h-10 w-10 text-primary" />
        <p className="mt-2 text-sm text-muted-foreground">Set your local timezone for accurate tracking</p>
      </div>

      <div className="space-y-2">
        <Select value={selected} onValueChange={handleChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select your timezone" />
          </SelectTrigger>
          <SelectContent>
            {COMMON_TIMEZONES.map((tz) => (
              <SelectItem key={tz.id} value={tz.id}>
                <div className="flex items-center justify-between gap-4">
                  <span>{tz.displayName}</span>
                  <span className="text-xs text-muted-foreground">{tz.utcOffset}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedTimezone && (
        <div className="rounded-lg border bg-card p-3 text-center">
          <p className="text-sm font-medium">{selectedTimezone.displayName}</p>
          <p className="text-xs text-muted-foreground">{selectedTimezone.utcOffset}</p>
        </div>
      )}

      <p className="text-center text-xs text-muted-foreground">
        This determines when your nightly tracking period starts and ends.
      </p>
    </div>
  );
}
