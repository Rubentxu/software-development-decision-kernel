//! Event correlation wiring tests (AC-EVT-LEDGER-03).
//!
//! Verifies that all 9 emit sites populate causation_id and correlation_id
//! on the resulting EventEnvelopeV1 when the input provides these values.

use sddk_domain::ActorKind;
use sddk_domain::EventStore;
use sddk_engine::TransitionOutcome;
use sddk_engine::event_bus::emit::{
    ApprovalDecisionInput, ApprovalRequestedInput, OutcomeEventInput, PhaseEventInput,
    WorkflowNodeEventInput, WorkflowRunEventInput, emit_approval_decision, emit_approval_requested,
    emit_outcome_event, emit_phase_event, emit_workflow_node_completed, emit_workflow_node_failed,
    emit_workflow_node_running, emit_workflow_run_completed, emit_workflow_run_started,
};
use sddk_storage::SqliteEventStore;

// =============================================================================
// Emit site 1: emit_phase_event — phase exited
// =============================================================================

fn make_phase_input() -> PhaseEventInput {
    PhaseEventInput {
        project_id: "p-test".into(),
        cycle_id: "c-phase-1".into(),
        from_phase: "explore".into(),
        to_phase: "build".into(),
        transition_at: "2026-09-01T10:00:00Z".into(),
        actor_id: "user:test".into(),
        actor_kind: ActorKind::Human,
        event_id_prefix: "ph".into(),
        causation_id: Some("causation-predecessor-1".into()),
        correlation_id: Some("correlation-frame-1".into()),
    }
}

#[test]
fn emit_phase_event_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_phase_input();
    let result = emit_phase_event(&mut store, &input);
    assert!(result.is_ok());
    let (exited, _entered) = result.unwrap();

    // Load the exited event by event_id
    let exited_env = store
        .load_by_event_id(&exited.event_id)
        .unwrap()
        .expect("exited event should exist");
    assert_eq!(
        exited_env.causation_id.as_deref(),
        Some("causation-predecessor-1"),
        "causation_id must be populated on phase exited event"
    );
    assert_eq!(
        exited_env.correlation_id.as_deref(),
        Some("correlation-frame-1"),
        "correlation_id must be populated on phase exited event"
    );
}

// =============================================================================
// Emit site 2: emit_outcome_event
// =============================================================================

fn make_outcome_input() -> OutcomeEventInput {
    OutcomeEventInput {
        project_id: "p-test".into(),
        cycle_id: "c-outcome-1".into(),
        transition_id: "tr-1".into(),
        from_phase: Some("build".into()),
        to_phase: Some("test".into()),
        transition_at: "2026-09-01T11:00:00Z".into(),
        actor_id: "user:test".into(),
        actor_kind: ActorKind::Human,
        event_id_prefix: "tr".into(),
        failed_gates: vec![],
        causation_id: Some("causation-outcome-1".into()),
        correlation_id: Some("correlation-outcome-1".into()),
    }
}

#[test]
fn emit_outcome_event_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_outcome_input();
    let result = emit_outcome_event(&mut store, &input, TransitionOutcome::Succeeded);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("c-outcome-1", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-outcome-1"),
        "causation_id must be populated on outcome event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-outcome-1"),
        "correlation_id must be populated on outcome event"
    );
}

// =============================================================================
// Emit site 3: emit_approval_requested
// =============================================================================

fn make_approval_requested_input() -> ApprovalRequestedInput {
    ApprovalRequestedInput {
        project_id: "p-approval-req".into(),
        cycle_id: "c-approval-1".into(),
        capability: "git.delete_branch".into(),
        request_hash: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
            .into(),
        expires_at: "2026-09-15T18:00:00Z".into(),
        occurred_at: "2026-09-01T12:00:00Z".into(),
        actor_id: "agent:orchestrator".into(),
        actor_kind: ActorKind::Agent,
        causation_id: Some("causation-approval-req-1".into()),
        correlation_id: Some("correlation-approval-req-1".into()),
    }
}

#[test]
fn emit_approval_requested_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_approval_requested_input();
    let result = emit_approval_requested(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("c-approval-1", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-approval-req-1"),
        "causation_id must be populated on approval requested event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-approval-req-1"),
        "correlation_id must be populated on approval requested event"
    );
}

// =============================================================================
// Emit site 4: emit_approval_decision
// =============================================================================

fn make_approval_decision_input() -> ApprovalDecisionInput {
    ApprovalDecisionInput {
        project_id: "p-approval-dec".into(),
        cycle_id: "c-approval-1".into(),
        capability: "git.delete_branch".into(),
        request_hash: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
            .into(),
        decision: sddk_domain::ApprovalDecision::Granted,
        actor_id: "user:alice".into(),
        actor_kind: ActorKind::Human,
        reason: "approved for testing".into(),
        occurred_at: "2026-09-01T14:00:00Z".into(),
        causation_id: Some("causation-approval-dec-1".into()),
        correlation_id: Some("correlation-approval-dec-1".into()),
    }
}

#[test]
fn emit_approval_decision_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_approval_decision_input();
    let result = emit_approval_decision(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("c-approval-1", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-approval-dec-1"),
        "causation_id must be populated on approval decision event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-approval-dec-1"),
        "correlation_id must be populated on approval decision event"
    );
}

// =============================================================================
// Emit site 5: emit_workflow_run_started
// =============================================================================

fn make_workflow_run_input() -> WorkflowRunEventInput {
    WorkflowRunEventInput {
        project_id: "p-wf".into(),
        run_id: "run-001".into(),
        occurred_at: "2026-09-01T15:00:00Z".into(),
        actor_id: "workflow-runtime".into(),
        actor_kind: ActorKind::System,
        causation_id: Some("causation-wf-run-1".into()),
        correlation_id: Some("correlation-wf-run-1".into()),
    }
}

#[test]
fn emit_workflow_run_started_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_workflow_run_input();
    let result = emit_workflow_run_started(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("run-001", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-wf-run-1"),
        "causation_id must be populated on workflow run started event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-wf-run-1"),
        "correlation_id must be populated on workflow run started event"
    );
}

// =============================================================================
// Emit site 6: emit_workflow_run_completed
// =============================================================================

#[test]
fn emit_workflow_run_completed_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_workflow_run_input();
    let result = emit_workflow_run_completed(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("run-001", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-wf-run-1"),
        "causation_id must be populated on workflow run completed event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-wf-run-1"),
        "correlation_id must be populated on workflow run completed event"
    );
}

// =============================================================================
// Emit site 7: emit_workflow_node_running
// =============================================================================

fn make_workflow_node_input() -> WorkflowNodeEventInput {
    WorkflowNodeEventInput {
        project_id: "p-wf".into(),
        run_id: "run-001".into(),
        node_id: "node-001".into(),
        occurred_at: "2026-09-01T16:00:00Z".into(),
        actor_id: "workflow-runtime".into(),
        actor_kind: ActorKind::System,
        reason: None,
        causation_id: Some("causation-wf-node-1".into()),
        correlation_id: Some("correlation-wf-node-1".into()),
    }
}

#[test]
fn emit_workflow_node_running_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_workflow_node_input();
    let result = emit_workflow_node_running(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("run-001", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-wf-node-1"),
        "causation_id must be populated on workflow node running event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-wf-node-1"),
        "correlation_id must be populated on workflow node running event"
    );
}

// =============================================================================
// Emit site 8: emit_workflow_node_completed
// =============================================================================

#[test]
fn emit_workflow_node_completed_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_workflow_node_input();
    let result = emit_workflow_node_completed(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("run-001", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-wf-node-1"),
        "causation_id must be populated on workflow node completed event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-wf-node-1"),
        "correlation_id must be populated on workflow node completed event"
    );
}

// =============================================================================
// Emit site 9: emit_workflow_node_failed
// =============================================================================

#[test]
fn emit_workflow_node_failed_wires_causation_and_correlation() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let mut input = make_workflow_node_input();
    input.reason = Some("node failed intentionally".into());
    let result = emit_workflow_node_failed(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let events = store.load_stream("run-001", None, 10).unwrap();
    let env = events.first().unwrap();
    assert_eq!(
        env.causation_id.as_deref(),
        Some("causation-wf-node-1"),
        "causation_id must be populated on workflow node failed event"
    );
    assert_eq!(
        env.correlation_id.as_deref(),
        Some("correlation-wf-node-1"),
        "correlation_id must be populated on workflow node failed event"
    );
}

// =============================================================================
// Idempotency: helpers are no-op when fields are already set
// =============================================================================

#[test]
fn emit_phase_event_helpers_are_idempotent_when_preset() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let mut input = make_phase_input();
    input.correlation_id = Some("preset-correlation".into());
    input.causation_id = Some("preset-causation".into());

    let result = emit_phase_event(&mut store, &input);
    assert!(result.is_ok());
    let (exited, _entered) = result.unwrap();

    let exited_env = store
        .load_by_event_id(&exited.event_id)
        .unwrap()
        .expect("exited event should exist");
    assert_eq!(
        exited_env.correlation_id.as_deref(),
        Some("preset-correlation"),
        "correlation_id should remain preset (additive/idempotent)"
    );
    assert_eq!(
        exited_env.causation_id.as_deref(),
        Some("preset-causation"),
        "causation_id should remain preset (additive/idempotent)"
    );
}
