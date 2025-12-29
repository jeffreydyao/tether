import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/queryClient";
import { getConfig } from "@/generated";
import type { ConfigResponse } from "@/generated";

export type OnboardingStep =
  | "bluetooth_device"
  | "signal_threshold"
  | "passes_config"
  | "wifi_primary"
  | "wifi_additional"
  | "timezone"
  | "complete";

export interface UseOnboardingStateReturn {
  isOnboardingComplete: boolean;
  currentStep: OnboardingStep | null;
  isLoading: boolean;
  isInitialLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useOnboardingState(): UseOnboardingStateReturn {
  const query = useQuery<ConfigResponse, Error>({
    queryKey: queryKeys.onboarding.status(),
    queryFn: async (): Promise<ConfigResponse> => {
      const response = await getConfig();
      if (response.error) {
        throw new Error("Failed to fetch config");
      }
      if (!response.data) {
        throw new Error("No data received");
      }
      return response.data;
    },
    staleTime: 60 * 1000,
    gcTime: 10 * 60 * 1000,
    retry: 3,
    retryDelay: 1000,
    refetchOnMount: "always",
  });

  return {
    isOnboardingComplete: query.data?.onboarding_complete ?? false,
    currentStep: null,
    isLoading: query.isLoading,
    isInitialLoading: query.isLoading && !query.data,
    error: query.error,
    refetch: query.refetch,
  };
}
