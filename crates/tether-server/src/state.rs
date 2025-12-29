//! Application state shared across handlers.

use std::sync::Arc;

use tether_core::{BluetoothScanner, PassManager, Storage, TetherConfig};
use tokio::sync::RwLock;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub config: RwLock<TetherConfig>,
    pub pass_manager: RwLock<PassManager>,
    pub bluetooth: RwLock<Option<BluetoothScanner>>,
}

impl AppState {
    /// Create new application state.
    pub async fn new() -> anyhow::Result<Self> {
        let config = TetherConfig::load()?;
        let storage = Storage::default()?;
        let pass_manager = PassManager::new(storage, config.clone());

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config: RwLock::new(config),
                pass_manager: RwLock::new(pass_manager),
                bluetooth: RwLock::new(None),
            }),
        })
    }

    /// Get read access to config.
    pub async fn config(&self) -> tokio::sync::RwLockReadGuard<'_, TetherConfig> {
        self.inner.config.read().await
    }

    /// Get write access to config.
    pub async fn config_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, TetherConfig> {
        self.inner.config.write().await
    }

    /// Get write access to pass manager.
    pub async fn pass_manager(&self) -> tokio::sync::RwLockWriteGuard<'_, PassManager> {
        self.inner.pass_manager.write().await
    }
}
