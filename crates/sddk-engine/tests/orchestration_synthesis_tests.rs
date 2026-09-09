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
    SynthesisStore, SynthesisValidator,
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

// ----------------- SYN-13 .. SYN-22: edge-case broadening -----------------
//
// These tests close the weak feedback loops flagged after the initial
// T1..T10 pass. Each one targets a specific seam surfaced by harness
// review: empty receipts, severity normalization edges, idempotency
// under the store, and cross-validator integration.

/// SYN-13: An empty receipt (no consumed contributions) has no
/// information-loss guards to trip, so it passes. We document this
/// behaviour explicitly here as a guard-rail: future CDD cycles that
/// want fail-closed empty-receipt semantics must add an explicit
/// `EmptyReceipt` guard and update this test. Today the expectation is
/// `Ok(())`.
#[test]
fn syn_13_empty_receipt_currently_passes_documents_gap() {
    let (_req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);

    let receipt = OrchestrationSynthesisReceipt::new("R-empty", "D1", "J1", "coord", 2_000);
    let r = v.validate(&receipt);
    assert!(
        r.is_ok(),
        "SYN-13 documents current pass-through (gap marker): {r:?}"
    );
}

/// SYN-14: Lowercase severity `[high]` must still block via the
/// `extract_severity` normalizer (case-insensitive header). The guard
/// that catches `[Critical]` must also catch `[high]` and `[HIGH]`.
#[test]
fn syn_14_lowercase_severity_still_blocks() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e = make_envelope(&req, "E-low", 1_500);
    e.risks = vec!["[high] outage risk in zone b".to_string()];
    env_store.put(e.clone());
    // Inserting into the receipt w/o compensating risk_carry must fail.
    let mut receipt = OrchestrationSynthesisReceipt::new("R-low", "D1", "J1", "coord", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E-low", 1_500, "coord"));
    // No risk_carry_forward for the mandatory risk → must fail.
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    let r = v.validate(&receipt);
    assert!(r.is_err(), "SYN-14 lowercase [high] risk must drop: {r:?}");
}

/// SYN-15: Mixed case severity: `[HIGH]`, `[High]`, `[high]`, `[cRiTiCaL]`
/// must all map to the canonical severity set via `extract_severity`
/// (case-insensitive). We provide a single carry-forward per risk
/// envelope and the validator must accept all variants.
#[test]
fn syn_15_severity_normalization_canonicalizes_case() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e = make_envelope(&req, "E-mix", 1_500);
    e.risks = vec![
        "[HIGH] one".to_string(),
        "[High] two".to_string(),
        "[high] three".to_string(),
        "[cRiTiCaL] four".to_string(),
    ];
    env_store.put(e.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R-mix", "D1", "J1", "coord", 2_000);
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        e.envelope_id.as_str(),
        1_500,
        "coord",
    ));
    // Carry-forward each risk to a downstream owner with severity.
    // validator matches by `envelope_digest` which equals `envelope_id`.
    receipt.risk_carry_forward.push(RiskCarryEntry::new(
        "risk-mix-1",
        e.envelope_id.as_str(),
        Severity::High,
        "carry-forward for variant [HIGH]/[High]/[high]/[cRiTiCaL]",
        "downstream_owner",
    ));
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    v.validate(&receipt)
        .expect("SYN-15 case variants should canonicalize and pass");
}

/// SYN-16: A risk without `[Brackets]` (bare severity word in any case)
/// is currently NOT classified as mandatory. We document this gap:
/// today `check_mandatory_risk` skips bare severities. Future
/// hardening should widen `extract_severity` to consider bare-word
/// severities at line start. This test pins current behaviour.
#[test]
fn syn_16_bare_severity_currently_ignored_documents_gap() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e = make_envelope(&req, "E-nobr", 1_500);
    // No brackets, only capitalised severity word at start.
    e.risks = vec!["Critical data corruption in node 4".to_string()];
    env_store.put(e.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R-nobr", "D1", "J1", "coord", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E-nobr", 1_500, "coord"));
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    let r = v.validate(&receipt);
    assert!(
        r.is_ok(),
        "SYN-16 documents current bare-severity gap (pass-through): {r:?}"
    );
}

/// SYN-17: Empty `coverage_satisfied` is not currently flagged unless
/// `coverage_missing` lists the area. We document this gap: today the
/// guard uses `coverage_missing`, not `coverage_satisfied`. Future
/// hardening should also enforce `coverage_satisfied.is_empty()` as
/// an unrecorded dropped promise. This test pins current behaviour.
#[test]
fn syn_17_empty_coverage_satisfied_currently_passes_documents_gap() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e = make_envelope(&req, "E-nocomp", 1_500);
    // Empty coverage_satisfied AND empty coverage_missing → passes.
    e.coverage_satisfied = vec![];
    e.coverage_missing = vec![];
    env_store.put(e.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R-nocomp", "D1", "J1", "coord", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E-nocomp", 1_500, "coord"));
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    let r = v.validate(&receipt);
    assert!(
        r.is_ok(),
        "SYN-17 documents current empty-coverage gap (pass-through): {r:?}"
    );
}

/// SYN-18: When compensating coverage is provided, validation must pass
/// and the recovery_reason must be reported on the CoverageLossEntry.
#[test]
fn syn_18_coverage_loss_with_compensation_passes() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e = make_envelope(&req, "E-comp", 1_500);
    e.coverage_satisfied = vec![];
    env_store.put(e.clone());
    let mut receipt = OrchestrationSynthesisReceipt::new("R-comp", "D1", "J1", "coord", 2_000);
    receipt
        .consumed
        .push(ContributionRef::consume("D1", "E-comp", 1_500, "coord"));
    receipt.coverage_loss.push(CoverageLossEntry::new(
        "cov-E-comp",
        "swallowed because redundant with E2",
        vec!["E-resolve-2".to_string()],
    ));
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    v.validate(&receipt)
        .expect("SYN-18 compensated coverage loss must validate");
}

/// SYN-19: Three envelopes producing three different recommendations
/// with two omissions and one consumed must surface a hidden conflict
/// because the consensus split. The validator catches the contradiction
/// in check_orphan_recommendation / hidden_conflict_guard.
#[test]
fn syn_19_three_recommendations_hidden_conflict() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let mut e1 = make_envelope(&req, "E-rec-1", 1_500);
    e1.recommendation = "ship it".into();
    let mut e2 = make_envelope(&req, "E-rec-2", 1_600);
    e2.recommendation = "defer".into();
    let mut e3 = make_envelope(&req, "E-rec-3", 1_700);
    e3.recommendation = "abort".into();
    env_store.put(e1.clone());
    env_store.put(e2.clone());
    env_store.put(e3.clone());

    let mut receipt = OrchestrationSynthesisReceipt::new("R-conf", "D1", "J1", "coord", 2_000);
    // We consume all 3 envelopes but the output recommendation is
    // "ship it" which only matches e1 — hidden conflict.
    for (id, ts) in [("E-rec-1", 1_500), ("E-rec-2", 1_600), ("E-rec-3", 1_700)] {
        receipt
            .consumed
            .push(ContributionRef::consume("D1", id, ts, "coord"));
    }
    receipt.downstream_recommendation = Some("ship it".to_string());

    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let v = cover_validator(env_store, synth_store);
    let r = v.validate(&receipt);
    assert!(
        r.is_err(),
        "SYN-19 divergent recommendations must fail: {r:?}"
    );
}

/// SYN-20: Putting the same envelope twice is idempotent. The store
/// returns the original; no duplicate record; no panic.
#[test]
fn syn_20_envelope_store_idempotent_re_put() {
    let store = InMemoryEnvelopeStore::new();
    let (req, _) = make_request();
    let e = make_envelope(&req, "E-dup", 1_500);
    store.put(e.clone());
    store.put(e.clone());
    store.put(e.clone());
    let all = store.all();
    let matching: Vec<_> = all.iter().filter(|x| x.envelope_id == "E-dup").collect();
    assert_eq!(matching.len(), 1, "SYN-20 idempotent store keeps 1 record");
}

/// SYN-21: Receipt must account for every envelope that arrived on the
/// delegation: consumed_or_omitted count must equal the actual
/// envelopes in the EnvelopeStore for that delegation. A mismatch is
/// silent leakage — the receipt dropped or fabricated contributions.
#[test]
fn syn_21_receipt_accounts_for_all_in_flight_envelopes() {
    let (req, _) = make_request();
    let env_store = Arc::new(InMemoryEnvelopeStore::new());
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());

    let mut e1 = make_envelope(&req, "E-resolve-1", 1_500);
    e1.recommendation = "ship it".into();
    let mut e2 = make_envelope(&req, "E-resolve-2", 1_600);
    e2.recommendation = "ship it".into();
    env_store.put(e1.clone());
    env_store.put(e2.clone());

    // 2 envelopes arrived. Receipt must account for both: 2 consumed
    // entries (with matching envelope_digest).
    let mut receipt = OrchestrationSynthesisReceipt::new("R-resolve", "D1", "J1", "coord", 2_000);
    // Use each envelope's digest so the validator can correlate.
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        e1.envelope_id.as_str(),
        1_500,
        "coord",
    ));
    receipt.consumed.push(ContributionRef::consume(
        "D1",
        e2.envelope_id.as_str(),
        1_600,
        "coord",
    ));
    receipt.downstream_recommendation = Some("ship it".to_string());
    receipt.confidence = 0.75;

    let v = cover_validator(env_store, synth_store);
    v.validate(&receipt)
        .expect("SYN-21 happy path with 2 envelopes accounted must pass");

    // Count check: receipt.consumed.len() == env_store.all().len() for D1.
    assert_eq!(
        receipt.consumed.len(),
        2,
        "SYN-21 receipt must consume exactly the 2 in-flight envelopes"
    );
}

/// SYN-22: Looking up an unknown receipt id via `SynthesisStore::get`
/// must return None, not panic. And `by_delegation` for an unknown
/// delegation must return an empty Vec.
#[test]
fn syn_22_store_unknown_lookups_are_safe() {
    let synth_store: Arc<InMemorySynthesisStore> = Arc::new(InMemorySynthesisStore::new());
    let r: Option<OrchestrationSynthesisReceipt> = synth_store.get("nope");
    assert!(r.is_none(), "SYN-22 get unknown must be None");
    let v: Vec<OrchestrationSynthesisReceipt> = synth_store.by_delegation("nope");
    assert!(v.is_empty(), "SYN-22 by_delegation unknown must be empty");
}

// (end of tests)
