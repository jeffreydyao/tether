//! Configuration management for the Tether phone proximity tracker.
//!
//! This module provides:
//! - Strongly-typed configuration structures with serde support
//! - TOML file loading and saving
//! - Validation for all configuration fields
//! - Sensible defaults where appropriate
//!
//! # Configuration File Location
//!
//! The default configuration file is located at `/etc/tether/config.toml` on the
//! Raspberry Pi. For development, use a local path.
//!
//! # Example
//!
//! ```rust,no_run
//! use tether_core::config::Config;
//!
//! // Load configuration
//! let config = Config::load("/etc/tether/config.toml")?;
//!
//! // Access Bluetooth settings
//! println!("Tracking device: {}", config.bluetooth.target_name);
//!
//! // Modify and save
//! let mut config = config;
//! config.passes.pending_per_month = Some(5);
//! config.save("/etc/tether/config.toml")?;
//! # Ok::<(), tether_core::config::ConfigError>(())
//! ```

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Errors that can occur during configuration operations.
///
/// This enum covers all failure modes for loading, saving, and validating
/// configuration. Each variant includes contextual information to help
/// diagnose the issue.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read the configuration file from disk.
    ///
    /// This typically occurs when:
    /// - The file does not exist
    /// - The process lacks read permissions
    /// - The path is invalid
    #[error("Failed to read configuration file '{path}': {source}")]
    ReadError {
        /// Path to the file that could not be read.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write the configuration file to disk.
    ///
    /// This typically occurs when:
    /// - The directory does not exist
    /// - The process lacks write permissions
    /// - The disk is full
    #[error("Failed to write configuration file '{path}': {source}")]
    WriteError {
        /// Path to the file that could not be written.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse the TOML content.
    ///
    /// This occurs when the file contains invalid TOML syntax or
    /// the structure does not match the expected configuration schema.
    #[error("Failed to parse TOML configuration: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize the configuration to TOML.
    ///
    /// This is rare but can occur with certain edge cases in string formatting.
    #[error("Failed to serialize configuration to TOML: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// The configuration file does not exist at the specified path.
    ///
    /// Use `Config::default()` to create a new configuration or ensure
    /// the file exists before loading.
    #[error("Configuration file not found: {0}")]
    NotFound(String),

    /// A configuration value failed validation.
    ///
    /// The `field` indicates which configuration key is invalid, and
    /// `message` describes what is wrong with the value.
    #[error("Invalid configuration value for '{field}': {message}")]
    ValidationError {
        /// The configuration field that failed validation.
        field: String,
        /// Description of what is wrong with the value.
        message: String,
    },

    /// Multiple validation errors occurred.
    ///
    /// This is returned by `Config::validate()` when multiple fields
    /// have invalid values. All errors are collected rather than
    /// failing on the first one.
    #[error("Configuration validation failed with {} error(s)", .0.len())]
    MultipleValidationErrors(Vec<ConfigError>),
}

/// Result type alias for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

// =============================================================================
// BLUETOOTH CONFIGURATION
// =============================================================================

/// Bluetooth device tracking configuration.
///
/// Specifies which Bluetooth device to monitor for proximity detection.
/// The device is identified by its MAC address and optionally by its
/// advertised name.
///
/// # RSSI Threshold
///
/// The `rssi_threshold` determines how close the device must be to be
/// considered "nearby". RSSI (Received Signal Strength Indicator) is
/// measured in dBm:
///
/// - `-30 dBm`: Extremely close (within ~1 meter)
/// - `-50 dBm`: Close (within ~3 meters)
/// - `-70 dBm`: Medium range (within ~10 meters)
/// - `-90 dBm`: Far (edge of Bluetooth range)
///
/// A device with RSSI **greater than or equal to** the threshold is
/// considered nearby.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BluetoothConfig {
    /// The MAC address of the target Bluetooth device.
    ///
    /// Must be in standard format: `XX:XX:XX:XX:XX:XX` where X is a
    /// hexadecimal digit (0-9, A-F, a-f). Both uppercase and lowercase
    /// are accepted.
    ///
    /// # Example
    ///
    /// ```text
    /// "A4:C1:38:12:34:56"
    /// "a4:c1:38:12:34:56"
    /// ```
    pub target_address: String,

    /// Human-readable name of the target device.
    ///
    /// This is the Bluetooth device name as advertised by the phone.
    /// It is used for display purposes in the UI and logs. The actual
    /// device matching is done by MAC address.
    ///
    /// # Example
    ///
    /// ```text
    /// "Jeffrey's iPhone"
    /// "Pixel 8 Pro"
    /// ```
    pub target_name: String,

    /// RSSI threshold for proximity detection (in dBm).
    ///
    /// Devices with RSSI >= this value are considered "nearby".
    /// Typical values range from -90 (far) to -30 (very close).
    ///
    /// # Default
    ///
    /// The default value is `-60` dBm, which typically corresponds to
    /// a device within about 5 meters.
    #[serde(default = "default_rssi_threshold")]
    pub rssi_threshold: i8,
}

/// Returns the default RSSI threshold (-60 dBm).
fn default_rssi_threshold() -> i8 {
    -60
}

impl Default for BluetoothConfig {
    /// Creates a placeholder Bluetooth configuration.
    ///
    /// The default values use placeholder strings that must be replaced
    /// during onboarding. The RSSI threshold is set to -60 dBm.
    fn default() -> Self {
        Self {
            target_address: String::from("00:00:00:00:00:00"),
            target_name: String::from("Unconfigured Device"),
            rssi_threshold: default_rssi_threshold(),
        }
    }
}

impl BluetoothConfig {
    /// Validates the Bluetooth configuration.
    ///
    /// # Validation Rules
    ///
    /// - `target_address` must be a valid MAC address in `XX:XX:XX:XX:XX:XX` format
    /// - `target_name` must not be empty
    /// - `rssi_threshold` must be between -100 and 0 dBm
    ///
    /// # Returns
    ///
    /// A vector of validation errors. Empty if all fields are valid.
    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        // Validate MAC address format
        if !is_valid_mac_address(&self.target_address) {
            errors.push(ConfigError::ValidationError {
                field: "bluetooth.target_address".to_string(),
                message: format!(
                    "Invalid MAC address format '{}'. Expected format: XX:XX:XX:XX:XX:XX",
                    self.target_address
                ),
            });
        }

        // Validate target name is not empty
        if self.target_name.trim().is_empty() {
            errors.push(ConfigError::ValidationError {
                field: "bluetooth.target_name".to_string(),
                message: "Target device name cannot be empty".to_string(),
            });
        }

        // Validate RSSI is in reasonable range
        if self.rssi_threshold > 0 || self.rssi_threshold < -100 {
            errors.push(ConfigError::ValidationError {
                field: "bluetooth.rssi_threshold".to_string(),
                message: format!(
                    "RSSI threshold {} is out of valid range (-100 to 0 dBm)",
                    self.rssi_threshold
                ),
            });
        }

        errors
    }
}

// =============================================================================
// WIFI CONFIGURATION
// =============================================================================

/// A single WiFi network configuration.
///
/// Represents credentials and settings for one WiFi network that the
/// Raspberry Pi can connect to. Networks are tried in order when the
/// primary network is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WifiNetwork {
    /// The SSID (network name) of the WiFi network.
    ///
    /// This is case-sensitive and must match the network's advertised name
    /// exactly. Hidden networks are supported.
    ///
    /// # Example
    ///
    /// ```text
    /// "HomeNetwork"
    /// "Starbucks Free WiFi"
    /// ```
    pub ssid: String,

    /// The password for the WiFi network.
    ///
    /// For open networks (no password), use an empty string.
    /// WPA2/WPA3 passwords are supported.
    ///
    /// # Security Note
    ///
    /// This value is stored in plaintext in the configuration file.
    /// Ensure the configuration file has appropriate permissions (0600).
    #[serde(default)]
    pub password: String,

    /// Whether this is the primary network.
    ///
    /// Only one network should be marked as primary. The primary network
    /// is tried first when connecting. If multiple networks are marked
    /// as primary, the first one in the list takes precedence.
    ///
    /// # Default
    ///
    /// `false` - Networks are not primary by default.
    #[serde(default)]
    pub primary: bool,
}

impl WifiNetwork {
    /// Creates a new WiFi network configuration.
    ///
    /// # Arguments
    ///
    /// * `ssid` - The network name
    /// * `password` - The network password (empty string for open networks)
    /// * `primary` - Whether this is the primary network
    ///
    /// # Example
    ///
    /// ```rust
    /// use tether_core::config::WifiNetwork;
    ///
    /// let home = WifiNetwork::new("HomeNetwork", "secret123", true);
    /// let backup = WifiNetwork::new("MobileHotspot", "backup456", false);
    /// ```
    pub fn new(ssid: impl Into<String>, password: impl Into<String>, primary: bool) -> Self {
        Self {
            ssid: ssid.into(),
            password: password.into(),
            primary,
        }
    }

    /// Validates the WiFi network configuration.
    ///
    /// # Validation Rules
    ///
    /// - `ssid` must not be empty
    /// - `ssid` must not exceed 32 characters (IEEE 802.11 limit)
    ///
    /// # Returns
    ///
    /// A vector of validation errors. Empty if all fields are valid.
    pub fn validate(&self, index: usize) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        if self.ssid.trim().is_empty() {
            errors.push(ConfigError::ValidationError {
                field: format!("wifi.networks[{}].ssid", index),
                message: "WiFi SSID cannot be empty".to_string(),
            });
        }

        if self.ssid.len() > 32 {
            errors.push(ConfigError::ValidationError {
                field: format!("wifi.networks[{}].ssid", index),
                message: format!(
                    "WiFi SSID '{}' exceeds maximum length of 32 characters",
                    self.ssid
                ),
            });
        }

        errors
    }
}

/// WiFi configuration containing all configured networks.
///
/// Maintains an ordered list of WiFi networks. The primary network is
/// tried first, followed by the others in list order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WifiConfig {
    /// List of configured WiFi networks.
    ///
    /// At least one network should be configured after onboarding.
    /// Networks are tried in order when connecting.
    #[serde(default)]
    pub networks: Vec<WifiNetwork>,
}

impl WifiConfig {
    /// Returns the primary WiFi network, if configured.
    ///
    /// If multiple networks are marked as primary, returns the first one.
    /// Returns `None` if no networks are configured or none are marked primary.
    pub fn primary_network(&self) -> Option<&WifiNetwork> {
        self.networks.iter().find(|n| n.primary)
    }

    /// Returns a mutable reference to the primary WiFi network.
    pub fn primary_network_mut(&mut self) -> Option<&mut WifiNetwork> {
        self.networks.iter_mut().find(|n| n.primary)
    }

    /// Sets a network as primary by SSID.
    ///
    /// Clears the primary flag from all other networks and sets it on the
    /// network with the matching SSID.
    ///
    /// # Returns
    ///
    /// `true` if a network with the SSID was found and marked as primary,
    /// `false` if no matching network exists.
    pub fn set_primary(&mut self, ssid: &str) -> bool {
        let mut found = false;
        for network in &mut self.networks {
            if network.ssid == ssid {
                network.primary = true;
                found = true;
            } else {
                network.primary = false;
            }
        }
        found
    }

    /// Validates the WiFi configuration.
    ///
    /// # Validation Rules
    ///
    /// - Each network must pass its individual validation
    /// - No duplicate SSIDs are allowed
    /// - At most one network should be marked as primary (warning only)
    ///
    /// # Returns
    ///
    /// A vector of validation errors. Empty if all fields are valid.
    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        // Validate each network
        for (index, network) in self.networks.iter().enumerate() {
            errors.extend(network.validate(index));
        }

        // Check for duplicate SSIDs
        let mut seen_ssids = std::collections::HashSet::new();
        for (index, network) in self.networks.iter().enumerate() {
            if !seen_ssids.insert(&network.ssid) {
                errors.push(ConfigError::ValidationError {
                    field: format!("wifi.networks[{}].ssid", index),
                    message: format!("Duplicate WiFi SSID '{}' found", network.ssid),
                });
            }
        }

        errors
    }
}

// =============================================================================
// PASSES CONFIGURATION
// =============================================================================

/// Configuration for the monthly pass system.
///
/// Passes allow the user to keep their phone nearby on nights when they
/// need it (emergencies, on-call, etc.). The number of passes refreshes
/// at midnight on the first day of each month.
///
/// # Pending Changes
///
/// If the user changes `per_month` mid-month, the change takes effect
/// the following month. The `pending_per_month` field stores this
/// pending value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PassesConfig {
    /// Number of passes granted per month.
    ///
    /// This value is immutable during a month. To change it, set
    /// `pending_per_month` which will take effect next month.
    ///
    /// # Default
    ///
    /// 3 passes per month.
    #[serde(default = "default_passes_per_month")]
    pub per_month: u8,

    /// Pending change to passes per month.
    ///
    /// If set, this value will replace `per_month` at midnight on the
    /// first day of next month. After the change takes effect, this
    /// field is cleared (set to `None`).
    ///
    /// # Example
    ///
    /// User has 3 passes per month. On January 15th, they change it to 5.
    /// - `per_month` remains 3 until January 31st
    /// - `pending_per_month` is set to `Some(5)`
    /// - On February 1st, `per_month` becomes 5 and `pending_per_month` is cleared
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_per_month: Option<u8>,
}

/// Returns the default passes per month (3).
fn default_passes_per_month() -> u8 {
    3
}

impl Default for PassesConfig {
    fn default() -> Self {
        Self {
            per_month: default_passes_per_month(),
            pending_per_month: None,
        }
    }
}

impl PassesConfig {
    /// Returns the effective passes per month.
    ///
    /// This always returns `per_month`, not the pending value.
    /// The pending value only takes effect after the month changes.
    pub fn effective_per_month(&self) -> u8 {
        self.per_month
    }

    /// Sets a pending change to passes per month.
    ///
    /// The change will take effect at the start of next month.
    /// Call `apply_pending()` at month rollover.
    pub fn set_pending(&mut self, value: u8) {
        self.pending_per_month = Some(value);
    }

    /// Applies any pending passes change.
    ///
    /// Call this method at midnight on the first day of each month.
    /// If there is a pending change, it becomes the new `per_month`
    /// value and the pending field is cleared.
    ///
    /// # Returns
    ///
    /// `true` if a pending change was applied, `false` otherwise.
    pub fn apply_pending(&mut self) -> bool {
        if let Some(pending) = self.pending_per_month.take() {
            self.per_month = pending;
            true
        } else {
            false
        }
    }

    /// Validates the passes configuration.
    ///
    /// # Validation Rules
    ///
    /// - `per_month` must be between 0 and 31 (max days in a month)
    /// - `pending_per_month` (if set) must be between 0 and 31
    ///
    /// # Returns
    ///
    /// A vector of validation errors. Empty if all fields are valid.
    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        if self.per_month > 31 {
            errors.push(ConfigError::ValidationError {
                field: "passes.per_month".to_string(),
                message: format!("Passes per month ({}) cannot exceed 31", self.per_month),
            });
        }

        if let Some(pending) = self.pending_per_month {
            if pending > 31 {
                errors.push(ConfigError::ValidationError {
                    field: "passes.pending_per_month".to_string(),
                    message: format!("Pending passes per month ({}) cannot exceed 31", pending),
                });
            }
        }

        errors
    }
}

// =============================================================================
// SYSTEM CONFIGURATION
// =============================================================================

/// System-level configuration.
///
/// Contains settings that affect the overall system behavior, such as
/// timezone and onboarding state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemConfig {
    /// The IANA timezone string for the system.
    ///
    /// This is used for determining midnight for pass refresh and for
    /// displaying times in the UI. The timezone is typically auto-detected
    /// from the connected WiFi network during onboarding.
    ///
    /// # Format
    ///
    /// Standard IANA timezone format: `Region/City`
    ///
    /// # Examples
    ///
    /// ```text
    /// "America/New_York"
    /// "Europe/London"
    /// "Asia/Tokyo"
    /// "Australia/Sydney"
    /// ```
    ///
    /// # Default
    ///
    /// `"UTC"` - Universal Coordinated Time
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// Whether the onboarding process has been completed.
    ///
    /// This flag is set to `true` after the user completes the initial
    /// setup wizard. When `false`, the web UI shows the onboarding flow.
    ///
    /// # Default
    ///
    /// `false` - Onboarding is required by default.
    #[serde(default)]
    pub onboarding_complete: bool,
}

/// Returns the default timezone ("UTC").
fn default_timezone() -> String {
    String::from("UTC")
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            timezone: default_timezone(),
            onboarding_complete: false,
        }
    }
}

impl SystemConfig {
    /// Validates the system configuration.
    ///
    /// # Validation Rules
    ///
    /// - `timezone` must not be empty
    /// - `timezone` must be a valid IANA timezone identifier
    ///
    /// # Note
    ///
    /// Full timezone validation requires the `chrono-tz` crate. This
    /// implementation performs basic format validation only.
    ///
    /// # Returns
    ///
    /// A vector of validation errors. Empty if all fields are valid.
    pub fn validate(&self) -> Vec<ConfigError> {
        let mut errors = Vec::new();

        if self.timezone.trim().is_empty() {
            errors.push(ConfigError::ValidationError {
                field: "system.timezone".to_string(),
                message: "Timezone cannot be empty".to_string(),
            });
        }

        // Basic timezone format validation (Region/City or single word like UTC)
        if !is_valid_timezone_format(&self.timezone) {
            errors.push(ConfigError::ValidationError {
                field: "system.timezone".to_string(),
                message: format!(
                    "Invalid timezone format '{}'. Expected IANA format like 'America/New_York'",
                    self.timezone
                ),
            });
        }

        errors
    }
}

// =============================================================================
// MAIN CONFIG STRUCT
// =============================================================================

/// The complete Tether configuration.
///
/// This is the root configuration structure that contains all settings
/// for the Tether phone proximity tracker. It is serialized to and
/// deserialized from a TOML file.
///
/// # File Location
///
/// The default configuration file is at `/etc/tether/config.toml`.
///
/// # Example TOML
///
/// ```toml
/// [bluetooth]
/// target_address = "A4:C1:38:12:34:56"
/// target_name = "Jeffrey's iPhone"
/// rssi_threshold = -60
///
/// [wifi]
/// [[wifi.networks]]
/// ssid = "HomeNetwork"
/// password = "secret123"
/// primary = true
///
/// [[wifi.networks]]
/// ssid = "MobileHotspot"
/// password = "backup456"
/// primary = false
///
/// [passes]
/// per_month = 3
///
/// [system]
/// timezone = "America/New_York"
/// onboarding_complete = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Bluetooth device tracking configuration.
    pub bluetooth: BluetoothConfig,

    /// WiFi network configuration.
    #[serde(default)]
    pub wifi: WifiConfig,

    /// Monthly pass configuration.
    #[serde(default)]
    pub passes: PassesConfig,

    /// System configuration.
    #[serde(default)]
    pub system: SystemConfig,
}

impl Default for Config {
    /// Creates a default configuration for first-time setup.
    ///
    /// The default configuration has:
    /// - Placeholder Bluetooth settings (must be configured during onboarding)
    /// - No WiFi networks configured
    /// - 3 passes per month
    /// - UTC timezone
    /// - Onboarding not complete
    fn default() -> Self {
        Self {
            bluetooth: BluetoothConfig::default(),
            wifi: WifiConfig::default(),
            passes: PassesConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

impl Config {
    /// Loads configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The parsed configuration, or an error if the file cannot be read
    /// or contains invalid TOML.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::NotFound`] - The file does not exist
    /// - [`ConfigError::ReadError`] - Failed to read the file
    /// - [`ConfigError::ParseError`] - Invalid TOML syntax or structure
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tether_core::config::Config;
    ///
    /// let config = Config::load("/etc/tether/config.toml")?;
    /// println!("Tracking: {}", config.bluetooth.target_name);
    /// # Ok::<(), tether_core::config::ConfigError>(())
    /// ```
    pub fn load<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        // Check if file exists
        if !path.exists() {
            return Err(ConfigError::NotFound(path_str));
        }

        // Read file contents
        let contents = fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
            path: path_str.clone(),
            source: e,
        })?;

        // Parse TOML
        let config: Config = toml::from_str(&contents)?;

        Ok(config)
    }

    /// Loads configuration from a TOML file, with validation.
    ///
    /// This method loads the configuration and validates all fields.
    /// If validation fails, it returns a [`ConfigError::MultipleValidationErrors`]
    /// containing all validation errors found.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The parsed and validated configuration.
    ///
    /// # Errors
    ///
    /// All errors from [`Config::load`], plus:
    /// - [`ConfigError::MultipleValidationErrors`] - One or more fields failed validation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tether_core::config::Config;
    ///
    /// match Config::load_and_validate("/etc/tether/config.toml") {
    ///     Ok(config) => println!("Config loaded successfully"),
    ///     Err(e) => eprintln!("Config error: {}", e),
    /// }
    /// ```
    pub fn load_and_validate<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let config = Self::load(path)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads configuration or returns default if file doesn't exist.
    ///
    /// This is useful during first boot when no configuration file exists yet.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The loaded configuration, or a default configuration if the file
    /// doesn't exist. Other errors (read errors, parse errors) are still returned.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tether_core::config::Config;
    ///
    /// let config = Config::load_or_default("/etc/tether/config.toml")?;
    /// if !config.system.onboarding_complete {
    ///     println!("Starting onboarding...");
    /// }
    /// # Ok::<(), tether_core::config::ConfigError>(())
    /// ```
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        match Self::load(path) {
            Ok(config) => Ok(config),
            Err(ConfigError::NotFound(_)) => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Saves configuration to a TOML file.
    ///
    /// The configuration is serialized to TOML format and written to the
    /// specified path. Parent directories must already exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to write the configuration file
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::SerializeError`] - Failed to serialize to TOML
    /// - [`ConfigError::WriteError`] - Failed to write the file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tether_core::config::Config;
    ///
    /// let mut config = Config::default();
    /// config.bluetooth.target_address = "A4:C1:38:12:34:56".to_string();
    /// config.bluetooth.target_name = "My Phone".to_string();
    /// config.save("/etc/tether/config.toml")?;
    /// # Ok::<(), tether_core::config::ConfigError>(())
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        // Serialize to TOML
        let contents = toml::to_string_pretty(self)?;

        // Write to file
        fs::write(path, contents).map_err(|e| ConfigError::WriteError {
            path: path_str,
            source: e,
        })?;

        Ok(())
    }

    /// Saves configuration with validation.
    ///
    /// Validates the configuration before saving. If validation fails,
    /// the file is not written.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to write the configuration file
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// All errors from [`Config::save`], plus:
    /// - [`ConfigError::MultipleValidationErrors`] - One or more fields failed validation
    pub fn validate_and_save<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        self.validate()?;
        self.save(path)
    }

    /// Validates all configuration fields.
    ///
    /// Checks all configuration values against their validation rules.
    /// All errors are collected and returned together.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all fields are valid.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::MultipleValidationErrors`] - Contains all validation errors found
    ///
    /// # Example
    ///
    /// ```rust
    /// use tether_core::config::Config;
    ///
    /// let config = Config::default();
    /// match config.validate() {
    ///     Ok(()) => println!("Configuration is valid"),
    ///     Err(e) => eprintln!("Validation errors: {}", e),
    /// }
    /// ```
    pub fn validate(&self) -> ConfigResult<()> {
        let mut errors = Vec::new();

        errors.extend(self.bluetooth.validate());
        errors.extend(self.wifi.validate());
        errors.extend(self.passes.validate());
        errors.extend(self.system.validate());

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::MultipleValidationErrors(errors))
        }
    }

    /// Checks if onboarding is complete.
    ///
    /// Convenience method that checks `system.onboarding_complete`.
    pub fn is_onboarding_complete(&self) -> bool {
        self.system.onboarding_complete
    }

    /// Marks onboarding as complete.
    ///
    /// Sets `system.onboarding_complete` to `true`.
    pub fn complete_onboarding(&mut self) {
        self.system.onboarding_complete = true;
    }
}

// =============================================================================
// VALIDATION HELPERS
// =============================================================================

/// Lazy-compiled regex for MAC address validation.
///
/// Matches MAC addresses in format `XX:XX:XX:XX:XX:XX` where X is
/// a hexadecimal digit (0-9, A-F, a-f).
static MAC_ADDRESS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$").expect("Invalid MAC address regex pattern")
});

/// Lazy-compiled regex for basic timezone format validation.
///
/// Matches IANA timezone formats:
/// - Single word: `UTC`, `GMT`
/// - Region/City: `America/New_York`, `Europe/London`
/// - Region/Sub/City: `America/Indiana/Indianapolis`
static TIMEZONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z_]+(/[A-Za-z_]+)*$").expect("Invalid timezone regex pattern")
});

/// Validates a MAC address string.
///
/// # Arguments
///
/// * `address` - The MAC address to validate
///
/// # Returns
///
/// `true` if the address is a valid MAC address in `XX:XX:XX:XX:XX:XX` format.
///
/// # Example
///
/// ```rust
/// use tether_core::config::is_valid_mac_address;
///
/// assert!(is_valid_mac_address("A4:C1:38:12:34:56"));
/// assert!(is_valid_mac_address("a4:c1:38:12:34:56")); // lowercase OK
/// assert!(!is_valid_mac_address("A4:C1:38:12:34"));   // too short
/// assert!(!is_valid_mac_address("A4-C1-38-12-34-56")); // wrong delimiter
/// ```
pub fn is_valid_mac_address(address: &str) -> bool {
    MAC_ADDRESS_REGEX.is_match(address)
}

/// Validates a timezone string format.
///
/// This performs basic format validation only. It does not verify that
/// the timezone is a valid IANA timezone identifier.
///
/// # Arguments
///
/// * `timezone` - The timezone string to validate
///
/// # Returns
///
/// `true` if the timezone has valid format.
///
/// # Example
///
/// ```rust
/// use tether_core::config::is_valid_timezone_format;
///
/// assert!(is_valid_timezone_format("UTC"));
/// assert!(is_valid_timezone_format("America/New_York"));
/// assert!(is_valid_timezone_format("America/Indiana/Indianapolis"));
/// assert!(!is_valid_timezone_format(""));
/// assert!(!is_valid_timezone_format("New York")); // space not allowed
/// ```
pub fn is_valid_timezone_format(timezone: &str) -> bool {
    if timezone.is_empty() {
        return false;
    }
    TIMEZONE_REGEX.is_match(timezone)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // -------------------------------------------------------------------------
    // MAC Address Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_mac_addresses() {
        assert!(is_valid_mac_address("A4:C1:38:12:34:56"));
        assert!(is_valid_mac_address("a4:c1:38:12:34:56"));
        assert!(is_valid_mac_address("00:00:00:00:00:00"));
        assert!(is_valid_mac_address("FF:FF:FF:FF:FF:FF"));
        assert!(is_valid_mac_address("aB:cD:eF:12:34:56"));
    }

    #[test]
    fn test_invalid_mac_addresses() {
        assert!(!is_valid_mac_address(""));
        assert!(!is_valid_mac_address("A4:C1:38:12:34")); // too short
        assert!(!is_valid_mac_address("A4:C1:38:12:34:56:78")); // too long
        assert!(!is_valid_mac_address("A4-C1-38-12-34-56")); // wrong delimiter
        assert!(!is_valid_mac_address("A4.C1.38.12.34.56")); // dot delimiter
        assert!(!is_valid_mac_address("A4:C1:38:12:34:GG")); // invalid hex
        assert!(!is_valid_mac_address("A4:C1:38:12:34:5")); // missing digit
        assert!(!is_valid_mac_address("A4:C1:38:12:34:567")); // extra digit
    }

    // -------------------------------------------------------------------------
    // Timezone Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_timezone_formats() {
        assert!(is_valid_timezone_format("UTC"));
        assert!(is_valid_timezone_format("GMT"));
        assert!(is_valid_timezone_format("America/New_York"));
        assert!(is_valid_timezone_format("Europe/London"));
        assert!(is_valid_timezone_format("Asia/Tokyo"));
        assert!(is_valid_timezone_format("America/Indiana/Indianapolis"));
        assert!(is_valid_timezone_format("Etc/GMT"));
    }

    #[test]
    fn test_invalid_timezone_formats() {
        assert!(!is_valid_timezone_format(""));
        assert!(!is_valid_timezone_format("New York")); // space
        assert!(!is_valid_timezone_format("America/New York")); // space
        assert!(!is_valid_timezone_format("/America")); // leading slash
        assert!(!is_valid_timezone_format("America/")); // trailing slash
        assert!(!is_valid_timezone_format("123/ABC")); // numbers
    }

    // -------------------------------------------------------------------------
    // BluetoothConfig Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_bluetooth_config_default() {
        let config = BluetoothConfig::default();
        assert_eq!(config.target_address, "00:00:00:00:00:00");
        assert_eq!(config.target_name, "Unconfigured Device");
        assert_eq!(config.rssi_threshold, -60);
    }

    #[test]
    fn test_bluetooth_config_validation_valid() {
        let config = BluetoothConfig {
            target_address: "A4:C1:38:12:34:56".to_string(),
            target_name: "My iPhone".to_string(),
            rssi_threshold: -60,
        };
        assert!(config.validate().is_empty());
    }

    #[test]
    fn test_bluetooth_config_validation_invalid_mac() {
        let config = BluetoothConfig {
            target_address: "invalid".to_string(),
            target_name: "My iPhone".to_string(),
            rssi_threshold: -60,
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ConfigError::ValidationError { field, .. } if field == "bluetooth.target_address"
        ));
    }

    #[test]
    fn test_bluetooth_config_validation_empty_name() {
        let config = BluetoothConfig {
            target_address: "A4:C1:38:12:34:56".to_string(),
            target_name: "   ".to_string(),
            rssi_threshold: -60,
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ConfigError::ValidationError { field, .. } if field == "bluetooth.target_name"
        ));
    }

    #[test]
    fn test_bluetooth_config_validation_rssi_out_of_range() {
        let config = BluetoothConfig {
            target_address: "A4:C1:38:12:34:56".to_string(),
            target_name: "My iPhone".to_string(),
            rssi_threshold: 10, // Invalid: positive
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ConfigError::ValidationError { field, .. } if field == "bluetooth.rssi_threshold"
        ));
    }

    // -------------------------------------------------------------------------
    // WifiConfig Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_wifi_network_new() {
        let network = WifiNetwork::new("HomeNetwork", "password123", true);
        assert_eq!(network.ssid, "HomeNetwork");
        assert_eq!(network.password, "password123");
        assert!(network.primary);
    }

    #[test]
    fn test_wifi_config_primary_network() {
        let mut config = WifiConfig {
            networks: vec![
                WifiNetwork::new("Network1", "pass1", false),
                WifiNetwork::new("Network2", "pass2", true),
                WifiNetwork::new("Network3", "pass3", false),
            ],
        };

        let primary = config.primary_network().unwrap();
        assert_eq!(primary.ssid, "Network2");

        // Test set_primary
        assert!(config.set_primary("Network3"));
        let primary = config.primary_network().unwrap();
        assert_eq!(primary.ssid, "Network3");

        // Test set_primary with non-existent SSID
        assert!(!config.set_primary("NonExistent"));
    }

    #[test]
    fn test_wifi_config_validation_duplicate_ssids() {
        let config = WifiConfig {
            networks: vec![
                WifiNetwork::new("SameNetwork", "pass1", true),
                WifiNetwork::new("SameNetwork", "pass2", false),
            ],
        };
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigError::ValidationError { message, .. } if message.contains("Duplicate")
        )));
    }

    #[test]
    fn test_wifi_config_validation_ssid_too_long() {
        let config = WifiConfig {
            networks: vec![WifiNetwork::new(
                "This SSID is way too long and exceeds the 32 character limit",
                "password",
                true,
            )],
        };
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(
            e,
            ConfigError::ValidationError { message, .. } if message.contains("exceeds maximum length")
        )));
    }

    // -------------------------------------------------------------------------
    // PassesConfig Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_passes_config_default() {
        let config = PassesConfig::default();
        assert_eq!(config.per_month, 3);
        assert!(config.pending_per_month.is_none());
    }

    #[test]
    fn test_passes_config_pending() {
        let mut config = PassesConfig::default();
        assert_eq!(config.effective_per_month(), 3);

        config.set_pending(5);
        assert_eq!(config.effective_per_month(), 3); // Still 3
        assert_eq!(config.pending_per_month, Some(5));

        assert!(config.apply_pending());
        assert_eq!(config.effective_per_month(), 5);
        assert!(config.pending_per_month.is_none());

        // Applying again should return false
        assert!(!config.apply_pending());
    }

    #[test]
    fn test_passes_config_validation() {
        let config = PassesConfig {
            per_month: 50,                // Invalid: > 31
            pending_per_month: Some(100), // Invalid: > 31
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Config Load/Save Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_load_not_found() {
        let result = Config::load("/nonexistent/path/config.toml");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_config_load_or_default_returns_default() {
        let result = Config::load_or_default("/nonexistent/path/config.toml");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.system.onboarding_complete);
    }

    #[test]
    fn test_config_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let original = Config {
            bluetooth: BluetoothConfig {
                target_address: "A4:C1:38:12:34:56".to_string(),
                target_name: "Test Phone".to_string(),
                rssi_threshold: -70,
            },
            wifi: WifiConfig {
                networks: vec![
                    WifiNetwork::new("HomeNetwork", "password123", true),
                    WifiNetwork::new("BackupNetwork", "backup456", false),
                ],
            },
            passes: PassesConfig {
                per_month: 5,
                pending_per_month: Some(10),
            },
            system: SystemConfig {
                timezone: "America/New_York".to_string(),
                onboarding_complete: true,
            },
        };

        // Save
        original.save(&path).unwrap();

        // Load
        let loaded = Config::load(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn test_config_parse_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml {{{{").unwrap();

        let result = Config::load(temp_file.path());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_config_validate() {
        let config = Config {
            bluetooth: BluetoothConfig {
                target_address: "invalid".to_string(),
                target_name: "".to_string(),
                rssi_threshold: 10,
            },
            wifi: WifiConfig::default(),
            passes: PassesConfig {
                per_month: 100,
                pending_per_month: None,
            },
            system: SystemConfig {
                timezone: "".to_string(),
                onboarding_complete: false,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::MultipleValidationErrors(errors)) = result {
            assert!(errors.len() >= 4); // At least 4 validation errors
        }
    }

    // -------------------------------------------------------------------------
    // TOML Serialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_toml_serialization_format() {
        let config = Config {
            bluetooth: BluetoothConfig {
                target_address: "A4:C1:38:12:34:56".to_string(),
                target_name: "Jeffrey's iPhone".to_string(),
                rssi_threshold: -60,
            },
            wifi: WifiConfig {
                networks: vec![WifiNetwork::new("HomeNetwork", "secret123", true)],
            },
            passes: PassesConfig {
                per_month: 3,
                pending_per_month: None,
            },
            system: SystemConfig {
                timezone: "America/New_York".to_string(),
                onboarding_complete: true,
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Verify structure
        assert!(toml_str.contains("[bluetooth]"));
        assert!(toml_str.contains("[[wifi.networks]]"));
        assert!(toml_str.contains("[passes]"));
        assert!(toml_str.contains("[system]"));
        assert!(toml_str.contains("target_address = \"A4:C1:38:12:34:56\""));
    }

    #[test]
    fn test_toml_deserialization_with_defaults() {
        let toml_str = r#"
            [bluetooth]
            target_address = "A4:C1:38:12:34:56"
            target_name = "My Phone"
            # rssi_threshold omitted - should use default

            # wifi, passes, system sections omitted - should use defaults
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.bluetooth.rssi_threshold, -60); // default
        assert!(config.wifi.networks.is_empty()); // default
        assert_eq!(config.passes.per_month, 3); // default
        assert_eq!(config.system.timezone, "UTC"); // default
        assert!(!config.system.onboarding_complete); // default
    }
}
