import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000,
      gcTime: 5 * 60 * 1000,
      retry: 2,
      retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
      refetchOnWindowFocus: true,
      refetchOnReconnect: true,
      refetchOnMount: true,
    },
    mutations: {
      retry: 1,
    },
  },
});

export const queryKeys = {
  onboarding: {
    all: ["onboarding"] as const,
    status: () => [...queryKeys.onboarding.all, "status"] as const,
  },
  device: {
    all: ["device"] as const,
    proximity: () => [...queryKeys.device.all, "proximity"] as const,
    signalStrength: () => [...queryKeys.device.all, "signal-strength"] as const,
  },
  passes: {
    all: ["passes"] as const,
    remaining: () => [...queryKeys.passes.all, "remaining"] as const,
    history: (month?: string) => [...queryKeys.passes.all, "history", month ?? "current"] as const,
  },
  config: {
    all: ["config"] as const,
    wifi: () => [...queryKeys.config.all, "wifi"] as const,
    bluetooth: () => [...queryKeys.config.all, "bluetooth"] as const,
    timezone: () => [...queryKeys.config.all, "timezone"] as const,
    ticket: () => [...queryKeys.config.all, "ticket"] as const,
  },
} as const;
