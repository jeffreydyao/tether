//! HTTP API routes and handlers.
//!
//! This module contains all HTTP endpoint implementations organized by domain:
//! - `bluetooth` - Bluetooth proximity detection and device scanning
//! - `config` - System configuration management
//! - `health` - Service health checks
//! - `passes` - Monthly pass management
//! - `error` - API error types
//! - `openapi` - OpenAPI specification generation

use axum::routing::get;
use axum::Router;

use crate::state::SharedState;

pub mod bluetooth;
pub mod config;
pub mod error;
pub mod health;
pub mod openapi;
pub mod passes;
pub mod system;

// Re-export commonly used types
#[allow(unused_imports)]
pub use error::{ApiError, ApiResult, ErrorResponse};

// Re-export OpenAPI utilities for the gen-openapi binary
#[allow(unused_imports)]
pub use openapi::get_openapi_json;

/// Creates the combined API router with all endpoints.
///
/// # Route Structure
///
/// ```text
/// /health                - Health check
/// /api
/// ├── /proximity         - Bluetooth proximity check
/// ├── /passes            - Pass status, history, and usage
/// ├── /config            - Configuration management
/// ├── /devices           - Bluetooth device scanning
/// ├── /system            - System status, ticket, restart
/// └── /openapi.json      - OpenAPI specification
/// ```
pub fn create_router(state: SharedState) -> Router {
    // Initialize server start time for uptime tracking
    system::init_start_time();

    Router::new()
        .nest("/health", health::router())
        .nest(
            "/api",
            Router::new()
                // Proximity check at /api/proximity
                .route("/proximity", get(bluetooth::check_proximity))
                // Device scanning at /api/devices
                .route("/devices", get(bluetooth::scan_devices))
                // OpenAPI spec at /api/openapi.json
                .route("/openapi.json", get(openapi::get_openapi_spec))
                // Pass management
                .nest("/passes", passes::router())
                // Configuration management
                .nest("/config", config::router())
                // System management
                .nest("/system", system::router()),
        )
        .with_state(state)
}
