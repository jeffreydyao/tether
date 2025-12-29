//! Shared types and OpenAPI schemas.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bluetooth device information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BluetoothDevice {
    /// Device name (if available).
    #[schema(example = "iPhone")]
    pub name: Option<String>,

    /// Device MAC address.
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub address: String,

    /// Current RSSI value (signal strength).
    #[schema(example = -55)]
    pub rssi: Option<i16>,
}

/// Proximity status for a tracked device.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProximityStatus {
    /// The device being tracked.
    pub device: BluetoothDevice,

    /// Whether the device is considered nearby.
    pub is_nearby: bool,

    /// Current RSSI value.
    pub current_rssi: Option<i16>,

    /// Configured RSSI threshold.
    pub threshold: i16,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status.
    #[schema(example = "ok")]
    pub status: String,

    /// Service version.
    #[schema(example = "0.1.0")]
    pub version: String,
}
