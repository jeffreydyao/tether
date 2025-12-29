//! Bluetooth RSSI proximity detection for Tether.
//!
//! This module provides lazy Bluetooth scanning for proximity detection.
//! It supports:
//! - Checking if a configured device is nearby based on RSSI threshold
//! - Device discovery for onboarding (listing visible devices)
//! - Getting raw RSSI values for calibration
//!
//! # Feature Flags
//!
//! - `bluetooth`: Enables real Bluetooth hardware access via `bluer` (default)
//! - `mock-bluetooth`: Uses mock implementation for local development
//!
//! # Example
//!
//! ```rust,ignore
//! use tether_core::bluetooth::{BluetoothScanner, BluetoothConfig, ProximityResult};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), BluetoothError> {
//!     let scanner = BluetoothScanner::new().await?;
//!
//!     let config = BluetoothConfig {
//!         device_address: "AA:BB:CC:DD:EE:FF".to_string(),
//!         rssi_threshold: -70,
//!     };
//!
//!     let result = scanner.check_proximity(&config).await?;
//!     println!("Device nearby: {}", result.nearby);
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};
use utoipa::ToSchema;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during Bluetooth operations.
#[derive(Debug, Error)]
pub enum BluetoothError {
    /// No Bluetooth adapter was found on the system.
    /// This typically means BlueZ is not installed or no adapter is present.
    #[error("No Bluetooth adapter found. Ensure BlueZ is installed and a Bluetooth adapter is available.")]
    AdapterNotFound,

    /// The Bluetooth adapter exists but is powered off.
    /// Call `set_powered(true)` to enable it.
    #[error("Bluetooth adapter is powered off. Enable Bluetooth to continue.")]
    AdapterPoweredOff,

    /// The target device was not found during scanning.
    /// This can happen if the device is out of range, powered off, or not advertising.
    #[error("Device with address '{address}' was not found during scan")]
    DeviceNotFound {
        /// The MAC address that was not found.
        address: String,
    },

    /// The scan operation timed out before completing.
    /// Try increasing the scan duration or ensure the device is advertising.
    #[error("Bluetooth scan timed out after {duration_secs} seconds")]
    ScanTimeout {
        /// How long we waited before timing out.
        duration_secs: u64,
    },

    /// Invalid Bluetooth MAC address format.
    /// Expected format: "XX:XX:XX:XX:XX:XX" where X is a hex digit.
    #[error("Invalid Bluetooth address format: '{address}'. Expected format: XX:XX:XX:XX:XX:XX")]
    InvalidAddress {
        /// The invalid address that was provided.
        address: String,
    },

    /// Failed to initialize the Bluetooth session.
    /// This usually means BlueZ daemon (bluetoothd) is not running.
    #[error("Failed to initialize Bluetooth session: {message}")]
    SessionInitFailed {
        /// Detailed error message from the underlying library.
        message: String,
    },

    /// Failed to start device discovery.
    #[error("Failed to start device discovery: {message}")]
    DiscoveryFailed {
        /// Detailed error message.
        message: String,
    },

    /// A generic internal error occurred.
    #[error("Bluetooth internal error: {message}")]
    Internal {
        /// Detailed error message.
        message: String,
    },
}

/// Result type alias for Bluetooth operations.
pub type BluetoothResult<T> = Result<T, BluetoothError>;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Configuration for proximity checking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BluetoothConfig {
    /// The MAC address of the device to track.
    /// Format: "XX:XX:XX:XX:XX:XX" (colon-separated hex).
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub device_address: String,

    /// RSSI threshold for considering a device "nearby".
    /// Typical values:
    /// - `-30` to `-50`: Very close (within 1 meter)
    /// - `-50` to `-70`: Nearby (within 3-5 meters)
    /// - `-70` to `-90`: Far (within 10 meters)
    /// - Below `-90`: Very far or weak signal
    ///
    /// A device is considered "nearby" if its RSSI >= this threshold.
    #[schema(example = -60)]
    pub rssi_threshold: i16,
}

impl BluetoothConfig {
    /// Validates the configuration.
    pub fn validate(&self) -> BluetoothResult<()> {
        self.validate_address()?;
        Ok(())
    }

    /// Validates the Bluetooth address format.
    fn validate_address(&self) -> BluetoothResult<()> {
        let parts: Vec<&str> = self.device_address.split(':').collect();
        if parts.len() != 6 {
            return Err(BluetoothError::InvalidAddress {
                address: self.device_address.clone(),
            });
        }

        for part in parts {
            if part.len() != 2 {
                return Err(BluetoothError::InvalidAddress {
                    address: self.device_address.clone(),
                });
            }
            if !part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(BluetoothError::InvalidAddress {
                    address: self.device_address.clone(),
                });
            }
        }

        Ok(())
    }
}

/// A discovered Bluetooth device.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BluetoothDevice {
    /// The MAC address of the device.
    #[schema(example = "AA:BB:CC:DD:EE:FF")]
    pub address: String,

    /// The human-readable name of the device, if available.
    /// This may be `None` if the device hasn't advertised its name.
    #[schema(example = "iPhone")]
    pub name: Option<String>,

    /// The received signal strength indicator (RSSI) in dBm.
    /// Higher values (closer to 0) indicate stronger signals.
    /// This may be `None` if RSSI wasn't available during discovery.
    #[schema(example = -55)]
    pub rssi: Option<i16>,
}

/// Result of a proximity check.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProximityResult {
    /// Whether the device is considered nearby based on the RSSI threshold.
    pub nearby: bool,

    /// The actual RSSI value if the device was found.
    /// `None` if the device was not detected during the scan.
    pub rssi: Option<i16>,

    /// The device name if available.
    pub device_name: Option<String>,

    /// The MAC address that was checked.
    pub device_address: String,

    /// Timestamp of the check (Unix timestamp in seconds).
    pub timestamp: u64,
}

// ============================================================================
// REAL BLUETOOTH IMPLEMENTATION (feature = "bluetooth")
// ============================================================================

#[cfg(all(feature = "bluetooth", not(feature = "mock-bluetooth")))]
mod real_impl {
    use super::*;
    use bluer::{Adapter, AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session};
    use futures::StreamExt;
    use std::collections::HashMap;
    use std::str::FromStr;
    use tokio::sync::Mutex;
    use tokio::time::{timeout, Instant};

    /// Bluetooth scanner for proximity detection.
    ///
    /// This struct manages a connection to the BlueZ daemon and provides
    /// methods for scanning devices and checking proximity.
    ///
    /// # Thread Safety
    ///
    /// `BluetoothScanner` is thread-safe and can be shared across tasks
    /// using `Arc<BluetoothScanner>`.
    pub struct BluetoothScanner {
        /// The BlueZ session handle.
        _session: Session,
        /// The default Bluetooth adapter.
        adapter: Adapter,
        /// Mutex to prevent concurrent scans (BlueZ doesn't support this well).
        scan_lock: Mutex<()>,
    }

    impl BluetoothScanner {
        /// Default scan duration in seconds for proximity checks.
        const DEFAULT_SCAN_DURATION_SECS: u64 = 3;

        /// Maximum scan duration to prevent indefinite hangs.
        const MAX_SCAN_DURATION_SECS: u64 = 30;

        /// Creates a new Bluetooth scanner.
        ///
        /// This initializes a connection to the BlueZ daemon and obtains
        /// a handle to the default Bluetooth adapter.
        ///
        /// # Errors
        ///
        /// - `BluetoothError::SessionInitFailed`: BlueZ daemon not running
        /// - `BluetoothError::AdapterNotFound`: No Bluetooth adapter available
        /// - `BluetoothError::AdapterPoweredOff`: Adapter exists but is off
        #[instrument(name = "bluetooth_scanner_new")]
        pub async fn new() -> BluetoothResult<Self> {
            info!("Initializing Bluetooth scanner");

            // Create session to BlueZ daemon
            let session = Session::new().await.map_err(|e| {
                error!("Failed to create BlueZ session: {}", e);
                BluetoothError::SessionInitFailed {
                    message: e.to_string(),
                }
            })?;

            // Get default adapter
            let adapter = session.default_adapter().await.map_err(|e| {
                error!("Failed to get default adapter: {}", e);
                BluetoothError::AdapterNotFound
            })?;

            // Check if adapter is powered
            let is_powered = adapter.is_powered().await.map_err(|e| {
                error!("Failed to check adapter power state: {}", e);
                BluetoothError::Internal {
                    message: format!("Failed to check adapter power: {}", e),
                }
            })?;

            if !is_powered {
                warn!("Bluetooth adapter is powered off, attempting to power on");
                adapter.set_powered(true).await.map_err(|e| {
                    error!("Failed to power on adapter: {}", e);
                    BluetoothError::AdapterPoweredOff
                })?;
                info!("Bluetooth adapter powered on successfully");
            }

            let adapter_addr = adapter
                .address()
                .await
                .map_err(|e| BluetoothError::Internal {
                    message: format!("Failed to get adapter address: {}", e),
                })?;

            info!(
                adapter_address = %adapter_addr,
                "Bluetooth scanner initialized successfully"
            );

            Ok(Self {
                _session: session,
                adapter,
                scan_lock: Mutex::new(()),
            })
        }

        /// Checks if a configured device is nearby based on RSSI threshold.
        #[instrument(skip(self), fields(device_address = %config.device_address, threshold = config.rssi_threshold))]
        pub async fn check_proximity(
            &self,
            config: &BluetoothConfig,
        ) -> BluetoothResult<ProximityResult> {
            // Validate configuration
            config.validate()?;

            let target_address = Address::from_str(&config.device_address).map_err(|_| {
                BluetoothError::InvalidAddress {
                    address: config.device_address.clone(),
                }
            })?;

            info!("Checking proximity for device {}", config.device_address);

            // Acquire scan lock to prevent concurrent scans
            let _lock = self.scan_lock.lock().await;

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Try to get device info from a quick scan
            let scan_result = self
                .scan_for_device(
                    &target_address,
                    Duration::from_secs(Self::DEFAULT_SCAN_DURATION_SECS),
                )
                .await?;

            let (rssi, device_name) = match scan_result {
                Some((rssi, name)) => (Some(rssi), name),
                None => {
                    // Device not found during scan
                    debug!("Device {} not found during scan", config.device_address);
                    (None, None)
                }
            };

            // Determine if nearby based on RSSI threshold
            let nearby = rssi.map(|r| r >= config.rssi_threshold).unwrap_or(false);

            let result = ProximityResult {
                nearby,
                rssi,
                device_name,
                device_address: config.device_address.clone(),
                timestamp,
            };

            info!(
                nearby = result.nearby,
                rssi = ?result.rssi,
                device_name = ?result.device_name,
                "Proximity check complete"
            );

            Ok(result)
        }

        /// Discovers all visible Bluetooth devices.
        #[instrument(skip(self), fields(duration_secs))]
        pub async fn discover_devices(
            &self,
            duration_secs: u64,
        ) -> BluetoothResult<Vec<BluetoothDevice>> {
            let duration_secs = duration_secs.min(Self::MAX_SCAN_DURATION_SECS);
            let duration = Duration::from_secs(duration_secs);

            info!("Starting device discovery for {} seconds", duration_secs);

            // Acquire scan lock
            let _lock = self.scan_lock.lock().await;

            // Set up discovery filter for all devices (BR/EDR and LE)
            let filter = DiscoveryFilter {
                transport: DiscoveryTransport::Auto,
                duplicate_data: false,
                ..Default::default()
            };

            self.adapter
                .set_discovery_filter(filter)
                .await
                .map_err(|e| {
                    error!("Failed to set discovery filter: {}", e);
                    BluetoothError::DiscoveryFailed {
                        message: e.to_string(),
                    }
                })?;

            // Start discovery
            let events = self.adapter.discover_devices().await.map_err(|e| {
                error!("Failed to start device discovery: {}", e);
                BluetoothError::DiscoveryFailed {
                    message: e.to_string(),
                }
            })?;

            // Collect devices during the scan period
            let mut devices: HashMap<Address, BluetoothDevice> = HashMap::new();
            let start = Instant::now();

            tokio::pin!(events);

            loop {
                let remaining = duration.saturating_sub(start.elapsed());
                if remaining.is_zero() {
                    break;
                }

                match timeout(remaining, events.next()).await {
                    Ok(Some(event)) => {
                        if let AdapterEvent::DeviceAdded(addr) = event {
                            if let Ok(device) = self.adapter.device(addr) {
                                // Query device properties
                                let name = device.name().await.ok().flatten();
                                let rssi = device.rssi().await.ok().flatten();

                                let bt_device = BluetoothDevice {
                                    address: addr.to_string(),
                                    name,
                                    rssi,
                                };

                                debug!(
                                    address = %addr,
                                    name = ?bt_device.name,
                                    rssi = ?bt_device.rssi,
                                    "Discovered device"
                                );

                                devices.insert(addr, bt_device);
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended
                        break;
                    }
                    Err(_) => {
                        // Timeout - scan duration complete
                        break;
                    }
                }
            }

            let result: Vec<BluetoothDevice> = devices.into_values().collect();

            info!(device_count = result.len(), "Device discovery complete");

            Ok(result)
        }

        /// Gets the current RSSI value for a specific device.
        #[instrument(skip(self), fields(address = %address))]
        pub async fn get_device_rssi(&self, address: &str) -> BluetoothResult<Option<i16>> {
            // Validate address format
            let target_address =
                Address::from_str(address).map_err(|_| BluetoothError::InvalidAddress {
                    address: address.to_string(),
                })?;

            debug!("Getting RSSI for device {}", address);

            // Acquire scan lock
            let _lock = self.scan_lock.lock().await;

            // Quick scan to find the device
            let result = self
                .scan_for_device(
                    &target_address,
                    Duration::from_secs(Self::DEFAULT_SCAN_DURATION_SECS),
                )
                .await?;

            let rssi = result.map(|(rssi, _)| rssi);

            debug!(rssi = ?rssi, "RSSI query complete");

            Ok(rssi)
        }

        /// Internal helper to scan for a specific device.
        async fn scan_for_device(
            &self,
            target_address: &Address,
            duration: Duration,
        ) -> BluetoothResult<Option<(i16, Option<String>)>> {
            // Set up discovery filter
            let filter = DiscoveryFilter {
                transport: DiscoveryTransport::Auto,
                duplicate_data: true,
                ..Default::default()
            };

            self.adapter
                .set_discovery_filter(filter)
                .await
                .map_err(|e| BluetoothError::DiscoveryFailed {
                    message: e.to_string(),
                })?;

            // Start discovery
            let events = self.adapter.discover_devices().await.map_err(|e| {
                BluetoothError::DiscoveryFailed {
                    message: e.to_string(),
                }
            })?;

            let start = Instant::now();
            let mut found_device: Option<(i16, Option<String>)> = None;

            tokio::pin!(events);

            loop {
                let remaining = duration.saturating_sub(start.elapsed());
                if remaining.is_zero() {
                    break;
                }

                match timeout(remaining, events.next()).await {
                    Ok(Some(event)) => {
                        if let AdapterEvent::DeviceAdded(addr) = event {
                            if addr == *target_address {
                                if let Ok(device) = self.adapter.device(addr) {
                                    if let Ok(Some(rssi)) = device.rssi().await {
                                        let name = device.name().await.ok().flatten();
                                        debug!(rssi = rssi, name = ?name, "Found target device");
                                        found_device = Some((rssi, name));
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }

            // Also check if we already know about this device
            if found_device.is_none() {
                if let Ok(device) = self.adapter.device(*target_address) {
                    if let Ok(Some(rssi)) = device.rssi().await {
                        let name = device.name().await.ok().flatten();
                        found_device = Some((rssi, name));
                    }
                }
            }

            Ok(found_device)
        }

        /// Checks if the Bluetooth adapter is powered on.
        pub async fn is_adapter_powered(&self) -> BluetoothResult<bool> {
            self.adapter
                .is_powered()
                .await
                .map_err(|e| BluetoothError::Internal {
                    message: format!("Failed to check adapter power: {}", e),
                })
        }

        /// Gets the adapter's Bluetooth address.
        pub async fn adapter_address(&self) -> BluetoothResult<String> {
            let addr = self
                .adapter
                .address()
                .await
                .map_err(|e| BluetoothError::Internal {
                    message: format!("Failed to get adapter address: {}", e),
                })?;
            Ok(addr.to_string())
        }
    }
}

// ============================================================================
// MOCK IMPLEMENTATION (feature = "mock-bluetooth" or no bluetooth feature)
// ============================================================================

#[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
mod mock_impl {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock device configuration for testing.
    #[derive(Debug, Clone)]
    pub struct MockDevice {
        /// The MAC address.
        pub address: String,
        /// The device name.
        pub name: Option<String>,
        /// The RSSI value.
        pub rssi: Option<i16>,
        /// Whether the device is visible.
        pub is_visible: bool,
    }

    /// Mock Bluetooth scanner for local development and testing.
    pub struct BluetoothScanner {
        /// Mock devices keyed by address.
        mock_devices: Arc<RwLock<HashMap<String, MockDevice>>>,
        /// Simulated scan delay.
        scan_delay_ms: u64,
        /// Whether the adapter is "powered on".
        is_powered: Arc<RwLock<bool>>,
    }

    impl BluetoothScanner {
        /// Creates a new mock Bluetooth scanner.
        #[instrument(name = "mock_bluetooth_scanner_new")]
        pub async fn new() -> BluetoothResult<Self> {
            info!("Initializing MOCK Bluetooth scanner (no hardware access)");

            let mut mock_devices = HashMap::new();

            // Add some default mock devices for testing
            mock_devices.insert(
                "AA:BB:CC:DD:EE:FF".to_string(),
                MockDevice {
                    address: "AA:BB:CC:DD:EE:FF".to_string(),
                    name: Some("Test iPhone".to_string()),
                    rssi: Some(-55),
                    is_visible: true,
                },
            );

            mock_devices.insert(
                "11:22:33:44:55:66".to_string(),
                MockDevice {
                    address: "11:22:33:44:55:66".to_string(),
                    name: Some("Test Android".to_string()),
                    rssi: Some(-72),
                    is_visible: true,
                },
            );

            mock_devices.insert(
                "DE:AD:BE:EF:CA:FE".to_string(),
                MockDevice {
                    address: "DE:AD:BE:EF:CA:FE".to_string(),
                    name: None,
                    rssi: Some(-85),
                    is_visible: true,
                },
            );

            Ok(Self {
                mock_devices: Arc::new(RwLock::new(mock_devices)),
                scan_delay_ms: 100,
                is_powered: Arc::new(RwLock::new(true)),
            })
        }

        /// Adds a mock device for testing.
        pub async fn add_mock_device(&self, device: MockDevice) {
            let mut devices = self.mock_devices.write().await;
            info!(
                address = %device.address,
                name = ?device.name,
                rssi = ?device.rssi,
                "Adding mock device"
            );
            devices.insert(device.address.clone(), device);
        }

        /// Updates a mock device's RSSI.
        pub async fn set_mock_device_rssi(&self, address: &str, rssi: Option<i16>) {
            let mut devices = self.mock_devices.write().await;
            if let Some(device) = devices.get_mut(address) {
                device.rssi = rssi;
            }
        }

        /// Sets whether a mock device is visible.
        pub async fn set_mock_device_visible(&self, address: &str, is_visible: bool) {
            let mut devices = self.mock_devices.write().await;
            if let Some(device) = devices.get_mut(address) {
                device.is_visible = is_visible;
            }
        }

        /// Sets the mock adapter power state.
        pub async fn set_adapter_powered(&self, powered: bool) {
            *self.is_powered.write().await = powered;
        }

        /// Checks proximity for a configured device (mock implementation).
        #[instrument(skip(self), fields(device_address = %config.device_address, threshold = config.rssi_threshold))]
        pub async fn check_proximity(
            &self,
            config: &BluetoothConfig,
        ) -> BluetoothResult<ProximityResult> {
            config.validate()?;

            // Check if adapter is "powered"
            if !*self.is_powered.read().await {
                return Err(BluetoothError::AdapterPoweredOff);
            }

            info!(
                "[MOCK] Checking proximity for device {}",
                config.device_address
            );

            // Simulate scan delay
            tokio::time::sleep(Duration::from_millis(self.scan_delay_ms)).await;

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let devices = self.mock_devices.read().await;

            // Normalize address to uppercase for comparison
            let normalized_address = config.device_address.to_uppercase();

            let (rssi, device_name) = if let Some(device) = devices.get(&normalized_address) {
                if device.is_visible {
                    (device.rssi, device.name.clone())
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let nearby = rssi.map(|r| r >= config.rssi_threshold).unwrap_or(false);

            let result = ProximityResult {
                nearby,
                rssi,
                device_name,
                device_address: config.device_address.clone(),
                timestamp,
            };

            info!(
                nearby = result.nearby,
                rssi = ?result.rssi,
                "[MOCK] Proximity check complete"
            );

            Ok(result)
        }

        /// Discovers all visible mock devices.
        #[instrument(skip(self), fields(duration_secs))]
        pub async fn discover_devices(
            &self,
            duration_secs: u64,
        ) -> BluetoothResult<Vec<BluetoothDevice>> {
            // Check if adapter is "powered"
            if !*self.is_powered.read().await {
                return Err(BluetoothError::AdapterPoweredOff);
            }

            info!("[MOCK] Discovering devices for {} seconds", duration_secs);

            // Simulate some scan time
            let delay = std::cmp::min(duration_secs * 50, self.scan_delay_ms);
            tokio::time::sleep(Duration::from_millis(delay)).await;

            let devices = self.mock_devices.read().await;
            let result: Vec<BluetoothDevice> = devices
                .values()
                .filter(|d| d.is_visible)
                .map(|d| BluetoothDevice {
                    address: d.address.clone(),
                    name: d.name.clone(),
                    rssi: d.rssi,
                })
                .collect();

            info!(device_count = result.len(), "[MOCK] Discovery complete");

            Ok(result)
        }

        /// Gets the RSSI for a specific mock device.
        #[instrument(skip(self), fields(address = %address))]
        pub async fn get_device_rssi(&self, address: &str) -> BluetoothResult<Option<i16>> {
            // Validate address format
            let config = BluetoothConfig {
                device_address: address.to_string(),
                rssi_threshold: -100,
            };
            config.validate()?;

            // Check if adapter is "powered"
            if !*self.is_powered.read().await {
                return Err(BluetoothError::AdapterPoweredOff);
            }

            // Simulate scan delay
            tokio::time::sleep(Duration::from_millis(self.scan_delay_ms)).await;

            let devices = self.mock_devices.read().await;
            let normalized = address.to_uppercase();

            let rssi = devices
                .get(&normalized)
                .filter(|d| d.is_visible)
                .and_then(|d| d.rssi);

            debug!(rssi = ?rssi, "[MOCK] RSSI query complete");

            Ok(rssi)
        }

        /// Checks if the mock adapter is "powered on".
        pub async fn is_adapter_powered(&self) -> BluetoothResult<bool> {
            Ok(*self.is_powered.read().await)
        }

        /// Returns a mock adapter address.
        pub async fn adapter_address(&self) -> BluetoothResult<String> {
            Ok("00:00:00:00:00:00".to_string())
        }
    }
}

// ============================================================================
// RE-EXPORTS
// ============================================================================

#[cfg(all(feature = "bluetooth", not(feature = "mock-bluetooth")))]
pub use real_impl::BluetoothScanner;

#[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
pub use mock_impl::{BluetoothScanner, MockDevice};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bluetooth_config_validation_valid() {
        let config = BluetoothConfig {
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi_threshold: -70,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bluetooth_config_validation_lowercase() {
        let config = BluetoothConfig {
            device_address: "aa:bb:cc:dd:ee:ff".to_string(),
            rssi_threshold: -70,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bluetooth_config_validation_invalid_too_short() {
        let config = BluetoothConfig {
            device_address: "AA:BB:CC".to_string(),
            rssi_threshold: -70,
        };
        assert!(matches!(
            config.validate(),
            Err(BluetoothError::InvalidAddress { .. })
        ));
    }

    #[test]
    fn test_bluetooth_config_validation_invalid_format() {
        let config = BluetoothConfig {
            device_address: "AABBCCDDEEFF".to_string(),
            rssi_threshold: -70,
        };
        assert!(matches!(
            config.validate(),
            Err(BluetoothError::InvalidAddress { .. })
        ));
    }

    #[test]
    fn test_bluetooth_config_validation_invalid_chars() {
        let config = BluetoothConfig {
            device_address: "GG:HH:II:JJ:KK:LL".to_string(),
            rssi_threshold: -70,
        };
        assert!(matches!(
            config.validate(),
            Err(BluetoothError::InvalidAddress { .. })
        ));
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_proximity_nearby() {
        let scanner = BluetoothScanner::new().await.unwrap();

        let config = BluetoothConfig {
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi_threshold: -60,
        };

        let result = scanner.check_proximity(&config).await.unwrap();
        assert!(result.nearby);
        assert_eq!(result.rssi, Some(-55));
        assert_eq!(result.device_name, Some("Test iPhone".to_string()));
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_proximity_too_far() {
        let scanner = BluetoothScanner::new().await.unwrap();

        let config = BluetoothConfig {
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi_threshold: -50,
        };

        let result = scanner.check_proximity(&config).await.unwrap();
        assert!(!result.nearby);
        assert_eq!(result.rssi, Some(-55));
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_device_not_found() {
        let scanner = BluetoothScanner::new().await.unwrap();

        let config = BluetoothConfig {
            device_address: "99:99:99:99:99:99".to_string(),
            rssi_threshold: -70,
        };

        let result = scanner.check_proximity(&config).await.unwrap();
        assert!(!result.nearby);
        assert!(result.rssi.is_none());
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_discover_devices() {
        let scanner = BluetoothScanner::new().await.unwrap();

        let devices = scanner.discover_devices(1).await.unwrap();
        assert_eq!(devices.len(), 3);
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_device_visibility() {
        let scanner = BluetoothScanner::new().await.unwrap();

        scanner
            .set_mock_device_visible("AA:BB:CC:DD:EE:FF", false)
            .await;

        let config = BluetoothConfig {
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi_threshold: -70,
        };

        let result = scanner.check_proximity(&config).await.unwrap();
        assert!(!result.nearby);
        assert!(result.rssi.is_none());

        let devices = scanner.discover_devices(1).await.unwrap();
        assert_eq!(devices.len(), 2);
    }

    #[tokio::test]
    #[cfg(any(feature = "mock-bluetooth", not(feature = "bluetooth")))]
    async fn test_mock_scanner_powered_off() {
        let scanner = BluetoothScanner::new().await.unwrap();
        scanner.set_adapter_powered(false).await;

        let config = BluetoothConfig {
            device_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi_threshold: -70,
        };

        let result = scanner.check_proximity(&config).await;
        assert!(matches!(result, Err(BluetoothError::AdapterPoweredOff)));
    }
}
