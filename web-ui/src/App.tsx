import type { ReactNode } from "react";
import { Suspense, lazy } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route, Navigate, Outlet, useLocation } from "react-router-dom";
import { ErrorBoundary } from "react-error-boundary";
import { ThemeProvider } from "next-themes";
import { Toaster } from "@/components/ui/sonner";
import { queryClient } from "@/lib/queryClient";
import { useOnboardingState } from "@/hooks/useOnboardingState";
import { AppShell } from "@/components/layout/AppShell";

const OnboardingPage = lazy(() => import("@/pages/OnboardingPage"));
const DashboardPage = lazy(() => import("@/pages/DashboardPage"));

function PageLoadingFallback(): ReactNode {
  return (
    <div className="flex h-screen w-full items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-4">
        <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    </div>
  );
}

function LoadingSpinner(): ReactNode {
  return (
    <div className="flex h-full w-full items-center justify-center p-8">
      <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
    </div>
  );
}

interface ErrorFallbackProps {
  error: Error;
  resetErrorBoundary: () => void;
}

function ErrorFallback({ error, resetErrorBoundary }: ErrorFallbackProps): ReactNode {
  return (
    <div className="flex h-screen w-full flex-col items-center justify-center gap-4 bg-background p-4">
      <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-6 text-center">
        <h2 className="mb-2 text-lg font-semibold text-destructive">Something went wrong</h2>
        <p className="mb-4 text-sm text-muted-foreground">
          {error.message || "An unexpected error occurred"}
        </p>
        <button
          onClick={resetErrorBoundary}
          className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          Try again
        </button>
      </div>
    </div>
  );
}

interface ProtectedRouteProps {
  requireOnboarding?: boolean;
  redirectTo?: string;
  children?: ReactNode;
}

function ProtectedRoute({
  requireOnboarding = true,
  redirectTo,
  children,
}: ProtectedRouteProps): ReactNode {
  const location = useLocation();
  const { isOnboardingComplete, isInitialLoading } = useOnboardingState();

  if (isInitialLoading) {
    return <LoadingSpinner />;
  }

  if (requireOnboarding && !isOnboardingComplete) {
    return <Navigate to="/onboarding" state={{ from: location }} replace />;
  }

  if (!requireOnboarding && isOnboardingComplete) {
    return <Navigate to={redirectTo || "/dashboard"} replace />;
  }

  return children ?? <Outlet />;
}

function RootRedirect(): ReactNode {
  const { isOnboardingComplete, isInitialLoading } = useOnboardingState();

  if (isInitialLoading) {
    return <PageLoadingFallback />;
  }

  return <Navigate to={isOnboardingComplete ? "/dashboard" : "/onboarding"} replace />;
}

function AppRoutes(): ReactNode {
  return (
    <Routes>
      <Route path="/" element={<RootRedirect />} />

      <Route
        path="/onboarding"
        element={
          <ProtectedRoute requireOnboarding={false} redirectTo="/dashboard">
            <Suspense fallback={<PageLoadingFallback />}>
              <OnboardingPage />
            </Suspense>
          </ProtectedRoute>
        }
      />

      <Route
        path="/dashboard"
        element={
          <ProtectedRoute requireOnboarding={true}>
            <AppShell>
              <Suspense fallback={<LoadingSpinner />}>
                <DashboardPage />
              </Suspense>
            </AppShell>
          </ProtectedRoute>
        }
      />

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

function App(): ReactNode {
  return (
    <ErrorBoundary
      FallbackComponent={ErrorFallback}
      onReset={() => {
        queryClient.clear();
        window.location.href = "/";
      }}
    >
      <QueryClientProvider client={queryClient}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
          <BrowserRouter>
            <AppRoutes />
          </BrowserRouter>
          <Toaster />
        </ThemeProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

export default App;
