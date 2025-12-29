//! # tether-core
//!
//! Core business logic for the tether phone proximity tracking system.
//!
//! This crate provides:
//! - Bluetooth device discovery and proximity detection
//! - Pass management (monthly passes with history tracking)
//! - Configuration management (Wi-Fi, Bluetooth device, timezone)
//! - Persistent storage for pass data
//!
//! ## Architecture
//!
//! The crate is organized into the following modules:
//!
//! - [`bluetooth`] - Bluetooth Low Energy scanning and RSSI-based proximity detection
//! - [`config`] - Application configuration loading, saving, and validation
//! - [`passes`] - Monthly pass allocation, usage tracking, and history
//! - [`storage`] - Persistent storage for pass data using JSON files
//! - [`error`] - Unified error types for the crate
//! - [`types`] - Shared types and OpenAPI schemas

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![warn(missing_docs)]

pub mod bluetooth;
pub mod config;
pub mod error;
pub mod passes;
pub mod storage;
pub mod types;

// Re-export primary types for convenience
#[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
pub use bluetooth::MockDevice;
pub use bluetooth::{
    BluetoothConfig as BtConfig, BluetoothDevice, BluetoothError, BluetoothResult,
    BluetoothScanner, ProximityResult,
};
pub use config::{
    is_valid_mac_address, is_valid_timezone_format, BluetoothConfig, Config, ConfigError,
    ConfigResult, PassesConfig, SystemConfig, WifiConfig, WifiNetwork,
};
pub use error::{Error, Result, TetherError};
pub use passes::{
    current_month_string, is_valid_month_string, PassData, PassEntry, PassError, PassManager,
    PassResult, MAX_REASON_LENGTH,
};
pub use storage::{default_data_dir, default_passes_path};
pub use types::HealthResponse;
