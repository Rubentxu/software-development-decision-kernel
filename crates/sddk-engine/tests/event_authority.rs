//! Authority tests for event emission (emit_approval_requested and emit_approval_decision).
//!
//! Validates ADR-069 §4: Only Agent may emit `ApprovalRequested`; only Human or System
//! may emit `ApprovalDecision`. Agent attempting to emit ApprovalDecision and vice versa
//! must return `StorageError::Other` fail-closed.

use sddk_domain::{
    ActorKind, ApprovalDecision, EventAppended, EventEnvelopeV1, StorageError as DomainStorageError,
};
use sddk_engine::{
    authority::{AuthorityContext, WritableSurface, infer_actor_kind},
    event_bus::{
        ApprovalDecisionInput, ApprovalRequestedInput, emit_approval_decision,
        emit_approval_requested,
    },
};

// Minimal in-memory store wrapper — only implements the methods needed by the emit functions.
struct TestStore {
    events: Vec<EventEnvelopeV1>,
}

impl TestStore {
    fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl sddk_domain::EventStore for TestStore {
    fn append(&mut self, envelope: &EventEnvelopeV1) -> Result<EventAppended, DomainStorageError> {
        self.events.push(envelope.clone());
        Ok(EventAppended {
            event_id: "test-event".into(),
            stream_id: "test-stream".into(),
            sequence: 1,
            content_hash: envelope.content_hash.clone(),
            recorded_at: "2026-08-04T10:00:00Z".into(),
            chain_hash: "testchainhash".into(),
        })
    }

    fn load_by_event_id(
        &self,
        _event_id: &str,
    ) -> Result<Option<EventEnvelopeV1>, DomainStorageError> {
        Ok(None)
    }

    fn load_stream(
        &self,
        _stream_id: &str,
        _after_sequence: Option<u64>,
        _limit: u32,
    ) -> Result<Vec<EventEnvelopeV1>, DomainStorageError> {
        Ok(Vec::new())
    }

    fn last_sequence(&self, _stream_id: &str) -> Result<Option<u64>, DomainStorageError> {
        Ok(Some(0))
    }

    fn count(&self) -> Result<u64, DomainStorageError> {
        Ok(0)
    }

    fn head_hash(&self, _stream_id: &str) -> Result<Option<String>, DomainStorageError> {
        Ok(None)
    }

    fn head_chain_hash(&self, _stream_id: &str) -> Result<Option<String>, DomainStorageError> {
        Ok(None)
    }

    fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), DomainStorageError> {
        Ok(())
    }

    fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), DomainStorageError> {
        Ok(())
    }

    fn backfill_chain_hash(&mut self, _stream_id: &str) -> Result<usize, DomainStorageError> {
        Ok(0)
    }

    fn load_by_sequence(
        &self,
        _stream_id: &str,
        _seq: u64,
    ) -> Result<Option<EventEnvelopeV1>, DomainStorageError> {
        Ok(None)
    }
}

fn make_request_input(actor_kind: ActorKind) -> ApprovalRequestedInput {
    ApprovalRequestedInput {
        project_id: "project-1".into(),
        cycle_id: "cycle-1".into(),
        capability: "test.capability".into(),
        request_hash: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1".into(),
        expires_at: "2026-12-31T00:00:00Z".into(),
        occurred_at: "2026-08-04T10:00:00Z".into(),
        actor_id: match actor_kind {
            ActorKind::Agent => "agent:test".into(),
            ActorKind::Human => "user:test".into(),
            ActorKind::System => "sddk-cli".into(),
        },
        actor_kind,
        causation_id: None,
        correlation_id: None,
    }
}

fn make_decision_input(actor_kind: ActorKind) -> ApprovalDecisionInput {
    ApprovalDecisionInput {
        project_id: "project-1".into(),
        cycle_id: "cycle-1".into(),
        capability: "test.capability".into(),
        request_hash: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1".into(),
        decision: ApprovalDecision::Granted,
        actor_id: match actor_kind {
            ActorKind::Agent => "agent:test".into(),
            ActorKind::Human => "user:test".into(),
            ActorKind::System => "sddk-cli".into(),
        },
        actor_kind,
        reason: "Test approval reason".into(),
        occurred_at: "2026-08-04T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
    }
}

// ── emit_approval_requested authority tests ────────────────────────────────────

#[test]
fn emit_approval_requested_accepts_agent() {
    // AC-AUTH-EVT-01: Agent may emit ApprovalRequested (ADR-069 §4).
    let mut store = TestStore::new();
    let input = make_request_input(ActorKind::Agent);
    let result = emit_approval_requested(&mut store, &input);
    assert!(
        result.is_ok(),
        "Agent should be allowed to emit ApprovalRequested, got: {:?}",
        result
    );
}

#[test]
fn emit_approval_requested_rejects_human() {
    // Agent-only: Human must be rejected fail-closed.
    let mut store = TestStore::new();
    let input = make_request_input(ActorKind::Human);
    let result = emit_approval_requested(&mut store, &input);
    let err = result.expect_err("Human should be rejected for ApprovalRequested");
    assert!(
        matches!(err, DomainStorageError::Other(ref msg) if msg.contains("requires actor_kind Agent")),
        "Expected StorageError::Other with 'requires actor_kind Agent', got: {:?}",
        err
    );
}

#[test]
fn emit_approval_requested_rejects_system() {
    // Agent-only: System must be rejected fail-closed.
    let mut store = TestStore::new();
    let input = make_request_input(ActorKind::System);
    let result = emit_approval_requested(&mut store, &input);
    let err = result.expect_err("System should be rejected for ApprovalRequested");
    assert!(
        matches!(err, DomainStorageError::Other(ref msg) if msg.contains("requires actor_kind Agent")),
        "Expected StorageError::Other with 'requires actor_kind Agent', got: {:?}",
        err
    );
}

// ── emit_approval_decision authority tests ────────────────────────────────────

#[test]
fn emit_approval_decision_accepts_human() {
    // AC-AUTH-EVT-03: Human may emit ApprovalDecision (ADR-069 §4).
    let mut store = TestStore::new();
    let input = make_decision_input(ActorKind::Human);
    let result = emit_approval_decision(&mut store, &input);
    assert!(
        result.is_ok(),
        "Human should be allowed to emit ApprovalDecision, got: {:?}",
        result
    );
}

#[test]
fn emit_approval_decision_accepts_system() {
    // AC-AUTH-EVT-04: System may emit ApprovalDecision (ADR-069 §4).
    let mut store = TestStore::new();
    let input = make_decision_input(ActorKind::System);
    let result = emit_approval_decision(&mut store, &input);
    assert!(
        result.is_ok(),
        "System should be allowed to emit ApprovalDecision, got: {:?}",
        result
    );
}

#[test]
fn emit_approval_decision_rejects_agent() {
    // Human/System-only: Agent must be rejected fail-closed.
    let mut store = TestStore::new();
    let input = make_decision_input(ActorKind::Agent);
    let result = emit_approval_decision(&mut store, &input);
    let err = result.expect_err("Agent should be rejected for ApprovalDecision");
    assert!(
        matches!(err, DomainStorageError::Other(ref msg) if msg.contains("requires actor_kind Human or System")),
        "Expected StorageError::Other with 'requires actor_kind Human or System', got: {:?}",
        err
    );
}

// ── AuthorityContext infer tests ────────────────────────────────────────────────

#[test]
fn infer_actor_kind_user_prefix() {
    assert_eq!(infer_actor_kind("user:alice"), ActorKind::Human);
    assert_eq!(infer_actor_kind("user:bob"), ActorKind::Human);
}

#[test]
fn infer_actor_kind_agent_prefix() {
    assert_eq!(infer_actor_kind("agent:orchestrator-1"), ActorKind::Agent);
    assert_eq!(infer_actor_kind("agent:test-agent"), ActorKind::Agent);
}

#[test]
fn infer_actor_kind_system_fallback() {
    assert_eq!(infer_actor_kind("sddk-cli"), ActorKind::System);
    assert_eq!(infer_actor_kind("daemon-1"), ActorKind::System);
    assert_eq!(infer_actor_kind("unknown-actor"), ActorKind::System);
}

// ── WritableSurface matrix coverage ─────────────────────────────────────────────

#[test]
fn writable_surface_cycle_state_allows_all_actor_kinds() {
    // CycleState admits Human, Agent, and System (per WRITABLE_SURFACE_MATRIX).
    let auth_human = AuthorityContext::for_test(ActorKind::Human, "user:test");
    assert!(
        auth_human.validate(WritableSurface::CycleState).is_ok(),
        "Human on CycleState"
    );

    let auth_agent = AuthorityContext::for_test(ActorKind::Agent, "agent:test");
    assert!(
        auth_agent.validate(WritableSurface::CycleState).is_ok(),
        "Agent on CycleState"
    );

    let auth_system = AuthorityContext::for_test(ActorKind::System, "sddk-cli");
    assert!(
        auth_system.validate(WritableSurface::CycleState).is_ok(),
        "System on CycleState"
    );
}

#[test]
fn writable_surface_plan_revisions_allows_human_and_agent() {
    // PlanRevisions admits Human and Agent (per WRITABLE_SURFACE_MATRIX).
    let auth_human = AuthorityContext::for_test(ActorKind::Human, "user:test");
    assert!(
        auth_human.validate(WritableSurface::PlanRevisions).is_ok(),
        "Human on PlanRevisions"
    );

    let auth_agent = AuthorityContext::for_test(ActorKind::Agent, "agent:test");
    assert!(
        auth_agent.validate(WritableSurface::PlanRevisions).is_ok(),
        "Agent on PlanRevisions"
    );

    let auth_system = AuthorityContext::for_test(ActorKind::System, "sddk-cli");
    assert!(
        auth_system
            .validate(WritableSurface::PlanRevisions)
            .is_err(),
        "System on PlanRevisions"
    );
}

#[test]
fn writable_surface_gate_receipts_rejects_human_and_agent() {
    // GateReceipts admits only System (per WRITABLE_SURFACE_MATRIX).
    let auth_human = AuthorityContext::for_test(ActorKind::Human, "user:test");
    assert!(
        auth_human.validate(WritableSurface::GateReceipts).is_err(),
        "Human on GateReceipts"
    );

    let auth_agent = AuthorityContext::for_test(ActorKind::Agent, "agent:test");
    assert!(
        auth_agent.validate(WritableSurface::GateReceipts).is_err(),
        "Agent on GateReceipts"
    );

    let auth_system = AuthorityContext::for_test(ActorKind::System, "sddk-cli");
    assert!(
        auth_system.validate(WritableSurface::GateReceipts).is_ok(),
        "System on GateReceipts"
    );
}
