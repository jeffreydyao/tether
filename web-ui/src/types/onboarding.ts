export interface BluetoothDevice {
  id: string;
  name: string | null;
  rssi: number;
  lastSeen: string;
}

export interface BluetoothScanResponse {
  devices: BluetoothDevice[];
  scanning: boolean;
}

export interface DeviceRssiResponse {
  deviceId: string;
  rssi: number;
  connected: boolean;
  timestamp: string;
}

export interface WifiConfig {
  ssid: string;
  password: string;
  isPrimary: boolean;
}

export interface WifiConfigResponse {
  success: boolean;
  error?: string;
  newIpAddress?: string;
}

export interface TimezoneInfo {
  id: string;
  displayName: string;
  utcOffset: string;
}

export interface OnboardingConfig {
  bluetooth: {
    deviceId: string;
    deviceName: string | null;
    signalThreshold: number;
  };
  passes: {
    monthlyCount: number;
  };
  wifi: WifiConfig;
  timezone: string;
}

export interface WizardStepData {
  bluetooth: {
    selectedDevice: BluetoothDevice | null;
  };
  signalThreshold: {
    threshold: number;
  };
  passes: {
    monthlyCount: number;
  };
  wifi: {
    ssid: string;
    password: string;
  };
  timezone: {
    selected: string;
  };
}

export interface WizardStepProps {
  data: WizardStepData;
  onDataChange: <K extends keyof WizardStepData>(key: K, value: Partial<WizardStepData[K]>) => void;
  onNext: () => void;
  onBack?: () => void;
  canProceed: boolean;
  setCanProceed: (canProceed: boolean) => void;
}

export interface WizardStepDefinition {
  id: string;
  title: string;
  description: string;
  component: React.ComponentType<WizardStepProps>;
  validate: (data: WizardStepData) => boolean;
}

export const COMMON_TIMEZONES: TimezoneInfo[] = [
  { id: "America/New_York", displayName: "Eastern Time (US & Canada)", utcOffset: "UTC-05:00" },
  { id: "America/Chicago", displayName: "Central Time (US & Canada)", utcOffset: "UTC-06:00" },
  { id: "America/Denver", displayName: "Mountain Time (US & Canada)", utcOffset: "UTC-07:00" },
  { id: "America/Los_Angeles", displayName: "Pacific Time (US & Canada)", utcOffset: "UTC-08:00" },
  { id: "America/Anchorage", displayName: "Alaska", utcOffset: "UTC-09:00" },
  { id: "Pacific/Honolulu", displayName: "Hawaii", utcOffset: "UTC-10:00" },
  { id: "Europe/London", displayName: "London", utcOffset: "UTC+00:00" },
  { id: "Europe/Paris", displayName: "Paris, Berlin, Rome", utcOffset: "UTC+01:00" },
  { id: "Europe/Helsinki", displayName: "Helsinki, Kyiv, Riga", utcOffset: "UTC+02:00" },
  { id: "Europe/Moscow", displayName: "Moscow, St. Petersburg", utcOffset: "UTC+03:00" },
  { id: "Asia/Dubai", displayName: "Dubai, Abu Dhabi", utcOffset: "UTC+04:00" },
  { id: "Asia/Kolkata", displayName: "Mumbai, Kolkata, New Delhi", utcOffset: "UTC+05:30" },
  { id: "Asia/Bangkok", displayName: "Bangkok, Hanoi, Jakarta", utcOffset: "UTC+07:00" },
  { id: "Asia/Shanghai", displayName: "Beijing, Shanghai, Hong Kong", utcOffset: "UTC+08:00" },
  { id: "Asia/Tokyo", displayName: "Tokyo, Seoul", utcOffset: "UTC+09:00" },
  { id: "Australia/Sydney", displayName: "Sydney, Melbourne", utcOffset: "UTC+10:00" },
  { id: "Pacific/Auckland", displayName: "Auckland, Wellington", utcOffset: "UTC+12:00" },
];

export const DEFAULT_WIZARD_DATA: WizardStepData = {
  bluetooth: {
    selectedDevice: null,
  },
  signalThreshold: {
    threshold: -70,
  },
  passes: {
    monthlyCount: 3,
  },
  wifi: {
    ssid: "",
    password: "",
  },
  timezone: {
    selected: "",
  },
};
