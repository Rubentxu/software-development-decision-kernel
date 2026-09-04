//! Gate evaluation receipts and status.
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event_envelope::ActorRef;

/// Error when validating pass evidence for a gate receipt.
///
/// Per REQ-IPV (spec-v2), a phase gate MUST NOT be satisfied by an
/// agent-reported PASS alone.  For a Passed outcome, the evidence MUST contain
/// ALL THREE of:
/// - `argv`: the command executed (array of strings)
/// - `exit_code`: the process exit code (integer)
/// - `output_digest`: a SHA-256 or similar digest of the output (string)
///
/// Each error variant names the specific missing evidence category.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PassEvidenceError {
    /// Evidence is not a JSON object.
    #[error("evidence must be a JSON object")]
    NotAnObject,

    /// Passed outcome is missing the argv array.
    #[error("passed outcome is missing argv")]
    MissingArgv,

    /// Passed outcome is missing the exit_code field.
    #[error("passed outcome is missing exit_code")]
    MissingExitCode,

    /// Passed outcome is missing the output_digest field.
    #[error("passed outcome is missing output_digest")]
    MissingOutputDigest,
}

/// Validates that a passed gate outcome has complete independent rerun evidence.
///
/// Per REQ-IPV (spec-v2 cycle-44), a phase gate MUST NOT be satisfied by an
/// agent-reported PASS alone.  When the outcome is `Passed`, the evidence MUST
/// contain ALL THREE of:
/// - `argv`: the command executed (array of strings)
/// - `exit_code`: the process exit code (integer)
/// - `output_digest`: a SHA-256 or similar digest of the output (string)
///
/// This function validates the evidence structure only — the caller is
/// responsible for checking that the outcome is `Passed` before calling.
pub fn validate_pass_evidence(evidence: &Value) -> Result<(), PassEvidenceError> {
    let obj = evidence.as_object().ok_or(PassEvidenceError::NotAnObject)?;

    // For Passed outcome, all three fields are required (per REQ-IPV spec-v2)
    if !obj.contains_key("argv") {
        return Err(PassEvidenceError::MissingArgv);
    }
    if !obj.contains_key("exit_code") {
        return Err(PassEvidenceError::MissingExitCode);
    }
    if !obj.contains_key("output_digest") {
        return Err(PassEvidenceError::MissingOutputDigest);
    }
    Ok(())
}

/// Outcome recorded by an authorized gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateOutcomeStatus {
    Passed,
    Failed,
    Waived,
}

/// Data required to persist one authorized gate receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptInput {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    /// Deprecated: use `actor_ref` instead. Preserved for legacy corpus replay.
    pub actor: String,
    /// Canonical actor provenance (per ADR-069 §5 and ADR-071 §5).
    pub actor_ref: Option<ActorRef>,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
    pub seq: i64,
    /// Causation chain: event_id of the immediate predecessor in the same stream.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub causation_id: Option<String>,
    /// Correlation group: frame_id of the command that triggered this receipt.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub correlation_id: Option<String>,
}

/// Data required to persist one authorized gate receipt, with atomic seq allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptNextSeqInput {
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    /// Deprecated: use `actor_ref` instead. Preserved for legacy corpus replay.
    pub actor: String,
    /// Canonical actor provenance (per ADR-069 §5 and ADR-071 §5).
    pub actor_ref: Option<ActorRef>,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
    /// Causation chain: event_id of the immediate predecessor in the same stream.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub causation_id: Option<String>,
    /// Correlation group: frame_id of the command that triggered this receipt.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub correlation_id: Option<String>,
}

/// An authorized, persisted gate evaluation receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceipt {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    /// Deprecated: use `actor_ref` instead. Preserved for legacy corpus replay.
    pub actor: String,
    /// Canonical actor provenance (per ADR-069 §5 and ADR-071 §5).
    pub actor_ref: Option<ActorRef>,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
    pub seq: i64,
    /// Causation chain: event_id of the immediate predecessor in the same stream.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub causation_id: Option<String>,
    /// Correlation group: frame_id of the command that triggered this receipt.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    pub correlation_id: Option<String>,
}
