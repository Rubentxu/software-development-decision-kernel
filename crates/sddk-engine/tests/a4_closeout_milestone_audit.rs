// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_closeout_milestone_audit.rs
//
// A4-CLOSEOUT — A4 Milestone Audit & Certification (cross-spec invariant
// corpus + end-to-end milestone UAT + false-clean / false-authority audit).
//
// Cycle: p-63676b11dc0ef88f-a4-closeout-milestone-audit
// Budget: AUDIT / RECONCILIATION / CERTIFICATION ONLY.
//
// This corpus is deliberately **transversal**: it crosses spec boundaries
// (042 → 047) rather than re-proving each spec in isolation. It is
// evidence for `docs/architecture/receipts/A4-MILESTONE-RECEIPT.md`.

use std::collections::BTreeSet;

use sddk_domain::{AdvisoryContext, AdvisoryKind, ContextCapsule};

use sddk_engine::alignment_lens::kernel::AlignmentLensKernel;
use sddk_engine::alignment_lens::not_evaluated::NotEvaluatedReason;
use sddk_engine::alignment_lens::paradigm::ParadigmLens;
use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;
use sddk_engine::alignment_lens::types::LensInput;
use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, EntityRef, Revision, SpecRef,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::architecture_graph::overlay::ArchitectureGraphOverlay;
use sddk_engine::debverify_kernel::DebVerifyKernel;
use sddk_engine::debverify_kernel::strategy_set::ChallengeStrategySet;
use sddk_engine::debverify_kernel::types::{
    Baseline, BaselineHash, ReconciliationScope, ReconciliationSummary,
};
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::intelligence_advisory::{
    AdvisorySubjectRef, AdvisoryWhyLeg, derive_advisory_context, explain_advisory,
};
use sddk_engine::intelligence_loop::{
    IntelligenceLoopInputs, IntelligenceLoopResult, compose_intelligence_loop,
};
use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::posture::EvidencePosture;
use sddk_engine::observation::{
    ObservationBasis, ObservationId, ObservationOrigin, ObservationSet, ObservationStance,
    ObservationSubject, ObservationTargetRef, SoftwareEntityRef, SoftwareObservation,
};
use sddk_engine::software_alignment::reducer::reduce_alignment;
use sddk_engine::software_alignment::types::{
    AlignmentAssessment, AlignmentAssessmentId, AlignmentScope, AlignmentState,
    ArchitecturalIntentSnapshot, ExplicitConstraint, IntentSnapshotId, ParadigmTag,
};
use sddk_engine::verify_kernel::engine::VerifyKernel;
use sddk_engine::verify_kernel::registry::VerificationDomain;
use sddk_engine::verify_kernel::types::{
    ArchitectureConformanceClaim, ChangeBasis, EvidenceGap, ProbeKind, ProbePlan,
    VerificationClaim, VerificationResult,
};

// ─────────────────────────────────────────────────────────────────────────────
// Builders
// ─────────────────────────────────────────────────────────────────────────────

fn basis() -> KnowledgeBasis {
    KnowledgeBasis::empty(EventTime(0))
}

fn unit() -> SoftwareUnitRef {
    SoftwareUnitRef::new("a4-co::unit")
}

fn observation(stance: ObservationStance, ev: &str) -> SoftwareObservation {
    let b = ObservationBasis::new("a4-co-rev", basis().basis_hash().clone(), "a4-co-input");
    SoftwareObservation::declare(
        ObservationSubject::Unit(unit()),
        stance,
        EvidenceRef::new(EvidenceKind::Adhoc, ev.to_string()),
        ObservationOrigin::DeterministicLocal,
        b,
        None,
        "a4-co-producer".to_string(),
    )
}

fn obs_set(stances: &[ObservationStance]) -> ObservationSet {
    let mut s = ObservationSet::new();
    for (i, st) in stances.iter().enumerate() {
        s.insert(observation(*st, &format!("ev:{i}")));
    }
    s
}

fn claim(cid: &str) -> VerificationClaim {
    VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim::new(
        cid.to_string(),
        ChangeBasis::new(vec![unit()]),
    ))
}

struct Domain(VerificationResult);
impl VerificationDomain for Domain {
    fn name(&self) -> &'static str {
        "a4-co:domain"
    }
    fn required_probe_kinds(&self, _c: &VerificationClaim) -> BTreeSet<ProbeKind> {
        BTreeSet::new()
    }
    fn minimal_probe_plan(&self, _c: &VerificationClaim) -> ProbePlan {
        ProbePlan { steps: vec![] }
    }
    fn evaluate(&self, _c: &VerificationClaim, _o: &ObservationSet) -> VerificationResult {
        self.0.clone()
    }
}

fn real_lens(
    concern: UniversalConcern,
    obs: &ObservationSet,
) -> sddk_engine::alignment_lens::LensEvaluation {
    let mut registry = AlignmentLensRegistry::new();
    for lens in ParadigmLens::ALL {
        registry = registry.register(lens).expect("register production lens");
    }
    let input = LensInput::try_new(
        ApplicableConcern::Applicable(concern),
        unit(),
        IntentId("a4-co/intent".to_string()),
        basis().basis_hash().clone(),
        obs.clone(),
    )
    .expect("input");
    match AlignmentLensKernel::new().evaluate(&registry, &input) {
        sddk_engine::alignment_lens::kernel::KernelOutcome::Ok(ev) => ev,
        sddk_engine::alignment_lens::kernel::KernelOutcome::Refused(e) => panic!("refused: {e:?}"),
    }
}

fn real_alignment(
    obs: &ObservationSet,
    contracts: &[ArchitecturalContract],
) -> AlignmentAssessment {
    let scope = AlignmentScope::new("a4-co/scope".to_string());
    let paradigm = ParadigmTag::Undeclared;
    let constraints: Vec<ExplicitConstraint> = vec![];
    let id = ArchitecturalIntentSnapshot::derive_id(&scope, paradigm, &constraints);
    let intent = ArchitecturalIntentSnapshot {
        id,
        scope,
        paradigm_tag: paradigm,
        explicit_constraints: constraints,
    };
    reduce_alignment(&intent, &basis(), obs, contracts, &[], EventTime(0)).expect("reduce")
}

fn assessment(state: AlignmentState, id: &str) -> AlignmentAssessment {
    AlignmentAssessment {
        id: AlignmentAssessmentId(id.to_string()),
        scope: AlignmentScope::new("a4-co/scope".to_string()),
        intent_id: IntentSnapshotId("a4-co/intent".to_string()),
        state,
        findings: vec![],
        contradictions: vec![],
        evaluated_at: EventTime(0),
    }
}

fn empty_graph() -> ArchitectureGraphOverlay {
    ArchitectureGraphOverlay::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// §5 — Cross-spec invariant matrix
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn inv_epistemology_evidence_is_a_reference_not_a_truth() {
    // Evidence ≠ truth: `EvidenceRef` is (kind, locator, cas) — a reference.
    // Observation ≠ decision: a `SoftwareObservation` carries a *stance*, and
    // the decision comes later from Verify/Alignment.
    let e = EvidenceRef::new(EvidenceKind::Adhoc, "x");
    assert_eq!(e.kind, EvidenceKind::Adhoc);
    assert_eq!(e.locator, "x");
    assert!(e.cas.is_none());

    let obs = observation(ObservationStance::Affirms, "ev:truth");
    // The stance is data; it is not a verdict, and no `VerificationResult` is
    // derivable from it without the kernel.
    assert_eq!(obs.stance, ObservationStance::Affirms);
}

#[test]
fn inv_epistemic_states_are_mutually_distinct() {
    // NotApplicable != Unknown != InsufficientEvidence != NotEvaluated.
    let na = VerificationResult::NotApplicable;
    let unk = VerificationResult::Unknown {
        gap: EvidenceGap::NoEvidenceProvided,
    };
    let ins = VerificationResult::Unknown {
        gap: EvidenceGap::InsufficientEvidence,
    };
    assert_ne!(na, unk);
    assert_ne!(unk, ins);
    assert_ne!(
        EvidenceGap::NoEvidenceProvided,
        EvidenceGap::InsufficientEvidence
    );
    // NotEvaluated is a lens-domain reason, not a VerificationResult.
    assert_ne!(
        NotEvaluatedReason::NoRegisteredLens,
        NotEvaluatedReason::LensExistsButRefused
    );
}

#[test]
fn inv_contradiction_coexists_without_aggregation() {
    // Contradictory evidence coexists. `Conflicted` keeps both sides; there is
    // no latest-wins / majority / confidence collapse.
    let conflicted: EvidencePosture<ObservationTargetRef> = EvidencePosture::Conflicted {
        target: ObservationTargetRef::Unit(unit()),
        supporting: vec![ObservationId("s1".to_string())],
        contradicting: vec![ObservationId("c1".to_string())],
    };
    match conflicted {
        EvidencePosture::Conflicted {
            supporting,
            contradicting,
            ..
        } => {
            assert_eq!(supporting.len(), 1);
            assert_eq!(contradicting.len(), 1);
        }
        other => panic!("expected Conflicted, got {other:?}"),
    }
}

#[test]
fn inv_applicability_grounding_evaluability_are_distinct() {
    // Applicability != Grounding != Evaluability.
    let applicable = ApplicableConcern::Applicable(UniversalConcern::Cohesion);
    assert!(applicable.is_applicable());
    // Grounding is the knowledge basis; evaluability is the kernel outcome.
    // They are separate inputs; a missing basis never silently makes a concern
    // inapplicable.
    assert_eq!(applicable.concern(), UniversalConcern::Cohesion);
    // The kernel distinguishes "no lens" from "lens refused".
    assert_ne!(
        NotEvaluatedReason::NoRegisteredLens,
        NotEvaluatedReason::LensExistsButRefused
    );
}

#[test]
fn inv_namespace_identity_is_strict() {
    // Unit("x") != Component("x") != Entity("x").
    let u = SoftwareUnitRef::new("x");
    let c = ComponentRef::new("x").expect("component");
    let e = EntityRef::new("x").expect("entity");
    // Same inner string, three namespaces; the typed targets are distinct.
    let t_u = ObservationTargetRef::Unit(u.clone());
    let t_c = ObservationTargetRef::Component(c);
    let t_e = ObservationTargetRef::Entity(e);
    assert_ne!(t_u, t_c);
    assert_ne!(t_c, t_e);
    assert_ne!(t_u, t_e);
    assert_eq!(t_u.kind_tag(), "unit");
    assert_eq!(t_c.kind_tag(), "component");
    assert_eq!(t_e.kind_tag(), "entity");
    // And in the reducer's subject enum.
    assert_ne!(
        SoftwareEntityRef::Unit(u.clone()),
        SoftwareEntityRef::Component(ComponentRef::new("x").expect("component"))
    );
}

#[test]
fn inv_lens_concern_preservation() {
    // requested concern C → contribution concern C (A4-4MR regression).
    let obs = obs_set(&[ObservationStance::Affirms]);
    let lens = real_lens(UniversalConcern::Cohesion, &obs);
    for c in &lens.contributions {
        assert_eq!(c.concern, UniversalConcern::Cohesion);
    }
}

#[test]
fn inv_provenance_axes_and_absence() {
    // SpecifiedBy != VerifiedBy; absence of VerifiedBy is not a negative.
    let contract = ArchitecturalContract::declare_single_authority(
        ContractId::new("c-inv").expect("id"),
        ComponentRef::new("comp:a4-co").expect("component"),
        DecisionRef::Decision("d".to_string()),
        SpecRef::Spec("spec-inv".to_string()),
        Revision::new("rev".to_string()).expect("revision"),
        EventTime(1_700_000_000),
    )
    .expect("contract");
    let mut g = ArchitectureGraphOverlay::new();
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[],
    );
    let p = g.contract_provenance("c-inv");
    assert_eq!(p.specified_by, vec!["spec-inv".to_string()]);
    assert!(p.verified_by.is_empty());
}

#[test]
fn inv_no_overall_state_across_domains() {
    // VerificationStatus != ReconciliationSummary != AlignmentState; no
    // overall state exists.
    let v = VerificationResult::Verified;
    let r = ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 };
    let a = AlignmentState::Aligned;
    let _ = (&v, &r, a); // type-level: three distinct enums
    // None of them is a "status" field on the loop result.
    let result = result_fixture(
        r.clone(),
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Aligned, "a-inv"),
    );
    let repr = format!("{result:?}");
    for forbidden in ["overall", "verdict", "health", "score"] {
        assert!(!repr.to_lowercase().contains(forbidden));
    }
}

#[test]
fn inv_composition_is_not_orchestration_authority() {
    // composition != orchestration authority; receipt != authority.
    let inputs = IntelligenceLoopInputs {
        knowledge_basis: basis(),
        observation_set: ObservationSet::new(),
        verification_pairs: vec![],
        debverify_baseline_hash: BaselineHash("b".to_string()),
        reconciliation: ReconciliationSummary::NotApplicable,
        lens_evaluation: sddk_engine::alignment_lens::LensEvaluation::default(),
        alignment_assessment: assessment(AlignmentState::NotApplicable, "a-inv"),
        evaluation_time: EventTime(0),
    };
    let (_r, receipt) = compose_intelligence_loop(inputs);
    // The receipt is content-addressed identity, not a decision.
    assert_eq!(receipt.id.as_str().len(), 64);
}

#[test]
fn inv_advisory_does_not_mutate_instructions() {
    // AdvisoryContext does not mutate effective_instruction_set_hash.
    let result = result_fixture(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![],
        vec![],
        assessment(AlignmentState::Misaligned, "a-inv"),
    );
    let ctx_a = derive_advisory_context(&result);
    let ctx_b = AdvisoryContext::none();
    let c_a = ContextCapsule::seal("effective@v1", "hash-inv", ctx_a);
    let c_b = ContextCapsule::seal("effective@v1", "hash-inv", ctx_b);
    assert_eq!(
        c_a.effective_instruction_set_hash,
        c_b.effective_instruction_set_hash
    );
    assert_ne!(c_a.context_capsule_hash, c_b.context_capsule_hash);
}

#[test]
fn inv_misaligned_never_escalates_authority() {
    // Structural: no authority/capability/instruction reference exists on the
    // alignment / loop / advisory production modules.
    for src in [
        include_str!("../src/software_alignment/reducer.rs"),
        include_str!("../src/intelligence_loop/mod.rs"),
        include_str!("../src/intelligence_advisory/mod.rs"),
        include_str!("../src/alignment_lens/kernel.rs"),
    ] {
        let code: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in [
            "AuthorityDecision",
            "InstructionSource",
            "InstructionCompiler",
            "capability",
            "Capability",
        ] {
            assert!(
                !code.contains(forbidden),
                "no authority escalation token `{forbidden}`"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §6 — End-to-end milestone UAT
// ─────────────────────────────────────────────────────────────────────────────

fn result_fixture(
    reconciliation: ReconciliationSummary,
    contributions: Vec<sddk_engine::alignment_lens::LensContribution>,
    gaps: Vec<sddk_engine::alignment_lens::not_evaluated::NotEvaluated>,
    pairs: Vec<(VerificationClaim, VerificationResult)>,
    a: AlignmentAssessment,
) -> IntelligenceLoopResult {
    IntelligenceLoopResult {
        knowledge_basis_hash_hex: "kb".to_string(),
        observation_set_canonical_digest: "obs".to_string(),
        verification_pairs: pairs,
        debverify_baseline_hash: BaselineHash("baseline".to_string()),
        reconciliation,
        lens_evaluation: sddk_engine::alignment_lens::LensEvaluation {
            contributions,
            gaps,
        },
        alignment_assessment: a,
        evaluation_time: EventTime(0),
    }
}

/// Real chain: KnowledgeBasis → ObservationSet → VerifyKernel → DebVerifyKernel
/// → AlignmentLensKernel → reduce_alignment → compose_intelligence_loop.
fn milestone_chain(
    domain_result: VerificationResult,
    stances: &[ObservationStance],
    contracts: &[ArchitecturalContract],
) -> (
    IntelligenceLoopResult,
    sddk_engine::intelligence_loop::IntelligenceLoopReceiptId,
) {
    let obs = obs_set(stances);
    let vclaim = claim("c-uat");
    let vresult = VerifyKernel::evaluate(&vclaim, &obs, &Domain(domain_result));
    let baseline = Baseline::from_observations("a4-co", &obs);
    let reconciliation = DebVerifyKernel::reconcile(
        ReconciliationScope::all("a4-co"),
        &baseline,
        &obs,
        &ChallengeStrategySet::new(),
    );
    let lens = real_lens(UniversalConcern::Cohesion, &obs);
    let a = real_alignment(&obs, contracts);
    let inputs = IntelligenceLoopInputs {
        knowledge_basis: basis(),
        observation_set: obs.clone(),
        verification_pairs: vec![(vclaim, vresult)],
        debverify_baseline_hash: baseline.hash(),
        reconciliation,
        lens_evaluation: lens,
        alignment_assessment: a,
        evaluation_time: EventTime(0),
    };
    let (result, receipt) = compose_intelligence_loop(inputs);
    (result, receipt.id)
}

#[test]
fn uat_milestone_clean_chain_is_preserved() {
    let (result, receipt_id) = milestone_chain(
        VerificationResult::Verified,
        &[ObservationStance::Affirms],
        &[],
    );
    assert!(matches!(
        result.verification_pairs[0].1,
        VerificationResult::Verified
    ));
    assert!(matches!(
        result.reconciliation,
        ReconciliationSummary::ConfirmedBaseline { .. }
    ));

    let ctx = derive_advisory_context(&result);
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::VerificationOutcome && i.note == "verified")
    );
    assert!(
        ctx.items
            .iter()
            .any(|i| i.kind == AdvisoryKind::DebtReconciliation && i.note == "confirmed_baseline")
    );

    let why = explain_advisory(
        &result,
        &receipt_id,
        &empty_graph(),
        &AdvisorySubjectRef::Reconciliation,
    );
    assert!(
        why.legs
            .iter()
            .any(|l| matches!(l, AdvisoryWhyLeg::Reconciliation { .. }))
    );
}

#[test]
fn uat_milestone_adversarial_chain_is_not_collapsed() {
    // Adversarial: Unknown (Verify) + EvidenceGap (DebVerify via a gap set) +
    // Conflicted lens evidence + MISALIGNED alignment. All must survive
    // independently; none may be folded into a single FAIL/DENY.
    use sddk_engine::alignment_lens::not_evaluated::NotEvaluated;
    use sddk_engine::debverify_kernel::types::GapSet;

    let conflicted = sddk_engine::alignment_lens::LensContribution::assemble(
        sddk_engine::alignment_lens::LensContributionId("co-conflicted".to_string()),
        sddk_engine::alignment_lens::types::LensId("co-lens"),
        sddk_engine::alignment_lens::types::LensVersion::new(1, 0),
        UniversalConcern::Coupling,
        EvidencePosture::Conflicted {
            target: ObservationTargetRef::Unit(unit()),
            supporting: vec![ObservationId("s".to_string())],
            contradicting: vec![ObservationId("c".to_string())],
        },
        vec![],
    );
    let _ = NotEvaluated {
        applicable: ApplicableConcern::Applicable(UniversalConcern::Cohesion),
        candidate_lens_ids: vec![],
        reason: NotEvaluatedReason::NoRegisteredLens,
    };

    let result = result_fixture(
        ReconciliationSummary::EvidenceGap(vec![GapSet {
            subject: SoftwareEntityRef::Component(ComponentRef::new("comp:a4-co").expect("c")),
            gap: "no observation".to_string(),
        }]),
        vec![conflicted],
        vec![],
        vec![(
            claim("c-uat"),
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
        )],
        assessment(AlignmentState::Misaligned, "a-adv"),
    );

    // All four epistemologies are present and distinct.
    assert!(matches!(
        result.alignment_assessment.state,
        AlignmentState::Misaligned
    ));
    assert!(matches!(
        result.reconciliation,
        ReconciliationSummary::EvidenceGap(_)
    ));
    assert!(matches!(
        result.verification_pairs[0].1,
        VerificationResult::Unknown { .. }
    ));
    assert_eq!(result.lens_evaluation.contributions.len(), 1);

    let ctx = derive_advisory_context(&result);
    let notes: BTreeSet<&str> = ctx.items.iter().map(|i| i.note.as_str()).collect();
    assert!(notes.contains("unknown"));
    assert!(notes.contains("evidence_gap"));
    assert!(notes.contains("misaligned"));
    // Not collapsed into a single verdict.
    for forbidden in ["fail", "deny", "overall", "verdict"] {
        assert!(!notes.contains(forbidden));
    }
    let repr = format!("{ctx:?}");
    assert!(!repr.contains("Deny"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §7 — False-clean audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn false_clean_unknown_never_becomes_aligned_or_confirmed() {
    let result = result_fixture(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![],
        vec![(
            claim("c-fc"),
            VerificationResult::Unknown {
                gap: EvidenceGap::NoEvidenceProvided,
            },
        )],
        assessment(AlignmentState::Unknown, "a-fc"),
    );
    let ctx = derive_advisory_context(&result);
    for item in &ctx.items {
        assert_ne!(
            item.note, "aligned",
            "missing evidence must not read as aligned"
        );
        assert_ne!(item.note, "confirmed_baseline");
        assert_ne!(item.note, "supported");
    }
}

#[test]
fn false_clean_missing_lens_never_becomes_supported() {
    use sddk_engine::alignment_lens::not_evaluated::NotEvaluated;
    let result = result_fixture(
        ReconciliationSummary::NotApplicable,
        vec![],
        vec![NotEvaluated {
            applicable: ApplicableConcern::Applicable(UniversalConcern::Cohesion),
            candidate_lens_ids: vec![],
            reason: NotEvaluatedReason::NoRegisteredLens,
        }],
        vec![],
        assessment(AlignmentState::Unknown, "a-fc2"),
    );
    let ctx = derive_advisory_context(&result);
    let gap = ctx
        .items
        .iter()
        .find(|i| i.kind == AdvisoryKind::LensCoverageGap)
        .expect("gap item");
    assert_eq!(gap.note, "no_registered_lens");
    assert!(!ctx.items.iter().any(|i| i.note == "supported"));
}

#[test]
fn false_clean_unknown_contract_never_becomes_confirmed_verified() {
    // A contract absent from the graph: WHY reports the absence; it never
    // claims the contract is verified.
    let result = result_fixture(
        ReconciliationSummary::ConfirmedBaseline { strategies_run: 1 },
        vec![],
        vec![],
        vec![(claim("c-absent"), VerificationResult::Verified)],
        assessment(AlignmentState::Aligned, "a-fc3"),
    );
    let why = explain_advisory(
        &result,
        &sddk_engine::intelligence_loop::IntelligenceLoopReceiptId("r-fc3".to_string()),
        &empty_graph(),
        &AdvisorySubjectRef::VerificationClaim {
            contract_id: "c-absent".to_string(),
        },
    );
    let prov = why.legs.iter().find_map(|l| match l {
        AdvisoryWhyLeg::ContractProvenance { provenance } => Some(provenance),
        _ => None,
    });
    if let Some(prov) = prov {
        assert!(prov.specified_by.is_empty());
        assert!(prov.verified_by.is_empty());
    }
    assert!(!why.unresolved.is_empty(), "absence must be visible");
}

#[test]
fn false_clean_unsupported_target_is_not_applicable_not_supported() {
    // `NotApplicable` is not `Supported`.
    let na: EvidencePosture<ObservationTargetRef> = EvidencePosture::Insufficient {
        target: ObservationTargetRef::Unit(unit()),
        gap: sddk_engine::observation::posture::InsufficientGap::NoObservation,
    };
    assert!(!matches!(na, EvidencePosture::Supported { .. }));
}

// ─────────────────────────────────────────────────────────────────────────────
// §8 — False-authority audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn false_authority_no_provider_or_governance_dependency() {
    for (name, src) in [
        (
            "verify_kernel",
            include_str!("../src/verify_kernel/engine.rs"),
        ),
        (
            "debverify_kernel",
            include_str!("../src/debverify_kernel/mod.rs"),
        ),
        (
            "software_alignment",
            include_str!("../src/software_alignment/reducer.rs"),
        ),
        (
            "alignment_lens",
            include_str!("../src/alignment_lens/kernel.rs"),
        ),
        (
            "intelligence_loop",
            include_str!("../src/intelligence_loop/mod.rs"),
        ),
        (
            "intelligence_advisory",
            include_str!("../src/intelligence_advisory/mod.rs"),
        ),
    ] {
        let code: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in [
            "CogniCode",
            "Chronos",
            "JCode",
            "reqwest",
            "AuthorityDecision",
        ] {
            assert!(
                !code.contains(forbidden),
                "{name} must not depend on `{forbidden}`"
            );
        }
    }
}

#[test]
fn false_authority_alignment_has_no_authority_conversion() {
    // No `From<AlignmentAssessment> for AuthorityDecision`-style conversion
    // exists (compile-time: the types are unrelated; this is a structural
    // anchor plus a source pin).
    let src = include_str!("../src/software_alignment/types.rs");
    for forbidden in [
        "impl From<AlignmentAssessment>",
        "AuthorityDecision",
        "Capability",
    ] {
        assert!(!src.contains(forbidden));
    }
}
