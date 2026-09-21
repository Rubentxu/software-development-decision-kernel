//! AIW-S7c — AuthorityContext negative-grant integration tests.
//!
//! Closes AIW-S7 G05 (Secretary intenta release/gate/lease/receipt fuera
//! closed-set) and G07 (autoridad no otorgada: actor kind rejected on a
//! surface by the writable-surface matrix) against the REAL engine APIs
//! in `authority`, `secretary_closed_set` and `secretary_l1`.

use sddk_domain::ActorKind;
use sddk_domain::event_envelope::ActorRef;

use sddk_engine::EngineError;
use sddk_engine::authority::{AuthorityContext, WritableSurface};
use sddk_engine::risk_approval_policy::RiskTier;
use sddk_engine::secretary_closed_set::{
    SECRETARY_PROHIBITED_PREFIXES, SecretaryClosedSetError, is_secretary, validate_secretary_event,
};
use sddk_engine::secretary_l1::{
    BoundedWindow, ClosedSetKind, ProposalTemplate, SecretaryId, SecretaryL1Engine,
    SecretaryL1Error,
};

fn secretary_actor() -> ActorRef {
    ActorRef {
        kind: ActorKind::Agent,
        id: "agent:secretary-1".into(),
        definition_hash: None,
        policy_hash: None,
        model: None,
        role: Some("secretary".into()),
    }
}

// ---------------------------------------------------------------------------
// G07 — AuthorityContext negative grants (writable-surface matrix)
// ---------------------------------------------------------------------------

#[test]
fn human_rejected_on_framework_bundle_surface() {
    let ctx = AuthorityContext::for_test(ActorKind::Human, "user:alice");
    let result = ctx.validate(WritableSurface::FrameworkBundle);
    assert!(matches!(
        result,
        Err(EngineError::AuthorityContextRejected { ref surface, ref kind, .. })
            if surface == "framework_bundle" && kind == "Human"
    ));
}

#[test]
fn human_rejected_on_github_releases_surface() {
    let ctx = AuthorityContext::for_test(ActorKind::Human, "user:alice");
    assert!(matches!(
        ctx.validate(WritableSurface::GithubReleases),
        Err(EngineError::AuthorityContextRejected { .. })
    ));
}

#[test]
fn human_rejected_on_dependency_edge_surface() {
    let ctx = AuthorityContext::for_test(ActorKind::Human, "user:alice");
    assert!(matches!(
        ctx.validate(WritableSurface::DependencyEdge),
        Err(EngineError::AuthorityContextRejected { .. })
    ));
}

#[test]
fn agent_rejected_on_decision_record_surface() {
    // G07: an agent (secretary included) is NOT granted decision_record;
    // only Human is admitted there.
    let ctx = AuthorityContext::for_test(ActorKind::Agent, "agent:secretary-1");
    let result = ctx.validate(WritableSurface::DecisionRecord);
    assert!(matches!(
        result,
        Err(EngineError::AuthorityContextRejected { ref kind, .. }) if kind == "Agent"
    ));
}

#[test]
fn system_rejected_on_knowledge_graph_vault_surface() {
    let ctx = AuthorityContext::for_test(ActorKind::System, "sys");
    assert!(matches!(
        ctx.validate(WritableSurface::KnowledgeGraphVault),
        Err(EngineError::AuthorityContextRejected { .. })
    ));
}

#[test]
fn cli_admitted_on_evidence_attachment_surface() {
    // Positive control: CLI human caller IS granted evidence_attachment.
    let ctx = AuthorityContext::for_cli("user:cli-1", ActorKind::Human, None, None);
    assert!(ctx.validate(WritableSurface::EvidenceAttachment).is_ok());
}

#[test]
fn negative_grant_does_not_spill_to_admitted_surface() {
    // A rejected actor on surface A keeps its grants on surface B (grant
    // table is per-surface, fail-closed is not global revocation).
    let ctx = AuthorityContext::for_test(ActorKind::Human, "user:alice");
    assert!(ctx.validate(WritableSurface::FrameworkBundle).is_err());
    assert!(ctx.validate(WritableSurface::PlanItem).is_ok());
}

// ---------------------------------------------------------------------------
// G05 — Secretary closed-set enforcement (release/gate/lease/receipt)
// ---------------------------------------------------------------------------

#[test]
fn secretary_prohibited_on_all_four_exclusive_prefixes() {
    let actor = secretary_actor();
    assert!(is_secretary(&actor));
    for event_type in [
        "release.complete",
        "gate.passed",
        "lease.released",
        "receipt.replay",
    ] {
        // The exact prefixes are enforced as prohibited.
        assert!(
            SECRETARY_PROHIBITED_PREFIXES
                .iter()
                .any(|p| event_type.starts_with(p))
        );
        let result = validate_secretary_event(&actor, event_type);
        assert_eq!(
            result,
            Err(SecretaryClosedSetError::ProhibitedEventType {
                event_type: event_type.to_string()
            })
        );
    }
}

#[test]
fn secretary_state_mutation_outside_closed_set_requires_escalation() {
    let actor = secretary_actor();
    let result = validate_secretary_event(&actor, "cycle.transitioned");
    assert_eq!(
        result,
        Err(SecretaryClosedSetError::EscalationRequired {
            event_type: "cycle.transitioned".to_string()
        })
    );
    // The mandatory escalation path itself is admissible.
    assert_eq!(
        validate_secretary_event(&actor, "escalation.requested"),
        Ok(())
    );
}

#[test]
fn secretary_bound_to_role_only_for_agent_kind() {
    // A Human carrying role="secretary" is NOT bound: the closed-set
    // validator does not apply (their authority is AuthorityContext's job).
    let human = ActorRef {
        kind: ActorKind::Human,
        id: "user:alice".into(),
        definition_hash: None,
        policy_hash: None,
        model: None,
        role: Some("secretary".into()),
    };
    assert!(!is_secretary(&human));
    assert_eq!(validate_secretary_event(&human, "release.complete"), Ok(()));
}

// ---------------------------------------------------------------------------
// Secretary L1 negative grants (tier gate + unknown template)
// ---------------------------------------------------------------------------

fn window() -> BoundedWindow {
    BoundedWindow::new(SecretaryId::new("test"), 0, i64::MAX, u32::MAX)
}

#[test]
fn secretary_l1_proposal_with_high_tier_requires_evidence() {
    let engine = SecretaryL1Engine::new();
    engine
        .register_template(ProposalTemplate::new(
            "high-tier-test",
            ClosedSetKind::SuggestHumanDecision,
            "high tier needs evidence",
            RiskTier::High,
            window(),
        ))
        .expect("register");
    let result = engine.propose(
        1_000,
        "high-tier-test",
        vec![], // empty evidence_refs: High tier requires at least one
        vec!["satisfied".into()],
        Vec::new(),
        "test",
        0.8,
    );
    assert!(matches!(
        result,
        Err(SecretaryL1Error::EmptyCoverage { ref template_id }) if template_id == "high-tier-test"
    ));
}

#[test]
fn secretary_l1_unknown_template_rejected() {
    let engine = SecretaryL1Engine::new();
    let result = engine.propose(
        1_000,
        "not-registered",
        vec!["ev".into()],
        vec!["sat".into()],
        Vec::new(),
        "test",
        0.5,
    );
    assert!(matches!(
        result,
        Err(SecretaryL1Error::UnknownTemplate { ref template_id }) if template_id == "not-registered"
    ));
}

#[test]
fn secretary_proposal_kind_matches_template() {
    let engine = SecretaryL1Engine::new();
    engine
        .register_template(ProposalTemplate::new(
            "low-tier-test",
            ClosedSetKind::SuggestCandidate,
            "low tier",
            RiskTier::Low,
            window(),
        ))
        .expect("register");
    let proposal = engine
        .propose(
            1_000,
            "low-tier-test",
            vec![],
            vec!["satisfied".into()],
            Vec::new(),
            "ok",
            0.9,
        )
        .expect("propose");
    assert_eq!(proposal.template_id, "low-tier-test");
    assert_eq!(proposal.kind, ClosedSetKind::SuggestCandidate);
}
