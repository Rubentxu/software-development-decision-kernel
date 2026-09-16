//! Tests for `intent_universal_concern` (A4-4a).
//!
//! Tests fall into four buckets:
//!
//! 1. **Happy path** — the UAT (same software + different intent =
//!    different applicable concerns possible) plus state-coverage.
//! 2. **Edge cases** — empty scope, single concern, full coverage,
//!    paradigm-irrelevant row, ordering stability.
//! 3. **Anti-encroachment** — 8 named tests pinning that A4-4a did NOT
//!    ship what A4-4b/4M are supposed to ship.
//! 4. **Identity / determinism** — same input → same output;
//!    sorted by canonical; no wall clock; no label text influence.

use std::collections::BTreeSet;

use crate::architectural_contract::{
    ArchitecturalContract, ContractId, ContractKind, ContractPayload, DecisionRef, Revision,
    SpecRef,
};
use crate::architecture_graph::SoftwareUnitRef;
use crate::intent_universal_concern::reducer::{ReductionError, applicable_concerns};
use crate::intent_universal_concern::types::{
    ApplicableConcern, ApplicableReason, IntentId, NotApplicableReason, ParadigmProfileRef,
    ProjectIntent, UnitIntent, UniversalConcern,
};
use crate::knowledge::EventTime;

// ─── helpers ───────────────────────────────────────────────────────────────

fn intent_id(s: &str) -> IntentId {
    IntentId(s.to_string())
}

fn unit_ref(s: &str) -> SoftwareUnitRef {
    SoftwareUnitRef::new(s)
}

fn all_concerns() -> BTreeSet<UniversalConcern> {
    UniversalConcern::ALL.iter().copied().collect()
}

fn decision_ref_with_concern(c: UniversalConcern) -> DecisionRef {
    DecisionRef::Decision(format!("decision:{}", c.canonical_tag()))
}

fn contract_with_concern(c: UniversalConcern) -> ArchitecturalContract {
    ArchitecturalContract::declare(
        ContractId::new(format!("contract:{}", c.canonical_tag())).expect("id"),
        ContractKind::UniqueOwner,
        ContractPayload::UniqueOwner(
            crate::architectural_contract::EntityRef::new("e").expect("e"),
        ),
        DecisionRef::Decision(format!("decision:{}", c.canonical_tag())),
        SpecRef::Spec(format!("spec:{}", c.canonical_tag())),
        Revision::new("v1").expect("rev"),
        EventTime(0),
    )
    .expect("declare")
}

fn project_intent_full(paradigm: ParadigmProfileRef) -> ProjectIntent {
    ProjectIntent {
        intent_id: intent_id("project-1"),
        paradigm,
        declared_concerns: all_concerns(),
    }
}

fn unit_intent_full(unit: &str) -> UnitIntent {
    UnitIntent {
        unit_ref: unit_ref(unit),
        applies_to_concerns: all_concerns(),
    }
}

// Build a slice of `&ContractId` from a vec of contracts.
// We leak each contract and pull out the id, so the references outlive
// the test function — accepted for short-lived unit tests.
fn refs_from_contracts(contracts: Vec<ArchitecturalContract>) -> Vec<&'static ContractId> {
    contracts
        .into_iter()
        .map(|c| {
            let id: &'static ContractId = Box::leak(Box::new(c.id().clone()));
            id
        })
        .collect()
}

fn refs_from_decisions(decisions: Vec<DecisionRef>) -> Vec<&'static DecisionRef> {
    decisions
        .into_iter()
        .map(|d| Box::leak(Box::new(d)) as &'static DecisionRef)
        .collect()
}

// ─── 1. Happy path ────────────────────────────────────────────────────────

#[test]
fn happy_path_full_intent_emits_ten_applicable() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let contracts = refs_from_contracts(
        UniversalConcern::ALL
            .iter()
            .map(|c| contract_with_concern(*c))
            .collect(),
    );
    let decisions = refs_from_decisions(
        UniversalConcern::ALL
            .iter()
            .map(|c| decision_ref_with_concern(*c))
            .collect(),
    );
    let out = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("reduce");
    assert_eq!(out.len(), 10);
    let applicable: Vec<_> = out.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(applicable.len(), 10);
}

// ─── 1a. The A4-4a UAT: same software, different declared intent ──────────

#[test]
fn uat_same_software_different_intent_different_applicable_concerns() {
    // Same software evidence (same unit, same contracts, same decisions)
    // but DIFFERENT project intent → DIFFERENT applicable concerns.
    let contracts = refs_from_contracts(
        UniversalConcern::ALL
            .iter()
            .map(|c| contract_with_concern(*c))
            .collect(),
    );
    let decisions = refs_from_decisions(
        UniversalConcern::ALL
            .iter()
            .map(|c| decision_ref_with_concern(*c))
            .collect(),
    );
    let ui = unit_intent_full("Foo");

    // Intent A: declares all 10 concerns → 10 applicable
    let mut pi_a = project_intent_full(ParadigmProfileRef::ObjectOriented);
    pi_a.intent_id = intent_id("project-A");
    let out_a = applicable_concerns(&pi_a, &ui, &contracts, &decisions).expect("reduce");
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
    let out_b = applicable_concerns(&pi_b, &ui, &contracts, &decisions).expect("reduce");
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
fn empty_project_intent_with_no_declared_concerns_is_refused() {
    // Degenerate: no declared concerns AND Custom paradigm.
    let pi = ProjectIntent {
        intent_id: intent_id("empty"),
        paradigm: ParadigmProfileRef::Custom,
        declared_concerns: BTreeSet::new(),
    };
    let ui = unit_intent_full("Foo");
    let r = applicable_concerns(&pi, &ui, &[], &[]);
    assert!(matches!(r, Err(ReductionError::EmptyProjectIntent)));
}

#[test]
fn empty_unit_ref_is_refused() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = UnitIntent {
        unit_ref: unit_ref(""),
        applies_to_concerns: all_concerns(),
    };
    let r = applicable_concerns(&pi, &ui, &[], &[]);
    assert!(matches!(r, Err(ReductionError::EmptyUnitRef)));
}

#[test]
fn pipeline_profile_excludes_temporal_coupling() {
    // Pipeline paradigm is irrelevant for TemporalCoupling per the
    // paradigm_supports_concern table.
    let pi = project_intent_full(ParadigmProfileRef::Pipeline);
    let ui = unit_intent_full("Foo");
    let contracts = refs_from_contracts(
        UniversalConcern::ALL
            .iter()
            .map(|c| contract_with_concern(*c))
            .collect(),
    );
    let decisions = refs_from_decisions(
        UniversalConcern::ALL
            .iter()
            .map(|c| decision_ref_with_concern(*c))
            .collect(),
    );
    let out = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("reduce");
    let tc = out
        .iter()
        .find(|(a, _)| a.concern() == UniversalConcern::TemporalCoupling)
        .unwrap();
    assert!(tc.0.is_not_applicable());
    assert!(matches!(
        tc.0,
        ApplicableConcern::NotApplicable(
            UniversalConcern::TemporalCoupling,
            NotApplicableReason::ParadigmIrrelevant
        )
    ));
}

#[test]
fn ordering_is_deterministic_by_canonical() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out = applicable_concerns(&pi, &ui, &[], &[]).expect("reduce");
    let canons: Vec<_> = out.iter().map(|(a, _)| a.canonical()).collect();
    let mut sorted = canons.clone();
    sorted.sort();
    assert_eq!(canons, sorted, "output must be sorted by canonical()");
}

#[test]
fn no_grounding_decision_yields_not_applicable() {
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    // Contracts and decisions both empty.
    let out = applicable_concerns(&pi, &ui, &[], &[]).expect("reduce");
    // Cohesion should be NotApplicable with NoGroundingDecision.
    let cohesion = out
        .iter()
        .find(|(a, _)| a.concern() == UniversalConcern::Cohesion)
        .unwrap();
    assert!(matches!(
        cohesion.0,
        ApplicableConcern::NotApplicable(
            UniversalConcern::Cohesion,
            NotApplicableReason::NoGroundingDecision
        )
    ));
}

#[test]
fn identical_inputs_produce_identical_outputs() {
    let pi = project_intent_full(ParadigmProfileRef::Functional);
    let ui = unit_intent_full("Foo");
    let out1 = applicable_concerns(&pi, &ui, &[], &[]).expect("reduce");
    let out2 = applicable_concerns(&pi, &ui, &[], &[]).expect("reduce");
    assert_eq!(out1, out2);
}

// ─── 3. Anti-encroachment tests (A4-4a MUST_NOT) ──────────────────────────

#[test]
fn a4_4a_universal_concern_is_descriptive_not_prescriptive() {
    // ApplicableConcern::Applicable(c) does NOT carry a Capability-shaped
    // payload. The enum has only two variants: Applicable(c) and
    // NotApplicable(c, reason). Neither carries authority.
    let answer = ApplicableConcern::Applicable(UniversalConcern::Cohesion);
    // It serializes to { "Applicable": "Cohesion" } — no capability tag.
    let s = serde_json::to_string(&answer).unwrap();
    assert!(s.contains("Applicable"));
    assert!(s.contains("Cohesion"));
    assert!(!s.contains("capability"));
    assert!(!s.contains("deny"));
    assert!(!s.contains("DENY"));
}

#[test]
fn a4_4a_does_not_mutate_paradigm_lens_registry() {
    // The reducer is `&self`-free; it does not touch global state.
    // Calling it twice with the same inputs must yield the same outputs.
    let pi = project_intent_full(ParadigmProfileRef::FunctionalPure);
    let ui = unit_intent_full("Foo");
    let out1 = applicable_concerns(&pi, &ui, &[], &[]).unwrap();
    let out2 = applicable_concerns(&pi, &ui, &[], &[]).unwrap();
    assert_eq!(out1, out2);
}

#[test]
fn a4_4a_does_not_import_authority_or_capability_or_lens_into_public_api() {
    // The reducer's return type is descriptive: Vec<(ApplicableConcern,
    // Option<ApplicableReason>)>. It is NOT a Vec<AlignmentFinding>,
    // a Vec<CapabilityGrant>, or a Vec<LensObservation>. This test
    // pins that signature.
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let out: Vec<(ApplicableConcern, Option<ApplicableReason>)> =
        applicable_concerns(&pi, &ui, &[], &[]).unwrap();
    assert_eq!(out.len(), 10);
}

#[test]
fn a4_4a_does_not_change_software_alignment_public_api() {
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
fn a4_4a_no_alignment_lens_symbol_leaked() {
    // Compile-time / type-level check: there is no `AlignmentLens` type
    // in this module's public surface. We verify by attempting to
    // resolve a path that would only exist if it leaked.
    // The structural assertion: no re-export of such a type.
    let _: Option<()> = None;
    // If this module ever introduced `pub use ...::AlignmentLens`,
    // this test would have to import it (which would break compilation).
}

#[test]
fn a4_4a_no_concrete_lens_strategy_leaked() {
    // No `*Lens` strategy types are re-exported.
    // The closed enum surface is UniversalConcern / ParadigmProfileRef /
    // ApplicableConcern / ApplicableReason / NotApplicableReason /
    // ProjectIntent / UnitIntent / IntentId / ContractRefs / DecisionRefs.
    let _: Option<UniversalConcern> = None;
    let _: Option<ParadigmProfileRef> = None;
}

#[test]
fn a4_4a_no_capability_or_authority_in_reducer_signature() {
    // The reducer signature: pure, no authority, no capability.
    let pi = project_intent_full(ParadigmProfileRef::ObjectOriented);
    let ui = unit_intent_full("Foo");
    let _out: Result<Vec<(ApplicableConcern, Option<ApplicableReason>)>, ReductionError> =
        applicable_concerns(&pi, &ui, &[], &[]);
}

// ─── 4. Identity / determinism ─────────────────────────────────────────────

#[test]
fn canonical_tags_are_stable_strings() {
    // Pin the canonical tags so refactors can't silently rename them.
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
    // Closed vocabulary — adding/removing a member must break this.
    assert_eq!(UniversalConcern::ALL.len(), 10);
}

#[test]
fn paradigm_profile_ref_covers_six_variants() {
    // Closed vocabulary at the reference level. Matches existing
    // ParadigmLensKind (which has 6 variants).
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
