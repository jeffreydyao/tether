//! Application state shared across handlers.

use std::path::PathBuf;
use std::sync::Arc;

use tether_core::{BluetoothScanner, Config, PassManager};
use tokio::sync::RwLock;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: RwLock<Config>,
    pass_manager: RwLock<PassManager>,
    bluetooth: RwLock<Option<BluetoothScanner>>,
}

impl AppState {
    /// Create new application state.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration TOML file.
    /// * `passes_path` - Path to the passes JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be loaded or passes file cannot be created.
    pub fn new(config_path: &str, passes_path: PathBuf) -> anyhow::Result<Self> {
        let config = Config::load_or_default(config_path)?;
        let pass_manager =
            PassManager::load_or_create(&passes_path, config.passes.per_month as u32)?;

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config: RwLock::new(config),
                pass_manager: RwLock::new(pass_manager),
                bluetooth: RwLock::new(None),
            }),
        })
    }

    /// Get read access to config.
    pub async fn config(&self) -> tokio::sync::RwLockReadGuard<'_, Config> {
        self.inner.config.read().await
    }

    /// Get write access to config.
    pub async fn config_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, Config> {
        self.inner.config.write().await
    }

    /// Get read access to pass manager.
    pub async fn pass_manager(&self) -> tokio::sync::RwLockReadGuard<'_, PassManager> {
        self.inner.pass_manager.read().await
    }

    /// Get write access to pass manager.
    pub async fn pass_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, PassManager> {
        self.inner.pass_manager.write().await
    }
}
