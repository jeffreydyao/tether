//! Shared types and OpenAPI schemas.
//!
//! This module contains types that are shared across the application.
//! Most API types are defined in their respective modules (bluetooth, passes, config).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
