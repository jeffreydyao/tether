//! API error types and response handling.
//!
//! This module provides a unified error type for all API handlers
//! with automatic conversion to appropriate HTTP responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Result type alias for API handlers.
pub type ApiResult<T> = Result<T, ApiError>;

/// Unified API error type.
///
/// Each variant maps to a specific HTTP status code and produces a
/// consistent JSON error response.
#[derive(Debug, Clone)]
pub enum ApiError {
    /// 400 Bad Request - Invalid input from client.
    BadRequest {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
    },

    /// 404 Not Found - Resource does not exist.
    NotFound {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
    },

    /// 409 Conflict - Operation cannot be completed due to current state.
    Conflict {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
        /// Number of passes remaining (for pass-related conflicts).
        remaining: Option<u32>,
        /// When passes will reset (ISO 8601 timestamp).
        resets_at_utc: Option<String>,
    },

    /// 424 Failed Dependency - A required upstream service is not configured.
    FailedDependency {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
        /// Optional additional details.
        details: Option<String>,
    },

    /// 500 Internal Server Error - Unexpected server-side error.
    InternalError {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
        /// Optional details (not exposed to client in production).
        details: Option<String>,
    },

    /// 503 Service Unavailable - External service (Bluetooth, WiFi) is unavailable.
    ServiceUnavailable {
        /// Machine-readable error code.
        error_code: String,
        /// Human-readable error message.
        message: String,
        /// Optional additional details.
        details: Option<String>,
    },
}

/// Standard JSON error response body.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "error": "invalid_request",
    "message": "The provided value is not valid",
    "details": null
}))]
pub struct ErrorResponse {
    /// Machine-readable error code (e.g., "invalid_bluetooth_address").
    #[schema(example = "invalid_request")]
    pub error: String,

    /// Human-readable error message.
    #[schema(example = "The provided value is not valid")]
    pub message: String,

    /// Optional additional details for debugging.
    #[schema(nullable)]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            Self::BadRequest { error_code, message } => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: error_code,
                    message,
                    details: None,
                },
            ),

            Self::NotFound { error_code, message } => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: error_code,
                    message,
                    details: None,
                },
            ),

            Self::Conflict {
                error_code,
                message,
                remaining,
                resets_at_utc,
            } => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error: error_code,
                    message,
                    details: Some(serde_json::json!({
                        "remaining": remaining,
                        "resets_at_utc": resets_at_utc
                    })),
                },
            ),

            Self::FailedDependency {
                error_code,
                message,
                details,
            } => (
                StatusCode::from_u16(424).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                ErrorResponse {
                    error: error_code,
                    message,
                    details: details.map(|d| serde_json::json!(d)),
                },
            ),

            Self::InternalError {
                error_code,
                message,
                details,
            } => {
                // Log internal errors
                tracing::error!(
                    error_code = %error_code,
                    message = %message,
                    details = ?details,
                    "Internal server error"
                );

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: error_code,
                        message,
                        details: details.map(|d| serde_json::json!(d)),
                    },
                )
            }

            Self::ServiceUnavailable {
                error_code,
                message,
                details,
            } => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorResponse {
                    error: error_code,
                    message,
                    details: details.map(|d| serde_json::json!(d)),
                },
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest { message, .. } => write!(f, "Bad Request: {message}"),
            Self::NotFound { message, .. } => write!(f, "Not Found: {message}"),
            Self::Conflict { message, .. } => write!(f, "Conflict: {message}"),
            Self::FailedDependency { message, .. } => {
                write!(f, "Failed Dependency: {message}")
            }
            Self::InternalError { message, .. } => {
                write!(f, "Internal Error: {message}")
            }
            Self::ServiceUnavailable { message, .. } => {
                write!(f, "Service Unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for ApiError {}

/// Convert from tether_core errors.
impl From<tether_core::TetherError> for ApiError {
    fn from(err: tether_core::TetherError) -> Self {
        use tether_core::TetherError;

        match &err {
            TetherError::NoPassesRemaining => Self::Conflict {
                error_code: "no_passes_remaining".to_string(),
                message: err.to_string(),
                remaining: Some(0),
                resets_at_utc: None,
            },
            TetherError::EmptyPassReason | TetherError::PassReasonTooLong { .. } => {
                Self::BadRequest {
                    error_code: err.error_code().to_string(),
                    message: err.to_string(),
                }
            }
            TetherError::InvalidMonthFormat(_) => Self::BadRequest {
                error_code: "invalid_month_format".to_string(),
                message: err.to_string(),
            },
            TetherError::ConfigNotFound(_)
            | TetherError::ConfigParseError(_)
            | TetherError::ConfigValidationError(_) => Self::InternalError {
                error_code: err.error_code().to_string(),
                message: err.to_string(),
                details: None,
            },
            TetherError::BluetoothAdapterNotFound
            | TetherError::BluetoothAdapterPoweredOff
            | TetherError::BluetoothScanFailed(_) => Self::ServiceUnavailable {
                error_code: err.error_code().to_string(),
                message: err.to_string(),
                details: None,
            },
            TetherError::DeviceNotFound(addr) => Self::NotFound {
                error_code: "device_not_found".to_string(),
                message: format!("Bluetooth device not found: {addr}"),
            },
            TetherError::PersistenceError(_) | TetherError::IoError(_) => Self::InternalError {
                error_code: err.error_code().to_string(),
                message: err.to_string(),
                details: None,
            },
        }
    }
}

impl From<tether_core::PassError> for ApiError {
    fn from(err: tether_core::PassError) -> Self {
        Self::from(tether_core::TetherError::from(err))
    }
}

impl From<tether_core::ConfigError> for ApiError {
    fn from(err: tether_core::ConfigError) -> Self {
        Self::from(tether_core::TetherError::from(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request_error() {
        let err = ApiError::BadRequest {
            error_code: "test_error".to_string(),
            message: "Test message".to_string(),
        };
        assert!(err.to_string().contains("Bad Request"));
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "test_error".to_string(),
            message: "Test message".to_string(),
            details: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_error"));
    }
}
