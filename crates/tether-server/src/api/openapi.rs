//! OpenAPI specification generation for tether API.
//!
//! This module generates an OpenAPI 3.0 specification that is consumed by:
//! - Web UI (via @hey-api/openapi-ts for TypeScript client generation)
//! - MCP Server (via rmcp-openapi for AI tool generation)
//!
//! Descriptions are written to be understood by both human developers and AI agents.

use axum::routing::get;
use axum::{Json, Router};
use utoipa::OpenApi;

use crate::state::SharedState;

// Import all the handler modules to reference their types
use super::bluetooth::{DiscoveredDevice, ProximityResponse, ScanDevicesResponse};
use super::config::{
    BluetoothConfigResponse, CompleteOnboardingResponse, ConfigResponse, UpdateBluetoothRequest,
    UpdateBluetoothResponse, UpdatePassesPerMonthRequest, UpdatePassesPerMonthResponse,
    UpdateTimezoneRequest, UpdateTimezoneResponse,
};
use super::error::ErrorResponse;
use super::health::HealthResponse;
use super::passes::{
    PassHistoryEntry, PassHistoryResponse, PassesResponse, UsePassRequest,
    UsePassResponse,
};

/// Creates the OpenAPI documentation router.
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_openapi_spec))
        .route("/openapi.json", get(get_openapi_spec))
}

/// Serve the OpenAPI specification as JSON.
pub async fn get_openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Returns the OpenAPI specification as a string (for writing to file).
pub fn get_openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec")
}

/// Main OpenAPI document structure for tether.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "tether API",
        version = "0.1.0",
        description = r#"
# tether API

tether helps you hold yourself accountable to keep your phone away from your bedroom at night.

## Overview

This API runs on a Raspberry Pi and provides:

1. **Proximity Detection**: Check if your phone is near the Raspberry Pi via Bluetooth
2. **Emergency Passes**: A limited number of monthly passes for legitimate exceptions
3. **Configuration**: Manage Bluetooth devices and settings

## For AI Agents (MCP)

If you're accessing this API via MCP tools:

- **checkProximity**: Verify the phone is in its designated spot. Returns `is_nearby: true` when close.
- **getPasses**: Check how many emergency passes remain this month.
- **usePass**: Use when user has legitimate reason (on-call, emergency). Requires a reason.
- **getPassHistory**: Review past pass usage to identify patterns.

## Design Philosophy

- **Lazy evaluation**: Bluetooth checks only happen when requested
- **Intentional friction**: Passes require reasons to encourage mindfulness
- **Delayed effects**: Pass count changes only apply next month to prevent gaming
"#,
        license(name = "MIT", url = "https://opensource.org/licenses/MIT")
    ),
    servers(
        (url = "/", description = "Local tether server")
    ),
    tags(
        (
            name = "system",
            description = "Health checks and system status"
        ),
        (
            name = "passes",
            description = "Emergency pass management - allows keeping phone nearby for legitimate reasons"
        ),
        (
            name = "config",
            description = "System configuration including Bluetooth device, timezone, and pass settings"
        ),
        (
            name = "bluetooth",
            description = "Bluetooth proximity detection and device scanning"
        )
    ),
    paths(
        // Health endpoints
        super::health::health_check,
        // Pass endpoints
        super::passes::get_passes,
        super::passes::get_pass_history,
        super::passes::use_pass,
        // Config endpoints
        super::config::get_config,
        super::config::update_bluetooth,
        super::config::update_timezone,
        super::config::update_passes_per_month,
        super::config::complete_onboarding,
        // Bluetooth endpoints
        super::bluetooth::check_proximity,
        super::bluetooth::scan_devices,
    ),
    components(
        schemas(
            // Error types
            ErrorResponse,
            // Health types
            HealthResponse,
            // Pass types
            PassesResponse,
            PassHistoryEntry,
            PassHistoryResponse,
            UsePassRequest,
            UsePassResponse,
            // Config types
            ConfigResponse,
            BluetoothConfigResponse,
            UpdateBluetoothRequest,
            UpdateBluetoothResponse,
            UpdateTimezoneRequest,
            UpdateTimezoneResponse,
            UpdatePassesPerMonthRequest,
            UpdatePassesPerMonthResponse,
            CompleteOnboardingResponse,
            // Bluetooth types
            ProximityResponse,
            DiscoveredDevice,
            ScanDevicesResponse,
        )
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "tether API");
        assert!(!spec.paths.paths.is_empty());
    }

    #[test]
    fn test_openapi_json_serialization() {
        let json = get_openapi_json();
        assert!(json.contains("\"openapi\":"));
        assert!(json.contains("\"tether API\""));
    }
}
