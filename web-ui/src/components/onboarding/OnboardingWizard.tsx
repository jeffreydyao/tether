import type { ReactNode } from "react";
import { useState, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Progress } from "@/components/ui/progress";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ChevronLeft } from "lucide-react";

import type { WizardStepData, WizardStepDefinition } from "@/types/onboarding";
import { DEFAULT_WIZARD_DATA } from "@/types/onboarding";

import { BluetoothScanStep } from "./steps/BluetoothScanStep";
import { SignalThresholdStep } from "./steps/SignalThresholdStep";
import { PassesConfigStep } from "./steps/PassesConfigStep";
import { WifiConfigStep } from "./steps/WifiConfigStep";
import { TimezoneStep } from "./steps/TimezoneStep";
import { CompletionStep } from "./steps/CompletionStep";

const WIZARD_STEPS: WizardStepDefinition[] = [
  {
    id: "bluetooth-scan",
    title: "Find Device",
    description: "Scan for your phone via Bluetooth",
    component: BluetoothScanStep,
    validate: (data) => data.bluetooth.selectedDevice !== null,
  },
  {
    id: "signal-threshold",
    title: "Set Range",
    description: "Configure proximity detection range",
    component: SignalThresholdStep,
    validate: (data) => data.signalThreshold.threshold >= -100 && data.signalThreshold.threshold <= -30,
  },
  {
    id: "passes-config",
    title: "Passes",
    description: "Set monthly emergency passes",
    component: PassesConfigStep,
    validate: (data) => data.passes.monthlyCount >= 0 && data.passes.monthlyCount <= 31,
  },
  {
    id: "wifi-config",
    title: "WiFi",
    description: "Connect to your home network",
    component: WifiConfigStep,
    validate: (data) => data.wifi.ssid.trim().length > 0,
  },
  {
    id: "timezone",
    title: "Timezone",
    description: "Set your local timezone",
    component: TimezoneStep,
    validate: (data) => data.timezone.selected.length > 0,
  },
  {
    id: "completion",
    title: "Complete",
    description: "Setup complete!",
    component: CompletionStep,
    validate: () => true,
  },
];

interface OnboardingWizardProps {
  onComplete?: () => void;
  initialStep?: number;
  initialData?: Partial<WizardStepData>;
}

export function OnboardingWizard({
  onComplete,
  initialStep = 0,
  initialData,
}: OnboardingWizardProps): ReactNode {
  const navigate = useNavigate();
  const [currentStep, setCurrentStep] = useState(initialStep);
  const [wizardData, setWizardData] = useState<WizardStepData>(() => ({
    ...DEFAULT_WIZARD_DATA,
    ...initialData,
  }));
  const [canProceed, setCanProceed] = useState(false);

  const currentStepDef = WIZARD_STEPS[currentStep];

  const progressPercent = useMemo(() => {
    const totalSteps = WIZARD_STEPS.length - 1;
    return Math.round((currentStep / totalSteps) * 100);
  }, [currentStep]);

  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === WIZARD_STEPS.length - 1;

  const handleDataChange = useCallback(
    <K extends keyof WizardStepData>(key: K, value: Partial<WizardStepData[K]>) => {
      setWizardData((prev) => ({
        ...prev,
        [key]: {
          ...prev[key],
          ...value,
        },
      }));
    },
    []
  );

  const handleNext = useCallback(() => {
    if (!currentStepDef.validate(wizardData)) {
      return;
    }

    if (isLastStep) {
      onComplete?.();
      navigate("/dashboard");
      return;
    }

    setCurrentStep((prev) => prev + 1);
    setCanProceed(false);
  }, [currentStepDef, wizardData, isLastStep, onComplete, navigate]);

  const handleBack = useCallback(() => {
    if (isFirstStep) return;
    setCurrentStep((prev) => prev - 1);
    const prevStep = WIZARD_STEPS[currentStep - 1];
    setCanProceed(prevStep.validate(wizardData));
  }, [isFirstStep, currentStep, wizardData]);

  const StepComponent = currentStepDef.component;

  return (
    <div className="flex min-h-screen flex-col bg-background">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container mx-auto max-w-lg px-4 py-4">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2">
              {!isFirstStep && !isLastStep && (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleBack}
                  className="h-8 w-8"
                  aria-label="Go back"
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
              )}
              <span className="text-sm font-medium text-muted-foreground">
                Step {currentStep + 1} of {WIZARD_STEPS.length}
              </span>
            </div>
            <span className="text-sm font-medium">{currentStepDef.title}</span>
          </div>

          {!isLastStep && (
            <Progress value={progressPercent} className="h-1.5" aria-label={`Setup progress: ${progressPercent}%`} />
          )}
        </div>
      </header>

      <main className="container mx-auto max-w-lg flex-1 px-4 py-6">
        <Card className="border-0 shadow-none sm:border sm:shadow-sm">
          <CardHeader className="pb-4">
            <CardTitle className="text-xl">{currentStepDef.title}</CardTitle>
            <CardDescription>{currentStepDef.description}</CardDescription>
          </CardHeader>
          <CardContent>
            <StepComponent
              data={wizardData}
              onDataChange={handleDataChange}
              onNext={handleNext}
              onBack={isFirstStep ? undefined : handleBack}
              canProceed={canProceed}
              setCanProceed={setCanProceed}
            />
          </CardContent>
        </Card>
      </main>

      {!isLastStep && (
        <footer className="sticky bottom-0 border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="container mx-auto max-w-lg px-4 py-4">
            <div className="flex gap-3">
              {!isFirstStep && (
                <Button variant="outline" onClick={handleBack} className="flex-1 sm:flex-none">
                  Back
                </Button>
              )}
              <Button onClick={handleNext} disabled={!canProceed} className="flex-1">
                {currentStep === WIZARD_STEPS.length - 2 ? "Complete Setup" : "Continue"}
              </Button>
            </div>
          </div>
        </footer>
      )}
    </div>
  );
}

export default OnboardingWizard;
