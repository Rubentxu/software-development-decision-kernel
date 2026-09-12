//! `Engine::cycle_replan` — bounded in-place revision.
//!
//! Per [[REQ-Cycle-Replan-Bounded-Counter]] and [[REQ-Cycle-Replan-Receipt]]:
//!   - lease fence verified fail-closed before any mutation
//!   - counter ≤ 5 (`STORAGE_REPLAN_LIMIT` → `ReplanLimitExceeded`)
//!   - delta must be non-empty (STORAGE_REPLAN_EMPTY_DELTA)
//!   - each success increments `manifest.replan_count` by exactly 1,
//!     emits `cycle.replan.requested` + `cycle.replan.applied`, and
//!     writes an atomic `replan-receipt.json` under `{cycle_artifacts_dir}`.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

use crate::{Engine, EngineError, EventReceipt, write_atomic};
use sddk_domain::{LedgerEventInput, StorageError as DomainStorageError};

/// Maximum number of successful in-place replans per cycle.
pub const REPLAN_LIMIT: u32 = 5;

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

impl RestageTo {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestageTo::Propose => "propose",
            RestageTo::Specify => "specify",
            RestageTo::Design => "design",
            RestageTo::Tasks => "tasks",
            RestageTo::Apply => "apply",
        }
    }
}

/// Input for writing a replan receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleReplanInput {
    pub cycle_id: String,
    pub restage_to: String,
    pub changed_files: Vec<String>,
    pub reason: String,
    pub replan_count: u32,
    pub lease_owner: String,
    pub fencing_token: i64,
    pub replan_at: String,
}

impl<L: sddk_domain::Ledger> Engine<L> {
    /// Bounded in-place revision of a cycle (lease-fenced, counter-limited).
    ///
    /// Emits `cycle.replan.requested` + `cycle.replan.applied`, increments
    /// `manifest.replan_count`, restages the phase, and writes an atomic
    /// `replan-receipt.json` under `receipt_path/<cycle_id>/`.
    #[allow(clippy::too_many_arguments)]
    pub fn cycle_replan(
        &mut self,
        cycle_id: &str,
        restage_to: RestageTo,
        delta: &ReplanDelta,
        evidence_refs: &[String],
        actor: &str,
        command_id: &str,
        event_id: &str,
        occurred_at: &str,
        receipt_path: &Path,
        lease_owner: &str,
        fencing_token: i64,
    ) -> Result<EventReceipt, EngineError> {
        // Empty delta rejection before any state access.
        if delta.changed_files.is_empty() || delta.reason.is_empty() {
            return Err(EngineError::ReplanEmptyDelta);
        }

        // Lease fence: fail-closed when no lease is held.
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(i64::MAX);
        let lease_result =
            self.ledger
                .verify_cycle_lease(cycle_id, lease_owner, fencing_token, now_ms);
        match lease_result {
            Ok(_) => {}
            Err(DomainStorageError::NotFound { .. }) => {
                return Err(EngineError::Storage(DomainStorageError::LeaseConflict {
                    cycle_id: cycle_id.to_owned(),
                    owner: lease_owner.to_owned(),
                }));
            }
            Err(e) => return Err(EngineError::Storage(e)),
        }

        // Load current cycle.
        let current = self.ledger.get_cycle(cycle_id)?.manifest;

        // Bounded counter: reject when the limit is already reached.
        if current.replan_count >= REPLAN_LIMIT {
            return Err(EngineError::ReplanLimitExceeded);
        }

        // Build the two events: cycle.replan.requested + cycle.replan.applied
        let event_id_requested = event_id.to_owned();
        let event_id_applied = format!("{event_id}-applied");

        let new_count = current.replan_count + 1;

        let payload_requested = json!({
            "restage_to": restage_to.as_str(),
            "changed_files": delta.changed_files,
            "reason": delta.reason,
            "evidence_refs": evidence_refs,
            "replan_count": new_count,
        });

        let event_input_requested = LedgerEventInput {
            event_id: event_id_requested.clone(),
            project_id: current.project_id.clone(),
            cycle_id: Some(cycle_id.to_owned()),
            frame_id: format!("frame:{command_id}"),
            command_id: command_id.to_owned(),
            actor: actor.to_owned(),
            actor_ref: None,
            event_type: "cycle.replan.requested".to_owned(),
            occurred_at: occurred_at.to_owned(),
            state_before: None,
            state_after: None,
            payload: payload_requested,
            causation_id: None,
            correlation_id: None,
        };

        let event_input_applied = LedgerEventInput {
            event_id: event_id_applied.clone(),
            project_id: current.project_id.clone(),
            cycle_id: Some(cycle_id.to_owned()),
            frame_id: format!("frame:{command_id}"),
            command_id: command_id.to_owned(),
            actor: actor.to_owned(),
            actor_ref: None,
            event_type: "cycle.replan.applied".to_owned(),
            occurred_at: occurred_at.to_owned(),
            state_before: Some(
                serde_json::to_value(&current).map_err(EngineError::StateSerialization)?,
            ),
            state_after: None,
            payload: json!({
                "restage_to": restage_to.as_str(),
                "changed_files": delta.changed_files,
                "reason": delta.reason,
                "replan_count": new_count,
                "event_ids": [event_id_requested.clone(), event_id_applied.clone()],
            }),
            causation_id: None,
            correlation_id: None,
        };

        // Mutate the manifest: bump counter + restage phase.
        let mut updated_manifest = current.clone();
        updated_manifest.replan_count = new_count;
        updated_manifest.phase = match restage_to {
            RestageTo::Propose => sddk_domain::Phase::Explore,
            RestageTo::Specify => sddk_domain::Phase::Specify,
            RestageTo::Design => sddk_domain::Phase::Design,
            RestageTo::Tasks => sddk_domain::Phase::Plan,
            RestageTo::Apply => sddk_domain::Phase::Build,
        };

        // Write replan receipt atomically BEFORE mutating the ledger so a
        // failed write leaves the ledger untouched (fail-closed ordering).
        let receipt_input = CycleReplanInput {
            cycle_id: cycle_id.to_owned(),
            restage_to: restage_to.as_str().to_owned(),
            changed_files: delta.changed_files.clone(),
            reason: delta.reason.clone(),
            replan_count: new_count,
            lease_owner: lease_owner.to_owned(),
            fencing_token,
            replan_at: occurred_at.to_owned(),
        };
        let receipt_json = serde_json::to_string_pretty(&receipt_input)
            .map_err(EngineError::StateSerialization)?;
        let receipt_file_path = receipt_path.join(cycle_id).join("replan-receipt.json");
        if let Some(parent) = receipt_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                EngineError::Storage(sddk_domain::StorageError::Other(e.to_string()))
            })?;
        }
        write_atomic(&receipt_file_path, receipt_json.as_bytes()).map_err(|e| {
            EngineError::Storage(sddk_domain::StorageError::Other(format!(
                "failed to write replan receipt: {e}"
            )))
        })?;

        // Emit cycle.replan.requested (manifest restaged, counter bumped).
        let _event_requested = self
            .ledger
            .update_cycle_with_event(
                &updated_manifest,
                occurred_at,
                &event_input_requested,
                false,
            )
            .map_err(EngineError::Storage)?;

        // Emit cycle.replan.applied (manifest unchanged).
        let event_applied = self
            .ledger
            .update_cycle_with_event(&updated_manifest, occurred_at, &event_input_applied, false)
            .map_err(EngineError::Storage)?;

        Ok(EventReceipt {
            event_id: event_applied.event_id.clone(),
            sequence: event_applied.sequence,
            event_hash: event_applied.event_hash.clone(),
        })
    }
}
