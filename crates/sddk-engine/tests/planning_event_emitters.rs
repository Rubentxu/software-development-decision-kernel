//! Tests for the five planning CRUD event emitters (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-13 (FIND-PLN-005 closure):
//! - Each emitter produces the correct event Type::Custom name
//! - schema_version = 1
//! - Correct payload serialization
//! - Events are NOT in std_registry (ADR-071 compliance)

use sddk_domain::event_registry::schemas::std_registry;
use sddk_domain::{
    ActorKind, ActorRef, CasHash, DecisionId, DecisionKind, DependencyEdgeKind, EventAppended,
    EventEnvelopeV1, EventStore, EvidenceId, PlanningEvidenceKind, WorkItemId, WorkItemStatus,
};
use sddk_engine::authority::AuthorityContext;
use sddk_engine::event_bus::emit::{
    DecisionRecordedInput, DependencyAddedInput, EvidenceAttachedInput, WorkItemCreatedInput,
    WorkItemTransitionedInput, emit_decision_recorded, emit_dependency_added,
    emit_evidence_attached, emit_work_item_created, emit_work_item_transitioned,
};
use sddk_storage::SqliteEventStore;

// ── Test store wrapper ────────────────────────────────────────────────────────

fn make_test_store() -> SqliteEventStore {
    SqliteEventStore::open_in_memory().expect("in-memory store must open")
}

// ── emit_work_item_created ───────────────────────────────────────────────────

fn make_wi_created_input() -> WorkItemCreatedInput {
    WorkItemCreatedInput {
        project_id: "p-test".into(),
        cycle_id: "c-ledger-1".into(),
        work_item_id: "wi-test-001".into(),
        title: "Implement feature X".into(),
        description: "Detailed description of feature X".into(),
        status: "Draft".into(),
        actor_id: "agent:test".into(),
        actor_kind: ActorKind::Agent,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Agent, "agent:test"),
    }
}

#[test]
fn emit_work_item_created_produces_correct_event_type() {
    let mut store = make_test_store();
    let input = make_wi_created_input();
    let result = emit_work_item_created(&mut store, &input);
    assert!(result.is_ok(), "emit must succeed");
    let appended = result.unwrap();

    let env = store
        .load_by_event_id(&appended.event_id)
        .expect("store must not error")
        .expect("event must be stored");
    assert_eq!(
        env.event_type, "planning.work_item.created",
        "event_type must be planning.work_item.created"
    );
}

#[test]
fn emit_work_item_created_has_schema_version_one() {
    let mut store = make_test_store();
    let input = make_wi_created_input();
    let result = emit_work_item_created(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(
        env.schema_version, 1,
        "schema_version must be 1 per ADR-071"
    );
}

#[test]
fn emit_work_item_created_payload_contains_work_item_fields() {
    let mut store = make_test_store();
    let input = make_wi_created_input();
    let result = emit_work_item_created(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let payload = env.payload;
    assert_eq!(
        payload.get("work_item_id").and_then(|v| v.as_str()),
        Some("wi-test-001"),
        "payload must contain work_item_id"
    );
    assert_eq!(
        payload.get("cycle_id").and_then(|v| v.as_str()),
        Some("c-ledger-1"),
        "payload must contain cycle_id"
    );
    assert_eq!(
        payload.get("title").and_then(|v| v.as_str()),
        Some("Implement feature X"),
        "payload must contain title"
    );
}

#[test]
fn emit_work_item_created_not_in_std_registry() {
    let mut store = make_test_store();
    let input = make_wi_created_input();
    let result = emit_work_item_created(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let registry = std_registry();
    assert!(
        !registry.contains(&env.event_type, env.schema_version),
        "planning.work_item.created must NOT be in std_registry per ADR-071 §6"
    );
}

// ── emit_work_item_transitioned ─────────────────────────────────────────────

fn make_wi_transitioned_input() -> WorkItemTransitionedInput {
    WorkItemTransitionedInput {
        project_id: "p-test".into(),
        cycle_id: "c-ledger-1".into(),
        work_item_id: "wi-test-002".into(),
        from_status: "Draft".into(),
        to_status: "Active".into(),
        actor_id: "user:alice".into(),
        actor_kind: ActorKind::Human,
        occurred_at: "2026-09-05T11:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Human, "user:alice"),
    }
}

#[test]
fn emit_work_item_transitioned_produces_correct_event_type() {
    let mut store = make_test_store();
    let input = make_wi_transitioned_input();
    let result = emit_work_item_transitioned(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let env = store
        .load_by_event_id(&appended.event_id)
        .expect("store must not error")
        .expect("event must be stored");
    assert_eq!(
        env.event_type, "planning.work_item.transitioned",
        "event_type must be planning.work_item.transitioned"
    );
}

#[test]
fn emit_work_item_transitioned_has_schema_version_one() {
    let mut store = make_test_store();
    let input = make_wi_transitioned_input();
    let result = emit_work_item_transitioned(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.schema_version, 1);
}

#[test]
fn emit_work_item_transitioned_not_in_std_registry() {
    let mut store = make_test_store();
    let input = make_wi_transitioned_input();
    let result = emit_work_item_transitioned(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let registry = std_registry();
    assert!(
        !registry.contains(&env.event_type, env.schema_version),
        "planning.work_item.transitioned must NOT be in std_registry per ADR-071"
    );
}

// ── emit_dependency_added ─────────────────────────────────────────────────────

fn make_dep_added_input() -> DependencyAddedInput {
    DependencyAddedInput {
        project_id: "p-test".into(),
        cycle_id: "c-ledger-1".into(),
        from_work_item_id: "wi-alpha".into(),
        to_work_item_id: "wi-beta".into(),
        dependency_kind: "Blocks".into(),
        actor_id: "system:planner".into(),
        actor_kind: ActorKind::System,
        occurred_at: "2026-09-05T12:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::System, "system:planner"),
    }
}

#[test]
fn emit_dependency_added_produces_correct_event_type() {
    let mut store = make_test_store();
    let input = make_dep_added_input();
    let result = emit_dependency_added(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let env = store
        .load_by_event_id(&appended.event_id)
        .expect("store must not error")
        .expect("event must be stored");
    assert_eq!(
        env.event_type, "planning.dependency.added",
        "event_type must be planning.dependency.added"
    );
}

#[test]
fn emit_dependency_added_has_schema_version_one() {
    let mut store = make_test_store();
    let input = make_dep_added_input();
    let result = emit_dependency_added(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.schema_version, 1);
}

#[test]
fn emit_dependency_added_not_in_std_registry() {
    let mut store = make_test_store();
    let input = make_dep_added_input();
    let result = emit_dependency_added(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let registry = std_registry();
    assert!(
        !registry.contains(&env.event_type, env.schema_version),
        "planning.dependency.added must NOT be in std_registry per ADR-071"
    );
}

// ── emit_evidence_attached ────────────────────────────────────────────────────

fn make_evidence_attached_input() -> EvidenceAttachedInput {
    EvidenceAttachedInput {
        project_id: "p-test".into(),
        cycle_id: "c-ledger-1".into(),
        work_item_id: "wi-test-003".into(),
        evidence_id: "ev-001".into(),
        evidence_kind: "log".into(),
        cas_hash: "sha256:abc123def456".into(),
        actor_id: "agent:ci".into(),
        actor_kind: ActorKind::Agent,
        occurred_at: "2026-09-05T13:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Agent, "agent:ci"),
    }
}

#[test]
fn emit_evidence_attached_produces_correct_event_type() {
    let mut store = make_test_store();
    let input = make_evidence_attached_input();
    let result = emit_evidence_attached(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let env = store
        .load_by_event_id(&appended.event_id)
        .expect("store must not error")
        .expect("event must be stored");
    assert_eq!(
        env.event_type, "planning.evidence.attached",
        "event_type must be planning.evidence.attached"
    );
}

#[test]
fn emit_evidence_attached_has_schema_version_one() {
    let mut store = make_test_store();
    let input = make_evidence_attached_input();
    let result = emit_evidence_attached(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.schema_version, 1);
}

#[test]
fn emit_evidence_attached_not_in_std_registry() {
    let mut store = make_test_store();
    let input = make_evidence_attached_input();
    let result = emit_evidence_attached(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let registry = std_registry();
    assert!(
        !registry.contains(&env.event_type, env.schema_version),
        "planning.evidence.attached must NOT be in std_registry per ADR-071"
    );
}

// ── emit_decision_recorded ───────────────────────────────────────────────────

fn make_decision_recorded_input() -> DecisionRecordedInput {
    DecisionRecordedInput {
        project_id: "p-test".into(),
        cycle_id: "c-ledger-1".into(),
        work_item_id: "wi-test-004".into(),
        decision_id: "dec-001".into(),
        decision_kind: "accept".into(),
        rationale_summary: "Best approach given constraints".into(),
        actor_id: "user:bob".into(),
        actor_kind: ActorKind::Human,
        occurred_at: "2026-09-05T14:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Human, "user:bob"),
    }
}

#[test]
fn emit_decision_recorded_produces_correct_event_type() {
    let mut store = make_test_store();
    let input = make_decision_recorded_input();
    let result = emit_decision_recorded(&mut store, &input);
    assert!(result.is_ok());
    let appended = result.unwrap();

    let env = store
        .load_by_event_id(&appended.event_id)
        .expect("store must not error")
        .expect("event must be stored");
    assert_eq!(
        env.event_type, "planning.decision.recorded",
        "event_type must be planning.decision.recorded"
    );
}

#[test]
fn emit_decision_recorded_has_schema_version_one() {
    let mut store = make_test_store();
    let input = make_decision_recorded_input();
    let result = emit_decision_recorded(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.schema_version, 1);
}

#[test]
fn emit_decision_recorded_not_in_std_registry() {
    let mut store = make_test_store();
    let input = make_decision_recorded_input();
    let result = emit_decision_recorded(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();

    let registry = std_registry();
    assert!(
        !registry.contains(&env.event_type, env.schema_version),
        "planning.decision.recorded must NOT be in std_registry per ADR-071"
    );
}

// ── Actor propagation in all five emitters ───────────────────────────────────

#[test]
fn emit_work_item_created_propagates_actor() {
    let mut store = make_test_store();
    let input = make_wi_created_input();
    let result = emit_work_item_created(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.actor.kind, ActorKind::Agent);
    assert_eq!(env.actor.id, "agent:test");
}

#[test]
fn emit_work_item_transitioned_propagates_human_actor() {
    let mut store = make_test_store();
    let input = make_wi_transitioned_input();
    let result = emit_work_item_transitioned(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.actor.kind, ActorKind::Human);
    assert_eq!(env.actor.id, "user:alice");
}

#[test]
fn emit_dependency_added_propagates_system_actor() {
    let mut store = make_test_store();
    let input = make_dep_added_input();
    let result = emit_dependency_added(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.actor.kind, ActorKind::System);
}

#[test]
fn emit_evidence_attached_propagates_agent_actor() {
    let mut store = make_test_store();
    let input = make_evidence_attached_input();
    let result = emit_evidence_attached(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.actor.kind, ActorKind::Agent);
}

#[test]
fn emit_decision_recorded_propagates_human_actor() {
    let mut store = make_test_store();
    let input = make_decision_recorded_input();
    let result = emit_decision_recorded(&mut store, &input).unwrap();
    let env = store.load_by_event_id(&result.event_id).unwrap().unwrap();
    assert_eq!(env.actor.kind, ActorKind::Human);
    assert_eq!(env.actor.id, "user:bob");
}
