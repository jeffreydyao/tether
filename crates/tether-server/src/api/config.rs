//! Configuration API endpoints.
//!
//! Provides endpoints for reading and updating system configuration
//! including Bluetooth target device, timezone, and passes per month.

use axum::extract::State;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::{ApiError, ApiResult};
use crate::state::SharedState;

/// Creates the config router with all endpoints.
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_config))
        .route("/bluetooth", put(update_bluetooth))
        .route("/timezone", put(update_timezone))
        .route("/passes", put(update_passes_per_month))
        .route("/onboarding/complete", put(complete_onboarding))
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Current configuration response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "bluetooth": {
        "target_address": "AA:BB:CC:DD:EE:FF",
        "target_name": "iPhone 15 Pro",
        "rssi_threshold": -60
    },
    "timezone": "America/Los_Angeles",
    "passes_per_month": 3,
    "onboarding_complete": true
}))]
pub struct ConfigResponse {
    /// Bluetooth target configuration.
    pub bluetooth: BluetoothConfigResponse,

    /// Configured timezone (IANA format).
    #[schema(example = "America/Los_Angeles")]
    pub timezone: String,

    /// Number of passes allowed per month.
    #[schema(example = 3)]
    pub passes_per_month: u8,

    /// Whether initial onboarding has been completed.
    #[schema(example = true)]
    pub onboarding_complete: bool,
}

/// Bluetooth configuration in response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "target_address": "AA:BB:CC:DD:EE:FF",
    "target_name": "iPhone 15 Pro",
    "rssi_threshold": -60,
    "is_configured": true
}))]
pub struct BluetoothConfigResponse {
    /// Bluetooth MAC address of target device.
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub target_address: String,

    /// User-friendly name of the device.
    #[schema(example = "iPhone 15 Pro")]
    pub target_name: String,

    /// RSSI threshold for proximity detection.
    #[schema(example = -60)]
    pub rssi_threshold: i8,

    /// Whether a real device has been configured (not placeholder).
    #[schema(example = true)]
    pub is_configured: bool,
}

/// Request to update Bluetooth target device.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "target_address": "AA:BB:CC:DD:EE:FF",
    "target_name": "iPhone 15 Pro",
    "rssi_threshold": -60
}))]
pub struct UpdateBluetoothRequest {
    /// Bluetooth MAC address (XX:XX:XX:XX:XX:XX format).
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub target_address: String,

    /// User-friendly name for the device.
    #[schema(example = "iPhone 15 Pro")]
    pub target_name: String,

    /// Optional RSSI threshold (-100 to 0 dBm). Defaults to -60.
    #[schema(example = -60)]
    pub rssi_threshold: Option<i8>,
}

/// Response after updating Bluetooth configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBluetoothResponse {
    /// Whether the update was successful.
    pub success: bool,

    /// Updated configuration.
    pub bluetooth: BluetoothConfigResponse,
}

/// Request to update timezone.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "timezone": "America/Los_Angeles"
}))]
pub struct UpdateTimezoneRequest {
    /// IANA timezone name (e.g., "America/Los_Angeles").
    #[schema(example = "America/Los_Angeles")]
    pub timezone: String,
}

/// Response after updating timezone.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTimezoneResponse {
    /// Whether the update was successful.
    pub success: bool,

    /// Updated timezone.
    #[schema(example = "America/Los_Angeles")]
    pub timezone: String,
}

/// Request to update passes per month.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "per_month": 3
}))]
pub struct UpdatePassesPerMonthRequest {
    /// Number of passes per month (0-31).
    #[schema(example = 3, minimum = 0, maximum = 31)]
    pub per_month: u8,
}

/// Response after updating passes per month.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePassesPerMonthResponse {
    /// Whether the update was successful.
    pub success: bool,

    /// Updated passes per month value.
    #[schema(example = 3)]
    pub per_month: u8,

    /// Whether the change is pending (will apply next month).
    #[schema(example = true)]
    pub pending: bool,

    /// Message explaining when the change takes effect.
    pub message: String,
}

/// Response after completing onboarding.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompleteOnboardingResponse {
    /// Whether onboarding was completed.
    pub success: bool,

    /// Message about onboarding status.
    pub message: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Check if Bluetooth is configured (not placeholder address).
fn is_bluetooth_configured(address: &str) -> bool {
    address != "00:00:00:00:00:00"
}

/// Get current configuration.
#[utoipa::path(
    get,
    path = "/config",
    tag = "config",
    operation_id = "getConfig",
    summary = "Get current configuration",
    description = "Returns the current system configuration including Bluetooth \
        target, timezone, and passes per month.",
    responses(
        (status = 200, description = "Configuration retrieved", body = ConfigResponse)
    )
)]
pub async fn get_config(State(state): State<SharedState>) -> ApiResult<Json<ConfigResponse>> {
    let state_guard = state.read().await;
    let config = &state_guard.config;

    let bluetooth = BluetoothConfigResponse {
        target_address: config.bluetooth.target_address.clone(),
        target_name: config.bluetooth.target_name.clone(),
        rssi_threshold: config.bluetooth.rssi_threshold,
        is_configured: is_bluetooth_configured(&config.bluetooth.target_address),
    };

    Ok(Json(ConfigResponse {
        bluetooth,
        timezone: config.system.timezone.clone(),
        passes_per_month: config.passes.per_month,
        onboarding_complete: config.system.onboarding_complete,
    }))
}

/// Update Bluetooth target device.
#[utoipa::path(
    put,
    path = "/config/bluetooth",
    tag = "config",
    operation_id = "updateBluetooth",
    summary = "Update Bluetooth target device",
    description = "Updates the Bluetooth device to track for proximity detection.",
    request_body = UpdateBluetoothRequest,
    responses(
        (status = 200, description = "Bluetooth configuration updated", body = UpdateBluetoothResponse),
        (status = 400, description = "Invalid Bluetooth address format")
    )
)]
pub async fn update_bluetooth(
    State(state): State<SharedState>,
    Json(request): Json<UpdateBluetoothRequest>,
) -> ApiResult<Json<UpdateBluetoothResponse>> {
    // Validate Bluetooth address format
    if !tether_core::is_valid_mac_address(&request.target_address) {
        return Err(ApiError::BadRequest {
            error_code: "invalid_bluetooth_address".to_string(),
            message: "Bluetooth address must be in format XX:XX:XX:XX:XX:XX".to_string(),
        });
    }

    // Validate RSSI threshold if provided
    if let Some(threshold) = request.rssi_threshold {
        if !(-100..=0).contains(&i16::from(threshold)) {
            return Err(ApiError::BadRequest {
                error_code: "invalid_rssi_threshold".to_string(),
                message: "RSSI threshold must be between -100 and 0 dBm".to_string(),
            });
        }
    }

    let mut state_guard = state.write().await;

    // Update config
    state_guard.config.bluetooth.target_address = request.target_address.to_uppercase();
    state_guard.config.bluetooth.target_name = request.target_name.clone();
    if let Some(threshold) = request.rssi_threshold {
        state_guard.config.bluetooth.rssi_threshold = threshold;
    }

    // Save config
    state_guard.save_config().map_err(|e| ApiError::InternalError {
        error_code: "config_save_failed".to_string(),
        message: "Failed to save configuration".to_string(),
        details: Some(e.to_string()),
    })?;

    let bluetooth = BluetoothConfigResponse {
        target_address: state_guard.config.bluetooth.target_address.clone(),
        target_name: state_guard.config.bluetooth.target_name.clone(),
        rssi_threshold: state_guard.config.bluetooth.rssi_threshold,
        is_configured: is_bluetooth_configured(&state_guard.config.bluetooth.target_address),
    };

    Ok(Json(UpdateBluetoothResponse {
        success: true,
        bluetooth,
    }))
}

/// Update timezone.
#[utoipa::path(
    put,
    path = "/config/timezone",
    tag = "config",
    operation_id = "updateTimezone",
    summary = "Update timezone",
    description = "Updates the timezone used for pass reset calculations.",
    request_body = UpdateTimezoneRequest,
    responses(
        (status = 200, description = "Timezone updated", body = UpdateTimezoneResponse),
        (status = 400, description = "Invalid timezone")
    )
)]
pub async fn update_timezone(
    State(state): State<SharedState>,
    Json(request): Json<UpdateTimezoneRequest>,
) -> ApiResult<Json<UpdateTimezoneResponse>> {
    // Validate timezone
    if !tether_core::is_valid_timezone_format(&request.timezone) {
        return Err(ApiError::BadRequest {
            error_code: "invalid_timezone".to_string(),
            message: format!(
                "Unknown timezone: '{}'. Use IANA timezone names (e.g., 'America/Los_Angeles').",
                request.timezone
            ),
        });
    }

    let mut state_guard = state.write().await;

    state_guard.config.system.timezone = request.timezone.clone();

    state_guard.save_config().map_err(|e| ApiError::InternalError {
        error_code: "config_save_failed".to_string(),
        message: "Failed to save configuration".to_string(),
        details: Some(e.to_string()),
    })?;

    Ok(Json(UpdateTimezoneResponse {
        success: true,
        timezone: request.timezone,
    }))
}

/// Update passes per month.
#[utoipa::path(
    put,
    path = "/config/passes",
    tag = "config",
    operation_id = "updatePassesPerMonth",
    summary = "Update passes per month",
    description = "Updates the number of emergency passes allowed per month. \
        If passes have already been used this month, the change will take \
        effect next month.",
    request_body = UpdatePassesPerMonthRequest,
    responses(
        (status = 200, description = "Passes per month updated", body = UpdatePassesPerMonthResponse),
        (status = 400, description = "Invalid value")
    )
)]
pub async fn update_passes_per_month(
    State(state): State<SharedState>,
    Json(request): Json<UpdatePassesPerMonthRequest>,
) -> ApiResult<Json<UpdatePassesPerMonthResponse>> {
    // Validate range
    if request.per_month > 31 {
        return Err(ApiError::BadRequest {
            error_code: "invalid_passes_count".to_string(),
            message: "Passes per month must be between 0 and 31".to_string(),
        });
    }

    let mut state_guard = state.write().await;

    // Check if passes have been used this month
    let remaining = state_guard.pass_manager.remaining();
    let per_month = state_guard.pass_manager.per_month();
    let passes_used = per_month > remaining;

    // Update config
    state_guard.config.passes.per_month = request.per_month;

    // Update pass manager - will be deferred if passes used (saved internally)
    let pending = state_guard
        .pass_manager
        .set_per_month(request.per_month.into())?;

    // Save config
    state_guard.save_config().map_err(|e| ApiError::InternalError {
        error_code: "config_save_failed".to_string(),
        message: "Failed to save configuration".to_string(),
        details: Some(e.to_string()),
    })?;

    let message = if pending {
        "Change will take effect on the first of next month".to_string()
    } else {
        "Passes per month updated immediately".to_string()
    };

    Ok(Json(UpdatePassesPerMonthResponse {
        success: true,
        per_month: request.per_month,
        pending: pending && passes_used,
        message,
    }))
}

/// Complete onboarding.
#[utoipa::path(
    put,
    path = "/config/onboarding/complete",
    tag = "config",
    operation_id = "completeOnboarding",
    summary = "Complete onboarding",
    description = "Marks the initial onboarding as complete. Requires Bluetooth \
        target to be configured first.",
    responses(
        (status = 200, description = "Onboarding completed", body = CompleteOnboardingResponse),
        (status = 400, description = "Already completed"),
        (status = 424, description = "Prerequisites not met")
    )
)]
pub async fn complete_onboarding(
    State(state): State<SharedState>,
) -> ApiResult<Json<CompleteOnboardingResponse>> {
    let mut state_guard = state.write().await;

    // Check if already complete
    if state_guard.config.system.onboarding_complete {
        return Err(ApiError::BadRequest {
            error_code: "already_complete".to_string(),
            message: "Onboarding has already been completed".to_string(),
        });
    }

    // Check prerequisites - Bluetooth must be configured (not placeholder)
    if !is_bluetooth_configured(&state_guard.config.bluetooth.target_address) {
        return Err(ApiError::FailedDependency {
            error_code: "prerequisites_not_met".to_string(),
            message: "Cannot complete onboarding: Bluetooth device not configured".to_string(),
            details: None,
        });
    }

    // Mark as complete
    state_guard.config.system.onboarding_complete = true;

    state_guard.save_config().map_err(|e| ApiError::InternalError {
        error_code: "config_save_failed".to_string(),
        message: "Failed to save configuration".to_string(),
        details: Some(e.to_string()),
    })?;

    Ok(Json(CompleteOnboardingResponse {
        success: true,
        message: "Onboarding completed successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_response_serialization() {
        let response = ConfigResponse {
            bluetooth: BluetoothConfigResponse {
                target_address: "AA:BB:CC:DD:EE:FF".to_string(),
                target_name: "iPhone".to_string(),
                rssi_threshold: -60,
                is_configured: true,
            },
            timezone: "UTC".to_string(),
            passes_per_month: 3,
            onboarding_complete: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("AA:BB:CC:DD:EE:FF"));
    }

    #[test]
    fn test_update_bluetooth_request_deserialization() {
        let json = r#"{"target_address": "AA:BB:CC:DD:EE:FF", "target_name": "iPhone"}"#;
        let request: UpdateBluetoothRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.target_address, "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn test_is_bluetooth_configured() {
        assert!(!is_bluetooth_configured("00:00:00:00:00:00"));
        assert!(is_bluetooth_configured("AA:BB:CC:DD:EE:FF"));
    }
}
