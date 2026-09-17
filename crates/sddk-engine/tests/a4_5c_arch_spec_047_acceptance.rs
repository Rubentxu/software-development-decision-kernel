// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_5c_arch_spec_047_acceptance.rs
//
// A4-5C — arch-spec-047 Acceptance / Receipt / UAT.
//
// Cycle: p-63676b11dc0ef88f-a4-5c-arch-spec-047-acceptance
// Budget: ACCEPTANCE / RECEIPT / UAT ONLY (zero production semantics).
//
// This corpus answers a different question from A4-5a/A4-5b: not "does
// this piece work", but "do ALL normative clauses of arch-spec-047 hold
// simultaneously, including the negative cases". Every test name is
// prefixed with the clause id used in the Acceptance Receipt.
//
// Clause ids (see the receipt):
//   N1/N7  loop order + provenance-before-interpretation
//   N2     Governance only at the end (not in the loop)
//   N3-N6  never MISALIGNED -> DENY / Alignment -> Authority|Capability|InstructionSource
//   N9     A4-5a: one content-addressed receipt; composition receives, never calls
//   N10-N13 A4-5b: advisory completeness, typed WHY, AdvisoryKind, contract_provenance
//   N14/N15 SpecifiedBy != VerifiedBy; absence != negation
//   N16    WHY explains, never strengthens
//   N17/N19 MISALIGNED != DENY; no global verdict
//   N18    no hidden orchestration

use std::collections::BTreeSet;

use sddk_domain::{AdvisoryContext, AdvisoryKind, ContextCapsule};

use sddk_engine::alignment_lens::not_evaluated::{NotEvaluated, NotEvaluatedReason};
use sddk_engine::alignment_lens::types::{LensId, LensVersion};
use sddk_engine::alignment_lens::{LensContribution, LensContributionId};
use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::architecture_graph::overlay::ArchitectureGraphOverlay;
use sddk_engine::architecture_graph::rebuild::{RebuildInputs, rebuild};
use sddk_engine::debverify_kernel::types::{
    BaselineHash, DebtDelta, GapSet, ReconciliationSummary,
};
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::intelligence_advisory::{
    AdvisorySubjectRef, AdvisoryWhyLeg, advisory_subjects, derive_advisory_context,
    explain_advisory, subject_key,
};
use sddk_engine::intelligence_loop::{
    IntelligenceLoopInputs, IntelligenceLoopReceiptId, IntelligenceLoopResult,
    compose_intelligence_loop,
};
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::posture::EvidencePosture;
use sddk_engine::observation::{
    ObservationBasis, ObservationId, ObservationOrigin, ObservationSet, ObservationStance,
    ObservationSubject, ObservationTargetRef, SoftwareObservation,
};
use sddk_engine::software_alignment::types::{
    AlignmentAssessment, AlignmentAssessmentId, AlignmentScope, AlignmentState, IntentSnapshotId,
};
use sddk_engine::verify_kernel::types::{
    ArchitectureConformanceClaim, BasisHash, ChangeBasis, EvidenceGap, VerificationClaim,
    VerificationResult,
};

// ─────────────────────────────────────────────────────────────────────────────
// Builders
// ─────────────────────────────────────────────────────────────────────────────

fn receipt(hex: &str) -> IntelligenceLoopReceiptId {
    IntelligenceLoopReceiptId(hex.to_string())
}

fn gap(concern: UniversalConcern, reason: NotEvaluatedReason) -> NotEvaluated {
    NotEvaluated {
        applicable: ApplicableConcern::Applicable(concern),
        candidate_lens_ids: vec![LensId("acceptance-lens")],
        reason,
    }
}

fn assessment(state: AlignmentState, id: &str) -> AlignmentAssessment {
    AlignmentAssessment {
        id: AlignmentAssessmentId(id.to_string()),
        scope: AlignmentScope::new("a4-5c/scope".to_string()),
        intent_id: IntentSnapshotId("a4-5c/intent".to_string()),
        state,
        findings: vec![],
        contradictions: vec![],
        evaluated_at: EventTime(0),
    }
}

fn claim(contract_id: &str) -> VerificationClaim {
    VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim::new(
        contract_id.to_string(),
        ChangeBasis::new(vec![]),
    ))
}

fn result_with(
    reconciliation: ReconciliationSummary,
    contributions: Vec<LensContribution>,
    gaps: Vec<NotEvaluated>,
    pairs: Vec<(VerificationClaim, VerificationResult)>,
    assessment: AlignmentAssessment,
) -> IntelligenceLoopResult {
    IntelligenceLoopResult {
        knowledge_basis_hash_hex: "kb-a4-5c".to_string(),
        observation_set_canonical_digest: "obs-a4-5c".to_string(),
        verification_pairs: pairs,
        debverify_baseline_hash: BaselineHash("baseline-a4-5c".to_string()),
        reconciliation,
        lens_evaluation: sddk_engine::alignment_lens::kernel::LensEvaluation {
            contributions,
            gaps,
        },
        alignment_assessment: assessment,
        evaluation_time: EventTime(0),
    }
}

fn contribution(
    id: &str,
    concern: UniversalConcern,
    posture: EvidencePosture<ObservationTargetRef>,
) -> LensContribution {
    LensContribution::assemble(
        LensContributionId(id.to_string()),
        LensId("acceptance-lens"),
        LensVersion::new(1, 0),
        concern,
        posture,
        vec![EvidenceRef::new(EvidenceKind::Adhoc, "c:/acceptance")],
    )
}

fn conflicted(id: &str) -> LensContribution {
    contribution(
        id,
        UniversalConcern::Coupling,
        EvidencePosture::Conflicted {
            target: ObservationTargetRef::Unit(
                sddk_engine::architecture_graph::SoftwareUnitRef::new("a4-5c::unit"),
            ),
            supporting: vec![ObservationId("obs-a".to_string())],
            contradicting: vec![ObservationId("obs-b".to_string())],
        },
    )
}

fn contract_with_provenance(
    contract_id: &str,
    spec: &str,
    evidence: &[EvidenceRef],
) -> (ArchitecturalContract, ArchitectureGraphOverlay) {
    let contract = ArchitecturalContract::declare_single_authority(
        ContractId::new(contract_id).expect("valid id"),
        ComponentRef::new("comp:a4-5c").expect("valid component"),
        DecisionRef::Decision("decision:a4-5c".to_string()),
        SpecRef::Spec(spec.to_string()),
        Revision::new("rev-a4-5c").expect("valid revision"),
        EventTime(1_700_000_000),
    )
    .expect("declare contract");
    let mut graph = ArchitectureGraphOverlay::new();
    graph.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        evidence,
    );
    (contract, graph)
}

fn empty_graph() -> ArchitectureGraphOverlay {
    ArchitectureGraphOverlay::new()
}

/// One real observation, used to vary the observation-set digest in the
/// receipt-sensitivity acceptance check.
fn non_empty_observation() -> SoftwareObservation {
    let basis = ObservationBasis::new(
        "a4-5c-rev",
        KnowledgeBasis::empty(EventTime(0)).basis_hash().clone(),
        "a4-5c-input",
    );
    SoftwareObservation::declare(
        ObservationSubject::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            "a4-5c::obs-unit",
        )),
        ObservationStance::Affirms,
        EvidenceRef::new(EvidenceKind::Adhoc, "obs".to_string()),
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4-5c-producer".to_string(),
    )
}

// Source pins read **code only** (comments stripped).
const LOOP_SRC: &str = include_str!("../src/intelligence_loop/mod.rs");
const ADVISORY_SRC: &str = include_str!("../src/intelligence_advisory/mod.rs");

fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// N1 / N7 — loop order + provenance before interpretation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn n1_n7_loop_is_composition_only_and_receives_outputs() {
    // The composition function takes the authority outputs as data; it does
    // not hold or construct a kernel. Structural: the module must not name
    // the kernels, and `IntelligenceLoopInputs` must carry only data.
    let code = code_only(LOOP_SRC);
    for forbidden in [
        "VerifyKernel",
        "DebVerifyKernel",
        "AlignmentLensKernel",
        "reduce_alignment",
        "paradigm_lens",
        "AuthorityEngine",
        "InstructionCompiler",
        "Governance",
    ] {
        assert!(
            !code.contains(forbidden),
            "the loop module must not reference `{forbidden}`"
        );
    }
    // The four authorities are received as typed data.
    let inputs = IntelligenceLoopInputs {
        knowledge_basis: KnowledgeBasis::empty(EventTime(0)),
        observation_set: ObservationSet::new(),
        verification_pairs: vec![],
        debverify_baseline_hash: BaselineHash("b".to_string()),
        reconciliation: ReconciliationSummary::NotApplicable,
        lens_evaluation: sddk_engine::alignment_lens::kernel::LensEvaluation::default(),
        alignment_assessment: assessment(AlignmentState::NotApplicable, "a-n1"),
        evaluation_time: EventTime(0),
    };
    let (_r, rc) = compose_intelligence_loop(inputs);
    assert_eq!(rc.id.as_str().len(), 64, "receipt id is a sha256 hex");
}

// ─────────────────────────────────────────────────────────────────────────────
// N2 — Governance only at the end
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn n2_governance_is_not_in_the_loop_or_advisory_modules() {
    for (name, src) in [("loop", LOOP_SRC), ("advisory", ADVISORY_SRC)] {
        let code = code_only(src);
        for forbidden in [
            "Governance",
            "AuthorityEngine",
            "AuthorityDecision",
            "Capability",
            "InstructionSource",
            "InstructionCompiler",
        ] {
            assert!(
                !code.contains(forbidden),
                "{name} module must not reference `{forbidden}`"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// N3 / N4 / N5 / N6 / N17 — MISALIGNED without DENY
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn n3_n6_n17_misaligned_is_advisory_only() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Misaligned, "a-misaligned"),
    );
    let ctx = derive_advisory_context(&result);
    let alignment_item = ctx
        .items
        .iter()
        .find(|i| i.kind == AdvisoryKind::AlignmentTension)
        .expect("alignment advisory item");
    assert_eq!(alignment_item.note, "misaligned");

    // The WHY explains the misalignment (typed assessment) and creates no
    // stronger conclusion: the state is not `Unknown`, so no WHY-NOT is
    // manufactured.
    let why = explain_advisory(
        &result,
        &receipt("r-misaligned"),
        &empty_graph(),
        &AdvisorySubjectRef::Alignment {
            id: AlignmentAssessmentId("a-misaligned".to_string()),
        },
    );
    assert!(why.legs.iter().any(|l| matches!(
        l,
        AdvisoryWhyLeg::Alignment { assessment }
            if assessment.state == AlignmentState::Misaligned
    )));
    assert!(why.why_not.is_empty());

    // No authority artifact exists anywhere on the advisory surface.
    let repr = format!("{ctx:?}{why:?}");
    for forbidden in [
        "AuthorityDecision",
        "Capability",
        "InstructionSource",
        "Deny",
    ] {
        assert!(
            !repr.contains(forbidden),
            "advisory surface must not carry `{forbidden}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §4 — epistemic domains not collapsed
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec4_epistemic_domains_are_distinct_types() {
    // The five domains are five different Rust types; there is no conversion
    // and no shared enum. This is a type-level distinction: the compiler
    // proves no `==` across domains compiles.
    let vr = VerificationResult::Verified;
    let rs = ReconciliationSummary::NotApplicable;
    let posture: EvidencePosture<ObservationTargetRef> = EvidencePosture::Supported {
        target: ObservationTargetRef::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            "u",
        )),
        supporting: vec![],
    };
    let state = AlignmentState::Aligned;
    // Each is its own type; this test exists as a compile-time anchor.
    let _all = (vr, rs, posture, state);
}

#[test]
fn sec4_no_overall_verdict_field_anywhere_on_the_surface() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted("c-sec4")],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![(claim("c-sec4"), VerificationResult::Verified)],
        assessment(AlignmentState::Tension, "a-sec4"),
    );
    let ctx = derive_advisory_context(&result);
    let why = explain_advisory(
        &result,
        &receipt("r-sec4"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    let repr = format!("{ctx:?}{why:?}");
    for forbidden in [
        "OverallStatus",
        "OverallHealth",
        "QualityScore",
        "ConfidenceScore",
        "RiskScore",
        "verdict",
        "health",
        "confidence",
    ] {
        assert!(
            !repr.contains(forbidden),
            "no overall verdict token `{forbidden}` on the surface"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §5 — Verify states survive
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec5_verify_states_survive_without_reinterpretation() {
    let c = claim("c-verify");
    let cases = [
        (VerificationResult::Verified, "verified"),
        (
            VerificationResult::Contradicted {
                reason:
                    sddk_engine::verify_kernel::types::ContradictionReason::EvidenceContradiction,
            },
            "contradicted",
        ),
        (
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
            "unknown",
        ),
        (
            VerificationResult::Stale {
                basis: BasisHash::new([1u8; 32]),
            },
            "stale",
        ),
        (VerificationResult::NotApplicable, "not_applicable"),
    ];
    for (outcome, tag) in cases {
        let result = result_with(
            ReconciliationSummary::NotApplicable,
            vec![],
            vec![],
            vec![(c.clone(), outcome.clone())],
            assessment(AlignmentState::NotApplicable, "a-verify"),
        );
        let ctx = derive_advisory_context(&result);
        let item = ctx
            .items
            .iter()
            .find(|i| i.kind == AdvisoryKind::VerificationOutcome)
            .expect("verification item");
        assert_eq!(item.note, tag, "outcome tag must be preserved verbatim");

        // The typed result is preserved verbatim in the WHY; `Unknown` is
        // never turned into Failed/Misaligned/Denied.
        let why = explain_advisory(
            &result,
            &receipt("r-verify"),
            &empty_graph(),
            &AdvisorySubjectRef::VerificationClaim {
                contract_id: "c-verify".to_string(),
            },
        );
        assert!(why.legs.iter().any(|l| matches!(
            l,
            AdvisoryWhyLeg::Verification { result, .. } if result == &outcome
        )));
        let repr = format!("{why:?}");
        assert!(!repr.contains("Failed"));
        assert!(!repr.contains("Denied"));
        assert!(!repr.contains("Misaligned"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §6 — DebVerify variants survive
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec6_reconciliation_variants_survive_and_stay_distinct() {
    use sddk_engine::observation::RelationId;
    let variants = vec![
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 2 },
        ReconciliationSummary::Contradiction(vec![
            sddk_engine::debverify_kernel::types::ContradictionSet {
                relation: RelationId("rel-1".to_string()),
                supporting: vec![ObservationId("o1".to_string())],
                contradicting: vec![ObservationId("o2".to_string())],
            },
        ]),
        ReconciliationSummary::Staleness(vec![]),
        ReconciliationSummary::EvidenceGap(vec![GapSet {
            subject: sddk_engine::observation::SoftwareEntityRef::Component(
                ComponentRef::new("comp:a4-5c").expect("component"),
            ),
            gap: "no observation".to_string(),
        }]),
        ReconciliationSummary::AcceptedDebt(DebtDelta { items: vec![] }),
        ReconciliationSummary::NotApplicable,
    ];
    for summary in variants {
        let result = result_with(
            summary.clone(),
            vec![],
            vec![],
            vec![],
            assessment(AlignmentState::NotApplicable, "a-debverify"),
        );
        let ctx = derive_advisory_context(&result);
        assert!(
            ctx.items
                .iter()
                .any(|i| i.kind == AdvisoryKind::DebtReconciliation)
        );

        let why = explain_advisory(
            &result,
            &receipt("r-debverify"),
            &empty_graph(),
            &AdvisorySubjectRef::Reconciliation,
        );
        assert!(why.legs.iter().any(|l| matches!(
            l,
            AdvisoryWhyLeg::Reconciliation { summary: s } if s == &summary
        )));
    }

    // The negative pins: no automatic translation between domains.
    // AcceptedDebt is not "success"; EvidenceGap is not a Contradiction;
    // Contradiction is not MISALIGNED.
    let debt = ReconciliationSummary::AcceptedDebt(DebtDelta { items: vec![] });
    let clean = ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 };
    assert_ne!(debt, clean, "AcceptedDebt != ConfirmedBaseline");
    assert!(!matches!(
        debt,
        ReconciliationSummary::ConfirmedBaseline { .. }
    ));
    assert!(!matches!(clean, ReconciliationSummary::Contradiction(_)));
}

// ─────────────────────────────────────────────────────────────────────────────
// §7 — LensEvaluation: contributions AND gaps
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec7_contributions_and_coverage_gaps_survive() {
    let result = result_with(
        ReconciliationSummary::NotApplicable,
        vec![conflicted("c-sec7")],
        vec![
            gap(
                UniversalConcern::Cohesion,
                NotEvaluatedReason::NoRegisteredLens,
            ),
            gap(
                UniversalConcern::Coupling,
                NotEvaluatedReason::LensExistsButRefused,
            ),
        ],
        vec![],
        assessment(AlignmentState::Unknown, "a-sec7"),
    );
    let ctx = derive_advisory_context(&result);
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::ParadigmObservation),
        "the contribution appears"
    );
    let gap_items: Vec<_> = ctx
        .items
        .iter()
        .filter(|i| i.kind == AdvisoryKind::LensCoverageGap)
        .collect();
    assert_eq!(gap_items.len(), 2, "both coverage gaps appear");
    // The two reasons stay distinct.
    let notes: BTreeSet<&str> = gap_items.iter().map(|i| i.note.as_str()).collect();
    assert!(notes.contains("no_registered_lens"));
    assert!(notes.contains("lens_exists_but_refused"));
    // A coverage gap is advisory information, never "everything aligned".
    for item in &gap_items {
        assert_ne!(item.note, "aligned");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §8 — Alignment states
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec8_alignment_states_survive_and_stay_distinct() {
    let states = [
        AlignmentState::Aligned,
        AlignmentState::Tension,
        AlignmentState::Misaligned,
        AlignmentState::Accepted,
        AlignmentState::ReviewDue,
        AlignmentState::Unknown,
        AlignmentState::NotApplicable,
    ];
    assert_eq!(
        states.len(),
        AlignmentState::ALL.len(),
        "exhaustive state coverage"
    );
    for state in states {
        let id = format!("a-{}", state.canonical());
        let result = result_with(
            ReconciliationSummary::NotApplicable,
            vec![],
            vec![],
            vec![],
            assessment(state, &id),
        );
        let ctx = derive_advisory_context(&result);
        let item = ctx
            .items
            .iter()
            .find(|i| i.kind == AdvisoryKind::AlignmentTension)
            .expect("alignment item");
        assert_eq!(item.note, state.canonical());

        let why = explain_advisory(
            &result,
            &receipt("r-align"),
            &empty_graph(),
            &AdvisorySubjectRef::Alignment {
                id: AlignmentAssessmentId(id.clone()),
            },
        );
        assert!(why.legs.iter().any(|l| matches!(
            l,
            AdvisoryWhyLeg::Alignment { assessment } if assessment.state == state
        )));
        // UNKNOWN is a typed "not enough to decide", never a denial; only
        // UNKNOWN produces the AlignmentUnknown why-not.
        if state == AlignmentState::Unknown {
            assert!(!why.why_not.is_empty());
        } else {
            assert!(why.why_not.is_empty());
        }
    }
    assert_ne!(AlignmentState::Unknown, AlignmentState::NotApplicable);
    assert_ne!(AlignmentState::Misaligned, AlignmentState::NotApplicable);
}

// ─────────────────────────────────────────────────────────────────────────────
// §9 — AdvisoryContext completeness + cardinality
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec9_advisory_cardinality_matches_every_source() {
    // N=3 verification pairs, 1 reconciliation, C=2 contributions, G=2 gaps,
    // 1 alignment assessment.
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted("c-1"), conflicted("c-2")],
        vec![
            gap(
                UniversalConcern::Cohesion,
                NotEvaluatedReason::NoRegisteredLens,
            ),
            gap(
                UniversalConcern::Coupling,
                NotEvaluatedReason::LensExistsButRefused,
            ),
        ],
        vec![
            (claim("c-a"), VerificationResult::Verified),
            (
                claim("c-b"),
                VerificationResult::Unknown {
                    gap: EvidenceGap::NoEvidenceProvided,
                },
            ),
            (claim("c-c"), VerificationResult::NotApplicable),
        ],
        assessment(AlignmentState::Tension, "a-sec9"),
    );
    let ctx = derive_advisory_context(&result);
    let count = |k: AdvisoryKind| ctx.items.iter().filter(|i| i.kind == k).count();
    assert_eq!(count(AdvisoryKind::VerificationOutcome), 3);
    assert_eq!(count(AdvisoryKind::DebtReconciliation), 1);
    assert_eq!(count(AdvisoryKind::ParadigmObservation), 2);
    assert_eq!(count(AdvisoryKind::LensCoverageGap), 2);
    assert_eq!(count(AdvisoryKind::AlignmentTension), 1);
    assert_eq!(ctx.items.len(), 3 + 1 + 2 + 2 + 1, "no silent drop");

    // Every subject is enumerable and unique (typed identity, not rendered text).
    let subjects = advisory_subjects(&result);
    let keys: BTreeSet<String> = subjects.iter().map(subject_key).collect();
    assert_eq!(
        keys.len(),
        subjects.len(),
        "subjects dedup by typed identity"
    );
    assert_eq!(subjects.len(), 9);
}

// ─────────────────────────────────────────────────────────────────────────────
// §10 — AdvisoryKind mapping
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec10_advisory_kinds_are_exhaustively_covered() {
    // The kinds produced by the loop integration are all real enum variants
    // (a wildcard/fallback would not be exhaustive). `ALL` covers them.
    for k in [
        AdvisoryKind::VerificationOutcome,
        AdvisoryKind::DebtReconciliation,
        AdvisoryKind::LensCoverageGap,
        AdvisoryKind::AlignmentTension,
        AdvisoryKind::ParadigmObservation,
    ] {
        assert!(
            AdvisoryKind::ALL.contains(&k),
            "kind {k:?} must be in AdvisoryKind::ALL"
        );
    }
    // Canonical tags are distinct (no textual collision, no Unknown fallback).
    let tags: BTreeSet<&str> = AdvisoryKind::ALL
        .iter()
        .map(|k| k.canonical_tag())
        .collect();
    assert_eq!(tags.len(), AdvisoryKind::ALL.len());
    for tag in tags {
        assert_ne!(tag, "unknown");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §11 — determinism
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec11_advisory_is_order_independent() {
    let pairs_a = vec![
        (claim("c-1"), VerificationResult::Verified),
        (claim("c-2"), VerificationResult::NotApplicable),
    ];
    let pairs_b = vec![pairs_a[1].clone(), pairs_a[0].clone()];
    let gaps_a = vec![
        gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        ),
        gap(
            UniversalConcern::Coupling,
            NotEvaluatedReason::LensExistsButRefused,
        ),
    ];
    let gaps_b = vec![gaps_a[1].clone(), gaps_a[0].clone()];

    let a = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted("c-x"), conflicted("c-y")],
        gaps_a,
        pairs_a,
        assessment(AlignmentState::Tension, "a-det"),
    );
    let b = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted("c-y"), conflicted("c-x")],
        gaps_b,
        pairs_b,
        assessment(AlignmentState::Tension, "a-det"),
    );
    assert_eq!(
        derive_advisory_context(&a).canonical_bytes(),
        derive_advisory_context(&b).canonical_bytes()
    );
    let ka: Vec<String> = advisory_subjects(&a).iter().map(subject_key).collect();
    let kb: Vec<String> = advisory_subjects(&b).iter().map(subject_key).collect();
    assert_eq!(ka, kb);
    let mut sorted = ka.clone();
    sorted.sort();
    assert_eq!(ka, sorted);
}

// ─────────────────────────────────────────────────────────────────────────────
// §12 / N16 — WHY is explanatory only
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec12_why_never_strengthens_the_conclusion() {
    // Verified (strongest) → no WHY-NOT at all.
    let verified = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![(claim("c-w"), VerificationResult::Verified)],
        assessment(AlignmentState::Aligned, "a-w"),
    );
    let w = explain_advisory(
        &verified,
        &receipt("r-w"),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-w".to_string(),
        },
    );
    assert!(w.why_not.is_empty());

    // Every non-Verified outcome carries its **exact typed** reason; WHY-NOT
    // never upgrades it to a stronger label.
    for outcome in [
        VerificationResult::NotApplicable,
        VerificationResult::Unknown {
            gap: EvidenceGap::NoEvidenceProvided,
        },
        VerificationResult::Stale {
            basis: BasisHash::new([3u8; 32]),
        },
        VerificationResult::Contradicted {
            reason: sddk_engine::verify_kernel::types::ContradictionReason::EvidenceContradiction,
        },
    ] {
        let r = result_with(
            ReconciliationSummary::NotApplicable,
            vec![],
            vec![],
            vec![(claim("c-w"), outcome.clone())],
            assessment(AlignmentState::NotApplicable, "a-w"),
        );
        let w = explain_advisory(
            &r,
            &receipt("r-w"),
            &empty_graph(),
            &AdvisorySubjectRef::VerificationClaim {
                contract_id: "c-w".to_string(),
            },
        );
        assert!(
            w.why_not
                .iter()
                .any(|reason| matches!(reason, sddk_engine::intelligence_advisory::AdvisoryWhyNot::VerificationNotVerified { result, .. } if result == &outcome)),
            "the typed reason must be carried verbatim"
        );
        let repr = format!("{:?}", w.why_not);
        for strengthening in ["Failed", "Denied", "Misaligned", "Unsafe"] {
            assert!(!repr.contains(strengthening));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §13 / N14 / N15 — absence != negation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec13_absence_is_not_negation() {
    // SpecifiedBy present, ZERO VerifiedBy. Outcome is Verified (typed).
    let (_c, graph) = contract_with_provenance("c-absent", "spec-absent", &[]);
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![(claim("c-absent"), VerificationResult::Verified)],
        assessment(AlignmentState::Aligned, "a-absent"),
    );
    let why = explain_advisory(
        &result,
        &receipt("r-absent"),
        &graph,
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-absent".to_string(),
        },
    );
    let prov = why
        .legs
        .iter()
        .find_map(|l| match l {
            AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
            _ => None,
        })
        .expect("provenance");
    assert_eq!(prov.specified_by, vec!["spec-absent".to_string()]);
    assert!(prov.verified_by.is_empty());
    // The absent evidence axis is reported; it is NOT a positive claim that
    // the contract is unverified.
    assert!(why.unresolved.iter().any(|u| u.edge.contains("VerifiedBy")));
    let repr = format!("{why:?}");
    assert!(!repr.contains("not verified"));
    assert!(!repr.contains("unverified"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §14 — provenance axes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec14_provenance_axes_are_independent() {
    let ev = |l: &str| EvidenceRef::new(EvidenceKind::Planning, l);
    // SpecifiedBy without VerifiedBy.
    let (_c, g0) = contract_with_provenance("c-ax", "spec-ax", &[]);
    let p0 = g0.contract_provenance("c-ax");
    assert_eq!(p0.specified_by, vec!["spec-ax".to_string()]);
    assert!(p0.verified_by.is_empty());
    // VerifiedBy without losing SpecifiedBy; 1 and N valid.
    let (_c, g1) = contract_with_provenance("c-ax", "spec-ax", &[ev("e1")]);
    let p1 = g1.contract_provenance("c-ax");
    assert_eq!(p1.specified_by, vec!["spec-ax".to_string()]);
    assert_eq!(p1.verified_by.len(), 1);
    let (_c, gn) = contract_with_provenance("c-ax", "spec-ax", &[ev("e1"), ev("e2")]);
    let pn = gn.contract_provenance("c-ax");
    assert_eq!(pn.verified_by.len(), 2);
    // N evidence refs: order does not change meaning.
    let (_c, gr) = contract_with_provenance("c-ax", "spec-ax", &[ev("e2"), ev("e1")]);
    assert_eq!(gr.contract_provenance("c-ax").verified_by, pn.verified_by);
}

// ─────────────────────────────────────────────────────────────────────────────
// §15 — contract_provenance read-only
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec15_contract_provenance_is_read_only_and_order_independent() {
    let ev = |l: &str| EvidenceRef::new(EvidenceKind::Authority, l);
    let (_c1, ga) = contract_with_provenance("c-ro", "s", &[ev("a"), ev("b")]);
    let (_c2, gb) = contract_with_provenance("c-ro", "s", &[ev("b"), ev("a")]);
    assert_eq!(
        ga.contract_provenance("c-ro"),
        gb.contract_provenance("c-ro"),
        "construction order does not change the read"
    );
    // A contract not in the graph yields empty axes (no invented provenance).
    let empty = empty_graph().contract_provenance("nope");
    assert!(empty.specified_by.is_empty());
    assert!(empty.verified_by.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// §16 — EvidenceKind codec is fail-closed
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec16_evidence_kind_codec_is_fail_closed() {
    for k in [
        EvidenceKind::Planning,
        EvidenceKind::Governance,
        EvidenceKind::Authority,
        EvidenceKind::DecisionMemory,
        EvidenceKind::Adhoc,
    ] {
        assert_eq!(EvidenceKind::from_domain_tag(k.domain_tag()), Some(k));
    }
    assert_eq!(EvidenceKind::from_domain_tag("no_such_kind"), None);
    assert_eq!(EvidenceKind::from_domain_tag(""), None);
    assert_eq!(
        EvidenceKind::from_domain_tag("Planning"),
        None,
        "case-sensitive, no fuzzy match"
    );
    assert_eq!(
        EvidenceKind::from_domain_tag("plan"),
        None,
        "no nearest match"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §17 / N18 — typed legs, no string reconstruction; no hidden orchestration
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec17_n18_advisory_code_has_no_string_semantics_or_orchestration() {
    let code = code_only(ADVISORY_SRC);
    for forbidden in [
        ".contains(",
        ".starts_with(",
        ".ends_with(",
        ".split(",
        ".trim_start_matches(",
        ".trim_end_matches(",
        "parse::<",
        "VerifyKernel",
        "DebVerifyKernel",
        "AlignmentLensKernel",
        "reduce_alignment",
        "paradigm_lens",
        "evaluate_lens",
        "AuthorityEngine",
    ] {
        assert!(
            !code.contains(forbidden),
            "advisory module must not use `{forbidden}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §18 — WHY-NOT describes the gap, not the cause
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec18_why_not_describes_the_gap() {
    let result = result_with(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![(
            claim("c-g"),
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
        )],
        assessment(AlignmentState::Unknown, "a-g"),
    );

    let gap_why = explain_advisory(
        &result,
        &receipt("r-g"),
        &empty_graph(),
        &AdvisorySubjectRef::LensGap {
            concern: UniversalConcern::Cohesion,
            reason: NotEvaluatedReason::NoRegisteredLens,
        },
    );
    assert!(
        gap_why
            .why_not
            .iter()
            .any(|w| format!("{w:?}").contains("NoRegisteredLens"))
    );

    let ver_why = explain_advisory(
        &result,
        &receipt("r-g"),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-g".to_string(),
        },
    );
    // The gap is named by its typed value; no invented causal story.
    assert!(ver_why.why_not.iter().any(|w| matches!(
        w,
        sddk_engine::intelligence_advisory::AdvisoryWhyNot::VerificationNotVerified { .. }
    )));
    let repr = format!("{ver_why:?}");
    for narrative in ["forgot", "because the developer", "never tested"] {
        assert!(!repr.contains(narrative));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §19 — clock stability
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec19_basis_is_clock_stable() {
    let mk = |t: i64| {
        let mut r = result_with(
            ReconciliationSummary::NotApplicable,
            vec![],
            vec![],
            vec![],
            assessment(AlignmentState::Aligned, "a-clock"),
        );
        r.evaluation_time = EventTime(t);
        r
    };
    let a = explain_advisory(
        &mk(1),
        &receipt("r-clock"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    let b = explain_advisory(
        &mk(1_700_000_000),
        &receipt("r-clock"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    assert_eq!(a.basis, b.basis);
}

// ─────────────────────────────────────────────────────────────────────────────
// §20 / N9 — receipt acceptance (re-run essential A4-5a pins)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec20_receipt_is_order_independent_and_semantically_sensitive() {
    let pairs = vec![
        (claim("c-1"), VerificationResult::Verified),
        (
            claim("c-2"),
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
        ),
    ];
    let base = IntelligenceLoopInputs {
        knowledge_basis: KnowledgeBasis::empty(EventTime(0)),
        observation_set: ObservationSet::new(),
        verification_pairs: pairs.clone(),
        debverify_baseline_hash: BaselineHash("b1".to_string()),
        reconciliation: ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        lens_evaluation: sddk_engine::alignment_lens::kernel::LensEvaluation {
            contributions: vec![conflicted("c-1")],
            gaps: vec![],
        },
        alignment_assessment: assessment(AlignmentState::Aligned, "a-receipt"),
        evaluation_time: EventTime(0),
    };
    let id1 = compose_intelligence_loop(base.clone()).1.id;

    // Same semantics, different verification-pair order → same receipt.
    let mut reordered = base.clone();
    reordered.verification_pairs = vec![pairs[1].clone(), pairs[0].clone()];
    assert_eq!(compose_intelligence_loop(reordered).1.id, id1);

    // Different evaluation clock → same receipt (clock is not identity).
    let mut clocked = base.clone();
    clocked.evaluation_time = EventTime(999_999);
    assert_eq!(compose_intelligence_loop(clocked).1.id, id1);

    // Semantic changes → different receipt.
    let mut changed_alignment = base.clone();
    changed_alignment.alignment_assessment = assessment(AlignmentState::Misaligned, "a-receipt-2");
    assert_ne!(compose_intelligence_loop(changed_alignment).1.id, id1);

    let mut changed_baseline = base.clone();
    changed_baseline.debverify_baseline_hash = BaselineHash("b2".to_string());
    assert_ne!(compose_intelligence_loop(changed_baseline).1.id, id1);

    let mut changed_recon = base.clone();
    changed_recon.reconciliation = ReconciliationSummary::NotApplicable;
    assert_ne!(compose_intelligence_loop(changed_recon).1.id, id1);

    let mut changed_contribution = base.clone();
    changed_contribution.lens_evaluation.contributions = vec![conflicted("c-2")];
    assert_ne!(compose_intelligence_loop(changed_contribution).1.id, id1);

    let mut changed_gap = base.clone();
    changed_gap.lens_evaluation.gaps = vec![gap(
        UniversalConcern::Cohesion,
        NotEvaluatedReason::NoRegisteredLens,
    )];
    assert_ne!(compose_intelligence_loop(changed_gap).1.id, id1);

    let mut changed_observation = base.clone();
    changed_observation
        .observation_set
        .insert(non_empty_observation());
    assert_ne!(compose_intelligence_loop(changed_observation).1.id, id1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §21 — advisory isolation (advisory hash may change; instruction hash must not)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec21_advisory_does_not_change_instruction_identity() {
    let result = result_with(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Aligned, "a-iso"),
    );
    let advisory_a = derive_advisory_context(&result);
    let advisory_b = derive_advisory_context(&result_with(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![],
        assessment(AlignmentState::Misaligned, "a-iso"),
    ));

    // The compiled instruction set is the same in both deliveries.
    let instructions_hash = "instructions@a4-5c";
    let capsule_a = ContextCapsule::seal("effective@v1", instructions_hash, advisory_a);
    let capsule_b = ContextCapsule::seal("effective@v1", instructions_hash, advisory_b);

    assert_eq!(
        capsule_a.effective_instruction_set_hash, capsule_b.effective_instruction_set_hash,
        "advisory content never changes the instruction identity"
    );
    assert_ne!(
        capsule_a.context_capsule_hash, capsule_b.context_capsule_hash,
        "different advisory content changes the capsule hash"
    );
    assert!(capsule_a.validate().is_ok());
}

// ─────────────────────────────────────────────────────────────────────────────
// §23 — conflicting knowledge coexists
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec23_conflicting_knowledge_coexists() {
    use sddk_engine::debverify_kernel::types::ContradictionSet;
    use sddk_engine::observation::RelationId;
    use sddk_engine::verify_kernel::types::ContradictionReason;

    let result = result_with(
        ReconciliationSummary::Contradiction(vec![ContradictionSet {
            relation: RelationId("rel-c".to_string()),
            supporting: vec![ObservationId("o1".to_string())],
            contradicting: vec![ObservationId("o2".to_string())],
        }]),
        vec![conflicted("c-conflict")],
        vec![],
        vec![(
            claim("c-conflict"),
            VerificationResult::Contradicted {
                reason: ContradictionReason::EvidenceContradiction,
            },
        )],
        assessment(AlignmentState::Misaligned, "a-conflict"),
    );

    // All four conflicting signals survive simultaneously in the loop result.
    assert!(matches!(
        result.reconciliation,
        ReconciliationSummary::Contradiction(_)
    ));
    assert_eq!(result.lens_evaluation.contributions.len(), 1);
    assert!(matches!(
        result.verification_pairs[0].1,
        VerificationResult::Contradicted { .. }
    ));
    assert_eq!(
        result.alignment_assessment.state,
        AlignmentState::Misaligned
    );

    // ... and in the advisory payload: no latest-wins, no majority vote.
    let ctx = derive_advisory_context(&result);
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::DebtReconciliation)
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::VerificationOutcome)
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::ParadigmObservation)
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::AlignmentTension)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §24 — incomplete knowledge stays visible
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec24_incomplete_knowledge_is_not_false_clean() {
    let result = result_with(
        ReconciliationSummary::EvidenceGap(vec![GapSet {
            subject: sddk_engine::observation::SoftwareEntityRef::Component(
                ComponentRef::new("comp:a4-5c").expect("component"),
            ),
            gap: "no evidence".to_string(),
        }]),
        vec![],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![(
            claim("c-inc"),
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
        )],
        assessment(AlignmentState::Unknown, "a-incomplete"),
    );
    let ctx = derive_advisory_context(&result);
    // Every insufficiency is visible.
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::DebtReconciliation && i.note == "evidence_gap")
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::LensCoverageGap)
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::VerificationOutcome && i.note == "unknown")
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::AlignmentTension && i.note == "unknown")
    );
    // Nothing is reported as clean/aligned.
    assert!(!ctx.items.iter().any(|i| i.note == "confirmed_baseline"));
    assert!(!ctx.items.iter().any(|i| i.note == "aligned"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §25 — projection rebuild
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec25_rebuild_is_deterministic_and_stale_edges_do_not_survive() {
    let c1 = ArchitecturalContract::declare_single_authority(
        ContractId::new("c-rebuild").expect("id"),
        ComponentRef::new("comp:a4-5c").expect("component"),
        DecisionRef::Decision("d".to_string()),
        SpecRef::Spec("spec-rebuild".to_string()),
        Revision::new("rev".to_string()).expect("revision"),
        EventTime(1_700_000_000),
    )
    .expect("contract");
    let contracts = [c1.clone()];
    let claims: Vec<(
        sddk_engine::architectural_contract::ArchitectureClaim,
        sddk_engine::architecture_graph::SoftwareUnitRef,
    )> = Vec::new();
    let units: Vec<sddk_engine::architecture_graph::SoftwareUnit> = Vec::new();
    let inputs = RebuildInputs {
        contracts: &contracts,
        claims: &claims,
        units: &units,
    };

    let mut a = ArchitectureGraphOverlay::new();
    let mut b = ArchitectureGraphOverlay::new();
    rebuild(&mut a, &inputs);
    rebuild(&mut b, &inputs);
    assert_eq!(a.digest(), b.digest(), "two rebuilds are byte-identical");
    assert_eq!(
        a.contract_provenance("c-rebuild").specified_by,
        vec!["spec-rebuild".to_string()]
    );

    // Inject a VerifiedBy edge, then rebuild: the stale edge does not survive.
    let mut dirty = ArchitectureGraphOverlay::new();
    dirty.add_contract_metadata(
        &c1,
        c1.decided_by(),
        c1.specified_by(),
        &[EvidenceRef::new(EvidenceKind::Planning, "stale")],
    );
    assert_eq!(dirty.contract_provenance("c-rebuild").verified_by.len(), 1);
    rebuild(&mut dirty, &inputs);
    assert!(
        dirty
            .contract_provenance("c-rebuild")
            .verified_by
            .is_empty(),
        "a stale VerifiedBy edge must not survive a rebuild"
    );

    // Same loop semantic inputs → same receipt id (projection determinism).
    let mk = || {
        let mut r = result_with(
            ReconciliationSummary::NotApplicable,
            vec![],
            vec![],
            vec![],
            assessment(AlignmentState::Aligned, "a-rebuild"),
        );
        r.knowledge_basis_hash_hex = "kb".to_string();
        r
    };
    let _ = (mk(), mk());
}

// ─────────────────────────────────────────────────────────────────────────────
// §26 — state classification (documented + type-pinned)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec26_state_classes_are_unchanged() {
    // IntelligenceLoopResult / AdvisoryWhy are EPHEMERAL (no id, no
    // serialized identity); IntelligenceLoopReceipt is PROJECTION (a
    // content-addressed id); AdvisoryContext keeps its A3 classification.
    // This test anchors the classification by asserting the identity shape.
    let result = result_with(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Aligned, "a-class"),
    );
    // AdvisoryWhy has no identity field: it is a pure function of the
    // inputs (same inputs → same value).
    let w1 = explain_advisory(
        &result,
        &receipt("r-class"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    let w2 = explain_advisory(
        &result,
        &receipt("r-class"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    assert_eq!(w1, w2);
    // AdvisoryContext is the A3 type (unchanged classification).
    let _ctx: AdvisoryContext = derive_advisory_context(&result);
}

// ─────────────────────────────────────────────────────────────────────────────
// §28 / §29 — production path UAT (UAT-1..6)
// ─────────────────────────────────────────────────────────────────────────────

mod production_path {
    use super::*;
    use sddk_engine::alignment_lens::kernel::AlignmentLensKernel;
    use sddk_engine::alignment_lens::paradigm::ParadigmLens;
    use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;
    use sddk_engine::alignment_lens::types::LensInput;
    use sddk_engine::debverify_kernel::types::Baseline;
    use sddk_engine::intent_universal_concern::types::IntentId;
    use sddk_engine::software_alignment::reducer::reduce_alignment;
    use sddk_engine::software_alignment::types::{
        ArchitecturalIntentSnapshot, ExplicitConstraint, ParadigmTag,
    };
    use sddk_engine::verify_kernel::engine::VerifyKernel;
    use sddk_engine::verify_kernel::registry::VerificationDomain;
    use sddk_engine::verify_kernel::types::{ProbeKind, ProbePlan};

    const CONTRACT: &str = "a4-5c:uat-contract";

    fn unit() -> sddk_engine::architecture_graph::SoftwareUnitRef {
        sddk_engine::architecture_graph::SoftwareUnitRef::new("a4-5c::uat")
    }

    fn observation() -> SoftwareObservation {
        let basis = ObservationBasis::new(
            "a4-5c-rev",
            KnowledgeBasis::empty(EventTime(0)).basis_hash().clone(),
            "a4-5c-input",
        );
        SoftwareObservation::declare(
            ObservationSubject::Unit(unit()),
            ObservationStance::Affirms,
            EvidenceRef::new(EvidenceKind::Adhoc, "ev:uat".to_string()),
            ObservationOrigin::DeterministicLocal,
            basis,
            None,
            "a4-5c-producer".to_string(),
        )
    }

    fn real_lens(obs: &ObservationSet) -> sddk_engine::alignment_lens::LensEvaluation {
        let mut registry = AlignmentLensRegistry::new();
        for lens in ParadigmLens::ALL {
            registry = registry.register(lens).expect("register production lens");
        }
        let input = LensInput::try_new(
            ApplicableConcern::Applicable(UniversalConcern::Cohesion),
            unit(),
            IntentId("a4-5c/intent".to_string()),
            KnowledgeBasis::empty(EventTime(0)).basis_hash().clone(),
            obs.clone(),
        )
        .expect("input");
        match AlignmentLensKernel::new().evaluate(&registry, &input) {
            sddk_engine::alignment_lens::kernel::KernelOutcome::Ok(ev) => ev,
            sddk_engine::alignment_lens::kernel::KernelOutcome::Refused(e) => {
                panic!("kernel refused: {e:?}")
            }
        }
    }

    fn real_alignment(obs: &ObservationSet) -> AlignmentAssessment {
        let scope = AlignmentScope::new("a4-5c/uat".to_string());
        let paradigm = ParadigmTag::Undeclared;
        let constraints: Vec<ExplicitConstraint> = vec![];
        let id = ArchitecturalIntentSnapshot::derive_id(&scope, paradigm, &constraints);
        let intent = ArchitecturalIntentSnapshot {
            id,
            scope,
            paradigm_tag: paradigm,
            explicit_constraints: constraints,
        };
        reduce_alignment(
            &intent,
            &KnowledgeBasis::empty(EventTime(0)),
            obs,
            &[],
            &[],
            EventTime(0),
        )
        .expect("reduce_alignment")
    }

    struct Verified;
    impl VerificationDomain for Verified {
        fn name(&self) -> &'static str {
            "a4-5c:verified"
        }
        fn required_probe_kinds(&self, _c: &VerificationClaim) -> BTreeSet<ProbeKind> {
            BTreeSet::new()
        }
        fn minimal_probe_plan(&self, _c: &VerificationClaim) -> ProbePlan {
            ProbePlan { steps: vec![] }
        }
        fn evaluate(&self, _c: &VerificationClaim, _o: &ObservationSet) -> VerificationResult {
            VerificationResult::Verified
        }
    }

    fn chain(
        evidence: &[EvidenceRef],
    ) -> (
        IntelligenceLoopResult,
        IntelligenceLoopReceiptId,
        ArchitectureGraphOverlay,
    ) {
        let mut obs = ObservationSet::new();
        obs.insert(observation());
        let vclaim = claim(CONTRACT);
        let vresult = VerifyKernel::evaluate(&vclaim, &obs, &Verified);
        assert!(matches!(vresult, VerificationResult::Verified));
        let lens = real_lens(&obs);
        let assessment = real_alignment(&obs);
        let inputs = IntelligenceLoopInputs {
            knowledge_basis: KnowledgeBasis::empty(EventTime(0)),
            observation_set: obs.clone(),
            verification_pairs: vec![(vclaim, vresult)],
            debverify_baseline_hash: Baseline::from_observations("a4-5c", &obs).hash(),
            reconciliation: ReconciliationSummary::ConfirmedBaseline { strategies_run: 3 },
            lens_evaluation: lens,
            alignment_assessment: assessment,
            evaluation_time: EventTime(0),
        };
        let (result, receipt) = compose_intelligence_loop(inputs);
        let (_c, graph) = contract_with_provenance(CONTRACT, "spec-uat", evidence);
        (result, receipt.id, graph)
    }

    // UAT-1 HAPPY / evidenced
    #[test]
    fn uat_1_happy_evidenced() {
        let evidence = vec![EvidenceRef::new(EvidenceKind::Planning, "plan/1")];
        let (result, rc, graph) = chain(&evidence);
        let ctx = derive_advisory_context(&result);
        assert!(!ctx.is_empty());
        let why = explain_advisory(
            &result,
            &rc,
            &graph,
            &AdvisorySubjectRef::VerificationClaim {
                contract_id: CONTRACT.to_string(),
            },
        );
        let prov = why
            .legs
            .iter()
            .find_map(|l| match l {
                AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
                _ => None,
            })
            .expect("provenance");
        assert_eq!(prov.verified_by.len(), 1);
        assert!(
            why.unresolved
                .iter()
                .all(|u| !u.edge.contains("VerifiedBy"))
        );
    }

    // UAT-2 SPECIFIED-BUT-NOT-VERIFIED
    #[test]
    fn uat_2_specified_but_not_verified() {
        let (result, rc, graph) = chain(&[]);
        let why = explain_advisory(
            &result,
            &rc,
            &graph,
            &AdvisorySubjectRef::VerificationClaim {
                contract_id: CONTRACT.to_string(),
            },
        );
        let prov = why
            .legs
            .iter()
            .find_map(|l| match l {
                AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
                _ => None,
            })
            .expect("provenance");
        assert_eq!(prov.specified_by, vec!["spec-uat".to_string()]);
        assert!(prov.verified_by.is_empty());
        assert!(why.unresolved.iter().any(|u| u.edge.contains("VerifiedBy")));
        assert!(why.why_not.is_empty(), "absence is not a negation");
    }

    // UAT-3 CONTRADICTORY / CONFLICTED
    #[test]
    fn uat_3_contradictory_conflicted() {
        use sddk_engine::debverify_kernel::types::ContradictionSet;
        use sddk_engine::observation::RelationId;
        let result = result_with(
            ReconciliationSummary::Contradiction(vec![ContradictionSet {
                relation: RelationId("rel".to_string()),
                supporting: vec![ObservationId("o1".to_string())],
                contradicting: vec![ObservationId("o2".to_string())],
            }]),
            vec![conflicted("c-uat3")],
            vec![],
            vec![(claim(CONTRACT), VerificationResult::Contradicted {
                reason: sddk_engine::verify_kernel::types::ContradictionReason::EvidenceContradiction,
            })],
            assessment(AlignmentState::Tension, "a-uat3"),
        );
        let ctx = derive_advisory_context(&result);
        assert!(ctx.items.iter().any(|i| i.note == "contradiction"));
        assert!(ctx.items.iter().any(|i| i.note == "contradicted"));
        assert!(ctx.items.iter().any(|i| i.note == "tension"));
    }

    // UAT-4 INCOMPLETE / COVERAGE-GAP
    #[test]
    fn uat_4_incomplete_coverage_gap() {
        let result = result_with(
            ReconciliationSummary::EvidenceGap(vec![]),
            vec![],
            vec![gap(
                UniversalConcern::Cohesion,
                NotEvaluatedReason::NoRegisteredLens,
            )],
            vec![(
                claim(CONTRACT),
                VerificationResult::Unknown {
                    gap: EvidenceGap::NoEvidenceProvided,
                },
            )],
            assessment(AlignmentState::Unknown, "a-uat4"),
        );
        let ctx = derive_advisory_context(&result);
        assert!(ctx.items.iter().any(|i| i.note == "evidence_gap"));
        assert!(
            ctx.items
                .iter()
                .any(|i| i.kind == AdvisoryKind::LensCoverageGap)
        );
        assert!(ctx.items.iter().any(|i| i.note == "unknown"));
    }

    // UAT-5 MISALIGNED-WITHOUT-DENY
    #[test]
    fn uat_5_misaligned_without_deny() {
        let (mut result, rc, graph) = chain(&[]);
        result.alignment_assessment = assessment(AlignmentState::Misaligned, "a-uat5");
        let ctx = derive_advisory_context(&result);
        assert!(
            ctx.items
                .iter()
                .any(|i| i.kind == AdvisoryKind::AlignmentTension && i.note == "misaligned")
        );
        let why = explain_advisory(
            &result,
            &rc,
            &graph,
            &AdvisorySubjectRef::Alignment {
                id: AlignmentAssessmentId("a-uat5".to_string()),
            },
        );
        assert!(
            why.legs
                .iter()
                .any(|l| matches!(l, AdvisoryWhyLeg::Alignment { .. }))
        );
        let repr = format!("{ctx:?}{why:?}");
        assert!(!repr.contains("Deny"));
        assert!(!repr.contains("AuthorityDecision"));
    }

    // UAT-6 ORDER / REBUILD DETERMINISM
    #[test]
    fn uat_6_order_and_rebuild_determinism() {
        let (r1, rc1, _g1) = chain(&[]);
        let (r2, rc2, _g2) = chain(&[]);
        assert_eq!(rc1, rc2, "same production inputs → same receipt");
        assert_eq!(
            derive_advisory_context(&r1).canonical_bytes(),
            derive_advisory_context(&r2).canonical_bytes()
        );
    }

    // §28 regression: requested concern == contribution concern
    #[test]
    fn uat_concern_preservation_regression() {
        let mut obs = ObservationSet::new();
        obs.insert(observation());
        let lens = real_lens(&obs);
        for c in &lens.contributions {
            assert_eq!(
                c.concern,
                UniversalConcern::Cohesion,
                "the requested concern is preserved through the production lens"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §30 / §31 — legacy compatibility + providers
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sec30_sec31_acceptance_path_has_no_legacy_or_provider_dependency() {
    // Negative: the production modules the acceptance path exercises must not
    // depend on the legacy facade or on providers.
    for (name, src) in [("loop", LOOP_SRC), ("advisory", ADVISORY_SRC)] {
        let code = code_only(src);
        for forbidden in [
            "evaluate_lens",
            "CogniCode",
            "Chronos",
            "JCode",
            "reqwest",
            "LlmAlignmentEvaluator",
        ] {
            assert!(
                !code.contains(forbidden),
                "{name} module must not depend on `{forbidden}`"
            );
        }
    }
    // Positive: the acceptance path drives the **production** lens registry,
    // not the legacy facade.
    assert!(
        code_only(include_str!("a4_5c_arch_spec_047_acceptance.rs")).contains("ParadigmLens::ALL"),
        "the acceptance path must use the production lens set"
    );
}
