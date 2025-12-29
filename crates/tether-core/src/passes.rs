//! Pass management for monthly accountability passes.
//!
//! This module provides functionality for tracking and managing monthly passes
//! that users can "use" when they need to keep their phone with them at night.
//! Passes automatically reset at the beginning of each month.
//!
//! # Thread Safety
//!
//! `PassManager` is designed to be wrapped in an `std::sync::RwLock` or
//! `tokio::sync::RwLock` for concurrent access. All mutating operations
//! (`use_pass`, `maybe_reset_month`) modify internal state and require
//! exclusive access.
//!
//! # Persistence
//!
//! Data is stored in JSON format at the configured path. The file is
//! atomically updated on each mutation to prevent corruption.
//!
//! # Example
//!
//! ```no_run
//! use tether_core::passes::PassManager;
//! use std::path::PathBuf;
//!
//! let path = PathBuf::from("/var/lib/tether/passes.json");
//! let mut manager = PassManager::load_or_create(&path, 3)?;
//!
//! // Check remaining passes
//! println!("Remaining: {}", manager.remaining());
//!
//! // Use a pass
//! manager.use_pass("Medical appointment tonight".to_string())?;
//!
//! // View history
//! let history = manager.history(&tether_core::passes::current_month_string());
//! for entry in history {
//!     println!("{}: {}", entry.used_at_utc, entry.reason);
//! }
//! # Ok::<(), tether_core::passes::PassError>(())
//! ```

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use utoipa::ToSchema;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during pass operations.
#[derive(Debug, Error)]
pub enum PassError {
    /// No passes remaining for the current month.
    #[error("no passes remaining for {month} (0 of {max} available)")]
    NoPassesRemaining {
        /// The current month in YYYY-MM format.
        month: String,
        /// The maximum passes allowed per month.
        max: u32,
    },

    /// Failed to read the passes data file.
    #[error("failed to read passes file at {}: {source}", path.display())]
    ReadError {
        /// The path that failed to read.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Failed to write the passes data file.
    #[error("failed to write passes file at {}: {source}", path.display())]
    WriteError {
        /// The path that failed to write.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Failed to parse the passes data file as JSON.
    #[error("failed to parse passes file at {}: {source}", path.display())]
    ParseError {
        /// The path that failed to parse.
        path: PathBuf,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// Failed to serialize passes data to JSON.
    #[error("failed to serialize passes data: {0}")]
    SerializeError(#[from] serde_json::Error),

    /// Failed to create parent directory for passes file.
    #[error("failed to create parent directory for {}: {source}", path.display())]
    CreateDirError {
        /// The path whose parent directory failed to create.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// The reason provided for using a pass was empty.
    #[error("reason cannot be empty when using a pass")]
    EmptyReason,

    /// The reason provided exceeds the maximum allowed length.
    #[error("reason exceeds maximum length of {max} characters (got {actual})")]
    ReasonTooLong {
        /// Maximum allowed length.
        max: usize,
        /// Actual length provided.
        actual: usize,
    },
}

/// Result type alias for pass operations.
pub type PassResult<T> = Result<T, PassError>;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Maximum allowed length for a pass reason.
pub const MAX_REASON_LENGTH: usize = 500;

/// Represents a single pass usage entry.
///
/// Each entry records when a pass was used and the reason provided by the user.
/// The timestamp is always in UTC to avoid timezone ambiguity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PassEntry {
    /// The UTC timestamp when the pass was used, in RFC 3339 format.
    ///
    /// Example: "2025-01-15T03:45:00Z"
    pub used_at_utc: DateTime<Utc>,

    /// The reason provided by the user for using this pass.
    ///
    /// This is a free-form string with a maximum length of 500 characters.
    #[schema(example = "Medical appointment in the morning")]
    pub reason: String,
}

impl PassEntry {
    /// Creates a new pass entry with the current UTC timestamp.
    fn new(reason: String) -> Self {
        Self {
            used_at_utc: Utc::now(),
            reason,
        }
    }

    /// Creates a pass entry with a specific timestamp (for testing).
    #[cfg(test)]
    fn with_timestamp(used_at_utc: DateTime<Utc>, reason: String) -> Self {
        Self {
            used_at_utc,
            reason,
        }
    }
}

/// The persisted data structure for pass management.
///
/// This struct is serialized to and from JSON for persistence.
/// It contains all information needed to track passes across months.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassData {
    /// The current month in "YYYY-MM" format (e.g., "2025-01").
    pub current_month: String,

    /// The number of passes remaining for the current month.
    pub remaining: u32,

    /// The number of passes granted per month.
    pub per_month: u32,

    /// Pending per-month value to apply at the next month reset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_per_month: Option<u32>,

    /// Historical pass usage organized by month.
    #[serde(default)]
    pub history: HashMap<String, Vec<PassEntry>>,
}

impl PassData {
    /// Creates new pass data for a fresh start.
    fn new(per_month: u32) -> Self {
        Self {
            current_month: current_month_string(),
            remaining: per_month,
            per_month,
            pending_per_month: None,
            history: HashMap::new(),
        }
    }
}

impl Default for PassData {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// PASS MANAGER
// ============================================================================

/// Manages monthly accountability passes with JSON persistence.
///
/// `PassManager` provides the core business logic for:
/// - Loading and saving pass data to a JSON file
/// - Automatically resetting passes at the start of each month
/// - Tracking pass usage with reasons
/// - Querying pass history by month
///
/// # Thread Safety
///
/// `PassManager` is **not** internally thread-safe. It is designed to be
/// wrapped in an `RwLock` for concurrent access.
#[derive(Debug)]
pub struct PassManager {
    /// The path to the JSON file for persistence.
    path: PathBuf,

    /// The current pass data.
    data: PassData,
}

impl PassManager {
    /// Loads pass data from disk or creates a new file if none exists.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file for persistence.
    /// * `per_month` - The number of passes to grant per month. This is used
    ///   when creating a new file. For existing files, this parameter is
    ///   ignored (the stored `per_month` value takes precedence).
    ///
    /// # Errors
    ///
    /// - `PassError::ReadError` - Failed to read the file (other than not found)
    /// - `PassError::ParseError` - File exists but contains invalid JSON
    /// - `PassError::CreateDirError` - Failed to create parent directories
    /// - `PassError::WriteError` - Failed to write initial data
    pub fn load_or_create(path: &Path, per_month: u32) -> PassResult<Self> {
        let path = path.to_path_buf();

        let data = if path.exists() {
            // Load existing data
            let contents = fs::read_to_string(&path).map_err(|source| PassError::ReadError {
                path: path.clone(),
                source,
            })?;

            serde_json::from_str::<PassData>(&contents).map_err(|source| PassError::ParseError {
                path: path.clone(),
                source,
            })?
        } else {
            // Create new data
            let data = PassData::new(per_month);

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|source| PassError::CreateDirError {
                        path: path.clone(),
                        source,
                    })?;
                }
            }

            data
        };

        let mut manager = Self { path, data };

        // Check for month change and reset if needed
        manager.maybe_reset_month(None)?;

        // Ensure data is persisted (especially for new files)
        manager.save()?;

        Ok(manager)
    }

    /// Checks if the month has changed and resets passes if necessary.
    ///
    /// # Arguments
    ///
    /// * `new_pending_per_month` - Optional new pending value to set.
    ///
    /// # Returns
    ///
    /// `true` if the month was reset, `false` otherwise.
    pub fn maybe_reset_month(&mut self, new_pending_per_month: Option<u32>) -> PassResult<bool> {
        let current = current_month_string();
        let mut changed = false;

        // Set new pending value if provided
        if let Some(pending) = new_pending_per_month {
            // Only set pending if it differs from current per_month
            if pending != self.data.per_month {
                self.data.pending_per_month = Some(pending);
                changed = true;
            } else {
                // If setting to current value, clear any pending
                if self.data.pending_per_month.is_some() {
                    self.data.pending_per_month = None;
                    changed = true;
                }
            }
        }

        // Check if month has changed
        if current != self.data.current_month {
            // Apply pending per_month if set
            if let Some(pending) = self.data.pending_per_month.take() {
                self.data.per_month = pending;
            }

            // Reset remaining passes
            self.data.remaining = self.data.per_month;

            // Update current month
            self.data.current_month = current;

            changed = true;
        }

        if changed {
            self.save()?;
        }

        Ok(changed)
    }

    /// Returns the number of passes remaining for the current month.
    #[inline]
    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.data.remaining
    }

    /// Returns the number of passes granted per month.
    #[inline]
    #[must_use]
    pub fn per_month(&self) -> u32 {
        self.data.per_month
    }

    /// Returns the pending per-month value, if any.
    #[inline]
    #[must_use]
    pub fn pending_per_month(&self) -> Option<u32> {
        self.data.pending_per_month
    }

    /// Returns the current month string in "YYYY-MM" format.
    #[inline]
    #[must_use]
    pub fn current_month(&self) -> &str {
        &self.data.current_month
    }

    /// Uses one pass and records the reason.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for using this pass. Must be non-empty and
    ///   at most 500 characters. Leading/trailing whitespace is trimmed.
    ///
    /// # Returns
    ///
    /// The `PassEntry` that was recorded, containing the timestamp and reason.
    ///
    /// # Errors
    ///
    /// - `PassError::NoPassesRemaining` - No passes left for this month
    /// - `PassError::EmptyReason` - The reason was empty or whitespace-only
    /// - `PassError::ReasonTooLong` - The reason exceeds 500 characters
    /// - `PassError::WriteError` - Failed to persist changes
    pub fn use_pass(&mut self, reason: String) -> PassResult<PassEntry> {
        // Check for month reset first
        self.maybe_reset_month(None)?;

        // Validate reason
        let reason = reason.trim().to_string();
        if reason.is_empty() {
            return Err(PassError::EmptyReason);
        }
        if reason.len() > MAX_REASON_LENGTH {
            return Err(PassError::ReasonTooLong {
                max: MAX_REASON_LENGTH,
                actual: reason.len(),
            });
        }

        // Check if passes are available
        if self.data.remaining == 0 {
            return Err(PassError::NoPassesRemaining {
                month: self.data.current_month.clone(),
                max: self.data.per_month,
            });
        }

        // Use a pass
        self.data.remaining -= 1;

        // Record in history
        let entry = PassEntry::new(reason);
        self.data
            .history
            .entry(self.data.current_month.clone())
            .or_default()
            .push(entry.clone());

        // Persist
        self.save()?;

        Ok(entry)
    }

    /// Returns the pass usage history for a specific month.
    ///
    /// # Arguments
    ///
    /// * `month` - The month to query in "YYYY-MM" format (e.g., "2025-01").
    ///
    /// # Returns
    ///
    /// A vector of `PassEntry` for the specified month, in chronological order.
    #[must_use]
    pub fn history(&self, month: &str) -> Vec<PassEntry> {
        self.data.history.get(month).cloned().unwrap_or_default()
    }

    /// Returns all pass usage history across all months.
    #[must_use]
    pub fn all_history(&self) -> HashMap<String, Vec<PassEntry>> {
        self.data.history.clone()
    }

    /// Updates the per-month pass configuration.
    ///
    /// If called mid-month (passes already used), the new value is stored as
    /// `pending_per_month` and takes effect at the start of the next month.
    ///
    /// # Arguments
    ///
    /// * `new_per_month` - The new number of passes per month.
    ///
    /// # Returns
    ///
    /// `true` if the change was applied immediately, `false` if it was
    /// deferred to next month.
    pub fn set_per_month(&mut self, new_per_month: u32) -> PassResult<bool> {
        // First check for month reset
        self.maybe_reset_month(None)?;

        // If same as current, no change needed
        if new_per_month == self.data.per_month {
            // Clear any pending if exists
            if self.data.pending_per_month.is_some() {
                self.data.pending_per_month = None;
                self.save()?;
            }
            return Ok(true);
        }

        // Check if we can apply immediately:
        // - No passes have been used yet this month
        let can_apply_immediately = self.data.remaining == self.data.per_month;

        if can_apply_immediately {
            self.data.per_month = new_per_month;
            self.data.remaining = new_per_month;
            self.data.pending_per_month = None;
            self.save()?;
            Ok(true)
        } else {
            // Defer to next month
            self.data.pending_per_month = Some(new_per_month);
            self.save()?;
            Ok(false)
        }
    }

    /// Persists the current data to disk.
    ///
    /// Uses atomic write (write to temp file, then rename) to prevent
    /// data corruption if the process crashes mid-write.
    ///
    /// Note: This is called automatically by `use_pass`, `set_per_month`,
    /// and `maybe_reset_month`. You only need to call this directly if
    /// you're making custom modifications to the data.
    pub fn save(&self) -> PassResult<()> {
        let json = serde_json::to_string_pretty(&self.data)?;

        // Atomic write: write to temp file, then rename
        let temp_path = self.path.with_extension("json.tmp");

        fs::write(&temp_path, &json).map_err(|source| PassError::WriteError {
            path: temp_path.clone(),
            source,
        })?;

        fs::rename(&temp_path, &self.path).map_err(|source| PassError::WriteError {
            path: self.path.clone(),
            source,
        })?;

        Ok(())
    }

    /// Returns a reference to the underlying PassData (for testing/debugging).
    #[cfg(test)]
    pub fn data(&self) -> &PassData {
        &self.data
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Returns the current month as a string in "YYYY-MM" format.
///
/// This function uses UTC time to ensure consistency across timezones.
///
/// # Examples
///
/// ```
/// use tether_core::passes::current_month_string;
///
/// // Returns something like "2025-01" depending on current date
/// let month = current_month_string();
/// assert!(month.len() == 7);
/// assert!(month.chars().nth(4) == Some('-'));
/// ```
#[must_use]
pub fn current_month_string() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

/// Validates a month string format.
///
/// # Arguments
///
/// * `month` - The month string to validate.
///
/// # Returns
///
/// `true` if the string matches "YYYY-MM" format with valid values.
///
/// # Examples
///
/// ```
/// use tether_core::passes::is_valid_month_string;
///
/// assert!(is_valid_month_string("2025-01"));
/// assert!(is_valid_month_string("2024-12"));
/// assert!(!is_valid_month_string("2025-13")); // Invalid month
/// assert!(!is_valid_month_string("2025-1"));  // Missing leading zero
/// assert!(!is_valid_month_string("25-01"));   // Year too short
/// ```
#[must_use]
pub fn is_valid_month_string(month: &str) -> bool {
    if month.len() != 7 {
        return false;
    }

    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return false;
    }

    let year: Result<i32, _> = parts[0].parse();
    let month_num: Result<u32, _> = parts[1].parse();

    match (year, month_num) {
        (Ok(y), Ok(m)) => y >= 1970 && y <= 9999 && (1..=12).contains(&m),
        _ => false,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper to create a manager with a temp file
    fn create_temp_manager(per_month: u32) -> (PassManager, PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("passes.json");
        // Keep dir alive by leaking it (for testing only)
        let path_clone = path.clone();
        std::mem::forget(dir);
        let manager = PassManager::load_or_create(&path_clone, per_month).unwrap();
        (manager, path_clone)
    }

    #[test]
    fn test_current_month_string_format() {
        let month = current_month_string();
        assert_eq!(month.len(), 7);
        assert!(month.chars().nth(4) == Some('-'));
        assert!(is_valid_month_string(&month));
    }

    #[test]
    fn test_is_valid_month_string() {
        assert!(is_valid_month_string("2025-01"));
        assert!(is_valid_month_string("2024-12"));
        assert!(is_valid_month_string("1970-01"));
        assert!(is_valid_month_string("9999-12"));

        assert!(!is_valid_month_string("2025-13")); // Invalid month
        assert!(!is_valid_month_string("2025-00")); // Invalid month
        assert!(!is_valid_month_string("2025-1")); // Missing leading zero
        assert!(!is_valid_month_string("25-01")); // Year too short
        assert!(!is_valid_month_string(""));
        assert!(!is_valid_month_string("2025/01")); // Wrong separator
        assert!(!is_valid_month_string("abcd-01")); // Non-numeric year
    }

    #[test]
    fn test_new_manager_creates_file() {
        let (manager, path) = create_temp_manager(3);
        assert!(path.exists());
        assert_eq!(manager.remaining(), 3);
        assert_eq!(manager.per_month(), 3);
        assert_eq!(manager.pending_per_month(), None);
    }

    #[test]
    fn test_load_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("passes.json");

        // Create initial manager
        {
            let mut manager = PassManager::load_or_create(&path, 5).unwrap();
            manager.use_pass("Test reason".to_string()).unwrap();
            assert_eq!(manager.remaining(), 4);
        }

        // Load again
        let manager = PassManager::load_or_create(&path, 10).unwrap(); // per_month ignored for existing
        assert_eq!(manager.remaining(), 4);
        assert_eq!(manager.per_month(), 5); // Original value preserved
    }

    #[test]
    fn test_use_pass_decrements_remaining() {
        let (mut manager, _path) = create_temp_manager(3);
        assert_eq!(manager.remaining(), 3);

        manager.use_pass("First".to_string()).unwrap();
        assert_eq!(manager.remaining(), 2);

        manager.use_pass("Second".to_string()).unwrap();
        assert_eq!(manager.remaining(), 1);

        manager.use_pass("Third".to_string()).unwrap();
        assert_eq!(manager.remaining(), 0);
    }

    #[test]
    fn test_use_pass_records_history() {
        let (mut manager, _path) = create_temp_manager(3);

        let entry = manager.use_pass("Test reason".to_string()).unwrap();
        assert_eq!(entry.reason, "Test reason");

        let history = manager.history(&current_month_string());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].reason, "Test reason");
    }

    #[test]
    fn test_use_pass_no_passes_remaining() {
        let (mut manager, _path) = create_temp_manager(1);

        manager.use_pass("Only pass".to_string()).unwrap();

        let result = manager.use_pass("Should fail".to_string());
        assert!(matches!(result, Err(PassError::NoPassesRemaining { .. })));
    }

    #[test]
    fn test_use_pass_empty_reason() {
        let (mut manager, _path) = create_temp_manager(3);

        let result = manager.use_pass(String::new());
        assert!(matches!(result, Err(PassError::EmptyReason)));

        let result = manager.use_pass("   ".to_string());
        assert!(matches!(result, Err(PassError::EmptyReason)));
    }

    #[test]
    fn test_use_pass_reason_too_long() {
        let (mut manager, _path) = create_temp_manager(3);

        let long_reason = "x".repeat(MAX_REASON_LENGTH + 1);
        let result = manager.use_pass(long_reason);
        assert!(matches!(result, Err(PassError::ReasonTooLong { .. })));

        // Exactly max length should work
        let max_reason = "x".repeat(MAX_REASON_LENGTH);
        let result = manager.use_pass(max_reason);
        assert!(result.is_ok());
    }

    #[test]
    fn test_use_pass_trims_whitespace() {
        let (mut manager, _path) = create_temp_manager(3);

        let entry = manager.use_pass("  trimmed reason  ".to_string()).unwrap();
        assert_eq!(entry.reason, "trimmed reason");
    }

    #[test]
    fn test_history_empty_month() {
        let (manager, _path) = create_temp_manager(3);

        let history = manager.history("2020-01");
        assert!(history.is_empty());
    }

    #[test]
    fn test_all_history() {
        let (mut manager, _path) = create_temp_manager(3);

        manager.use_pass("Test".to_string()).unwrap();

        let all = manager.all_history();
        assert!(all.contains_key(&current_month_string()));
    }

    #[test]
    fn test_set_per_month_immediate_when_no_passes_used() {
        let (mut manager, _path) = create_temp_manager(3);
        assert_eq!(manager.remaining(), 3);

        let immediate = manager.set_per_month(5).unwrap();
        assert!(immediate);
        assert_eq!(manager.per_month(), 5);
        assert_eq!(manager.remaining(), 5);
        assert_eq!(manager.pending_per_month(), None);
    }

    #[test]
    fn test_set_per_month_deferred_when_passes_used() {
        let (mut manager, _path) = create_temp_manager(3);

        manager.use_pass("Use one".to_string()).unwrap();
        assert_eq!(manager.remaining(), 2);

        let immediate = manager.set_per_month(5).unwrap();
        assert!(!immediate);
        assert_eq!(manager.per_month(), 3); // Unchanged
        assert_eq!(manager.remaining(), 2); // Unchanged
        assert_eq!(manager.pending_per_month(), Some(5));
    }

    #[test]
    fn test_set_per_month_same_value() {
        let (mut manager, _path) = create_temp_manager(3);

        manager.use_pass("Use one".to_string()).unwrap();

        // Set pending
        manager.set_per_month(5).unwrap();
        assert_eq!(manager.pending_per_month(), Some(5));

        // Set back to current - should clear pending
        let immediate = manager.set_per_month(3).unwrap();
        assert!(immediate);
        assert_eq!(manager.pending_per_month(), None);
    }

    #[test]
    fn test_pass_data_serialization() {
        let mut data = PassData::new(3);
        data.history.insert(
            "2025-01".to_string(),
            vec![PassEntry {
                used_at_utc: Utc::now(),
                reason: "Test".to_string(),
            }],
        );

        let json = serde_json::to_string_pretty(&data).unwrap();
        let parsed: PassData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current_month, data.current_month);
        assert_eq!(parsed.remaining, data.remaining);
        assert_eq!(parsed.per_month, data.per_month);
        assert_eq!(parsed.history.len(), 1);
    }

    #[test]
    fn test_persistence_across_loads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("passes.json");

        // Create and modify
        {
            let mut manager = PassManager::load_or_create(&path, 3).unwrap();
            manager.use_pass("First".to_string()).unwrap();
            manager.use_pass("Second".to_string()).unwrap();
        }

        // Reload and verify
        {
            let manager = PassManager::load_or_create(&path, 3).unwrap();
            assert_eq!(manager.remaining(), 1);
            let history = manager.history(&current_month_string());
            assert_eq!(history.len(), 2);
            assert_eq!(history[0].reason, "First");
            assert_eq!(history[1].reason, "Second");
        }
    }

    #[test]
    fn test_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("nested").join("passes.json");

        let manager = PassManager::load_or_create(&path, 3).unwrap();
        assert!(path.exists());
        assert_eq!(manager.remaining(), 3);
    }

    #[test]
    fn test_pass_entry_new() {
        let entry = PassEntry::new("Test".to_string());
        assert_eq!(entry.reason, "Test");
        // Timestamp should be very recent
        let age = Utc::now() - entry.used_at_utc;
        assert!(age.num_seconds() < 1);
    }

    #[test]
    fn test_zero_passes_per_month() {
        let (mut manager, _path) = create_temp_manager(0);
        assert_eq!(manager.remaining(), 0);

        let result = manager.use_pass("Should fail".to_string());
        assert!(matches!(result, Err(PassError::NoPassesRemaining { .. })));
    }
}
