//! Bluetooth API endpoints.
//!
//! Provides endpoints for proximity detection and device scanning.

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::{ApiError, ApiResult};
use crate::state::SharedState;

// Note: Routes are now exposed directly in api.rs at /api/proximity and /api/devices
// This module still provides the handlers and types.

// ============================================================================
// Request/Response Types
// ============================================================================

/// Proximity check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "device_name": "iPhone 15 Pro",
    "device_address": "AA:BB:CC:DD:EE:FF",
    "is_nearby": true,
    "rssi_dbm": -45,
    "threshold_dbm": -60,
    "checked_at_utc": "2025-01-15T03:30:00Z"
}))]
pub struct ProximityResponse {
    /// The configured Bluetooth device name.
    #[schema(example = "iPhone 15 Pro")]
    pub device_name: String,

    /// The Bluetooth MAC address of the tracked device.
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub device_address: String,

    /// Whether the device is considered nearby based on RSSI threshold.
    #[schema(example = true)]
    pub is_nearby: bool,

    /// The current RSSI signal strength in dBm.
    #[schema(example = -45)]
    pub rssi_dbm: Option<i16>,

    /// The configured RSSI threshold in dBm.
    #[schema(example = -60)]
    pub threshold_dbm: i8,

    /// UTC timestamp of when this check was performed.
    #[schema(example = "2025-01-15T03:30:00Z")]
    pub checked_at_utc: String,
}

/// A discovered Bluetooth device.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "address": "AA:BB:CC:DD:EE:FF",
    "name": "iPhone 15 Pro",
    "rssi_dbm": -45
}))]
pub struct DiscoveredDevice {
    /// Bluetooth MAC address.
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub address: String,

    /// Device name (if broadcast).
    #[schema(example = "iPhone 15 Pro")]
    pub name: Option<String>,

    /// Signal strength in dBm.
    #[schema(example = -45)]
    pub rssi_dbm: Option<i16>,
}

/// Device scan response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "devices": [
        {
            "address": "AA:BB:CC:DD:EE:FF",
            "name": "iPhone 15 Pro",
            "rssi_dbm": -45
        }
    ],
    "scan_duration_secs": 5,
    "scanned_at_utc": "2025-01-15T03:30:00Z"
}))]
pub struct ScanDevicesResponse {
    /// List of discovered devices.
    pub devices: Vec<DiscoveredDevice>,

    /// How long the scan took.
    #[schema(example = 5)]
    pub scan_duration_secs: u64,

    /// When the scan completed.
    #[schema(example = "2025-01-15T03:30:00Z")]
    pub scanned_at_utc: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Default scan timeout in seconds.
const DEFAULT_SCAN_TIMEOUT_SECS: u64 = 10;

/// Check if the configured Bluetooth device is nearby.
///
/// Performs a lazy proximity check by scanning for the configured Bluetooth
/// device and comparing its RSSI signal strength against the threshold.
#[utoipa::path(
    get,
    path = "/proximity",
    tag = "proximity",
    operation_id = "checkProximity",
    summary = "Check if configured device is nearby",
    description = "Performs a Bluetooth scan to determine if the configured \
        device is within the proximity threshold. This is the primary endpoint \
        for checking accountability - if the device is NOT nearby, the user is \
        successfully keeping their phone away.",
    responses(
        (status = 200, description = "Proximity check completed", body = ProximityResponse),
        (status = 424, description = "Bluetooth device not configured"),
        (status = 503, description = "Bluetooth service unavailable")
    )
)]
pub async fn check_proximity(
    State(state): State<SharedState>,
) -> ApiResult<Json<ProximityResponse>> {
    let state_guard = state.read().await;

    // Check if Bluetooth target is configured (not placeholder)
    let target_address = &state_guard.config.bluetooth.target_address;
    if target_address == "00:00:00:00:00:00" {
        return Err(ApiError::FailedDependency {
            error_code: "device_not_configured".to_string(),
            message: "No Bluetooth device has been configured. Complete onboarding first."
                .to_string(),
            details: None,
        });
    }

    let target_name = state_guard.config.bluetooth.target_name.clone();
    let threshold_dbm = state_guard.config.bluetooth.rssi_threshold;

    // Check if Bluetooth scanner is available
    let scanner = state_guard.bluetooth.as_ref().ok_or_else(|| {
        ApiError::ServiceUnavailable {
            error_code: "bluetooth_unavailable".to_string(),
            message: "Bluetooth adapter is not available".to_string(),
            details: None,
        }
    })?;

    // Create a config for the scan
    let bt_config = tether_core::BtConfig {
        device_address: target_address.clone(),
        rssi_threshold: i16::from(threshold_dbm),
    };

    // Perform proximity check
    let result = scanner.check_proximity(&bt_config).await.map_err(|e| {
        ApiError::ServiceUnavailable {
            error_code: "bluetooth_scan_failed".to_string(),
            message: "Bluetooth scan failed".to_string(),
            details: Some(e.to_string()),
        }
    })?;

    let is_nearby = result
        .rssi
        .map(|rssi| rssi >= i16::from(threshold_dbm))
        .unwrap_or(false);

    Ok(Json(ProximityResponse {
        device_name: target_name,
        device_address: target_address.clone(),
        is_nearby,
        rssi_dbm: result.rssi,
        threshold_dbm,
        checked_at_utc: Utc::now().to_rfc3339(),
    }))
}

/// Scan for nearby Bluetooth devices.
///
/// Performs a brief Bluetooth scan and returns all discovered devices.
/// Used during onboarding to help users select their phone.
#[utoipa::path(
    get,
    path = "/devices",
    tag = "devices",
    operation_id = "scanDevices",
    summary = "Scan for Bluetooth devices",
    description = "Performs a Bluetooth scan and returns all discovered devices. \
        Use this during onboarding to find the user's phone.",
    responses(
        (status = 200, description = "Scan completed", body = ScanDevicesResponse),
        (status = 503, description = "Bluetooth service unavailable")
    )
)]
pub async fn scan_devices(State(state): State<SharedState>) -> ApiResult<Json<ScanDevicesResponse>> {
    let state_guard = state.read().await;

    // Check if Bluetooth scanner is available
    let scanner = state_guard.bluetooth.as_ref().ok_or_else(|| {
        ApiError::ServiceUnavailable {
            error_code: "bluetooth_unavailable".to_string(),
            message: "Bluetooth adapter is not available".to_string(),
            details: None,
        }
    })?;

    let timeout_secs = DEFAULT_SCAN_TIMEOUT_SECS;

    // Perform device scan
    let discovered = scanner
        .discover_devices(timeout_secs)
        .await
        .map_err(|e| ApiError::ServiceUnavailable {
            error_code: "bluetooth_scan_failed".to_string(),
            message: "Bluetooth scan failed".to_string(),
            details: Some(e.to_string()),
        })?;

    let devices: Vec<DiscoveredDevice> = discovered
        .into_iter()
        .map(|d| DiscoveredDevice {
            address: d.address,
            name: d.name,
            rssi_dbm: d.rssi,
        })
        .collect();

    Ok(Json(ScanDevicesResponse {
        devices,
        scan_duration_secs: timeout_secs,
        scanned_at_utc: Utc::now().to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proximity_response_serialization() {
        let response = ProximityResponse {
            device_name: "iPhone".to_string(),
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            is_nearby: true,
            rssi_dbm: Some(-45),
            threshold_dbm: -60,
            checked_at_utc: "2025-01-15T03:30:00Z".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"is_nearby\":true"));
    }

    #[test]
    fn test_scan_response_serialization() {
        let response = ScanDevicesResponse {
            devices: vec![DiscoveredDevice {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                name: Some("iPhone".to_string()),
                rssi_dbm: Some(-45),
            }],
            scan_duration_secs: 5,
            scanned_at_utc: "2025-01-15T03:30:00Z".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("devices"));
    }
}
