//! Persistent storage for pass data.
//!
//! Uses JSON files organized by year/month for efficient lookup.

use std::path::PathBuf;

use crate::error::Result;
use crate::passes::MonthlyPassState;

/// Storage backend for tether data.
#[derive(Debug, Clone)]
pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    /// Create a new storage instance.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Directory to store data files
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Get the default storage location.
    ///
    /// On Raspberry Pi: `/var/lib/tether/`
    /// For development: `~/.local/share/tether/`
    pub fn default() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self::new(PathBuf::from("/var/lib/tether")))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let dirs = directories::ProjectDirs::from("", "", "tether").ok_or_else(|| {
                crate::error::Error::Storage("Cannot determine data directory".into())
            })?;
            Ok(Self::new(dirs.data_dir().to_path_buf()))
        }
    }

    /// Load pass state for a specific month.
    pub fn load_month_state(&self, year: i32, month: u32) -> Result<Option<MonthlyPassState>> {
        let path = self.month_path(year, month);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let state: MonthlyPassState = serde_json::from_str(&content)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Save pass state for a specific month.
    pub fn save_month_state(&self, state: &MonthlyPassState) -> Result<()> {
        let path = self.month_path(state.year, state.month);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn month_path(&self, year: i32, month: u32) -> PathBuf {
        self.data_dir
            .join("passes")
            .join(format!("{year}"))
            .join(format!("{month:02}.json"))
    }
}
