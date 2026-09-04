//! Event-correlation widening tests (AC-EVT-LEDGER-02, AC-EVT-LEDGER-03).
//!
//! Verifies that:
//! - AC-EVT-LEDGER-02: four carriers are widened to include ActorRef (additive)
//! - AC-EVT-LEDGER-03: causation_id and correlation_id are added to three carriers (additive)
//! - Legacy events without the new fields deserialize successfully (#[serde(default)])

use sddk_domain::ActorRef;
use sddk_domain::event_envelope::ActorKind;
use sddk_domain::models::gate_receipt::{
    GateOutcomeStatus, GateReceiptInput, GateReceiptNextSeqInput,
};
use sddk_domain::models::ledger::LedgerEventInput;
use serde_json::json;

// =============================================================================
// AC-EVT-LEDGER-02: Four carriers widened to ActorRef (additive)
// =============================================================================

/// LedgerEventInput accepts actor_ref = None (additive widening).
#[test]
fn ledger_event_input_actor_ref_is_optional() {
    let input = LedgerEventInput {
        event_id: "evt-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        frame_id: "frame-1".into(),
        command_id: "cmd-1".into(),
        actor: "system".into(),
        actor_ref: None, // widened: additive, defaults to None
        event_type: "workflow.phase.entered".into(),
        occurred_at: "2026-09-01T10:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: json!({}),
        causation_id: None,
        correlation_id: None,
    };
    // Must compile — Option<ActorRef> is valid
    assert!(input.actor_ref.is_none());
}

/// GateReceiptInput accepts actor_ref = None (additive widening).
#[test]
fn gate_receipt_input_actor_ref_is_optional() {
    let input = GateReceiptInput {
        receipt_id: "gate-test-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        gate: "test-gate".into(),
        evaluator: "sddk-engine".into(),
        transition_id: "trans-1".into(),
        plan_hash: "sha256:abcd1234567890".into(),
        outcome: GateOutcomeStatus::Passed,
        evidence: json!({}),
        actor: "system".into(),
        actor_ref: None, // widened: additive
        command_id: "cmd-1".into(),
        frame_id: "frame-1".into(),
        evaluated_at: "2026-09-01T10:00:00Z".into(),
        seq: 1,
        causation_id: None,
        correlation_id: None,
    };
    assert!(input.actor_ref.is_none());
}

/// GateReceiptNextSeqInput accepts actor_ref = None (additive widening).
#[test]
fn gate_receipt_next_seq_input_actor_ref_is_optional() {
    let input = GateReceiptNextSeqInput {
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        gate: "test-gate".into(),
        evaluator: "sddk-engine".into(),
        transition_id: "trans-1".into(),
        plan_hash: "sha256:abcd1234567890".into(),
        outcome: GateOutcomeStatus::Passed,
        evidence: json!({}),
        actor: "system".into(),
        actor_ref: None, // widened: additive
        command_id: "cmd-1".into(),
        frame_id: "frame-1".into(),
        evaluated_at: "2026-09-01T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
    };
    assert!(input.actor_ref.is_none());
}

/// LedgerEventInput serializes with actor_ref present (round-trip).
#[test]
fn ledger_event_input_actor_ref_roundtrips() {
    let input = LedgerEventInput {
        event_id: "evt-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        frame_id: "frame-1".into(),
        command_id: "cmd-1".into(),
        actor: "user:test".into(),
        actor_ref: Some(ActorRef {
            kind: ActorKind::Human,
            id: "user:test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        }),
        event_type: "workflow.phase.entered".into(),
        occurred_at: "2026-09-01T10:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: json!({}),
        causation_id: Some("evt-pred".into()),
        correlation_id: Some("frame-1".into()),
    };
    let bytes = serde_json::to_vec(&input).unwrap();
    let rt: LedgerEventInput = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(input, rt);
}

// =============================================================================
// AC-EVT-LEDGER-03: causation_id and correlation_id added to three carriers
// =============================================================================

/// LedgerEventInput accepts None for causation_id and correlation_id.
#[test]
fn ledger_event_input_causation_and_correlation_are_optional() {
    let input = LedgerEventInput {
        event_id: "evt-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        frame_id: "frame-1".into(),
        command_id: "cmd-1".into(),
        actor: "system".into(),
        actor_ref: None,
        event_type: "workflow.phase.entered".into(),
        occurred_at: "2026-09-01T10:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: json!({}),
        causation_id: None,
        correlation_id: None,
    };
    assert!(input.causation_id.is_none());
    assert!(input.correlation_id.is_none());
}

/// causation_id and correlation_id round-trip through serde.
#[test]
fn ledger_event_input_causation_correlation_roundtrips() {
    let input = LedgerEventInput {
        event_id: "evt-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        frame_id: "frame-1".into(),
        command_id: "cmd-1".into(),
        actor: "system".into(),
        actor_ref: None,
        event_type: "workflow.phase.entered".into(),
        occurred_at: "2026-09-01T10:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: json!({}),
        causation_id: Some("evt-caused-by-001".into()),
        correlation_id: Some("frame-session-42".into()),
    };
    let bytes = serde_json::to_vec(&input).unwrap();
    let rt: LedgerEventInput = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(input.causation_id, rt.causation_id);
    assert_eq!(input.correlation_id, rt.correlation_id);
}

/// GateReceiptInput accepts causation_id and correlation_id.
#[test]
fn gate_receipt_input_causation_and_correlation_are_optional() {
    let input = GateReceiptInput {
        receipt_id: "gate-test-1".into(),
        project_id: "p-1".into(),
        cycle_id: Some("c-1".into()),
        gate: "test-gate".into(),
        evaluator: "sddk-engine".into(),
        transition_id: "trans-1".into(),
        plan_hash: "sha256:abcd1234567890".into(),
        outcome: GateOutcomeStatus::Passed,
        evidence: json!({}),
        actor: "system".into(),
        actor_ref: None,
        command_id: "cmd-1".into(),
        frame_id: "frame-1".into(),
        evaluated_at: "2026-09-01T10:00:00Z".into(),
        seq: 1,
        causation_id: None,
        correlation_id: None,
    };
    assert!(input.causation_id.is_none());
    assert!(input.correlation_id.is_none());
}

// =============================================================================
// Legacy corpus backward-compatibility (#[serde(default)])
// =============================================================================

/// LedgerEventInput deserializes from JSON without actor_ref field.
#[test]
fn legacy_corpus_ledger_event_without_actor_ref_deserializes() {
    let json_str = r#"{
        "event_id": "evt-legacy-1",
        "project_id": "p-1",
        "cycle_id": "c-1",
        "frame_id": "frame-1",
        "command_id": "cmd-1",
        "actor": "system",
        "event_type": "workflow.phase.entered",
        "occurred_at": "2026-09-01T10:00:00Z",
        "state_before": null,
        "state_after": null,
        "payload": {}
    }"#;
    let input: LedgerEventInput = serde_json::from_str(json_str).unwrap();
    assert!(input.actor_ref.is_none());
    assert_eq!(input.event_id, "evt-legacy-1");
}

/// LedgerEventInput deserializes from JSON without causation_id / correlation_id.
#[test]
fn legacy_corpus_ledger_event_without_causation_id_deserializes() {
    let json_str = r#"{
        "event_id": "evt-legacy-2",
        "project_id": "p-1",
        "cycle_id": "c-1",
        "frame_id": "frame-1",
        "command_id": "cmd-1",
        "actor": "system",
        "event_type": "workflow.phase.entered",
        "occurred_at": "2026-09-01T10:00:00Z",
        "payload": {}
    }"#;
    let input: LedgerEventInput = serde_json::from_str(json_str).unwrap();
    assert!(input.causation_id.is_none());
    assert!(input.correlation_id.is_none());
}
