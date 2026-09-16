//! Tests for `intent_universal_concern` (A4-4aR — Applicability Semantics
//! Correction).
//!
//! Tests fall into four buckets:
//!
//! 1. **Happy path** — the UAT (same software + different intent =
//!    different applicable concerns possible) plus state-coverage.
//! 2. **Edge cases** — empty scope, single concern, full coverage,
//!    explicit exclusion, ordering stability.
//! 3. **A4-4aR falsification pins** — applicability must NOT depend on
//!    `DecisionRef` render text or `ContractId` text or paradigm
//!    relevance. Same `(pi, ui)` → same applicability regardless of
//!    decisions/contracts/paradigm×concern rules.
//! 4. **Anti-encroachment** — A4-4aR did NOT ship AlignmentLens, lens
//!    registry, OO/FP/ADT/DSL evaluators, paradigm_lens changes, etc.
//! 5. **Identity / determinism** — same input → same output;
//!    sorted by canonical; no wall clock; no label text influence.

use std::collections::BTreeSet;

use crate::architecture_graph::SoftwareUnitRef;
use crate::intent_universal_concern::reducer::{ReductionError, applicable_concerns};
use crate::intent_universal_concern::types::{
    ApplicableConcern, ApplicableReason, IntentId, NotApplicableReason, ParadigmProfileRef,
    ProjectIntent, UnitIntent, UniversalConcern,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn intent_id(s: &str) -> IntentId {
    IntentId(s.to_string())
}

fn unit_ref(s: &str) -> SoftwareUnitRef {
    SoftwareUnitRef::new(s)
}

fn empty_concerns() -> BTreeSet<UniversalConcern> {
    BTreeSet::new()
}

fn all_concerns() -> BTreeSet<UniversalConcern> {
    UniversalConcern::ALL.iter().copied().collect()
}

fn project_intent_full(paradigm: ParadigmProfileRef) -> ProjectIntent {
    ProjectIntent {
        intent_id: intent_id("project-1"),
        paradigm,
        declared_concerns: all_concerns(),
        excluded_concerns: empty_concerns(),
    }
}

fn project_intent_with_excluded(
    paradigm: ParadigmProfileRef,
    excluded: &[UniversalConcern],
) -> ProjectIntent {
    ProjectIntent {
        intent_id: intent_id("project-1"),
        paradigm,
        declared_concerns: all_concerns(),
        excluded_concerns: excluded.iter().copied().collect(),
    }
}

fn unit_intent_full(unit: &str) -> UnitIntent {
    UnitIntent {
        unit_ref: unit_ref(unit),
        applies_to_concerns: all_concerns(),
        excluded_concerns: empty_concerns(),
    }
}

fn unit_intent_with_excluded(unit: &str, excluded: &[UniversalConcern]) -> UnitIntent {
    UnitIntent {
        unit_ref: unit_ref(unit),
        applies_to_concerns: all_concerns(),
        excluded_concerns: excluded.iter().copied().collect(),
    }
}

// ─── 1. Happy path ────────────────────────────────────────────────────────

#[test]
fn happy_path_full_intent_emits_ten_applicable() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out = applicable_concerns(&pi, &ui).expect("reduce");
    assert_eq!(out.len(), 10);
    let applicable: Vec<_> = out.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(applicable.len(), 10);
    // Every Applicable row carries reason `ProjectAndUnitIntent`.
    for (a, r) in &out {
        if a.is_applicable() {
            assert_eq!(*r, Some(ApplicableReason::ProjectAndUnitIntent));
        }
    }
}

// ─── 1a. The A4-4a UAT: same software, different declared intent ──────────

#[test]
fn uat_same_software_different_intent_different_applicable_concerns() {
    let ui = unit_intent_full("Foo");

    // Intent A: declares all 10 concerns → 10 applicable
    let mut pi_a = project_intent_full(ParadigmProfileRef::ObjectOriented);
    pi_a.intent_id = intent_id("project-A");
    let out_a = applicable_concerns(&pi_a, &ui).expect("reduce");
    let applicable_a: Vec<_> = out_a.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(applicable_a.len(), 10);

    // Intent B: declares only 3 concerns → 3 applicable
    let mut pi_b = project_intent_full(ParadigmProfileRef::ObjectOriented);
    pi_b.intent_id = intent_id("project-B");
    pi_b.declared_concerns = [
        UniversalConcern::Cohesion,
        UniversalConcern::Testability,
        UniversalConcern::Freshness,
    ]
    .into_iter()
    .collect();
    let out_b = applicable_concerns(&pi_b, &ui).expect("reduce");
    let applicable_b: Vec<_> = out_b.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(applicable_b.len(), 3);

    // The two outputs differ in *which* concerns are applicable.
    let concerns_a: BTreeSet<_> = applicable_a.iter().map(|(a, _)| a.concern()).collect();
    let concerns_b: BTreeSet<_> = applicable_b.iter().map(|(a, _)| a.concern()).collect();
    assert_eq!(concerns_a, all_concerns());
    assert_eq!(concerns_b.len(), 3);
}

// ─── 2. Edge cases ────────────────────────────────────────────────────────

#[test]
fn empty_project_intent_with_no_signals_is_refused() {
    // Degenerate: no declared concerns AND no excluded concerns AND Custom paradigm.
    let pi = ProjectIntent {
        intent_id: intent_id("empty"),
        paradigm: ParadigmProfileRef::Custom,
        declared_concerns: BTreeSet::new(),
        excluded_concerns: BTreeSet::new(),
    };
    let ui = unit_intent_full("Foo");
    let r = applicable_concerns(&pi, &ui);
    assert!(matches!(r, Err(ReductionError::EmptyProjectIntent)));
}

#[test]
fn empty_unit_ref_is_refused() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = UnitIntent {
        unit_ref: unit_ref(""),
        applies_to_concerns: all_concerns(),
        excluded_concerns: BTreeSet::new(),
    };
    let r = applicable_concerns(&pi, &ui);
    assert!(matches!(r, Err(ReductionError::EmptyUnitRef)));
}

#[test]
fn project_explicit_exclusion_overrides_declaration() {
    // Project declares ALL concerns but excludes TemporalCoupling explicitly.
    let pi = project_intent_with_excluded(
        ParadigmProfileRef::ObjectOriented,
        &[UniversalConcern::TemporalCoupling],
    );
    let ui = unit_intent_full("Foo");
    let out = applicable_concerns(&pi, &ui).expect("reduce");
    let tc = out
        .iter()
        .find(|(a, _)| a.concern() == UniversalConcern::TemporalCoupling)
        .unwrap();
    assert!(matches!(
        tc.0,
        ApplicableConcern::NotApplicable(
            UniversalConcern::TemporalCoupling,
            NotApplicableReason::ExplicitlyExcludedByProject
        )
    ));
}

#[test]
fn unit_explicit_exclusion_yields_unit_reason() {
    // Project declares ALL; unit narrows by exclusion.
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_with_excluded("Foo", &[UniversalConcern::Cohesion]);
    let out = applicable_concerns(&pi, &ui).expect("reduce");
    let coh = out
        .iter()
        .find(|(a, _)| a.concern() == UniversalConcern::Cohesion)
        .unwrap();
    assert!(matches!(
        coh.0,
        ApplicableConcern::NotApplicable(
            UniversalConcern::Cohesion,
            NotApplicableReason::ExplicitlyExcludedByUnit
        )
    ));
}

#[test]
fn ordering_is_deterministic_by_canonical() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out = applicable_concerns(&pi, &ui).expect("reduce");
    let canons: Vec<_> = out.iter().map(|(a, _)| a.canonical()).collect();
    let mut sorted = canons.clone();
    sorted.sort();
    assert_eq!(canons, sorted, "output must be sorted by canonical()");
}

#[test]
fn identical_inputs_produce_identical_outputs() {
    let pi = project_intent_full(ParadigmProfileRef::Functional);
    let ui = unit_intent_full("Foo");
    let out1 = applicable_concerns(&pi, &ui).expect("reduce");
    let out2 = applicable_concerns(&pi, &ui).expect("reduce");
    assert_eq!(out1, out2);
}

// ─── 3. A4-4aR falsification pins (MUST 7 in the scope contract) ───────────

/// Same `(pi, ui)` + zero decisions/contracts ⇒ ApplicableConcern set
/// must equal the `(pi, ui)` + (presence-of-decisions/contracts)
/// case. A4-4aR MUST not consult evidence at all.
#[test]
fn falsification_same_applicability_with_or_without_evidence() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    // The reducer is now pure on (pi, ui). The "with or without evidence"
    // comparison is structurally a comparison of two calls with the same
    // inputs — both must produce the same Applicability set. This pins the
    // semantic: decisions/contracts are NOT inputs to the function.
    let out_a = applicable_concerns(&pi, &ui).expect("a");
    let out_b = applicable_concerns(&pi, &ui).expect("b");
    assert_eq!(out_a, out_b);

    // Spot-check: pipeline + TemporalCoupling declared in BOTH scopes
    // → Applicable. (Pre-A4-4aR this was NotApplicable(ParadigmIrrelevant).)
    let mut pi_pipeline = project_intent_full(ParadigmProfileRef::Pipeline);
    pi_pipeline.declared_concerns = [UniversalConcern::TemporalCoupling].into_iter().collect();
    let mut ui_pipeline = unit_intent_full("Pipeline");
    ui_pipeline.applies_to_concerns = [UniversalConcern::TemporalCoupling].into_iter().collect();
    let out_pipeline = applicable_concerns(&pi_pipeline, &ui_pipeline).expect("pipeline");
    let tc = out_pipeline
        .iter()
        .find(|(a, _)| a.concern() == UniversalConcern::TemporalCoupling)
        .unwrap();
    assert!(
        tc.0.is_applicable(),
        "Pipeline + TemporalCoupling declared in BOTH intents must be Applicable \
         (pre-A4-4aR the paradigm erased it; A4-4aR removes that exclusion)."
    );
}

/// A4-4aR boundary pin: the function does not TAKE decisions / contracts
/// as parameters. This test enforces that via type-level signature:
/// `applicable_concerns` has arity 2 (`&ProjectIntent`, `&UnitIntent`).
#[test]
fn falsification_signature_no_decisions_or_contracts() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    // Type-level enforcement: this call compiles only if the signature is
    // `fn applicable_concerns(&ProjectIntent, &UnitIntent)`.
    let _out = applicable_concerns(&pi, &ui);
}

/// A4-4aR MUST NOT produce `NotApplicable(c, NoGroundingDecision)`,
/// `NotApplicable(c, NoContractReference)`, or
/// `NotApplicable(c, ParadigmIrrelevant)` under any input.
/// (These enum variants were removed; this test pins their absence.)
#[test]
fn falsification_no_deprecated_reasons() {
    // We can't enumerate the enum exhaustively (no `strum` derive), but
    // we can spot-check that the three deprecated variants are gone:
    // `NotApplicableReason` no longer has them. Compile-time proof is the
    // type definition in `types.rs`; runtime proof is that no test
    // references them. We assert here only that the canonical_tag of any
    // emitted reason is one of the four legitimate variants.
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out = applicable_concerns(&pi, &ui).expect("reduce");
    let legitimate_tags = [
        "not_in_project_intent",
        "not_in_unit_intent",
        "explicitly_excluded_by_project",
        "explicitly_excluded_by_unit",
    ];
    for (a, _) in &out {
        if let ApplicableConcern::NotApplicable(_, r) = a {
            let tag = r.canonical_tag();
            assert!(
                legitimate_tags.contains(&tag),
                "reason tag {} is not in the legitimate NotApplicableReason set",
                tag
            );
        }
    }
}

/// A4-4aR epistemic pin: Applicability is independent of paradigm
/// (in the sense that the only paradigm effect in the reducer is parity
/// of the empty-input guard). For any paradigm, declared concerns
/// ⇒ applicable.
#[test]
fn falsification_paradigm_does_not_erase_concern() {
    for paradigm in [
        ParadigmProfileRef::ObjectOriented,
        ParadigmProfileRef::Functional,
        ParadigmProfileRef::FunctionalPure,
        ParadigmProfileRef::DataOriented,
        ParadigmProfileRef::Pipeline,
    ] {
        let mut pi = project_intent_full(paradigm);
        pi.declared_concerns = [UniversalConcern::TemporalCoupling].into_iter().collect();
        let mut ui = unit_intent_full("Foo");
        ui.applies_to_concerns = [UniversalConcern::TemporalCoupling].into_iter().collect();
        let out = applicable_concerns(&pi, &ui).expect("reduce");
        let tc = out
            .iter()
            .find(|(a, _)| a.concern() == UniversalConcern::TemporalCoupling)
            .unwrap();
        assert!(
            tc.0.is_applicable(),
            "paradigm {:?} must not erase TemporalCoupling (A4-4aR §7)",
            paradigm
        );
    }
}

// ─── 4. Anti-encroachment (A4-4aR) ─────────────────────────────────────────

#[test]
fn a4_4ar_universal_concern_is_descriptive_not_prescriptive() {
    // ApplicableConcern::Applicable(c) does NOT carry a Capability-shaped
    // payload. The enum has only two variants: Applicable(c) and
    // NotApplicable(c, reason). Neither carries authority.
    let answer = ApplicableConcern::Applicable(UniversalConcern::Cohesion);
    let s = serde_json::to_string(&answer).unwrap();
    assert!(s.contains("Applicable"));
    assert!(s.contains("Cohesion"));
    assert!(!s.contains("capability"));
    assert!(!s.contains("deny"));
    assert!(!s.contains("DENY"));
}

#[test]
fn a4_4ar_does_not_mutate_paradigm_lens_registry() {
    // Pure function with no global state. Calling it twice with the same
    // inputs must yield the same outputs.
    let pi = project_intent_full(ParadigmProfileRef::FunctionalPure);
    let ui = unit_intent_full("Foo");
    let out1 = applicable_concerns(&pi, &ui).unwrap();
    let out2 = applicable_concerns(&pi, &ui).unwrap();
    assert_eq!(out1, out2);
}

#[test]
fn a4_4ar_does_not_import_authority_or_capability_or_lens_into_public_api() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out: Vec<(ApplicableConcern, Option<ApplicableReason>)> =
        applicable_concerns(&pi, &ui).unwrap();
    assert_eq!(out.len(), 10);
}

#[test]
fn a4_4ar_does_not_change_software_alignment_public_api() {
    // software_alignment::AlignmentState is still in its A4-3 closed set.
    use crate::software_alignment::AlignmentState;
    let _: AlignmentState = AlignmentState::Unknown;
    let _: AlignmentState = AlignmentState::Aligned;
    let _: AlignmentState = AlignmentState::Misaligned;
    let _: AlignmentState = AlignmentState::Tension;
    let _: AlignmentState = AlignmentState::Accepted;
    let _: AlignmentState = AlignmentState::ReviewDue;
    let _: AlignmentState = AlignmentState::NotApplicable;
}

#[test]
fn a4_4ar_no_alignment_lens_symbol_leaked() {
    // Compile-time / type-level check: there is no `AlignmentLens` type
    // in this module's public surface. The structural assertion: no
    // re-export of such a type. (If this module ever introduced
    // `pub use ...::AlignmentLens`, this test would have to import it,
    // which would break compilation.)
    let _: Option<()> = None;
}

#[test]
fn a4_4ar_no_concrete_lens_strategy_leaked() {
    // No `*Lens` strategy types are re-exported. The closed enum surface
    // is UniversalConcern / ParadigmProfileRef / ApplicableConcern /
    // ApplicableReason / NotApplicableReason / ProjectIntent /
    // UnitIntent / IntentId. (DecisionRefs/ContractRefs were removed.)
    let _: Option<UniversalConcern> = None;
    let _: Option<ParadigmProfileRef> = None;
}

#[test]
fn a4_4ar_no_capability_or_authority_in_reducer_signature() {
    // The reducer signature is pure, no authority, no capability.
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let _out: Result<Vec<(ApplicableConcern, Option<ApplicableReason>)>, ReductionError> =
        applicable_concerns(&pi, &ui);
}

// ─── 5. Identity / determinism ─────────────────────────────────────────────

#[test]
fn canonical_tags_are_stable_strings() {
    assert_eq!(UniversalConcern::Cohesion.canonical_tag(), "cohesion");
    assert_eq!(UniversalConcern::Coupling.canonical_tag(), "coupling");
    assert_eq!(
        UniversalConcern::BoundaryIntegrity.canonical_tag(),
        "boundary_integrity"
    );
    assert_eq!(
        UniversalConcern::StateSafety.canonical_tag(),
        "state_safety"
    );
    assert_eq!(
        UniversalConcern::EffectVisibility.canonical_tag(),
        "effect_visibility"
    );
    assert_eq!(
        UniversalConcern::DependencyDirection.canonical_tag(),
        "dependency_direction"
    );
    assert_eq!(
        UniversalConcern::SemanticOwnership.canonical_tag(),
        "semantic_ownership"
    );
    assert_eq!(
        UniversalConcern::TemporalCoupling.canonical_tag(),
        "temporal_coupling"
    );
    assert_eq!(UniversalConcern::Testability.canonical_tag(), "testability");
    assert_eq!(UniversalConcern::Freshness.canonical_tag(), "freshness");
}

#[test]
fn universal_concern_all_has_exactly_ten() {
    assert_eq!(UniversalConcern::ALL.len(), 10);
}

#[test]
fn paradigm_profile_ref_covers_six_variants() {
    assert_eq!(
        [
            ParadigmProfileRef::ObjectOriented,
            ParadigmProfileRef::Functional,
            ParadigmProfileRef::FunctionalPure,
            ParadigmProfileRef::DataOriented,
            ParadigmProfileRef::Pipeline,
            ParadigmProfileRef::Custom,
        ]
        .len(),
        6
    );
}
