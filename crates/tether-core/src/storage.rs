//! Storage utilities.
//!
//! This module provides helper functions for determining storage paths.
//! The actual persistence is handled by [`PassManager`](crate::passes::PassManager).

use std::path::PathBuf;

/// Returns the default data directory for tether.
///
/// On Raspberry Pi (Linux): `/var/lib/tether/`
/// For development (non-Linux): `~/.local/share/tether/`
#[must_use]
pub fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/lib/tether")
    }
    #[cfg(not(target_os = "linux"))]
    {
        directories::ProjectDirs::from("", "", "tether")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./data"))
    }
}

/// Returns the default path for the passes JSON file.
#[must_use]
pub fn default_passes_path() -> PathBuf {
    default_data_dir().join("passes.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_data_dir_exists() {
        let dir = default_data_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_default_passes_path() {
        let path = default_passes_path();
        assert!(path.ends_with("passes.json"));
    }
}
