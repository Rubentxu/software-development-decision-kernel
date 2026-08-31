//! `Engine::cycle_replan` — bounded in-place revision.
//!
//! Per [[REQ-Cycle-Replan-Bounded-Counter]] and [[REQ-Cycle-Replan-Receipt]]:
//!   - counter ≤ 5 (STORAGE_REPLAN_LIMIT)
//!   - delta must be non-empty (STORAGE_REPLAN_EMPTY_DELTA)
//!   - `--confirm-apply` flag for restage-to=Apply
//!
//! Stub implementation — full cycle replan to be added.

use crate::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Delta for a replan operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplanDelta {
    pub changed_files: Vec<String>,
    pub reason: String,
}

/// Target phase for a replan restage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestageTo {
    Propose,
    Specify,
    Design,
    Tasks,
    Apply,
}

impl<L: sddk_domain::Ledger> Engine<L> {
    /// Bounded in-place revision of a cycle.
    ///
    /// Stub: returns `EngineError::ReplanLimitExceeded` until implemented.
    #[allow(clippy::too_many_arguments)]
    pub fn cycle_replan(
        &mut self,
        _cycle_id: &str,
        _restage_to: RestageTo,
        _delta: &ReplanDelta,
        _evidence_refs: &[String],
        _actor: &str,
        _command_id: &str,
        _event_id: &str,
        _occurred_at: &str,
        _receipt_path: &Path,
        _lease_owner: &str,
        _fencing_token: i64,
    ) -> Result<(), EngineError> {
        Err(EngineError::ReplanLimitExceeded)
    }
}
