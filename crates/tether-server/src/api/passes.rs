//! Pass management API endpoints.
//!
//! Tether provides a configurable number of emergency passes per month.
//! When users need to keep their phone nearby (e.g., on-call, sick child),
//! they can use a pass with a reason. Passes refresh automatically on the
//! first day of each month.

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::api::error::{ApiError, ApiResult};
use crate::state::SharedState;

/// Creates the passes router with all endpoints.
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_passes))
        .route("/history", get(get_pass_history))
        .route("/use", post(use_pass))
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Current pass status for the month.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "remaining": 2,
    "total_per_month": 3,
    "used_this_month": 1,
    "month": "2025-01",
    "resets_at_utc": "2025-02-01T08:00:00Z",
    "timezone": "America/Los_Angeles"
}))]
pub struct PassesResponse {
    /// Number of passes remaining this month.
    #[schema(example = 2, minimum = 0)]
    pub remaining: u32,

    /// Total passes allocated per month (from config).
    #[schema(example = 3, minimum = 0)]
    pub total_per_month: u32,

    /// Number of passes used this month.
    #[schema(example = 1, minimum = 0)]
    pub used_this_month: u32,

    /// Current month in YYYY-MM format.
    #[schema(example = "2025-01")]
    pub month: String,

    /// UTC timestamp when passes will reset.
    #[schema(example = "2025-02-01T08:00:00Z")]
    pub resets_at_utc: String,

    /// Configured timezone for reset calculation.
    #[schema(example = "America/Los_Angeles")]
    pub timezone: String,
}

/// Query parameters for pass history endpoint.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PassHistoryQuery {
    /// Month to retrieve history for in YYYY-MM format.
    /// Defaults to current month if not specified.
    #[param(example = "2025-01")]
    pub month: Option<String>,
}

/// A single pass usage entry in history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "used_at_utc": "2025-01-15T03:30:00Z",
    "reason": "On-call for production incident"
}))]
pub struct PassHistoryEntry {
    /// UTC timestamp when the pass was used.
    #[schema(example = "2025-01-15T03:30:00Z")]
    pub used_at_utc: String,

    /// Reason provided when using the pass.
    #[schema(example = "On-call for production incident")]
    pub reason: String,
}

/// Pass usage history response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "month": "2025-01",
    "entries": [
        {
            "used_at_utc": "2025-01-15T03:30:00Z",
            "reason": "On-call for production incident"
        }
    ],
    "total_used": 1,
    "total_per_month": 3
}))]
pub struct PassHistoryResponse {
    /// Month in YYYY-MM format.
    #[schema(example = "2025-01")]
    pub month: String,

    /// List of pass usage entries for the month.
    pub entries: Vec<PassHistoryEntry>,

    /// Total passes used this month.
    #[schema(example = 1)]
    pub total_used: usize,

    /// Total passes allocated per month.
    #[schema(example = 3)]
    pub total_per_month: u32,
}

/// Request body for using a pass.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "reason": "On-call for production incident tonight"
}))]
pub struct UsePassRequest {
    /// Reason for using the pass. Required and must be non-empty.
    /// Maximum 500 characters.
    #[schema(example = "On-call for production incident tonight", min_length = 1, max_length = 500)]
    pub reason: String,
}

/// Response after successfully using a pass.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "success": true,
    "remaining": 1,
    "used_at_utc": "2025-01-15T03:30:00Z",
    "reason": "On-call for production incident tonight"
}))]
pub struct UsePassResponse {
    /// Whether the pass was successfully used.
    #[schema(example = true)]
    pub success: bool,

    /// Number of passes remaining after this use.
    #[schema(example = 1)]
    pub remaining: u32,

    /// UTC timestamp when the pass was used.
    #[schema(example = "2025-01-15T03:30:00Z")]
    pub used_at_utc: String,

    /// The reason that was recorded.
    #[schema(example = "On-call for production incident tonight")]
    pub reason: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get remaining passes for the current month.
///
/// Returns the number of passes remaining, total allocated, and when they reset.
#[utoipa::path(
    get,
    path = "/passes",
    tag = "passes",
    operation_id = "getPasses",
    summary = "Get remaining passes this month",
    description = "Returns current pass status including remaining count, total \
        allocated, and the UTC timestamp when passes will reset.",
    responses(
        (status = 200, description = "Pass status retrieved", body = PassesResponse)
    )
)]
pub async fn get_passes(State(state): State<SharedState>) -> ApiResult<Json<PassesResponse>> {
    let state_guard = state.read().await;

    let remaining = state_guard.pass_manager.remaining();
    let per_month = state_guard.pass_manager.per_month();
    let month = state_guard.pass_manager.current_month().to_string();
    let used_this_month = per_month.saturating_sub(remaining);

    let timezone = &state_guard.config.system.timezone;
    let resets_at_utc = calculate_next_reset_utc(timezone);

    Ok(Json(PassesResponse {
        remaining,
        total_per_month: per_month,
        used_this_month,
        month,
        resets_at_utc,
        timezone: timezone.clone(),
    }))
}

/// Get pass usage history for a specific month.
///
/// Returns all pass usage entries for the specified month (or current month if not specified).
#[utoipa::path(
    get,
    path = "/passes/history",
    tag = "passes",
    operation_id = "getPassHistory",
    summary = "Get pass usage history",
    description = "Returns all pass usage entries for a specific month. \
        Defaults to the current month if no month is specified.",
    params(PassHistoryQuery),
    responses(
        (status = 200, description = "History retrieved", body = PassHistoryResponse),
        (status = 400, description = "Invalid month format")
    )
)]
pub async fn get_pass_history(
    State(state): State<SharedState>,
    Query(query): Query<PassHistoryQuery>,
) -> ApiResult<Json<PassHistoryResponse>> {
    let state_guard = state.read().await;

    // Determine which month to query
    let month = match query.month {
        Some(m) => {
            // Validate month format: YYYY-MM
            if !tether_core::is_valid_month_string(&m) {
                return Err(ApiError::BadRequest {
                    error_code: "invalid_month_format".to_string(),
                    message: "Month must be in YYYY-MM format (e.g., 2025-01)".to_string(),
                });
            }
            m
        }
        None => state_guard.pass_manager.current_month().to_string(),
    };

    let history = state_guard.pass_manager.history(&month);
    let entries: Vec<PassHistoryEntry> = history
        .iter()
        .map(|entry| PassHistoryEntry {
            used_at_utc: entry.used_at_utc.to_rfc3339(),
            reason: entry.reason.clone(),
        })
        .collect();

    let total_used = entries.len();
    let per_month = state_guard.pass_manager.per_month();

    Ok(Json(PassHistoryResponse {
        month,
        entries,
        total_used,
        total_per_month: per_month,
    }))
}

/// Use a pass for today.
///
/// Uses one of the remaining passes with a required reason.
#[utoipa::path(
    post,
    path = "/passes/use",
    tag = "passes",
    operation_id = "usePass",
    summary = "Use a pass",
    description = "Uses one of the remaining passes for this month. A reason \
        is required and will be recorded in the history.",
    request_body = UsePassRequest,
    responses(
        (status = 200, description = "Pass used successfully", body = UsePassResponse),
        (status = 400, description = "Invalid request (empty reason or too long)"),
        (status = 409, description = "No passes remaining")
    )
)]
pub async fn use_pass(
    State(state): State<SharedState>,
    Json(request): Json<UsePassRequest>,
) -> ApiResult<Json<UsePassResponse>> {
    let mut state_guard = state.write().await;

    // Use the pass (validation and persistence happen in PassManager)
    let entry = state_guard.pass_manager.use_pass(request.reason)?;
    let remaining = state_guard.pass_manager.remaining();

    Ok(Json(UsePassResponse {
        success: true,
        remaining,
        used_at_utc: entry.used_at_utc.to_rfc3339(),
        reason: entry.reason,
    }))
}

// ============================================================================
// Helpers
// ============================================================================

/// Calculate when passes will reset (first of next month at midnight local time).
fn calculate_next_reset_utc(timezone: &str) -> String {
    let tz: chrono_tz::Tz = timezone.parse().unwrap_or(chrono_tz::UTC);
    let now_local = Utc::now().with_timezone(&tz);

    // First of next month at midnight local time
    let next_month = if now_local.month() == 12 {
        tz.with_ymd_and_hms(now_local.year() + 1, 1, 1, 0, 0, 0)
    } else {
        tz.with_ymd_and_hms(now_local.year(), now_local.month() + 1, 1, 0, 0, 0)
    };

    match next_month {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc).to_rfc3339(),
        _ => Utc::now().to_rfc3339(), // Fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passes_response_serialization() {
        let response = PassesResponse {
            remaining: 2,
            total_per_month: 3,
            used_this_month: 1,
            month: "2025-01".to_string(),
            resets_at_utc: "2025-02-01T08:00:00Z".to_string(),
            timezone: "America/Los_Angeles".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"remaining\":2"));
    }

    #[test]
    fn test_use_pass_request_deserialization() {
        let json = r#"{"reason": "Test reason"}"#;
        let request: UsePassRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.reason, "Test reason");
    }

    #[test]
    fn test_calculate_next_reset_utc() {
        let reset = calculate_next_reset_utc("UTC");
        assert!(!reset.is_empty());
        // Should be a valid RFC3339 timestamp
        assert!(reset.contains('T'));
    }
}
