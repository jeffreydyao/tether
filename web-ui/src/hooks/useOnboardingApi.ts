import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  scanDevices,
  checkProximity,
  updateWifi,
  updateTimezone,
  updateBluetooth,
  updatePassesPerMonth,
  completeOnboarding,
} from "@/generated";
import type { BluetoothScanResponse, DeviceRssiResponse } from "@/types/onboarding";

export function useBluetoothScan() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (): Promise<BluetoothScanResponse> => {
      const response = await scanDevices();
      if (response.error || !response.data) {
        throw new Error("Failed to scan for devices");
      }
      return {
        devices: response.data.devices.map((d) => ({
          id: d.address,
          name: d.name ?? null,
          rssi: d.rssi_dbm ?? -100,
          lastSeen: new Date().toISOString(),
        })),
        scanning: false,
      };
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["bluetooth", "devices"] });
    },
  });
}

export function useBluetoothDevices(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: ["bluetooth", "devices"],
    queryFn: async (): Promise<BluetoothScanResponse> => {
      const response = await scanDevices();
      if (response.error || !response.data) {
        return { devices: [], scanning: false };
      }
      return {
        devices: response.data.devices.map((d) => ({
          id: d.address,
          name: d.name ?? null,
          rssi: d.rssi_dbm ?? -100,
          lastSeen: new Date().toISOString(),
        })),
        scanning: false,
      };
    },
    enabled: options?.enabled ?? true,
    staleTime: 5000,
  });
}

export function useDeviceRssi(
  deviceId: string | null,
  options?: { enabled?: boolean; pollingInterval?: number }
) {
  const { enabled = true, pollingInterval = 1000 } = options ?? {};

  return useQuery({
    queryKey: ["bluetooth", "rssi", deviceId],
    queryFn: async (): Promise<DeviceRssiResponse> => {
      if (!deviceId) {
        throw new Error("No device ID provided");
      }
      const response = await checkProximity();
      if (response.error || !response.data) {
        throw new Error("Failed to check proximity");
      }
      return {
        deviceId,
        rssi: response.data.rssi_dbm ?? -100,
        connected: response.data.is_nearby,
        timestamp: new Date().toISOString(),
      };
    },
    enabled: enabled && !!deviceId,
    refetchInterval: enabled ? pollingInterval : false,
    placeholderData: (previousData) => previousData,
    retry: 1,
    retryDelay: 100,
  });
}

export function useWifiConfig() {
  return useMutation({
    mutationFn: async (config: { ssid: string; password: string }) => {
      const response = await updateWifi({
        body: {
          networks: [
            {
              ssid: config.ssid,
              password: config.password,
              is_primary: true,
            },
          ],
        },
      });
      if (response.error) {
        throw new Error("Failed to configure WiFi");
      }
      return response.data;
    },
  });
}

export function useTimezoneConfig() {
  return useMutation({
    mutationFn: async (timezone: string) => {
      const response = await updateTimezone({
        body: { timezone },
      });
      if (response.error) {
        throw new Error("Failed to configure timezone");
      }
      return response.data;
    },
  });
}

export function useBluetoothConfig() {
  return useMutation({
    mutationFn: async (config: { deviceAddress: string; deviceName?: string; signalThreshold: number }) => {
      const response = await updateBluetooth({
        body: {
          target_address: config.deviceAddress,
          target_name: config.deviceName ?? "Phone",
          rssi_threshold: config.signalThreshold,
        },
      });
      if (response.error) {
        throw new Error("Failed to configure Bluetooth");
      }
      return response.data;
    },
  });
}

export function usePassesConfig() {
  return useMutation({
    mutationFn: async (monthlyCount: number) => {
      const response = await updatePassesPerMonth({
        body: { per_month: monthlyCount },
      });
      if (response.error) {
        throw new Error("Failed to configure passes");
      }
      return response.data;
    },
  });
}

export function useCompleteOnboarding() {
  return useMutation({
    mutationFn: async () => {
      const response = await completeOnboarding();
      if (response.error) {
        throw new Error("Failed to complete onboarding");
      }
      return response.data;
    },
  });
}
