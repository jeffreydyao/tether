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
pub use bluetooth::BluetoothScanner;
pub use config::{
    is_valid_mac_address, is_valid_timezone_format, BluetoothConfig, Config, ConfigError,
    ConfigResult, PassesConfig, SystemConfig, WifiConfig, WifiNetwork,
};
pub use error::{Error, Result};
pub use passes::PassManager;
pub use storage::Storage;
pub use types::*;
