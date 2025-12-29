//! System API endpoints.
//!
//! Provides endpoints for system status, dumbpipe ticket retrieval, and system restart.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::{ApiError, ApiResult};
use crate::state::SharedState;

/// Creates the system router with all endpoints.
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/ticket", get(get_ticket))
        .route("/restart", post(restart))
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// System status response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "version": "0.1.0",
    "uptime_secs": 3600,
    "bluetooth_available": true,
    "config_loaded": true,
    "onboarding_complete": true
}))]
pub struct SystemStatusResponse {
    /// Server version.
    #[schema(example = "0.1.0")]
    pub version: String,

    /// Server uptime in seconds.
    #[schema(example = 3600)]
    pub uptime_secs: u64,

    /// Whether Bluetooth is available.
    #[schema(example = true)]
    pub bluetooth_available: bool,

    /// Whether configuration is loaded.
    #[schema(example = true)]
    pub config_loaded: bool,

    /// Whether onboarding is complete.
    #[schema(example = true)]
    pub onboarding_complete: bool,
}

/// Dumbpipe ticket response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "ticket": "blobfd23abc...",
    "expires_at_utc": "2025-01-15T04:30:00Z",
    "node_id": "n0abc123..."
}))]
pub struct DumbpipeTicketResponse {
    /// The dumbpipe ticket for remote access.
    /// This is a base32-encoded iroh ticket.
    #[schema(example = "blobfd23abc...")]
    pub ticket: Option<String>,

    /// When the ticket expires (if applicable).
    #[schema(example = "2025-01-15T04:30:00Z")]
    pub expires_at_utc: Option<String>,

    /// The node ID of this tether instance.
    #[schema(example = "n0abc123...")]
    pub node_id: Option<String>,

    /// Whether dumbpipe is available.
    #[schema(example = true)]
    pub available: bool,

    /// Message if not available.
    pub message: Option<String>,
}

/// System restart request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "delay_secs": 5
}))]
pub struct RestartRequest {
    /// Delay before restart in seconds (0-60).
    #[schema(example = 5, minimum = 0, maximum = 60)]
    pub delay_secs: Option<u32>,
}

/// System restart response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "accepted": true,
    "message": "System will restart in 5 seconds",
    "delay_secs": 5
}))]
pub struct RestartResponse {
    /// Whether the restart request was accepted.
    #[schema(example = true)]
    pub accepted: bool,

    /// Message about the restart.
    #[schema(example = "System will restart in 5 seconds")]
    pub message: String,

    /// Delay before restart.
    #[schema(example = 5)]
    pub delay_secs: u32,
}

// ============================================================================
// Static state for uptime tracking
// ============================================================================

use std::sync::OnceLock;
use std::time::Instant;

static SERVER_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Initialize the server start time. Call this once at startup.
pub fn init_start_time() {
    SERVER_START_TIME.get_or_init(Instant::now);
}

/// Get server uptime in seconds.
fn get_uptime_secs() -> u64 {
    SERVER_START_TIME
        .get()
        .map(|start| start.elapsed().as_secs())
        .unwrap_or(0)
}

// ============================================================================
// Handlers
// ============================================================================

/// Get system status.
#[utoipa::path(
    get,
    path = "/system/status",
    tag = "system",
    operation_id = "getSystemStatus",
    summary = "Get system status",
    description = "Returns the current system status including version, uptime, \
        and component availability.",
    responses(
        (status = 200, description = "System status retrieved", body = SystemStatusResponse)
    )
)]
pub async fn get_status(State(state): State<SharedState>) -> ApiResult<Json<SystemStatusResponse>> {
    let state_guard = state.read().await;

    Ok(Json(SystemStatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: get_uptime_secs(),
        bluetooth_available: state_guard.bluetooth.is_some(),
        config_loaded: true,
        onboarding_complete: state_guard.config.system.onboarding_complete,
    }))
}

/// Get dumbpipe ticket for remote access.
#[utoipa::path(
    get,
    path = "/system/ticket",
    tag = "system",
    operation_id = "getDumbpipeTicket",
    summary = "Get dumbpipe ticket",
    description = "Returns the dumbpipe ticket for establishing remote P2P connections \
        via iroh. This ticket is used by the MCP server to connect to this tether instance.",
    responses(
        (status = 200, description = "Ticket retrieved", body = DumbpipeTicketResponse),
        (status = 503, description = "Dumbpipe not available")
    )
)]
pub async fn get_ticket(
    State(_state): State<SharedState>,
) -> ApiResult<Json<DumbpipeTicketResponse>> {
    // TODO: Implement actual dumbpipe ticket retrieval
    // For now, return a placeholder response indicating it's not available
    Ok(Json(DumbpipeTicketResponse {
        ticket: None,
        expires_at_utc: None,
        node_id: None,
        available: false,
        message: Some("Dumbpipe integration not yet implemented".to_string()),
    }))
}

/// Request system restart.
#[utoipa::path(
    post,
    path = "/system/restart",
    tag = "system",
    operation_id = "restartSystem",
    summary = "Request system restart",
    description = "Initiates a system restart. The restart can be delayed by up to 60 seconds. \
        This is useful for applying configuration changes that require a restart.",
    request_body = RestartRequest,
    responses(
        (status = 200, description = "Restart accepted", body = RestartResponse),
        (status = 400, description = "Invalid delay value"),
        (status = 503, description = "Restart not available")
    )
)]
pub async fn restart(
    State(_state): State<SharedState>,
    Json(request): Json<RestartRequest>,
) -> ApiResult<Json<RestartResponse>> {
    let delay_secs = request.delay_secs.unwrap_or(5);

    // Validate delay
    if delay_secs > 60 {
        return Err(ApiError::BadRequest {
            error_code: "invalid_delay".to_string(),
            message: "Delay must be between 0 and 60 seconds".to_string(),
        });
    }

    // TODO: Actually implement system restart
    // For now, return a response indicating it's not implemented
    // In production, this would spawn a task to restart the system after the delay

    Ok(Json(RestartResponse {
        accepted: false,
        message: format!(
            "Restart not implemented. Would restart in {} seconds.",
            delay_secs
        ),
        delay_secs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_status_response_serialization() {
        let response = SystemStatusResponse {
            version: "0.1.0".to_string(),
            uptime_secs: 3600,
            bluetooth_available: true,
            config_loaded: true,
            onboarding_complete: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn test_dumbpipe_ticket_response_serialization() {
        let response = DumbpipeTicketResponse {
            ticket: Some("test_ticket".to_string()),
            expires_at_utc: None,
            node_id: Some("n0test".to_string()),
            available: true,
            message: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_ticket"));
    }
}
