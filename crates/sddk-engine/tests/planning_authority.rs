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

use sddk_domain::ActorKind;
use sddk_engine::{
    EngineError,
    authority::{AuthorityContext, WritableSurface},
};

fn validate(surface: WritableSurface, actor_kind: ActorKind) -> Result<(), EngineError> {
    let ctx = AuthorityContext::for_test(actor_kind, "test-actor");
    ctx.validate(surface)
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
