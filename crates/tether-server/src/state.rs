//! Application state management for the tether server.
//!
//! This module provides the [`AppState`] struct which holds all shared state
//! including configuration, pass management, and Bluetooth connectivity.
//! State is wrapped in [`SharedState`] (Arc<RwLock<AppState>>) for safe
//! concurrent access across async request handlers.

use std::path::PathBuf;
use std::sync::Arc;

use tether_core::{BluetoothScanner, Config, PassManager};
use tokio::sync::RwLock;

/// Type alias for thread-safe shared application state.
///
/// Uses `Arc` for reference counting across async tasks and `RwLock` for
/// interior mutability with read-write semantics. Prefer `read()` when
/// only reading state to allow concurrent readers.
pub type SharedState = Arc<RwLock<AppState>>;

/// Core application state shared across all HTTP handlers.
///
/// # Fields
///
/// - `config`: Server and application configuration loaded from disk
/// - `pass_manager`: Manages monthly passes, history, and persistence
/// - `bluetooth`: Handles Bluetooth device proximity detection
/// - `config_path`: Path to the config file for saving changes
/// - `passes_path`: Path to the passes JSON file
///
/// # Thread Safety
///
/// All fields are designed to be accessed through the `SharedState` wrapper.
/// The `PassManager` handles its own persistence, so writes to state should
/// be followed by explicit persistence calls where needed.
pub struct AppState {
    /// Application configuration loaded from TOML file.
    pub config: Config,

    /// Manages pass allocation, usage, and history.
    pub pass_manager: PassManager,

    /// Bluetooth scanner for proximity detection.
    pub bluetooth: Option<BluetoothScanner>,

    /// Path to the configuration file.
    pub config_path: PathBuf,

    /// Path to the passes data file.
    pub passes_path: PathBuf,
}

impl AppState {
    /// Creates a new `AppState` instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Loaded configuration from disk
    /// * `pass_manager` - Initialized pass manager with loaded history
    /// * `bluetooth` - Optional Bluetooth scanner (None if not available)
    /// * `config_path` - Path to the config file
    /// * `passes_path` - Path to the passes JSON file
    pub fn new(
        config: Config,
        pass_manager: PassManager,
        bluetooth: Option<BluetoothScanner>,
        config_path: PathBuf,
        passes_path: PathBuf,
    ) -> Self {
        Self {
            config,
            pass_manager,
            bluetooth,
            config_path,
            passes_path,
        }
    }

    /// Wraps the `AppState` in an `Arc<RwLock<_>>` for shared access.
    ///
    /// This is the preferred way to create state for use with Axum handlers.
    #[must_use]
    pub fn into_shared(self) -> SharedState {
        Arc::new(RwLock::new(self))
    }

    /// Saves the current configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be written.
    pub fn save_config(&self) -> tether_core::ConfigResult<()> {
        self.config.save(&self.config_path)
    }
}

/// Extension trait for SharedState to provide ergonomic access patterns.
pub trait SharedStateExt {
    /// Acquires a read lock and returns a clone of the config.
    fn get_config(&self) -> impl std::future::Future<Output = Config> + Send;

    /// Checks if the application has completed initial setup.
    fn is_configured(&self) -> impl std::future::Future<Output = bool> + Send;
}

impl SharedStateExt for SharedState {
    async fn get_config(&self) -> Config {
        self.read().await.config.clone()
    }

    async fn is_configured(&self) -> bool {
        let state = self.read().await;
        state.config.system.onboarding_complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tether_core::{Config, PassManager};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_shared_state_creation() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let passes_path = dir.path().join("passes.json");

        let config = Config::default();
        let pass_manager = PassManager::load_or_create(&passes_path, 3).unwrap();

        let state = AppState::new(
            config.clone(),
            pass_manager,
            None,
            config_path,
            passes_path,
        );
        let shared = state.into_shared();

        // Test that we can read from shared state
        let config = shared.get_config().await;
        assert!(!config.system.onboarding_complete);
    }

    #[tokio::test]
    async fn test_is_configured() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let passes_path = dir.path().join("passes.json");

        let config = Config::default();
        let pass_manager = PassManager::load_or_create(&passes_path, 3).unwrap();

        let state = AppState::new(config, pass_manager, None, config_path, passes_path);
        let shared = state.into_shared();

        assert!(!shared.is_configured().await);

        // Mark as configured
        {
            let mut state = shared.write().await;
            state.config.system.onboarding_complete = true;
        }

        assert!(shared.is_configured().await);
    }
}
