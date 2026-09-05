//! Tests for WRITABLE_SURFACE_MATRIX invariants on the four planning surfaces (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-14 (FIND-PLN-006 closure):
//! - PlanItem: Human+Agent admitted, System rejected
//! - DependencyEdge: System admitted, Human+Agent rejected
//! - EvidenceAttachment: Human+Agent admitted, System rejected
//! - DecisionRecord: Human admitted, Agent+System rejected
//!
//! Per ADR-069 §3 + ADR-072 planning surfaces, the matrix is enforced at
//! the engine layer via `AuthorityContext::validate()`.

use sddk_domain::{ActorKind, EventStore};
use sddk_engine::event_bus::emit::{
    DecisionRecordedInput, DependencyAddedInput, EvidenceAttachedInput, WorkItemCreatedInput,
    WorkItemTransitionedInput, emit_decision_recorded, emit_dependency_added,
    emit_evidence_attached, emit_work_item_created, emit_work_item_transitioned,
};
use sddk_engine::{
    EngineError,
    authority::{AuthorityContext, WritableSurface},
};
use sddk_storage::SqliteEventStore;

fn validate(surface: WritableSurface, actor_kind: ActorKind) -> Result<(), EngineError> {
    let ctx = AuthorityContext::for_test(actor_kind, "test-actor");
    ctx.validate(surface)
}

fn make_test_store() -> SqliteEventStore {
    SqliteEventStore::open_in_memory().expect("in-memory store must open")
}

// ── PlanItem — Human+Agent admitted, System rejected ─────────────────────────

#[test]
fn plan_item_allows_human() {
    assert!(
        validate(WritableSurface::PlanItem, ActorKind::Human).is_ok(),
        "PlanItem must admit Human"
    );
}

#[test]
fn plan_item_allows_agent() {
    assert!(
        validate(WritableSurface::PlanItem, ActorKind::Agent).is_ok(),
        "PlanItem must admit Agent"
    );
}

#[test]
fn plan_item_rejects_system() {
    assert!(
        validate(WritableSurface::PlanItem, ActorKind::System).is_err(),
        "PlanItem must reject System"
    );
}

// ── DependencyEdge — System admitted, Human+Agent rejected ───────────────────

#[test]
fn dependency_edge_allows_system() {
    assert!(
        validate(WritableSurface::DependencyEdge, ActorKind::System).is_ok(),
        "DependencyEdge must admit System"
    );
}

#[test]
fn dependency_edge_rejects_human() {
    assert!(
        validate(WritableSurface::DependencyEdge, ActorKind::Human).is_err(),
        "DependencyEdge must reject Human"
    );
}

#[test]
fn dependency_edge_rejects_agent() {
    assert!(
        validate(WritableSurface::DependencyEdge, ActorKind::Agent).is_err(),
        "DependencyEdge must reject Agent"
    );
}

// ── EvidenceAttachment — Human+Agent admitted, System rejected ───────────────

#[test]
fn evidence_attachment_allows_human() {
    assert!(
        validate(WritableSurface::EvidenceAttachment, ActorKind::Human).is_ok(),
        "EvidenceAttachment must admit Human"
    );
}

#[test]
fn evidence_attachment_allows_agent() {
    assert!(
        validate(WritableSurface::EvidenceAttachment, ActorKind::Agent).is_ok(),
        "EvidenceAttachment must admit Agent"
    );
}

#[test]
fn evidence_attachment_rejects_system() {
    assert!(
        validate(WritableSurface::EvidenceAttachment, ActorKind::System).is_err(),
        "EvidenceAttachment must reject System"
    );
}

// ── DecisionRecord — Human admitted, Agent+System rejected ─────────────────

#[test]
fn decision_record_allows_human() {
    assert!(
        validate(WritableSurface::DecisionRecord, ActorKind::Human).is_ok(),
        "DecisionRecord must admit Human"
    );
}

#[test]
fn decision_record_rejects_agent() {
    assert!(
        validate(WritableSurface::DecisionRecord, ActorKind::Agent).is_err(),
        "DecisionRecord must reject Agent"
    );
}

#[test]
fn decision_record_rejects_system() {
    assert!(
        validate(WritableSurface::DecisionRecord, ActorKind::System).is_err(),
        "DecisionRecord must reject System"
    );
}

// ── Rejection carries surface name and actor kind ──────────────────────────

#[test]
fn plan_item_system_rejection_contains_surface_name() {
    let ctx = AuthorityContext::for_test(ActorKind::System, "sys:test");
    let err = ctx.validate(WritableSurface::PlanItem).unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("plan_item"),
        "rejection must name the surface: got {err_msg}"
    );
}

#[test]
fn dependency_edge_human_rejection_contains_actor_kind() {
    let ctx = AuthorityContext::for_test(ActorKind::Human, "user:test");
    let err = ctx.validate(WritableSurface::DependencyEdge).unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("Human"),
        "rejection must name the actor kind: got {err_msg}"
    );
}

#[test]
fn evidence_attachment_system_rejection_contains_surface_name() {
    let ctx = AuthorityContext::for_test(ActorKind::System, "sys:test");
    let err = ctx
        .validate(WritableSurface::EvidenceAttachment)
        .unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("evidence_attachment"),
        "rejection must name the surface: got {err_msg}"
    );
}

#[test]
fn decision_record_agent_rejection_contains_actor_kind() {
    let ctx = AuthorityContext::for_test(ActorKind::Agent, "agent:test");
    let err = ctx.validate(WritableSurface::DecisionRecord).unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("Agent"),
        "rejection must name the actor kind: got {err_msg}"
    );
}

// ── Authority precedes emission — fail-closed on rejected actor kind ────────────

// REQ-PLN2-AUTH-003: authority check MUST precede event emission.
// These tests prove that a rejected authority context produces NO appended event.

#[test]
fn emit_work_item_created_rejects_system_authority_no_event_appended() {
    let mut store = make_test_store();
    // System is NOT admitted on PlanItem (only Human+Agent)
    let input = WorkItemCreatedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        work_item_id: "wi-1".into(),
        title: "Test".into(),
        description: "Test desc".into(),
        status: "Draft".into(),
        actor_id: "system:test".into(),
        actor_kind: ActorKind::System,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::System, "system:test"),
    };
    let result = emit_work_item_created(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_work_item_created must fail when System writes PlanItem"
    );
    // Store must be empty — no partial event appended
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}

#[test]
fn emit_work_item_transitioned_rejects_system_authority_no_event_appended() {
    let mut store = make_test_store();
    let input = WorkItemTransitionedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        work_item_id: "wi-1".into(),
        from_status: "Draft".into(),
        to_status: "Active".into(),
        actor_id: "system:test".into(),
        actor_kind: ActorKind::System,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::System, "system:test"),
    };
    let result = emit_work_item_transitioned(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_work_item_transitioned must fail when System writes PlanItem"
    );
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}

#[test]
fn emit_dependency_added_rejects_human_authority_no_event_appended() {
    let mut store = make_test_store();
    // Human is NOT admitted on DependencyEdge (only System)
    let input = DependencyAddedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        from_work_item_id: "wi-a".into(),
        to_work_item_id: "wi-b".into(),
        dependency_kind: "Blocks".into(),
        actor_id: "user:test".into(),
        actor_kind: ActorKind::Human,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Human, "user:test"),
    };
    let result = emit_dependency_added(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_dependency_added must fail when Human writes DependencyEdge"
    );
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}

#[test]
fn emit_evidence_attached_rejects_system_authority_no_event_appended() {
    let mut store = make_test_store();
    // System is NOT admitted on EvidenceAttachment (only Human+Agent)
    let input = EvidenceAttachedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        work_item_id: "wi-1".into(),
        evidence_id: "ev-1".into(),
        evidence_kind: "log".into(),
        cas_hash: "sha256:abc123".into(),
        actor_id: "system:test".into(),
        actor_kind: ActorKind::System,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::System, "system:test"),
    };
    let result = emit_evidence_attached(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_evidence_attached must fail when System writes EvidenceAttachment"
    );
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}

#[test]
fn emit_decision_recorded_rejects_agent_authority_no_event_appended() {
    let mut store = make_test_store();
    // Agent is NOT admitted on DecisionRecord (only Human)
    let input = DecisionRecordedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        work_item_id: "wi-1".into(),
        decision_id: "dec-1".into(),
        decision_kind: "accept".into(),
        rationale_summary: "Test rationale".into(),
        actor_id: "agent:test".into(),
        actor_kind: ActorKind::Agent,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::Agent, "agent:test"),
    };
    let result = emit_decision_recorded(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_decision_recorded must fail when Agent writes DecisionRecord"
    );
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}

#[test]
fn emit_decision_recorded_rejects_system_authority_no_event_appended() {
    let mut store = make_test_store();
    let input = DecisionRecordedInput {
        project_id: "p-test".into(),
        cycle_id: "c-1".into(),
        work_item_id: "wi-1".into(),
        decision_id: "dec-1".into(),
        decision_kind: "accept".into(),
        rationale_summary: "Test rationale".into(),
        actor_id: "system:test".into(),
        actor_kind: ActorKind::System,
        occurred_at: "2026-09-05T10:00:00Z".into(),
        causation_id: None,
        correlation_id: None,
        authority: AuthorityContext::for_test(ActorKind::System, "system:test"),
    };
    let result = emit_decision_recorded(&mut store, &input);
    assert!(
        result.is_err(),
        "emit_decision_recorded must fail when System writes DecisionRecord"
    );
    let count = store.count().expect("store must not error");
    assert_eq!(
        count, 0,
        "no event must be appended when authority is rejected"
    );
}
