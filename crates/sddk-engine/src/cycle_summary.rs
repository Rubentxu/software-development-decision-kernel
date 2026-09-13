//! Derived cycle runtime summary (WU-C3 cutover, DELTA-CONF-004).
//!
//! Since the cycle/run lifecycle cutover, runtime-derived states
//! (approval-wait, UAT-wait, remediation, recovery) are NOT canonical
//! Cycle truth: the Cycle record stays at its last delivery-level status
//! and the runtime detail is derived from durable facts on the ledger —
//!
//! - approval waits: `approval.capability.requested` events without a
//!   matching `approval.capability.granted`/`denied` decision (same fact
//!   family as the `ApprovalProjection` in the CLI);
//! - UAT waits: a succeeded `phase.verify.uat.sync` transition
//!   (legacy `UAT_WAITING` entry) without a later succeeded
//!   `phase.uat.complete`;
//! - remediation/recovery: failed `cycle.transitioned` outcomes not yet
//!   closed by a succeeded remediation transition (`phase.verify.remediate`
//!   / `phase.build.remediate`). Recovery rounds (`release.recover`) fold
//!   into the same counter — both are failure-closure work.
//!
//! Everything here is READ-ONLY derivation over the `Ledger` port. No
//! function in this module writes a cycle row or constructs a
//! runtime-derived `CycleStatus` value (decode-only since C3).

use std::collections::BTreeSet;

use sddk_domain::{CycleStatus, Ledger, StorageError};

/// Derived runtime state of one cycle, computed from durable facts.
///
/// This is a projection summary, never persisted on the Cycle record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CycleRuntimeSummary {
    /// Persisted delivery-level status of the Cycle record.
    pub persisted_status: CycleStatus,
    /// True while at least one approval request awaits a decision.
    pub approval_waiting: bool,
    /// Capabilities with an unresolved approval request, sorted.
    pub approval_waiting_on: Vec<String>,
    /// True while a UAT sync has succeeded without a later completion.
    pub uat_waiting: bool,
    /// True while the latest failure has no remediation completion after it.
    pub remediating: bool,
    /// Number of failed-gate (remediation/recovery round) transitions.
    pub remediation_rounds: u32,
    /// The single label this summary derives (empty when none apply).
    ///
    /// Precedence (deterministic, no-contradiction rule):
    /// approval-waiting > uat-waiting > remediating > persisted status.
    pub derived_state: String,
}

/// Transition whose succeeded outcome starts a UAT wait.
const UAT_START_TRANSITION: &str = "phase.verify.uat.sync";
/// Transition whose succeeded outcome closes a UAT wait.
const UAT_COMPLETE_TRANSITION: &str = "phase.uat.complete";
/// Transitions whose succeeded outcome closes an open remediation.
const REMEDIATION_DONE_TRANSITIONS: [&str; 2] = ["phase.verify.remediate", "phase.build.remediate"];

/// Derives the runtime summary for one cycle from ledger facts.
///
/// Read-only: walks the cycle's ledger events; never touches cycle rows.
pub fn derive_cycle_summary(
    ledger: &dyn Ledger,
    cycle_id: &str,
) -> Result<CycleRuntimeSummary, StorageError> {
    let record = ledger.get_cycle(cycle_id)?;
    let events = ledger.list_cycle_events(cycle_id)?;

    // ── Approval facts ──────────────────────────────────────────────────
    // Same fact family as ApprovalProjection: a requested event whose
    // (capability, request_hash) has no later granted/denied decision.
    let mut requested: Vec<(String, String)> = Vec::new();
    let mut decided: BTreeSet<(String, String)> = BTreeSet::new();
    for event in &events {
        let Some(capability) = event.payload.get("capability").and_then(|v| v.as_str()) else {
            continue;
        };
        let hash = event
            .payload
            .get("request_hash")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        match event.event_type.as_str() {
            "approval.capability.requested" => {
                requested.push((capability.to_owned(), hash.to_owned()));
            }
            "approval.capability.granted" | "approval.capability.denied" => {
                decided.insert((capability.to_owned(), hash.to_owned()));
            }
            _ => {}
        }
    }
    let mut approval_waiting_on: Vec<String> = requested
        .iter()
        .filter(|(capability, hash)| !decided.contains(&(capability.clone(), hash.clone())))
        .map(|(capability, _)| capability.clone())
        .collect();
    approval_waiting_on.sort();
    approval_waiting_on.dedup();
    let approval_waiting = !approval_waiting_on.is_empty();

    // ── Transition outcome facts ────────────────────────────────────────
    // `cycle.transitioned` (engine apply path) and
    // `workflow.transition.succeeded`/`workflow.transition.failed`
    // (event-bus canonical events) both carry transition_id + outcome.
    let mut uat_started = false;
    let mut uat_completed = false;
    let mut remediation_open = false;
    let mut remediation_rounds: u32 = 0;
    for event in &events {
        let transition_id = event
            .payload
            .get("transition_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let outcome = event
            .payload
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let succeeded = outcome == "succeeded";
        match event.event_type.as_str() {
            "cycle.transitioned" | "workflow.transition.succeeded" if succeeded => {
                if REMEDIATION_DONE_TRANSITIONS.contains(&transition_id) {
                    remediation_open = false;
                }
                if transition_id == UAT_START_TRANSITION {
                    uat_started = true;
                }
                if transition_id == UAT_COMPLETE_TRANSITION {
                    uat_completed = true;
                }
            }
            "cycle.transitioned" | "workflow.transition.failed" if outcome == "failed" => {
                remediation_rounds += 1;
                remediation_open = true;
            }
            _ => {}
        }
    }
    let uat_waiting = uat_started && !uat_completed;

    // ── Deterministic single label (no-contradiction rule) ──────────────
    let derived_state = if approval_waiting {
        "approval-waiting".to_owned()
    } else if uat_waiting {
        "uat-waiting".to_owned()
    } else if remediation_open {
        "remediating".to_owned()
    } else {
        String::new()
    };

    Ok(CycleRuntimeSummary {
        persisted_status: record.manifest.status,
        approval_waiting,
        approval_waiting_on,
        uat_waiting,
        remediating: remediation_open,
        remediation_rounds,
        derived_state,
    })
}
