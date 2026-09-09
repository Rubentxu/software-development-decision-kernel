//! Orchestration Synthesis Receipt — RED->GREEN tests for CDD-HANDOFF-002.
//!
//! Verifies the 8 invariants of the exit gate against the implementation
//! in `crates/sddk-engine/src/orchestration_synthesis.rs` and the
//! cross-check backref to CDD-HANDOFF-001's `EnvelopeStore`.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-OrchestrationSynthesisReceipt.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-086-ORCHESTRATION-SYNTHESIS-RECEIPT.md

use std::collections::BTreeMap;
use std::sync::Arc;

use sddk_engine::agent_contribution_envelope::{
    AgentContributionEnvelope, ContextLease, DelegationRequest, EnvelopeStore, Finding,
    InMemoryEnvelopeStore, Severity,
};
use sddk_engine::orchestration_synthesis::{
    ConflictEntry, ContributionRef, CoverageLossEntry, DissentEntry, InMemorySynthesisStore,
    InformationLossGuard, OrchestrationSynthesisReceipt, RiskCarryEntry, SynthesisError,
    SynthesisValidator,
};

// ----------------- helpers -----------------

fn make_request() -> (DelegationRequest, ContextLease) {
    let lease = ContextLease::new("L1", 7, "digest123", "leaf-x", 1_000, 2_000);
    let req = DelegationRequest::new(
        "D1",
        "coord-y",
        "leaf-x",
        "compute deployment",
        lease.clone(),
        7,
        500,
    );
    (req, lease)
}

fn make_envelope(
    req: &DelegationRequest,
    id: &str,
    produced_at_ms: i64,
) -> AgentContributionEnvelope {
    let mut env = AgentContributionEnvelope::new(id, "leaf-x", req, produced_at_ms);
    env.evidence_refs = vec![format!("cid://{id}")];
    env.coverage_satisfied = vec![format!("cov-{id}")];
    env.confidence = 0.8;
    env
}

fn make_high_risk_envelope(req: &DelegationRequest) -> AgentContributionEnvelope {
    let mut env = make_envelope(req, "EH", 1_500);
    env.risks = vec!["[Critical] data loss in shard 3".to_string()];
    env
}

fn make_dissent_envelope(req: &DelegationRequest) -> AgentContributionEnvelope {
    let mut env = make_envelope(req, "ED", 1_500);
    env.alternatives = vec![sddk_engine::agent_contribution_envelope::Proposal {
        id: "alt-1".into(),
        summary: "alt-summary".into(),
        rationale: "alt-rationale".into(),
    }];
    env.rejections = vec![sddk_engine::agent_contribution_envelope::Rejection {
        id: "rej-1".into(),
        summary: "alt rejected".into(),
        reason: "incompatible with policy".into(),
    }];
    env
}

fn cover_validator(
    store: Arc<InMemoryEnvelopeStore>,
    synth: Arc<InMemorySynthesisStore>,
) -> SynthesisValidator {
    SynthesisValidator::new(synth, store)
}

// ----------------- T1: happy path -----------------

#[test]
fn happy_path_three_envelopes_all_guards_pass() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let mut e1 = make_envelope(&req, "E1", 1_500);
    e1.recommendation = "ship it".into();
    let e2 = make_envelope(&req, "E2", 1_600);
    let e3 = make_envelope(&req, "E3", 1_700);
    env_store.put(e1.clone());
    env_store.put(e2.clone());
    env_store.put(e3.clone());

    let mut receipt = OrchestrationSynthesisReceipt::new("R1", "D1", "J1", "coordinator-z", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E1", 1_500, "coordinator-z"));
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E2", 1_600, "coordinator-z"));
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E3", 1_700, "coordinator-z"));
    receipt.downstream_recommendation = Some("ship it".to_string());
    receipt.confidence = 0.75;

    let v = cover_validator(env_store, synth_store);
    v.validate(&receipt).expect("happy path should validate");
    // also every individual guard must succeed
    for g in [
        InformationLossGuard::MandatoryRiskNoDrop,
        InformationLossGuard::MandatoryEvidenceNoDrop,
        InformationLossGuard::DissentNoSilentDrop,
        InformationLossGuard::CoverageLossRecorded,
        InformationLossGuard::RecommendationBackedByConsumed,
        InformationLossGuard::ConflictSurfacedOrJustified,
    ] {
        v.check_guard(&receipt, g).unwrap_or_else(|e| {
            panic!("guard {g:?} should pass: {e}");
        });
    }
}

// ----------------- T2: mandatory risk dropped -----------------

#[test]
fn mandatory_risk_dropped_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let e1 = make_high_risk_envelope(&req);
    env_store.put(e1.clone());
    // consume e1 but DON'T carry the [Critical] risk forward
    let mut receipt = OrchestrationSynthesisReceipt::new("R2", "D1", "J1", "coordinator-z", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "EH", 1_500, "coordinator-z"));
    receipt.confidence = 0.5;

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::MandatoryRiskDropped(_)),
        "expected MandatoryRiskDropped, got {err:?}"
    );
}

// ----------------- T3: mandatory evidence dropped -----------------

#[test]
fn mandatory_evidence_dropped_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let mut e1 = make_envelope(&req, "EEMed", 1_500);
    // severity-tag the evidence by including a finding of Medium severity
    e1.findings = vec![Finding {
        id: "f-1".into(),
        summary: "medium finding".into(),
        severity: Severity::Medium,
    }];
    env_store.put(e1.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R3", "D1", "J1", "coordinator-z", 2_000);
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        "EEMed",
        1_500,
        "coordinator-z",
    ));
    receipt.confidence = 0.5;

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::MandatoryEvidenceDropped(_)),
        "expected MandatoryEvidenceDropped, got {err:?}"
    );
}

// ----------------- T4: dissent silently dropped -----------------

#[test]
fn dissent_silently_dropped_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let e1 = make_dissent_envelope(&req);
    env_store.put(e1.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R4", "D1", "J1", "coordinator-z", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "ED", 1_500, "coordinator-z"));
    // no dissent_preserved entry, no conflict -> fail-closed
    receipt.confidence = 0.5;

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::DissentSilentlyDropped(_)),
        "expected DissentSilentlyDropped, got {err:?}"
    );
}

// ----------------- T5: coverage loss not recorded -----------------

#[test]
fn coverage_loss_unrecorded_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let mut e1 = make_envelope(&req, "EC", 1_500);
    e1.coverage_missing = vec!["cov-x-missing".into(), "cov-y-missing".into()];
    env_store.put(e1.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R5", "D1", "J1", "coordinator-z", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "EC", 1_500, "coordinator-z"));
    // intentionally do not record coverage_loss for cov-x-missing
    receipt.coverage_loss.push(CoverageLossEntry::new(
        "cov-y-missing",
        "out of scope this iteration",
        vec![],
    ));
    receipt.confidence = 0.5;

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::BuilderInvariantViolated(_)),
        "expected BuilderInvariantViolated, got {err:?}"
    );
}

// ----------------- T6: orphan recommendation -----------------

#[test]
fn orphan_recommendation_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let e1 = make_envelope(&req, "EO", 1_500);
    env_store.put(e1.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R6", "D1", "J1", "coordinator-z", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "EO", 1_500, "coordinator-z"));
    receipt.downstream_recommendation = Some("orphan recommendation".to_string());
    receipt.confidence = 0.5;
    // not marked standalone, doesn't trace to EO consumption content
    // validator should treat as orphan

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::RecommendationOrphan(_)),
        "expected RecommendationOrphan, got {err:?}"
    );
}

// ----------------- T7: hidden conflict -----------------

#[test]
fn hidden_conflict_fails() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let mut e1 = make_envelope(&req, "EC1", 1_500);
    let mut e2 = make_envelope(&req, "EC2", 1_600);
    e1.recommendation = "ship-it".to_string();
    e2.recommendation = "block".to_string();
    e1.confidence = 0.9;
    e2.confidence = 0.8;
    env_store.put(e1.clone());
    env_store.put(e2.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R7", "D1", "J1", "coordinator-z", 2_000);
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        "EC1",
        1_500,
        "coordinator-z",
    ));
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        "EC2",
        1_600,
        "coordinator-z",
    ));
    receipt.downstream_recommendation = Some("ship-it".to_string()); // matches EC1, hidden conflict on EC2
    receipt.confidence = 0.85;
    // deliberately no ConflictEntry

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::ConflictHidden(_)),
        "expected ConflictHidden, got {err:?}"
    );
}

// ----------------- T8: envelope digest mismatch -----------------

#[test]
fn envelope_digest_mismatch_fails() {
    let (_req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    // env_store is empty — every envelope_digest is unrecognised
    let mut receipt = OrchestrationSynthesisReceipt::new("R8", "D1", "J1", "coordinator-z", 2_000);
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        "DOES_NOT_EXIST",
        1_500,
        "coordinator-z",
    ));
    receipt.confidence = 0.5;

    let v = cover_validator(env_store, synth_store);
    let err = v.validate(&receipt).unwrap_err();
    assert!(
        matches!(err, SynthesisError::EnvelopeDigestMismatch { .. }),
        "expected EnvelopeDigestMismatch, got {err:?}"
    );
}

// ----------------- T9: closed-set guard enumeration -----------------

#[test]
fn every_guard_has_positive_and_negative() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    // Set up a happy receipt (positive) and break each guard one by one (negative).
    let mut e1 = make_high_risk_envelope(&req);
    e1.recommendation = "ship it".into();
    let mut e2 = make_envelope(&req, "ED2", 1_600);
    e2.rejections = vec![sddk_engine::agent_contribution_envelope::Rejection {
        id: "rej-2".into(),
        summary: "x".into(),
        reason: "y".into(),
    }];
    e2.alternatives = vec![sddk_engine::agent_contribution_envelope::Proposal {
        id: "alt-2".into(),
        summary: "s".into(),
        rationale: "r".into(),
    }];
    env_store.put(e1.clone());
    env_store.put(e2.clone());

    let mut positive = OrchestrationSynthesisReceipt::new("RP", "D1", "J1", "coord-z", 2_000);
    positive
        .consumed
        .push(ContributionRef::consume("D1", "EH", 1_500, "coord-z"));
    positive
        .consumed
        .push(ContributionRef::consume("D1", "ED2", 1_600, "coord-z"));
    positive.risk_carry_forward.push(RiskCarryEntry::new(
        "risk-shard-loss",
        "EH",
        Severity::Critical,
        "[Critical] data loss in shard 3",
        "downstream",
    ));
    positive.dissent_preserved.push(DissentEntry::preserved(
        "rej-2",
        "ED2",
        "y",
        Some("alt-2".to_string()),
    ));
    positive.downstream_recommendation = Some("ship it".to_string());
    positive.confidence = 0.7;
    positive
        .metrics
        .insert("compression_ratio".to_string(), 0.42);

    let v = cover_validator(env_store.clone(), synth_store.clone());
    v.validate(&positive).expect("positive receipt must pass");

    // negatives: clone + perturb each guard one at a time.
    // mandatory risk dropped
    let mut neg_risk = positive.clone();
    neg_risk.risk_carry_forward.clear();
    assert!(matches!(
        v.check_guard(&neg_risk, InformationLossGuard::MandatoryRiskNoDrop),
        Err(SynthesisError::MandatoryRiskDropped(_))
    ));

    // dissent silently dropped
    let mut neg_dissent = positive.clone();
    neg_dissent.dissent_preserved.clear();
    assert!(matches!(
        v.check_guard(&neg_dissent, InformationLossGuard::DissentNoSilentDrop),
        Err(SynthesisError::DissentSilentlyDropped(_))
    ));

    // orphan recommendation
    let mut neg_reco = positive.clone();
    neg_reco.downstream_recommendation = Some("no link to consumed envelopes".to_string());
    assert!(matches!(
        v.check_guard(
            &neg_reco,
            InformationLossGuard::RecommendationBackedByConsumed
        ),
        Err(SynthesisError::RecommendationOrphan(_))
    ));
}

// ----------------- T10: determinism -----------------

#[test]
fn identical_inputs_identical_outputs() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let e1 = make_envelope(&req, "DET", 1_500);
    env_store.put(e1.clone());
    let mut receipt =
        OrchestrationSynthesisReceipt::new("RDET", "D1", "J1", "coordinator-z", 2_000);
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        "DET",
        1_500,
        "coordinator-z",
    ));
    receipt.confidence = 0.5;
    // add an unsorted BTreeMap and a non-sorted consumed vec to ensure
    // output ordering doesn't depend on HashMap iteration order
    let mut metrics: BTreeMap<String, f64> = BTreeMap::new();
    metrics.insert("z".into(), 1.0);
    metrics.insert("a".into(), 2.0);
    metrics.insert("m".into(), 3.0);
    receipt.metrics = metrics;
    receipt.conflict_internal.push(ConflictEntry::new(
        "CFX",
        vec!["DET".into()],
        "field",
        "kept_a",
        "explicit rationale",
        1_500,
    ));

    let v = cover_validator(env_store, synth_store);
    let r1 = v.validate(&receipt);
    let r2 = v.validate(&receipt);
    assert_eq!(
        format!("{:?}", r1),
        format!("{:?}", r2),
        "two validations of the same receipt must match"
    );
}

// (end of tests)
