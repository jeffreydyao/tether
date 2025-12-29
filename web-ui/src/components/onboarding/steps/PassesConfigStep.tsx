import type { ReactNode } from "react";
import { useEffect } from "react";
import { Slider } from "@/components/ui/slider";
import { Ticket } from "lucide-react";
import type { WizardStepProps } from "@/types/onboarding";
import { cn } from "@/lib/utils";

const PRESETS = [3, 5, 7, 10];

export function PassesConfigStep({ data, onDataChange, setCanProceed }: WizardStepProps): ReactNode {
  const monthlyCount = data.passes.monthlyCount;

  useEffect(() => {
    setCanProceed(monthlyCount >= 0 && monthlyCount <= 31);
  }, [monthlyCount, setCanProceed]);

  const handlePresetClick = (value: number) => {
    onDataChange("passes", { monthlyCount: value });
  };

  const handleSliderChange = (value: number[]) => {
    onDataChange("passes", { monthlyCount: value[0] });
  };

  return (
    <div className="space-y-6">
      <div className="rounded-lg bg-muted/50 p-4 text-center">
        <Ticket className="mx-auto h-10 w-10 text-primary" />
        <p className="mt-2 text-3xl font-bold">{monthlyCount}</p>
        <p className="text-sm text-muted-foreground">passes per month</p>
      </div>

      <div>
        <p className="mb-3 text-sm font-medium">Quick select</p>
        <div className="grid grid-cols-4 gap-2">
          {PRESETS.map((preset) => (
            <button
              key={preset}
              onClick={() => handlePresetClick(preset)}
              className={cn(
                "rounded-lg border py-3 text-center font-medium transition-colors",
                monthlyCount === preset
                  ? "border-primary bg-primary text-primary-foreground"
                  : "border-border hover:bg-muted/50"
              )}
            >
              {preset}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">Custom amount</label>
          <span className="text-sm font-bold">{monthlyCount}</span>
        </div>
        <Slider
          value={[monthlyCount]}
          onValueChange={handleSliderChange}
          min={0}
          max={31}
          step={1}
          className="w-full"
        />
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>0 (strict)</span>
          <span>31 (daily)</span>
        </div>
      </div>

      <div className="rounded-lg border bg-card p-3">
        <p className="text-xs text-muted-foreground">
          Passes let you keep your phone nearby on nights when you need it. Use them for travel, emergencies, or when
          you just need a break. Passes reset on the first of each month.
        </p>
      </div>
    </div>
  );
}
