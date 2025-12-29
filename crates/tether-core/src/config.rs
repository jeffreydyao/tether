//! Application configuration management.
//!
//! Handles loading, saving, and validating tether configuration including:
//! - Bluetooth device to track
//! - RSSI threshold for proximity
//! - Monthly pass allocation
//! - Timezone settings
//! - Wi-Fi network configuration

use std::path::PathBuf;

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TetherConfig {
    /// Bluetooth device MAC address to track.
    pub bluetooth_device: Option<String>,

    /// RSSI threshold for considering device "nearby".
    /// Typical values: -70 (far) to -30 (very close).
    pub rssi_threshold: i16,

    /// Number of passes available per month.
    pub passes_per_month: u8,

    /// Timezone for pass refresh (midnight local time).
    #[serde(with = "timezone_serde")]
    pub timezone: Tz,

    /// Wi-Fi networks in priority order.
    pub wifi_networks: Vec<WifiNetwork>,

    /// Whether initial setup has been completed.
    pub setup_completed: bool,
}

/// Wi-Fi network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    /// Network SSID.
    pub ssid: String,

    /// Network password (stored securely).
    #[serde(skip_serializing)]
    pub password: Option<String>,

    /// Whether this is the primary network.
    pub is_primary: bool,
}

impl Default for TetherConfig {
    fn default() -> Self {
        Self {
            bluetooth_device: None,
            rssi_threshold: -60,
            passes_per_month: 3,
            timezone: chrono_tz::UTC,
            wifi_networks: Vec::new(),
            setup_completed: false,
        }
    }
}

impl TetherConfig {
    /// Load configuration from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be written.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the configuration file path.
    fn config_path() -> Result<PathBuf> {
        // On Raspberry Pi: /etc/tether/config.toml
        // For development: ~/.config/tether/config.toml
        #[cfg(target_os = "linux")]
        {
            Ok(PathBuf::from("/etc/tether/config.toml"))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let dirs = directories::ProjectDirs::from("", "", "tether").ok_or_else(|| {
                crate::error::Error::Config("Cannot determine config directory".into())
            })?;
            Ok(dirs.config_dir().join("config.toml"))
        }
    }
}

mod timezone_serde {
    use chrono_tz::Tz;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(tz: &Tz, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(tz.name())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Tz, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
