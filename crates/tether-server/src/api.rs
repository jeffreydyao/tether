//! HTTP API routes and handlers.
//!
//! This module contains all HTTP endpoint implementations organized by domain:
//! - `bluetooth` - Bluetooth proximity detection and device scanning
//! - `config` - System configuration management
//! - `health` - Service health checks
//! - `passes` - Monthly pass management
//! - `error` - API error types

use axum::Router;

use crate::state::SharedState;

pub mod bluetooth;
pub mod config;
pub mod error;
pub mod health;
pub mod passes;

// Re-export commonly used types
#[allow(unused_imports)]
pub use error::{ApiError, ApiResult, ErrorResponse};

/// Creates the combined API router with all endpoints.
///
/// # Route Structure
///
/// ```text
/// /health              - Health check
/// /api
/// ├── /passes          - Pass status, history, and usage
/// ├── /config          - Configuration management
/// └── /bluetooth       - Proximity and device scanning
/// ```
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .nest("/health", health::router())
        .nest(
            "/api",
            Router::new()
                .nest("/passes", passes::router())
                .nest("/config", config::router())
                .nest("/bluetooth", bluetooth::router()),
        )
        .with_state(state)
}
