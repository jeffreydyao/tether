import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Smartphone } from "lucide-react";

export default function OnboardingPage(): ReactNode {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <Smartphone className="h-8 w-8 text-primary" />
          </div>
          <CardTitle className="text-2xl">Welcome to Tether</CardTitle>
          <CardDescription>
            Let's set up your device to help you maintain healthy phone habits.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-3">
            <div className="flex items-start gap-3 rounded-lg bg-muted/50 p-3">
              <div className="mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-primary/20 text-xs font-medium text-primary">
                1
              </div>
              <div>
                <p className="text-sm font-medium">Connect your phone</p>
                <p className="text-xs text-muted-foreground">
                  We'll detect your phone via Bluetooth
                </p>
              </div>
            </div>

            <div className="flex items-start gap-3 rounded-lg bg-muted/50 p-3">
              <div className="mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-primary/20 text-xs font-medium text-primary">
                2
              </div>
              <div>
                <p className="text-sm font-medium">Set your boundaries</p>
                <p className="text-xs text-muted-foreground">
                  Configure passes for nights you need flexibility
                </p>
              </div>
            </div>

            <div className="flex items-start gap-3 rounded-lg bg-muted/50 p-3">
              <div className="mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-primary/20 text-xs font-medium text-primary">
                3
              </div>
              <div>
                <p className="text-sm font-medium">Connect to Wi-Fi</p>
                <p className="text-xs text-muted-foreground">Enable remote access from anywhere</p>
              </div>
            </div>
          </div>

          <Button className="w-full" size="lg">
            Get Started
          </Button>

          <p className="text-center text-xs text-muted-foreground">
            Full onboarding wizard coming in Phase 2.3
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
