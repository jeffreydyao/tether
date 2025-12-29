//! Error types for tether-core.

use thiserror::Error;

/// Result type alias using tether's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for tether-core operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Bluetooth-related errors.
    #[error("Bluetooth error: {0}")]
    Bluetooth(String),

    /// Configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Storage/persistence errors.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Pass management errors.
    #[error("Pass error: {0}")]
    Pass(String),

    /// No passes remaining for the current month.
    #[error("No passes remaining for this month")]
    NoPassesRemaining,

    /// Device not found.
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// IO errors.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// TOML parsing errors.
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// TOML serialization errors.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// JSON errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
