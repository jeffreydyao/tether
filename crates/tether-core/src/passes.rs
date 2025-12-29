//! Monthly pass allocation and tracking.
//!
//! Passes allow users to bypass the proximity requirement for a night.
//! They refresh automatically at midnight on the first day of each month.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::config::PassesConfig;
use crate::error::{Error, Result};
use crate::storage::Storage;

/// A single pass usage record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PassUsage {
    /// When the pass was used (UTC).
    pub used_at_utc: DateTime<Utc>,

    /// User-provided reason for using the pass.
    #[schema(example = "Traveling for work")]
    pub reason: String,
}

/// Pass state for a specific month.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MonthlyPassState {
    /// Year of this pass state.
    pub year: i32,

    /// Month of this pass state (1-12).
    pub month: u32,

    /// Passes allocated for this month.
    pub allocated: u8,

    /// Passes used this month.
    pub used: Vec<PassUsage>,
}

impl MonthlyPassState {
    /// Get the number of remaining passes.
    #[must_use]
    pub fn remaining(&self) -> u8 {
        self.allocated.saturating_sub(self.used.len() as u8)
    }
}

/// Manager for pass allocation and usage tracking.
pub struct PassManager {
    storage: Storage,
    passes_config: PassesConfig,
}

impl PassManager {
    /// Create a new pass manager.
    pub fn new(storage: Storage, passes_config: PassesConfig) -> Self {
        Self {
            storage,
            passes_config,
        }
    }

    /// Get the current month's pass state.
    ///
    /// If no state exists for the current month, creates a new one.
    pub fn current_month_state(&self) -> Result<MonthlyPassState> {
        let now = Utc::now();
        self.get_or_create_month_state(now.year(), now.month())
    }

    /// Get passes remaining for the current month.
    pub fn remaining_passes(&self) -> Result<u8> {
        Ok(self.current_month_state()?.remaining())
    }

    /// Use a pass with the given reason.
    ///
    /// # Errors
    ///
    /// Returns `Error::NoPassesRemaining` if no passes are available.
    pub fn use_pass(&mut self, reason: String) -> Result<PassUsage> {
        let mut state = self.current_month_state()?;

        if state.remaining() == 0 {
            return Err(Error::NoPassesRemaining);
        }

        let usage = PassUsage {
            used_at_utc: Utc::now(),
            reason,
        };

        state.used.push(usage.clone());
        self.storage.save_month_state(&state)?;

        Ok(usage)
    }

    /// Get pass history for a specific month.
    pub fn get_month_history(&self, year: i32, month: u32) -> Result<Option<MonthlyPassState>> {
        self.storage.load_month_state(year, month)
    }

    fn get_or_create_month_state(&self, year: i32, month: u32) -> Result<MonthlyPassState> {
        match self.storage.load_month_state(year, month)? {
            Some(state) => Ok(state),
            None => Ok(MonthlyPassState {
                year,
                month,
                allocated: self.passes_config.per_month,
                used: Vec::new(),
            }),
        }
    }
}
