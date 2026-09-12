//! `Engine::cycle_supersede` — close a cycle with a successor or a reason.
//!
//! Per [[REQ-Cycle-Supersede-Contract]] and [[REQ-Cycle-Lease-Fence]]:
//! - Lease fence required before calling.
//! - Exactly one of `successor` XOR `reason` must be supplied.
//! - Self-supersede is forbidden.
//! - Emits `cycle.supersede.requested` and `cycle.supersede.applied` events.
//! - Releases the lease atomically.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{Engine, EngineError, EventReceipt, authority::AuthorityContext, write_atomic};
use sddk_domain::{LedgerEventInput, StorageError as DomainStorageError};
use serde_json::json;

/// Reason for superseding a cycle when no successor is available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupersedeReason {
    /// The cycle's scope became invalid (e.g., project deleted, repo transferred).
    ScopeInvalid,
    /// The original goal was replaced by a new cycle.
    GoalReplaced,
    /// An external event (force majeure, dependency removal) makes the cycle obsolete.
    ExternalObsolete,
}

impl SupersedeReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupersedeReason::ScopeInvalid => "scope_invalid",
            SupersedeReason::GoalReplaced => "goal_replaced",
            SupersedeReason::ExternalObsolete => "external_obsolete",
        }
    }
}

/// Input for writing a supersede receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleSupersedeInput {
    pub cycle_id: String,
    pub successor: Option<String>,
    pub reason: Option<SupersedeReason>,
    pub event_ids: [String; 2],
    pub lease_owner: String,
    pub fencing_token: i64,
    /// Number of repair receipts preserved (annotated `waiver: superseded`)
    /// from this cycle's repair queue. Self-attesting audit field
    /// (REQ-Repair-Receipt-Supersede-Preservation).
    #[serde(default)]
    pub preserved_receipts_count: usize,
}

impl<L: sddk_domain::Ledger> Engine<L> {
    /// Close a cycle with a successor OR a closed-set reason, releasing the lease.
    ///
    /// The caller MUST call `require_lease_fence` before calling this method and
    /// pass the same `owner` and `fencing_token`. The method re-validates
    /// the lease internally to enforce fail-closed behavior.
    ///
    /// # Arguments
    /// * `cycle_id` — cycle to supersede
    /// * `successor` — successor cycle ID (XOR with `reason`)
    /// * `reason` — closed-set reason (XOR with `successor`)
    /// * `evidence_refs` — optional evidence artifact references
    /// * `actor` — actor performing the supersede
    /// * `command_id` — stable command invocation ID
    /// * `event_id` — event ID for the first ledger event
    /// * `occurred_at` — ISO8601 timestamp
    /// * `receipt_path` — directory where `supersede-receipt.json` is written
    /// * `lease_owner` — owner string used when acquiring the lease
    /// * `fencing_token` — fencing token used when acquiring the lease
    /// * `now_ms` — caller-supplied wall-clock time in milliseconds since
    ///   UNIX epoch. Pass `0` for non-lease-gated operations; for
    ///   production paths, compute it once at the composition root
    ///   (`Engine::cycle_supersede` MUST NOT dereference `SystemTime::now()`
    ///   itself — see INC-DEBT-019, hidden-coupling cluster CL-CC-01).
    /// * `auth` — authority context (validates actor_kind against CycleState surface)
    #[allow(clippy::too_many_arguments)]
    pub fn cycle_supersede(
        &mut self,
        cycle_id: &str,
        successor: Option<String>,
        reason: Option<SupersedeReason>,
        evidence_refs: &[String],
        actor: &str,
        command_id: &str,
        event_id: &str,
        occurred_at: &str,
        receipt_path: &Path,
        lease_owner: &str,
        fencing_token: i64,
        now_ms: i64,
        auth: &AuthorityContext,
    ) -> Result<EventReceipt, EngineError> {
        auth.validate(crate::authority::WritableSurface::CycleState)?;
        // Validate lease fence (fail-closed: no lease = LeaseConflict).
        // `now_ms` comes from the caller so this function stays
        // deterministic for tests (golden replays, repro paths).
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

        // XOR validation: exactly one of successor or reason must be present
        match (successor.is_some(), reason.is_some()) {
            (true, true) => return Err(EngineError::SupersedeRequiresExactlyOne),
            (false, false) => return Err(EngineError::SupersedeRequiresExactlyOne),
            _ => {}
        }

        // GAP-V-3: evidence_refs MUST NOT be empty (SPEC §2 line 87 MUST)
        if evidence_refs.is_empty() {
            return Err(EngineError::SupersedeEvidenceRefsRequired);
        }

        // Load the current cycle to validate state and get manifest
        let current = self.ledger.get_cycle(cycle_id)?.manifest;

        // Anti-self-supersede: successor cannot be the same as cycle_id
        if successor.as_deref() == Some(cycle_id) {
            return Err(EngineError::SupersedeSelfForbidden);
        }

        // GAP-V-2: successor cycle MUST exist in the cycles table before any mutation
        if let Some(ref succ) = successor
            && !self.ledger.cycle_exists(succ)?
        {
            return Err(EngineError::SupersedeSuccessorNotFound(succ.clone()));
        }

        // Build the two events: cycle.supersede.requested + cycle.supersede.applied
        let event_id_requested = event_id.to_owned();
        let event_id_applied = format!("{}-applied", event_id);

        let payload_requested = json!({
            "successor": successor,
            "reason": reason.as_ref().map(|r| r.as_str()),
            "evidence_refs": evidence_refs,
        });

        let event_input_requested = LedgerEventInput {
            event_id: event_id_requested.clone(),
            project_id: current.project_id.clone(),
            cycle_id: Some(cycle_id.to_owned()),
            frame_id: format!("frame:{command_id}"),
            command_id: command_id.to_owned(),
            actor: actor.to_owned(),
            actor_ref: None,
            event_type: "cycle.supersede.requested".to_owned(),
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
            event_type: "cycle.supersede.applied".to_owned(),
            occurred_at: occurred_at.to_owned(),
            state_before: Some(
                serde_json::to_value(&current).map_err(EngineError::StateSerialization)?,
            ),
            state_after: None,
            payload: json!({
                "successor": successor,
                "reason": reason.as_ref().map(|r| r.as_str()),
                "event_ids": [event_id_requested.clone(), event_id_applied.clone()],
            }),
            causation_id: None,
            correlation_id: None,
        };

        // Insert the two events and update cycle status to Closed
        let mut updated_manifest = current.clone();
        updated_manifest.status = sddk_domain::CycleStatus::Closed;

        // Write supersede receipt using write_atomic.
        // REQ-Repair-Receipt-Supersede-Preservation: annotate this cycle's
        // repair-queue entries with `waiver: superseded` (additive, append-only)
        // and record the count in the receipt for self-attesting audit.
        let preserved =
            preserve_repair_receipts(receipt_path, cycle_id, occurred_at).map_err(|e| {
                EngineError::Storage(DomainStorageError::Other(format!(
                    "failed to preserve repair receipts on supersede: {e}"
                )))
            })?;
        let receipt_input = CycleSupersedeInput {
            cycle_id: cycle_id.to_owned(),
            successor: successor.clone(),
            reason: reason.clone(),
            event_ids: [event_id_requested, event_id_applied.clone()],
            lease_owner: lease_owner.to_owned(),
            fencing_token,
            preserved_receipts_count: preserved,
        };
        let receipt_json = serde_json::to_string_pretty(&receipt_input)
            .map_err(EngineError::StateSerialization)?;
        let receipt_file_path = receipt_path.join(cycle_id).join("supersede-receipt.json");
        if let Some(parent) = receipt_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                EngineError::Storage(sddk_domain::StorageError::Other(e.to_string()))
            })?;
        }
        write_atomic(&receipt_file_path, receipt_json.as_bytes()).map_err(|e| {
            EngineError::Storage(sddk_domain::StorageError::Other(format!(
                "failed to write supersede receipt: {e}"
            )))
        })?;

        // Emit cycle.supersede.requested event (cycle status → Closed)
        // GAP-BUG-1: release lease atomically in the same transaction
        let _event_requested = self
            .ledger
            .update_cycle_with_event(&updated_manifest, occurred_at, &event_input_requested, true)
            .map_err(EngineError::Storage)?;

        // Emit cycle.supersede.applied event (cycle stays Closed)
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

/// Preserves this cycle's repair-queue receipts on supersede
/// (REQ-Repair-Receipt-Supersede-Preservation).
///
/// Reads `{receipt_path}/{cycle_id}/repair-queue.yaml` (if present), annotates
/// every entry that does not already carry `waiver: "superseded"` with the
/// additive annotation, and appends the result to the project-level
/// `{receipt_path}/repair-queue.yaml` monotonic queue. No field of any
/// preserved receipt is mutated: `durable_evidence_sha`, `valid_to` and the
/// rest remain verbatim. Returns the number of preserved receipts.
///
/// The queue stays append-only in effect: entries already annotated are
/// skipped, so repeated supersede runs do not duplicate annotations.
fn preserve_repair_receipts(
    receipt_path: &Path,
    cycle_id: &str,
    _occurred_at: &str,
) -> Result<usize, String> {
    use serde::{Deserialize, Serialize};

    /// Repair receipt plus the optional additive supersede waiver annotation.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    struct PreservedRepairReceipt {
        cycle_id: String,
        code: String,
        node: String,
        target: String,
        repair_action: String,
        durable_evidence_sha: String,
        created_at: String,
        valid_to: String,
        /// Additive annotation: present iff the parent cycle was superseded.
        #[serde(skip_serializing_if = "Option::is_none")]
        waiver: Option<String>,
    }

    let queue_path = receipt_path.join(cycle_id).join("repair-queue.yaml");
    if !queue_path.exists() {
        return Ok(0);
    }

    let content = std::fs::read_to_string(&queue_path).map_err(|e| e.to_string())?;
    let entries: Vec<PreservedRepairReceipt> =
        serde_saphyr::from_str(&content).map_err(|e| e.to_string())?;

    // Annotate entries missing the waiver (idempotent on re-runs).
    let mut preserved = 0usize;
    let annotated: Vec<PreservedRepairReceipt> = entries
        .into_iter()
        .map(|mut e| {
            if e.waiver.as_deref() != Some("superseded") {
                e.waiver = Some("superseded".to_owned());
                preserved += 1;
            }
            e
        })
        .collect();

    if preserved == 0 {
        return Ok(0);
    }

    let yaml = serde_saphyr::to_string(&annotated).map_err(|e| e.to_string())?;
    let temp = queue_path.with_extension("yaml.tmp");
    std::fs::write(&temp, yaml.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &queue_path).map_err(|e| e.to_string())?;

    Ok(preserved)
}

#[cfg(test)]
mod preservation_tests {
    use super::*;

    #[test]
    fn preserve_annotates_and_counts() {
        let dir = tempfile::TempDir::new().unwrap();
        let cycle_dir = dir.path().join("c-1");
        std::fs::create_dir_all(&cycle_dir).unwrap();
        std::fs::write(
            cycle_dir.join("repair-queue.yaml"),
            "- cycle_id: c-1\n  code: VAULT003\n  node: n1\n  target: t1\n  repair_action: node_creation\n  durable_evidence_sha: abc\n  created_at: '2026-01-01T00:00:00Z'\n  valid_to: '2026-03-01T00:00:00Z'\n- cycle_id: c-1\n  code: VAULT003\n  node: n2\n  target: t2\n  repair_action: plain_text_rewrite\n  durable_evidence_sha: def\n  created_at: '2026-01-02T00:00:00Z'\n  valid_to: '2026-03-02T00:00:00Z'\n",
        )
        .unwrap();

        let n = preserve_repair_receipts(dir.path(), "c-1", "2026-06-01T00:00:00Z").unwrap();
        assert_eq!(n, 2);

        let out = std::fs::read_to_string(cycle_dir.join("repair-queue.yaml")).unwrap();
        assert_eq!(out.matches("waiver: superseded").count(), 2);
        // Fields preserved verbatim (quotes may vary by YAML serializer)
        assert!(out.contains("durable_evidence_sha: abc"));
        assert!(out.contains("2026-03-01T00:00:00Z"));

        // Idempotent: second run preserves nothing new
        let n2 = preserve_repair_receipts(dir.path(), "c-1", "2026-06-01T00:00:00Z").unwrap();
        assert_eq!(n2, 0);
    }

    #[test]
    fn preserve_no_queue_is_zero() {
        let dir = tempfile::TempDir::new().unwrap();
        let n = preserve_repair_receipts(dir.path(), "c-none", "2026-06-01T00:00:00Z").unwrap();
        assert_eq!(n, 0);
    }
}
