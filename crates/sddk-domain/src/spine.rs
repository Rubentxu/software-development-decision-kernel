//! Execution Spine parsing and types.
//!
//! Implements the PLN-LEDGER-003 spine import per ADR-073 Q4 and spec PLN-LEDGER-003 §6.
//!
//! The spine is the single source of truth for the backlog. It is parsed using
//! `serde_saphyr` (not `serde_yaml`) per ADR-073 Q4.

use serde::{Deserialize, Serialize};

use crate::planning::WorkItemStatus;

/// Structured error from spine parsing.
#[derive(Debug, Clone, thiserror::Error)]
#[error("spine parse error at {line}:{column}: {reason}")]
pub struct SpineParseError {
    /// Line number where the error was detected (1-indexed).
    pub line: u32,
    /// Column number where the error was detected (1-indexed).
    pub column: u32,
    /// Human-readable reason for the parse failure.
    pub reason: String,
}

/// Top-level spine document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSpineV1 {
    /// Schema version; must be 2.
    pub schema_version: u32,
    /// Plan identifier.
    pub plan_id: String,
    /// Baseline release info.
    #[serde(default)]
    pub baseline: SpineBaseline,
    /// Human-readable purpose.
    #[serde(default)]
    pub purpose: String,
    /// Status vocabulary definitions.
    #[serde(default, rename = "status_vocabulary")]
    pub status_vocabulary: SpineStatusVocabulary,
    /// Selection rule lines.
    #[serde(default, rename = "selection_rule")]
    pub selection_rule: Vec<String>,
    /// Cycle binding configuration.
    #[serde(default, rename = "cycle_binding")]
    pub cycle_binding: SpineCycleBinding,
    /// Terminal goal.
    #[serde(default, rename = "terminal_goal")]
    pub terminal_goal: SpineTerminalGoal,
    /// Horizon definitions.
    #[serde(default)]
    pub horizons: Vec<SpineHorizonDef>,
    /// Spine items (the actual work items).
    pub items: Vec<SpineItemV1>,
}

/// Baseline release info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpineBaseline {
    /// Release version string.
    pub release: String,
    /// ISO-8601 timestamp of last reconciliation.
    #[serde(default)]
    pub reconciled_at: String,
}

/// Status vocabulary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpineStatusVocabulary {
    /// Terminal (done) status values in this spine.
    #[serde(default)]
    pub terminal: Vec<String>,
    /// Executable (in-progress) status values in this spine.
    #[serde(default)]
    pub executable: Vec<String>,
    /// Non-executable (blocked/paused) status values in this spine.
    #[serde(default, rename = "non_executable")]
    pub non_executable: Vec<String>,
}

/// Cycle binding configuration from the spine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpineCycleBinding {
    /// Identity type; must be "semantic_work_item_id".
    #[serde(default)]
    pub identity: String,
    /// Execution instance type; must be "cycle_or_run_id".
    #[serde(default, rename = "execution_instance")]
    pub execution_instance: String,
    /// Binding rule text.
    #[serde(default)]
    pub rule: String,
}

/// Terminal goal from the spine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpineTerminalGoal {
    /// Goal identifier.
    #[serde(default)]
    pub id: String,
    /// Goal completion condition expression.
    #[serde(default)]
    pub condition: String,
}

/// Horizon definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineHorizonDef {
    /// Horizon classification (H0–H12).
    pub id: SpineHorizon,
    /// Human-readable name for this horizon.
    pub name: String,
}

/// A single spine work item row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpineItemV1 {
    /// Execution order.
    pub order: u32,
    /// Unique work item identifier.
    pub id: String,
    /// Horizon classification.
    pub horizon: SpineHorizon,
    /// Dependency list.
    #[serde(default, rename = "depends_on")]
    pub depends_on: Vec<String>,
    /// Current status.
    pub status: SpineStatus,
    /// Work objective.
    pub objective: String,
    /// Exit gate definition.
    #[serde(default, rename = "exit_gate")]
    pub exit_gate: String,
}

/// Spine horizon variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpineHorizon {
    /// Horizon 0 — immediate / current cycle.
    H0,
    /// Horizon 1 — next 1–3 cycles.
    H1,
    /// Horizon 2 — 3–10 cycles out.
    H2,
    /// Horizon 3 — 10–30 cycles out.
    H3,
    /// Horizon 4 — strategic window.
    H4,
    /// Horizon 5 — exploratory.
    H5,
    /// Horizon 6 — architectural.
    H6,
    /// Horizon 7 — research.
    H7,
    /// Horizon 8 — deprecated.
    H8,
    /// Horizon 9 — backlog.
    H9,
    /// Horizon 10 — icebox.
    H10,
    /// Horizon 11 — rejected.
    H11,
    /// Horizon 12 — shipped.
    H12,
}

/// Spine status variants (8 total).
///
/// Maps to `WorkItemStatus` per the locked table in spec PLN-LEDGER-003 §7:
/// - PROPOSED → Draft
/// - READY → Draft
/// - ACTIVE → Active
/// - PARTIAL → Active
/// - BLOCKED → Paused
/// - SHIPPED → Done
/// - ABSORBED → Done
/// - SUPERSEDED → Superseded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpineStatus {
    /// Proposed — not yet started.
    Proposed,
    /// Ready — cleared to start.
    Ready,
    /// Active — in progress.
    Active,
    /// Partial — partially complete.
    Partial,
    /// Blocked — paused due to dependency.
    Blocked,
    /// Shipped — completed and delivered.
    Shipped,
    /// Absorbed — incorporated into another item.
    Absorbed,
    /// Superseded — replaced by another item.
    Superseded,
}

impl SpineStatus {
    /// Maps a spine status to the corresponding `WorkItemStatus`.
    ///
    /// The mapping is total and deterministic per spec PLN-LEDGER-003 §7.
    ///
    /// Returns `Err(SpineParseError)` for unknown status values.
    pub fn to_work_item_status(self) -> Result<WorkItemStatus, SpineParseError> {
        match self {
            SpineStatus::Proposed => Ok(WorkItemStatus::Draft),
            SpineStatus::Ready => Ok(WorkItemStatus::Draft),
            SpineStatus::Active => Ok(WorkItemStatus::Active),
            SpineStatus::Partial => Ok(WorkItemStatus::Active),
            SpineStatus::Blocked => Ok(WorkItemStatus::Paused),
            SpineStatus::Shipped => Ok(WorkItemStatus::Done),
            SpineStatus::Absorbed => Ok(WorkItemStatus::Done),
            SpineStatus::Superseded => Ok(WorkItemStatus::Superseded),
        }
    }
}

/// Parses an EXECUTION-SPINE.yaml byte stream into `ExecutionSpineV1`.
///
/// Uses `serde_saphyr::from_str` (not `serde_yaml`) per ADR-073 Q4.
/// Inline comments (`status: SHIPPED # comment`) are tolerated by the parser.
///
/// Returns `SpineParseError` for malformed input including:
/// - Missing `schema_version`
/// - Unknown top-level fields
/// - Empty items array (accepted — empty spine is legal)
pub fn parse_spine_yaml(bytes: &[u8]) -> Result<ExecutionSpineV1, SpineParseError> {
    let spine_str = String::from_utf8(bytes.to_vec()).map_err(|_| SpineParseError {
        line: 1,
        column: 1,
        reason: "invalid_utf8".to_string(),
    })?;

    // Quick pre-check: verify schema_version: 2 appears in the input
    let has_schema_version_2 = spine_str
        .lines()
        .any(|line| line.trim().starts_with("schema_version:") && line.contains("2"));
    if !has_schema_version_2 {
        return Err(SpineParseError {
            line: 1,
            column: 1,
            reason: "missing_schema_version".to_string(),
        });
    }

    // Normalize horizon values: YAML uses H0/H1 but SpineHorizon uses snake_case (h0/h1).
    // Convert any ": H" (followed by a digit) to ": h" to handle both item-level
    // "horizon: H0" and horizons-array "id: H0" fields.
    let spine_str = spine_str
        .lines()
        .map(|line| {
            // Match patterns like "horizon: H0", "id: H0", "id: H10" etc.
            // Replace ": H" followed by a digit with ": h"
            let mut result = line.to_string();
            // Keep replacing while we find matches (handles multiple per line)
            while let Some(pos) = result.find(": H") {
                // Check that the next char after " H" is a digit (byte check)
                let bytes = result.as_bytes();
                if pos + 3 < bytes.len() && bytes[pos + 3].is_ascii_digit() {
                    result.replace_range(pos..pos + 3, ": h");
                } else {
                    break;
                }
            }
            result
        })
        .collect::<Vec<_>>()
        .join("\n");

    serde_saphyr::from_str(&spine_str).map_err(|e| {
        let location = e.location();
        let (line, column) = location
            .map(|l| (l.line() as u32, l.column() as u32))
            .unwrap_or((1, 1));

        // Map saphyr error to a clean message
        let reason = if e.to_string().contains("unknown field") {
            // Extract the field name from the error message
            let err_str = e.to_string();
            if let Some(start) = err_str.find("unknown field `") {
                let rest = &err_str[start + 13..];
                if let Some(end) = rest.find('`') {
                    let field = &rest[..end];
                    format!("unknown_field: {}", field)
                } else {
                    format!("saphyr_parse_error: {}", err_str)
                }
            } else {
                format!("saphyr_parse_error: {}", err_str)
            }
        } else {
            format!("saphyr_parse_error: {}", e)
        };

        SpineParseError {
            line,
            column,
            reason,
        }
    })
}

/// Canonicalizes spine YAML bytes to a deterministic form for content-addressing.
///
/// Strips trailing whitespace, normalizes line endings, and removes leading/trailing
/// blank lines so that two semantically identical YAML documents produce the same hash.
pub fn canonicalize_spine_bytes(bytes: &[u8]) -> Vec<u8> {
    // Remove leading/trailing blank lines and normalize to LF
    let text = String::from_utf8(bytes.to_vec()).unwrap_or_default();
    let canonical: String = text
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    // Remove trailing blank lines
    let canonical = canonical.trim_end();
    format!("{}\n", canonical).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spine_status_to_work_item_status_proposed() {
        assert_eq!(
            SpineStatus::Proposed.to_work_item_status().unwrap(),
            WorkItemStatus::Draft
        );
    }

    #[test]
    fn spine_status_to_work_item_status_ready() {
        assert_eq!(
            SpineStatus::Ready.to_work_item_status().unwrap(),
            WorkItemStatus::Draft
        );
    }

    #[test]
    fn spine_status_to_work_item_status_active() {
        assert_eq!(
            SpineStatus::Active.to_work_item_status().unwrap(),
            WorkItemStatus::Active
        );
    }

    #[test]
    fn spine_status_to_work_item_status_partial() {
        assert_eq!(
            SpineStatus::Partial.to_work_item_status().unwrap(),
            WorkItemStatus::Active
        );
    }

    #[test]
    fn spine_status_to_work_item_status_blocked() {
        assert_eq!(
            SpineStatus::Blocked.to_work_item_status().unwrap(),
            WorkItemStatus::Paused
        );
    }

    #[test]
    fn spine_status_to_work_item_status_shipped() {
        assert_eq!(
            SpineStatus::Shipped.to_work_item_status().unwrap(),
            WorkItemStatus::Done
        );
    }

    #[test]
    fn spine_status_to_work_item_status_absorbed() {
        assert_eq!(
            SpineStatus::Absorbed.to_work_item_status().unwrap(),
            WorkItemStatus::Done
        );
    }

    #[test]
    fn spine_status_to_work_item_status_superseded() {
        assert_eq!(
            SpineStatus::Superseded.to_work_item_status().unwrap(),
            WorkItemStatus::Superseded
        );
    }
}
