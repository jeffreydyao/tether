//! Unified error types for the tether core library.
//!
//! This module provides a unified error type [`TetherError`] that covers all failure
//! modes across the tether system. Each module also has its own specific error types
//! (ConfigError, PassError, BluetoothError) for internal use.
//!
//! # Design Principles
//!
//! - **Specific variants**: Each error variant captures exactly one failure mode
//! - **Actionable messages**: Error messages guide users toward resolution
//! - **Context preservation**: Wrapped errors maintain their original context
//! - **HTTP-ready**: Error types include HTTP status codes and error codes
//!
//! # Example
//!
//! ```rust
//! use tether_core::error::{TetherError, Result};
//! use std::path::PathBuf;
//!
//! fn load_config(path: &PathBuf) -> Result<()> {
//!     if !path.exists() {
//!         return Err(TetherError::ConfigNotFound(path.clone()));
//!     }
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

/// The unified error type for all tether operations.
///
/// This enum covers all failure modes that can occur in the tether system.
/// Each variant is designed to be:
///
/// 1. **Self-descriptive**: The variant name indicates the failure mode
/// 2. **Contextual**: Variants include relevant data for debugging
/// 3. **Actionable**: Error messages suggest how to resolve the issue
#[derive(Debug, Error)]
pub enum TetherError {
    // =========================================================================
    // BLUETOOTH ERRORS
    // =========================================================================
    /// No Bluetooth adapter was found on this system.
    #[error(
        "No Bluetooth adapter found. Ensure Bluetooth hardware is present and drivers are loaded."
    )]
    BluetoothAdapterNotFound,

    /// The Bluetooth adapter exists but is powered off.
    #[error("Bluetooth adapter is powered off. Run 'bluetoothctl power on' to enable.")]
    BluetoothAdapterPoweredOff,

    /// Bluetooth device scanning failed.
    #[error("Bluetooth scan failed: {0}")]
    BluetoothScanFailed(String),

    /// The configured Bluetooth device was not found during scanning.
    #[error("Device not found: '{0}'. Ensure the device is powered on and within range.")]
    DeviceNotFound(String),

    // =========================================================================
    // PASS MANAGEMENT ERRORS
    // =========================================================================
    /// All passes for the current month have been used.
    #[error("No passes remaining for this month. Passes will refresh on the 1st of next month.")]
    NoPassesRemaining,

    /// The provided month format is invalid.
    #[error("Invalid month format: '{0}'. Expected ISO 8601 format 'YYYY-MM' (e.g., '2025-01').")]
    InvalidMonthFormat(String),

    /// The reason provided for using a pass was empty.
    #[error("Pass reason cannot be empty")]
    EmptyPassReason,

    /// The reason provided exceeds the maximum allowed length.
    #[error("Pass reason exceeds maximum length of {max} characters (got {actual})")]
    PassReasonTooLong {
        /// Maximum allowed length.
        max: usize,
        /// Actual length provided.
        actual: usize,
    },

    // =========================================================================
    // CONFIGURATION ERRORS
    // =========================================================================
    /// The configuration file was not found at the expected path.
    #[error("Configuration file not found at: {}", .0.display())]
    ConfigNotFound(PathBuf),

    /// The configuration file exists but could not be parsed.
    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),

    /// The configuration was parsed but contains invalid values.
    #[error("Configuration validation failed: {0}")]
    ConfigValidationError(String),

    // =========================================================================
    // PERSISTENCE & I/O ERRORS
    // =========================================================================
    /// An error occurred while persisting or reading data.
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// A low-level I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// A specialized [`Result`] type for tether operations.
///
/// This type alias eliminates the need to specify the error type explicitly
/// when returning results from tether functions.
pub type Result<T> = std::result::Result<T, TetherError>;

/// Legacy error type alias for backwards compatibility.
///
/// Prefer using [`TetherError`] directly in new code.
pub type Error = TetherError;

impl TetherError {
    /// Returns `true` if this error is related to Bluetooth operations.
    #[inline]
    #[must_use]
    pub fn is_bluetooth_error(&self) -> bool {
        matches!(
            self,
            Self::BluetoothAdapterNotFound
                | Self::BluetoothAdapterPoweredOff
                | Self::BluetoothScanFailed(_)
                | Self::DeviceNotFound(_)
        )
    }

    /// Returns `true` if this error is related to configuration.
    #[inline]
    #[must_use]
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            Self::ConfigNotFound(_) | Self::ConfigParseError(_) | Self::ConfigValidationError(_)
        )
    }

    /// Returns `true` if this error is related to pass management.
    #[inline]
    #[must_use]
    pub fn is_pass_error(&self) -> bool {
        matches!(
            self,
            Self::NoPassesRemaining
                | Self::InvalidMonthFormat(_)
                | Self::EmptyPassReason
                | Self::PassReasonTooLong { .. }
        )
    }

    /// Returns `true` if this error is related to I/O or persistence.
    #[inline]
    #[must_use]
    pub fn is_io_error(&self) -> bool {
        matches!(self, Self::PersistenceError(_) | Self::IoError(_))
    }

    /// Returns `true` if this error represents an expected operational state.
    ///
    /// Some errors (like no passes remaining) are not system failures but
    /// expected operational conditions.
    #[inline]
    #[must_use]
    pub fn is_expected_state(&self) -> bool {
        matches!(self, Self::NoPassesRemaining)
    }

    /// Returns `true` if this error is likely recoverable without user intervention.
    #[inline]
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::DeviceNotFound(_) | Self::BluetoothScanFailed(_))
    }

    /// Returns an HTTP-appropriate status code for this error.
    #[inline]
    #[must_use]
    pub fn http_status_code(&self) -> u16 {
        match self {
            // 400 Bad Request - malformed input
            Self::InvalidMonthFormat(_)
            | Self::EmptyPassReason
            | Self::PassReasonTooLong { .. } => 400,

            // 403 Forbidden - understood but refused
            Self::NoPassesRemaining => 403,

            // 404 Not Found
            Self::ConfigNotFound(_) | Self::DeviceNotFound(_) => 404,

            // 422 Unprocessable Entity - semantic errors
            Self::ConfigParseError(_) | Self::ConfigValidationError(_) => 422,

            // 500 Internal Server Error - server-side issues
            Self::PersistenceError(_) | Self::IoError(_) => 500,

            // 503 Service Unavailable - Bluetooth hardware issues
            Self::BluetoothAdapterNotFound
            | Self::BluetoothAdapterPoweredOff
            | Self::BluetoothScanFailed(_) => 503,
        }
    }

    /// Returns a machine-readable error code for API responses.
    #[inline]
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::BluetoothAdapterNotFound => "BLUETOOTH_ADAPTER_NOT_FOUND",
            Self::BluetoothAdapterPoweredOff => "BLUETOOTH_ADAPTER_POWERED_OFF",
            Self::BluetoothScanFailed(_) => "BLUETOOTH_SCAN_FAILED",
            Self::DeviceNotFound(_) => "DEVICE_NOT_FOUND",
            Self::NoPassesRemaining => "NO_PASSES_REMAINING",
            Self::InvalidMonthFormat(_) => "INVALID_MONTH_FORMAT",
            Self::EmptyPassReason => "EMPTY_PASS_REASON",
            Self::PassReasonTooLong { .. } => "PASS_REASON_TOO_LONG",
            Self::ConfigNotFound(_) => "CONFIG_NOT_FOUND",
            Self::ConfigParseError(_) => "CONFIG_PARSE_ERROR",
            Self::ConfigValidationError(_) => "CONFIG_VALIDATION_ERROR",
            Self::PersistenceError(_) => "PERSISTENCE_ERROR",
            Self::IoError(_) => "IO_ERROR",
        }
    }
}

// =============================================================================
// CONVERSIONS FROM MODULE-SPECIFIC ERRORS
// =============================================================================

impl From<crate::config::ConfigError> for TetherError {
    fn from(err: crate::config::ConfigError) -> Self {
        use crate::config::ConfigError;
        match err {
            ConfigError::NotFound(path) => Self::ConfigNotFound(path.into()),
            ConfigError::ReadError { path, source } => {
                Self::PersistenceError(format!("Failed to read {}: {}", path, source))
            }
            ConfigError::WriteError { path, source } => {
                Self::PersistenceError(format!("Failed to write {}: {}", path, source))
            }
            ConfigError::ParseError(e) => Self::ConfigParseError(e.to_string()),
            ConfigError::SerializeError(e) => Self::ConfigParseError(e.to_string()),
            ConfigError::ValidationError { field, message } => {
                Self::ConfigValidationError(format!("{}: {}", field, message))
            }
            ConfigError::MultipleValidationErrors(errors) => {
                let messages: Vec<String> = errors.into_iter().map(|e| e.to_string()).collect();
                Self::ConfigValidationError(messages.join("; "))
            }
        }
    }
}

impl From<crate::passes::PassError> for TetherError {
    fn from(err: crate::passes::PassError) -> Self {
        use crate::passes::PassError;
        match err {
            PassError::NoPassesRemaining { .. } => Self::NoPassesRemaining,
            PassError::EmptyReason => Self::EmptyPassReason,
            PassError::ReasonTooLong { max, actual } => Self::PassReasonTooLong { max, actual },
            PassError::ReadError { path, source } => {
                Self::PersistenceError(format!("Failed to read {}: {}", path.display(), source))
            }
            PassError::WriteError { path, source } => {
                Self::PersistenceError(format!("Failed to write {}: {}", path.display(), source))
            }
            PassError::ParseError { path, source } => {
                Self::ConfigParseError(format!("Failed to parse {}: {}", path.display(), source))
            }
            PassError::SerializeError(e) => Self::ConfigParseError(e.to_string()),
            PassError::CreateDirError { path, source } => Self::PersistenceError(format!(
                "Failed to create directory {}: {}",
                path.display(),
                source
            )),
        }
    }
}

impl From<crate::bluetooth::BluetoothError> for TetherError {
    fn from(err: crate::bluetooth::BluetoothError) -> Self {
        use crate::bluetooth::BluetoothError;
        match err {
            BluetoothError::AdapterNotFound => Self::BluetoothAdapterNotFound,
            BluetoothError::AdapterPoweredOff => Self::BluetoothAdapterPoweredOff,
            BluetoothError::DeviceNotFound { address } => Self::DeviceNotFound(address),
            BluetoothError::ScanTimeout { duration_secs } => {
                Self::BluetoothScanFailed(format!("Scan timed out after {} seconds", duration_secs))
            }
            BluetoothError::InvalidAddress { address } => {
                Self::ConfigValidationError(format!("Invalid Bluetooth address: {}", address))
            }
            BluetoothError::SessionInitFailed { message } => Self::BluetoothScanFailed(message),
            BluetoothError::DiscoveryFailed { message } => Self::BluetoothScanFailed(message),
            BluetoothError::Internal { message } => Self::BluetoothScanFailed(message),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoErr, ErrorKind};

    #[test]
    fn test_bluetooth_error_classification() {
        assert!(TetherError::BluetoothAdapterNotFound.is_bluetooth_error());
        assert!(TetherError::BluetoothAdapterPoweredOff.is_bluetooth_error());
        assert!(TetherError::BluetoothScanFailed("test".into()).is_bluetooth_error());
        assert!(TetherError::DeviceNotFound("iPhone".into()).is_bluetooth_error());

        assert!(!TetherError::NoPassesRemaining.is_bluetooth_error());
    }

    #[test]
    fn test_config_error_classification() {
        assert!(TetherError::ConfigNotFound(PathBuf::from("/test")).is_config_error());
        assert!(TetherError::ConfigParseError("syntax error".into()).is_config_error());
        assert!(TetherError::ConfigValidationError("invalid value".into()).is_config_error());

        assert!(!TetherError::BluetoothAdapterNotFound.is_config_error());
    }

    #[test]
    fn test_pass_error_classification() {
        assert!(TetherError::NoPassesRemaining.is_pass_error());
        assert!(TetherError::InvalidMonthFormat("bad-format".into()).is_pass_error());
        assert!(TetherError::EmptyPassReason.is_pass_error());
        assert!(TetherError::PassReasonTooLong {
            max: 500,
            actual: 600
        }
        .is_pass_error());

        assert!(!TetherError::BluetoothAdapterNotFound.is_pass_error());
    }

    #[test]
    fn test_io_error_classification() {
        assert!(TetherError::PersistenceError("disk full".into()).is_io_error());
        assert!(TetherError::IoError(IoErr::new(ErrorKind::NotFound, "test")).is_io_error());

        assert!(!TetherError::BluetoothAdapterNotFound.is_io_error());
    }

    #[test]
    fn test_expected_state() {
        assert!(TetherError::NoPassesRemaining.is_expected_state());
        assert!(!TetherError::BluetoothAdapterNotFound.is_expected_state());
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(TetherError::DeviceNotFound("iPhone".into()).is_recoverable());
        assert!(TetherError::BluetoothScanFailed("timeout".into()).is_recoverable());
        assert!(!TetherError::BluetoothAdapterNotFound.is_recoverable());
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(
            TetherError::InvalidMonthFormat("bad".into()).http_status_code(),
            400
        );
        assert_eq!(TetherError::NoPassesRemaining.http_status_code(), 403);
        assert_eq!(
            TetherError::ConfigNotFound(PathBuf::new()).http_status_code(),
            404
        );
        assert_eq!(
            TetherError::DeviceNotFound("iPhone".into()).http_status_code(),
            404
        );
        assert_eq!(
            TetherError::ConfigParseError("error".into()).http_status_code(),
            422
        );
        assert_eq!(
            TetherError::PersistenceError("error".into()).http_status_code(),
            500
        );
        assert_eq!(
            TetherError::BluetoothAdapterNotFound.http_status_code(),
            503
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            TetherError::BluetoothAdapterNotFound.error_code(),
            "BLUETOOTH_ADAPTER_NOT_FOUND"
        );
        assert_eq!(
            TetherError::NoPassesRemaining.error_code(),
            "NO_PASSES_REMAINING"
        );
        assert_eq!(
            TetherError::ConfigNotFound(PathBuf::new()).error_code(),
            "CONFIG_NOT_FOUND"
        );
    }

    #[test]
    fn test_from_io_error() {
        let io_err = IoErr::new(ErrorKind::NotFound, "file not found");
        let tether_err: TetherError = io_err.into();
        assert!(matches!(tether_err, TetherError::IoError(_)));
        assert!(tether_err.is_io_error());
    }

    #[test]
    fn test_error_display_messages() {
        let err = TetherError::BluetoothAdapterNotFound;
        assert!(format!("{}", err).contains("No Bluetooth adapter found"));

        let err = TetherError::NoPassesRemaining;
        assert!(format!("{}", err).contains("No passes remaining"));

        let err = TetherError::DeviceNotFound("MyiPhone".into());
        assert!(format!("{}", err).contains("MyiPhone"));
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<TetherError>();
        assert_sync::<TetherError>();
    }

    #[test]
    fn test_result_type_alias() {
        fn example_function() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(example_function().unwrap(), 42);

        fn failing_function() -> Result<i32> {
            Err(TetherError::NoPassesRemaining)
        }
        assert!(failing_function().is_err());
    }
}
