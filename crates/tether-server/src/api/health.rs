//! Health check API endpoint.
//!
//! Provides a simple health check endpoint for monitoring and load balancers.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::SharedState;

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "status": "ok",
    "version": "0.1.0",
    "onboarding_complete": true
}))]
pub struct HealthResponse {
    /// Service status.
    #[schema(example = "ok")]
    pub status: String,

    /// Service version from Cargo.toml.
    #[schema(example = "0.1.0")]
    pub version: String,

    /// Whether initial onboarding has been completed.
    #[schema(example = true)]
    pub onboarding_complete: bool,
}

/// Creates the health router.
pub fn router() -> Router<SharedState> {
    Router::new().route("/", get(health_check))
}

/// Health check endpoint.
///
/// Returns basic service status information including version and
/// whether onboarding has been completed.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    operation_id = "healthCheck",
    summary = "Check service health",
    description = "Returns basic service status information. Use this endpoint \
        for load balancer health checks and monitoring.",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<SharedState>) -> Json<HealthResponse> {
    let state_guard = state.read().await;
    let onboarding_complete = state_guard.config.system.onboarding_complete;

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        onboarding_complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            onboarding_complete: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"onboarding_complete\":false"));
    }
}
