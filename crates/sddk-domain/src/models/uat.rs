//! UAT result row from the control-plane.
use serde::{Deserialize, Serialize};

/// A row of the control-plane `uat_results` table (SDDK2-103).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatResultRow {
    pub project_id: String,
    pub tag_version: String,
    pub verdict: String,
    pub coverage_pct: f64,
    pub defects: i64,
    pub session_count: i64,
    pub uat_duration_minutes: i64,
    pub recorded_at: String,
}
