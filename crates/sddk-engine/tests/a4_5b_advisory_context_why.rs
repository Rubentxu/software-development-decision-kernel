// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_5b_advisory_context_why.rs
//
// A4-5b — AdvisoryContext + WHY integration (falsification suite, P1..P19).
//
// Cycle: p-63676b11dc0ef88f-a4-5b-advisory-context-why-integration
// Spec:  arch-spec-047 part-2 + ADR-0128
//
// P1  canonical order
// P2  SpecifiedBy != VerifiedBy
// P3  VerifiedBy cardinality 0/1/N
// P4  supporting + contradicting coexist (no latest-wins)
// P5  NoRegisteredLens survives
// P6  LensExistsButRefused distinct from NoRegisteredLens
// P7  VerificationClaim/Result pairing preserved
// P8  ReconciliationSummary variants all preserved
// P9  AlignmentAssessment + id preserved
// P10 epistemic distinctions preserved
// P11 graph absence != negation
// P12 no string semantics (source pin)
// P13 no hidden orchestration (source pin)
// P14 advisory only (MISALIGNED != DENY)
// P15 no global verdict field
// P16 existing AdvisoryContext compatibility
// P17 deterministic WHY
// P18 no hidden clock
// P19 workspace regression (covered by the workspace run)

use sddk_domain::{AdvisoryContext, AdvisoryKind};

use sddk_engine::alignment_lens::not_evaluated::{NotEvaluated, NotEvaluatedReason};
use sddk_engine::alignment_lens::types::{LensId, LensVersion};
use sddk_engine::alignment_lens::{LensContribution, LensContributionId};
use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::architecture_graph::overlay::ArchitectureGraphOverlay;
use sddk_engine::debverify_kernel::types::{BaselineHash, ReconciliationSummary};
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::intelligence_advisory::{
    AdvisorySubjectRef, AdvisoryWhy, AdvisoryWhyLeg, advisory_subjects, derive_advisory_context,
    explain_advisory, subject_key,
};
use sddk_engine::intelligence_loop::{IntelligenceLoopReceiptId, IntelligenceLoopResult};
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::EventTime;
use sddk_engine::observation::posture::EvidencePosture;
use sddk_engine::observation::{ObservationId, ObservationTargetRef};
use sddk_engine::software_alignment::types::{
    AlignmentAssessment, AlignmentAssessmentId, AlignmentScope, AlignmentState, IntentSnapshotId,
};
use sddk_engine::verify_kernel::types::{
    ArchitectureConformanceClaim, ChangeBasis, VerificationClaim, VerificationResult,
};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn receipt(hex: &str) -> IntelligenceLoopReceiptId {
    IntelligenceLoopReceiptId(hex.to_string())
}

fn gap(concern: UniversalConcern, reason: NotEvaluatedReason) -> NotEvaluated {
    NotEvaluated {
        applicable: ApplicableConcern::Applicable(concern),
        candidate_lens_ids: vec![LensId("test-lens")],
        reason,
    }
}

fn assessment(state: AlignmentState, id: &str) -> AlignmentAssessment {
    AlignmentAssessment {
        id: AlignmentAssessmentId(id.to_string()),
        scope: AlignmentScope::new("a4-5b/scope".to_string()),
        intent_id: IntentSnapshotId("a4-5b/intent".to_string()),
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
        knowledge_basis_hash_hex: "kb-a4-5b".to_string(),
        observation_set_canonical_digest: "obs-a4-5b".to_string(),
        verification_pairs: pairs,
        debverify_baseline_hash: BaselineHash("baseline-a4-5b".to_string()),
        reconciliation,
        lens_evaluation: sddk_engine::alignment_lens::kernel::LensEvaluation {
            contributions,
            gaps,
        },
        alignment_assessment: assessment,
        evaluation_time: EventTime(0),
    }
}

/// A contribution whose posture is `Conflicted` (supporting AND
/// contradicting observations both present).
fn conflicted_contribution() -> LensContribution {
    let unit = sddk_engine::architecture_graph::SoftwareUnitRef::new("a4-5b::unit");
    let posture: EvidencePosture<ObservationTargetRef> = EvidencePosture::Conflicted {
        target: ObservationTargetRef::Unit(unit),
        supporting: vec![ObservationId("obs-1".to_string())],
        contradicting: vec![ObservationId("obs-2".to_string())],
    };
    LensContribution::assemble(
        LensContributionId("contrib-a4-5b".to_string()),
        LensId("test-lens"),
        LensVersion::new(1, 0),
        UniversalConcern::Coupling,
        posture,
        vec![EvidenceRef::new(EvidenceKind::Adhoc, "c:/x")],
    )
}

fn contract_with_provenance(
    contract_id: &str,
    spec: &str,
    evidence: &[EvidenceRef],
) -> (ArchitecturalContract, ArchitectureGraphOverlay) {
    let contract = ArchitecturalContract::declare_single_authority(
        ContractId::new(contract_id).expect("valid id"),
        ComponentRef::new("comp:a4-5b").expect("valid component"),
        DecisionRef::Decision("decision:a4-5b".to_string()),
        SpecRef::Spec(spec.to_string()),
        Revision::new("rev-a4-5b").expect("valid revision"),
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

// ─────────────────────────────────────────────────────────────────────────────
// P1 — canonical order
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p1_derivation_is_insertion_order_independent() {
    let pairs_a = vec![
        (claim("c-1"), VerificationResult::Verified),
        (
            claim("c-2"),
            VerificationResult::Unknown {
                gap: sddk_engine::verify_kernel::types::EvidenceGap::NoEvidenceProvided,
            },
        ),
    ];
    let pairs_b = vec![pairs_a[1].clone(), pairs_a[0].clone()];

    let a = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted_contribution()],
        vec![],
        pairs_a,
        assessment(AlignmentState::Aligned, "a-1"),
    );
    let b = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted_contribution()],
        vec![],
        pairs_b,
        assessment(AlignmentState::Aligned, "a-1"),
    );

    let ctx_a = derive_advisory_context(&a);
    let ctx_b = derive_advisory_context(&b);
    assert_eq!(
        ctx_a.canonical_bytes(),
        ctx_b.canonical_bytes(),
        "advisory payload must be canonical across insertion order"
    );

    let subj_a: Vec<String> = advisory_subjects(&a).iter().map(subject_key).collect();
    let subj_b: Vec<String> = advisory_subjects(&b).iter().map(subject_key).collect();
    assert_eq!(subj_a, subj_b, "subject order must be canonical");
    let mut sorted = subj_a.clone();
    sorted.sort();
    assert_eq!(
        subj_a, sorted,
        "subjects must be emitted in canonical order"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P2 — SpecifiedBy != VerifiedBy
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p2_specified_by_does_not_imply_verified_by() {
    // Contract has a spec but NO evidence. The verification outcome is
    // `Verified` (typed), so WHY must NOT deny verification, and the
    // contract provenance must show the spec with an empty evidence axis.
    let (_c, graph) = contract_with_provenance("c-p2", "spec-p2", &[]);
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![(claim("c-p2"), VerificationResult::Verified)],
        assessment(AlignmentState::Aligned, "a-p2"),
    );

    let why = explain_advisory(
        &result,
        &receipt("r-p2"),
        &graph,
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-p2".to_string(),
        },
    );

    let prov = why
        .legs
        .iter()
        .find_map(|l| match l {
            AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
            _ => None,
        })
        .expect("contract provenance leg");
    assert_eq!(prov.specified_by, vec!["spec-p2".to_string()]);
    assert!(prov.verified_by.is_empty(), "no evidence linked");

    // The empty evidence axis is reported as UNRESOLVED, never negated,
    // and never turns into a "not verified" reason.
    assert!(
        why.unresolved.iter().any(|u| u.edge.contains("VerifiedBy")),
        "the absent evidence leg must be visible"
    );
    assert!(
        why.why_not.is_empty(),
        "an empty evidence axis must not be turned into a negative claim"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P3 — VerifiedBy cardinality
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p3_verified_by_cardinality_is_preserved() {
    let ev = |loc: &str| EvidenceRef::new(EvidenceKind::Planning, loc);

    let (_c0, g0) = contract_with_provenance("c-p3", "spec-p3", &[]);
    assert_eq!(g0.contract_provenance("c-p3").verified_by.len(), 0);

    let (_c1, g1) = contract_with_provenance("c-p3", "spec-p3", &[ev("e1")]);
    assert_eq!(g1.contract_provenance("c-p3").verified_by.len(), 1);

    let (_cn, gn) = contract_with_provenance("c-p3", "spec-p3", &[ev("e1"), ev("e2"), ev("e3")]);
    assert_eq!(gn.contract_provenance("c-p3").verified_by.len(), 3);

    // Typed identity is preserved end-to-end (kind + locator recovered).
    let recovered = gn.contract_provenance("c-p3").verified_by;
    assert!(recovered.contains(&EvidenceRef::new(EvidenceKind::Planning, "e2")));

    // Duplicate typed refs canonicalize to one.
    let (_cd, gd) = contract_with_provenance("c-p3", "spec-p3", &[ev("e1"), ev("e1")]);
    assert_eq!(gd.contract_provenance("c-p3").verified_by.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// P4 — supporting + contradicting coexist
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p4_conflicted_posture_preserves_both_sides() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted_contribution()],
        vec![],
        vec![],
        assessment(AlignmentState::Tension, "a-p4"),
    );

    // The contribution id is visible in the payload.
    let ctx = derive_advisory_context(&result);
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::ParadigmObservation)
    );

    // WHY carries the full typed posture, both sides intact.
    let why = explain_advisory(
        &result,
        &receipt("r-p4"),
        &empty_graph(),
        &AdvisorySubjectRef::LensContribution {
            id: LensContributionId("contrib-a4-5b".to_string()),
        },
    );
    let posture = why
        .legs
        .iter()
        .find_map(|l| match l {
            AdvisoryWhyLeg::LensContribution { contribution } => {
                Some(&contribution.evidence_resolution)
            }
            _ => None,
        })
        .expect("contribution leg");
    match posture {
        EvidencePosture::Conflicted {
            supporting,
            contradicting,
            ..
        } => {
            assert_eq!(supporting.len(), 1);
            assert_eq!(contradicting.len(), 1);
        }
        other => panic!("expected Conflicted posture, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P5 / P6 — lens gap reasons survive and stay distinct
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p5_no_registered_lens_survives_to_advisory_and_why() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![],
        assessment(AlignmentState::Unknown, "a-p5"),
    );

    let ctx = derive_advisory_context(&result);
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::LensCoverageGap)
    );

    let why = explain_advisory(
        &result,
        &receipt("r-p5"),
        &empty_graph(),
        &AdvisorySubjectRef::LensGap {
            concern: UniversalConcern::Cohesion,
            reason: NotEvaluatedReason::NoRegisteredLens,
        },
    );
    assert!(
        why.legs
            .iter()
            .any(|l| matches!(l, AdvisoryWhyLeg::LensGap { .. }))
    );
}

#[test]
fn p6_lens_exists_but_refused_is_distinct_from_no_registered_lens() {
    let refused = gap(
        UniversalConcern::Coupling,
        NotEvaluatedReason::LensExistsButRefused,
    );
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![refused],
        vec![],
        assessment(AlignmentState::Unknown, "a-p6"),
    );

    // The two reason tags differ and produce different subjects.
    let k_none = subject_key(&AdvisorySubjectRef::LensGap {
        concern: UniversalConcern::Coupling,
        reason: NotEvaluatedReason::NoRegisteredLens,
    });
    let k_refused = subject_key(&AdvisorySubjectRef::LensGap {
        concern: UniversalConcern::Coupling,
        reason: NotEvaluatedReason::LensExistsButRefused,
    });
    assert_ne!(k_none, k_refused);

    // Querying the refused gap resolves; querying the no-registered-lens
    // gap on a result that only has the refused gap does NOT resolve (the
    // absence is reported, not conflated).
    let why_refused = explain_advisory(
        &result,
        &receipt("r-p6"),
        &empty_graph(),
        &AdvisorySubjectRef::LensGap {
            concern: UniversalConcern::Coupling,
            reason: NotEvaluatedReason::LensExistsButRefused,
        },
    );
    assert!(
        why_refused
            .legs
            .iter()
            .any(|l| matches!(l, AdvisoryWhyLeg::LensGap { .. }))
    );

    let why_none = explain_advisory(
        &result,
        &receipt("r-p6"),
        &empty_graph(),
        &AdvisorySubjectRef::LensGap {
            concern: UniversalConcern::Coupling,
            reason: NotEvaluatedReason::NoRegisteredLens,
        },
    );
    assert!(
        why_none.legs.is_empty(),
        "a different reason is not the same gap"
    );
    assert!(!why_none.unresolved.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// P7 — claim/result pairing
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p7_claim_result_pairing_is_not_flattened() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![
            (claim("c-a"), VerificationResult::Verified),
            (claim("c-b"), VerificationResult::NotApplicable),
        ],
        assessment(AlignmentState::Aligned, "a-p7"),
    );

    let why_a = explain_advisory(
        &result,
        &receipt("r-p7"),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-a".to_string(),
        },
    );
    let why_b = explain_advisory(
        &result,
        &receipt("r-p7"),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-b".to_string(),
        },
    );

    let pair = |why: &AdvisoryWhy| -> (String, VerificationResult) {
        why.legs
            .iter()
            .find_map(|l| match l {
                AdvisoryWhyLeg::Verification { claim, result } => {
                    let VerificationClaim::ArchitectureConformance(c) = claim;
                    Some((c.contract_id.clone(), result.clone()))
                }
                _ => None,
            })
            .expect("verification leg")
    };
    assert_eq!(
        pair(&why_a),
        ("c-a".to_string(), VerificationResult::Verified)
    );
    assert_eq!(
        pair(&why_b),
        ("c-b".to_string(), VerificationResult::NotApplicable)
    );
    // The pairing is exact: c-a is NOT reported with c-b's result.
    assert_ne!(pair(&why_a).1, pair(&why_b).1);
}

// ─────────────────────────────────────────────────────────────────────────────
// P8 — ReconciliationSummary variants
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p8_all_reconciliation_variants_are_preserved() {
    let variants = vec![
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 2 },
        ReconciliationSummary::Contradiction(vec![]),
        ReconciliationSummary::Staleness(vec![]),
        ReconciliationSummary::EvidenceGap(vec![]),
        ReconciliationSummary::NotApplicable,
    ];
    for summary in variants {
        let result = result_with(
            summary.clone(),
            vec![],
            vec![],
            vec![],
            assessment(AlignmentState::Aligned, "a-p8"),
        );
        let why = explain_advisory(
            &result,
            &receipt("r-p8"),
            &empty_graph(),
            &AdvisorySubjectRef::Reconciliation,
        );
        let got = why
            .legs
            .iter()
            .find_map(|l| match l {
                AdvisoryWhyLeg::Reconciliation { summary } => Some(summary.clone()),
                _ => None,
            })
            .expect("reconciliation leg");
        assert_eq!(got, summary, "reconciliation summary must survive verbatim");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P9 — alignment identity preserved
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p9_alignment_assessment_and_id_are_preserved() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Misaligned, "align-id-p9"),
    );
    let why = explain_advisory(
        &result,
        &receipt("r-p9"),
        &empty_graph(),
        &AdvisorySubjectRef::Alignment {
            id: AlignmentAssessmentId("align-id-p9".to_string()),
        },
    );
    let got = why
        .legs
        .iter()
        .find_map(|l| match l {
            AdvisoryWhyLeg::Alignment { assessment } => Some(assessment),
            _ => None,
        })
        .expect("alignment leg");
    assert_eq!(got.id.as_str(), "align-id-p9");
    assert_eq!(got.state, AlignmentState::Misaligned);
}

// ─────────────────────────────────────────────────────────────────────────────
// P10 — epistemic distinctions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p10_epistemic_distinctions_are_kept_apart() {
    // The three verification outcomes are distinct; each maps to its own
    // canonical note and is never collapsed.
    use sddk_engine::verify_kernel::types::{BasisHash, EvidenceGap};
    let c = claim("c-p10");
    let outcomes = [
        VerificationResult::Verified,
        VerificationResult::NotApplicable,
        VerificationResult::Unknown {
            gap: EvidenceGap::NoEvidenceProvided,
        },
        VerificationResult::Stale {
            basis: BasisHash::new([7u8; 32]),
        },
    ];
    let tags: Vec<String> = outcomes
        .iter()
        .map(|o| {
            let r = result_with(
                ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
                vec![],
                vec![],
                vec![(c.clone(), o.clone())],
                assessment(AlignmentState::Aligned, "a-p10"),
            );
            derive_advisory_context(&r)
                .items
                .iter()
                .find(|i| i.kind == AdvisoryKind::VerificationOutcome)
                .map(|i| i.note.clone())
                .expect("verification item")
        })
        .collect();
    let mut unique = tags.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        tags.len(),
        "outcome notes must be distinct: {tags:?}"
    );

    // NotEvaluated != NotApplicable: a lens gap is a LensCoverageGap, and
    // a verification NotApplicable is a VerificationOutcome — distinct kinds.
    assert_ne!(
        AdvisoryKind::LensCoverageGap,
        AdvisoryKind::VerificationOutcome
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P11 — graph absence != negation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p11_graph_absence_is_reported_not_negated() {
    // A contract that is NOT in the overlay at all.
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![(claim("c-absent"), VerificationResult::Verified)],
        assessment(AlignmentState::Aligned, "a-p11"),
    );
    let why = explain_advisory(
        &result,
        &receipt("r-p11"),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-absent".to_string(),
        },
    );
    // Contract provenance is empty on both axes, and the absence is made
    // visible as an unresolved leg — never as a negative finding.
    let prov = why
        .legs
        .iter()
        .find_map(|l| match l {
            AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
            _ => None,
        })
        .expect("contract provenance leg");
    assert!(prov.specified_by.is_empty());
    assert!(prov.verified_by.is_empty());
    assert!(!why.unresolved.is_empty(), "absence must be visible");
    assert!(
        why.why_not.is_empty(),
        "an unknown contract must not be turned into a negative claim"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P12 / P13 — source pins
// ─────────────────────────────────────────────────────────────────────────────

const ADVISORY_SRC: &str = include_str!("../src/intelligence_advisory/mod.rs");

/// Strip `//`-prefixed comment lines so the source pins inspect **code**
/// only, never prose (the module docs deliberately name the things it must
/// not do).
fn advisory_code_only() -> String {
    ADVISORY_SRC
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn p12_no_string_semantics_in_the_advisory_module() {
    let code = advisory_code_only();
    for forbidden in [
        ".contains(",
        ".starts_with(",
        ".ends_with(",
        ".split(",
        ".trim_start_matches(",
        ".trim_end_matches(",
        "parse::<",
    ] {
        assert!(
            !code.contains(forbidden),
            "advisory module must not infer meaning via `{forbidden}`"
        );
    }
}

#[test]
fn p13_no_hidden_orchestration_in_the_advisory_module() {
    let code = advisory_code_only();
    for forbidden in [
        "VerifyKernel",
        "DebVerifyKernel",
        "AlignmentLensKernel",
        "reduce_alignment",
        "paradigm_lens",
        "evaluate_lens",
        "reqwest",
        "CogniCode",
        "Chronos",
    ] {
        assert!(
            !code.contains(forbidden),
            "advisory module must not call `{forbidden}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P14 — advisory only
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p14_misaligned_is_advisory_and_never_a_denial() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Misaligned, "a-p14"),
    );
    let ctx = derive_advisory_context(&result);
    // The misalignment is delivered as advisory content.
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::AlignmentTension && i.note == "misaligned")
    );

    // No authority/capability/instruction surface is reachable from the
    // advisory module. The source pin above (P13) covers providers; here we
    // pin that the derivation has no DENY-shaped output at all.
    let code = advisory_code_only();
    for forbidden in [
        "AuthorityDecision",
        "Capability",
        "InstructionSource",
        "InstructionCompiler",
        "DENY",
        "Deny",
    ] {
        assert!(
            !code.contains(forbidden),
            "advisory module must not mention `{forbidden}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P15 — no global verdict
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p15_no_global_verdict_field() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted_contribution()],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![(claim("c-p15"), VerificationResult::Verified)],
        assessment(AlignmentState::Tension, "a-p15"),
    );
    let why = explain_advisory(
        &result,
        &receipt("r-p15"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    let repr = format!("{why:?}");
    for forbidden in [
        "status",
        "health",
        "score",
        "verdict",
        "confidence",
        "risk_score",
        "quality_score",
        "overall",
        "pass:",
        "fail:",
        "ready",
    ] {
        assert!(
            !repr.contains(forbidden),
            "AdvisoryWhy debug repr must not carry `{forbidden}`: {repr}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P16 — existing AdvisoryContext compatibility
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p16_advisory_context_type_is_reused_verbatim() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Aligned, "a-p16"),
    );
    // The derivation returns the A3 type itself.
    let ctx: AdvisoryContext = derive_advisory_context(&result);
    // And the A3 canonicalization invariant still holds: pushing a
    // duplicate changes nothing.
    let mut copy = ctx.clone();
    if let Some(first) = ctx.items.first().cloned() {
        copy.push(first);
    }
    assert_eq!(copy.canonical_bytes(), ctx.canonical_bytes());

    // The A3 empty payload is still expressible.
    assert!(AdvisoryContext::none().is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// P17 — deterministic WHY
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p17_why_is_deterministic() {
    let result = result_with(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![conflicted_contribution()],
        vec![gap(
            UniversalConcern::Cohesion,
            NotEvaluatedReason::NoRegisteredLens,
        )],
        vec![(claim("c-p17"), VerificationResult::Verified)],
        assessment(AlignmentState::Tension, "a-p17"),
    );
    let subjects = advisory_subjects(&result);
    let a: Vec<AdvisoryWhy> = subjects
        .iter()
        .map(|s| explain_advisory(&result, &receipt("r-p17"), &empty_graph(), s))
        .collect();
    let b: Vec<AdvisoryWhy> = subjects
        .iter()
        .rev()
        .map(|s| explain_advisory(&result, &receipt("r-p17"), &empty_graph(), s))
        .collect();
    // Same subjects, reversed iteration → same per-subject answers.
    let mut a_by_key: Vec<(String, AdvisoryWhy)> = a
        .iter()
        .map(|w| (subject_key(&w.subject), w.clone()))
        .collect();
    let mut b_by_key: Vec<(String, AdvisoryWhy)> = b
        .iter()
        .map(|w| (subject_key(&w.subject), w.clone()))
        .collect();
    a_by_key.sort_by(|x, y| x.0.cmp(&y.0));
    b_by_key.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(a_by_key, b_by_key);
}

// ─────────────────────────────────────────────────────────────────────────────
// P18 — no hidden clock
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p18_evaluation_time_is_not_in_the_basis() {
    let mk = |t: i64| {
        let mut r = result_with(
            ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
            vec![],
            vec![],
            vec![],
            assessment(AlignmentState::Aligned, "a-p18"),
        );
        r.evaluation_time = EventTime(t);
        r
    };
    let why_a = explain_advisory(
        &mk(1),
        &receipt("r-p18"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    let why_b = explain_advisory(
        &mk(999_999),
        &receipt("r-p18"),
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    // The basis is clock-stable: a different evaluation_time yields the
    // same basis.
    assert_eq!(why_a.basis, why_b.basis);
}

// ─────────────────────────────────────────────────────────────────────────────
// §11 UAT — real end-to-end through the production authority chain
// ─────────────────────────────────────────────────────────────────────────────

mod uat {
    use std::collections::BTreeSet;

    use super::*;
    use sddk_engine::alignment_lens::kernel::AlignmentLensKernel;
    use sddk_engine::alignment_lens::paradigm::ParadigmLens;
    use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;
    use sddk_engine::alignment_lens::types::LensInput;
    use sddk_engine::debverify_kernel::types::Baseline;
    use sddk_engine::intelligence_loop::{IntelligenceLoopInputs, compose_intelligence_loop};
    use sddk_engine::intent_universal_concern::types::IntentId;
    use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
    use sddk_engine::observation::{
        ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
        SoftwareObservation,
    };
    use sddk_engine::software_alignment::reducer::reduce_alignment;
    use sddk_engine::software_alignment::types::{
        ArchitecturalIntentSnapshot, ExplicitConstraint, ParadigmTag,
    };
    use sddk_engine::verify_kernel::engine::VerifyKernel;
    use sddk_engine::verify_kernel::registry::VerificationDomain;
    use sddk_engine::verify_kernel::types::{ProbeKind, ProbePlan};

    const CONTRACT: &str = "a4-5b:uat-contract";

    fn basis_empty() -> KnowledgeBasis {
        KnowledgeBasis::empty(EventTime(0))
    }

    fn uat_unit() -> sddk_engine::architecture_graph::SoftwareUnitRef {
        sddk_engine::architecture_graph::SoftwareUnitRef::new("a4-5b::uat")
    }

    fn observation() -> SoftwareObservation {
        let basis = ObservationBasis::new(
            "a4-5b-rev",
            basis_empty().basis_hash().clone(),
            "a4-5b-input",
        );
        SoftwareObservation::declare(
            ObservationSubject::Unit(uat_unit()),
            ObservationStance::Affirms,
            EvidenceRef::new(EvidenceKind::Adhoc, "ev:uat:1".to_string()),
            ObservationOrigin::DeterministicLocal,
            basis,
            None,
            "a4-5b-producer".to_string(),
        )
    }

    fn real_lens_evaluation(obs: &ObservationSet) -> sddk_engine::alignment_lens::LensEvaluation {
        let mut registry = AlignmentLensRegistry::new();
        for lens in ParadigmLens::ALL {
            registry = registry.register(lens).expect("lens registration");
        }
        let input = LensInput::try_new(
            ApplicableConcern::Applicable(UniversalConcern::Cohesion),
            uat_unit(),
            IntentId("a4-5b/intent".to_string()),
            basis_empty().basis_hash().clone(),
            obs.clone(),
        )
        .expect("Applicable input");
        match AlignmentLensKernel::new().evaluate(&registry, &input) {
            sddk_engine::alignment_lens::kernel::KernelOutcome::Ok(ev) => ev,
            sddk_engine::alignment_lens::kernel::KernelOutcome::Refused(e) => {
                panic!("kernel refused: {e:?}")
            }
        }
    }

    fn real_alignment(obs: &ObservationSet) -> AlignmentAssessment {
        let scope = AlignmentScope::new("a4-5b/uat-scope".to_string());
        let paradigm = ParadigmTag::Undeclared;
        let constraints: Vec<ExplicitConstraint> = vec![];
        let id = ArchitecturalIntentSnapshot::derive_id(&scope, paradigm, &constraints);
        let intent = ArchitecturalIntentSnapshot {
            id,
            scope,
            paradigm_tag: paradigm,
            explicit_constraints: constraints,
        };
        reduce_alignment(&intent, &basis_empty(), obs, &[], &[], EventTime(0))
            .expect("reduce_alignment")
    }

    struct VerifiedDomain;
    impl VerificationDomain for VerifiedDomain {
        fn name(&self) -> &'static str {
            "a4-5b:uat:verified"
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

    /// Run the real chain (VerifyKernel + AlignmentLensKernel +
    /// reduce_alignment + compose_intelligence_loop) and return the
    /// composed result, its receipt, and an overlay carrying `evidence`.
    fn run_chain(
        evidence: &[EvidenceRef],
    ) -> (
        IntelligenceLoopResult,
        IntelligenceLoopReceiptId,
        ArchitectureGraphOverlay,
    ) {
        let mut obs = ObservationSet::new();
        obs.insert(observation());

        let vclaim = claim(CONTRACT);
        let vresult = VerifyKernel::evaluate(&vclaim, &obs, &VerifiedDomain);
        assert!(matches!(vresult, VerificationResult::Verified));

        let lens_eval = real_lens_evaluation(&obs);
        let assessment = real_alignment(&obs);

        let inputs = IntelligenceLoopInputs {
            knowledge_basis: basis_empty(),
            observation_set: obs.clone(),
            verification_pairs: vec![(vclaim, vresult)],
            debverify_baseline_hash: Baseline::from_observations("a4-5b-scope", &obs).hash(),
            reconciliation: ReconciliationSummary::ConfirmedBaseline { strategies_run: 3 },
            lens_evaluation: lens_eval,
            alignment_assessment: assessment,
            evaluation_time: EventTime(0),
        };
        let (result, receipt) = compose_intelligence_loop(inputs);
        let (_c, graph) = contract_with_provenance(CONTRACT, "spec-uat", evidence);
        (result, receipt.id, graph)
    }

    #[test]
    fn uat_without_verified_by_shows_spec_but_not_verification() {
        let (result, receipt_id, graph) = run_chain(&[]);
        let ctx = derive_advisory_context(&result);
        assert!(!ctx.is_empty(), "the loop must produce advisory content");

        let why = explain_advisory(
            &result,
            &receipt_id,
            &graph,
            &AdvisorySubjectRef::VerificationClaim {
                contract_id: CONTRACT.to_string(),
            },
        );
        // Typed verification shows the real outcome.
        assert!(
            why.legs
                .iter()
                .any(|l| matches!(l, AdvisoryWhyLeg::Verification { result, .. }
            if matches!(result, VerificationResult::Verified)))
        );
        // Contract provenance: spec present, evidence absent.
        let prov = why
            .legs
            .iter()
            .find_map(|l| match l {
                AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
                _ => None,
            })
            .expect("provenance leg");
        assert_eq!(prov.specified_by, vec!["spec-uat".to_string()]);
        assert!(prov.verified_by.is_empty());
        // Absence is visible, and no negative is manufactured.
        assert!(why.unresolved.iter().any(|u| u.edge.contains("VerifiedBy")));
        assert!(why.why_not.is_empty());
    }

    #[test]
    fn uat_with_verified_by_shows_the_evidence() {
        let evidence = vec![
            EvidenceRef::new(EvidenceKind::Planning, "plan/uat-1"),
            EvidenceRef::new(EvidenceKind::Authority, "auth/uat-2"),
        ];
        let (result, receipt_id, graph) = run_chain(&evidence);
        let why = explain_advisory(
            &result,
            &receipt_id,
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
            .expect("provenance leg");
        assert_eq!(prov.verified_by.len(), 2, "both evidence refs are shown");
        assert!(
            prov.verified_by
                .contains(&EvidenceRef::new(EvidenceKind::Planning, "plan/uat-1"))
        );
        // The two cases differ ONLY by real provenance.
        assert!(
            why.unresolved
                .iter()
                .all(|u| !u.edge.contains("VerifiedBy"))
        );
    }
}
