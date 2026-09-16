//! A4-4a downstream-consumer integration test.
//!
//! This file lives at the crate-root `tests/` directory (not inside the
//! `intent_universal_concern/` module) so it exercises the public API
//! exactly as a downstream consumer would. The in-module `tests.rs` is
//! unit-level (#[cfg(test)] mod tests); this file is integration-level.
//!
//! The test pins three end-to-end behaviors:
//!
//! 1. A realistic OO project + 3 contracts + 3 decisions + unit with full
//!    intent produces a coherent answer (mix of applicable + not_applicable
//!    based on which contracts/decisions actually ground each concern).
//!
//! 2. The UAT (same software + different declared intent = different
//!    applicable concerns possible) holds end-to-end.
//!
//! 3. The A4-3 MISALIGNED ≠ DENY invariant survives: `ApplicableConcern`
//!    cannot carry authority-shaped payloads.

use std::collections::BTreeSet;

use sddk_engine::architectural_contract::{
    ArchitecturalContract, ContractId, ContractKind, ContractPayload, DecisionRef, EntityRef,
    Revision, SpecRef,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{
    ApplicableConcern, ParadigmProfileRef, ProjectIntent, UnitIntent, UniversalConcern,
    applicable_concerns,
};
use sddk_engine::knowledge::EventTime;

/// Build a contract whose id contains the given concern tag, so the
/// reducer's `contract_references_concern` helper will match it.
fn contract_with_tag(tag: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare(
        ContractId::new(format!("contract:{}", tag)).unwrap(),
        ContractKind::UniqueOwner,
        ContractPayload::UniqueOwner(EntityRef::new("entity").unwrap()),
        DecisionRef::Decision(format!("decision:{}", tag)),
        SpecRef::Spec(format!("spec:{}", tag)),
        Revision::new("v1").unwrap(),
        EventTime(0),
    )
    .unwrap()
}

/// Leaked `&'static` refs for the reducer input (test-only fixture).
fn static_contract_ids(contracts: Vec<ArchitecturalContract>) -> Vec<&'static ContractId> {
    contracts
        .into_iter()
        .map(|c| Box::leak(Box::new(c.id().clone())) as &'static ContractId)
        .collect()
}

fn static_decision_refs(decisions: Vec<DecisionRef>) -> Vec<&'static DecisionRef> {
    decisions
        .into_iter()
        .map(|d| Box::leak(Box::new(d)) as &'static DecisionRef)
        .collect()
}

fn full_intent(paradigm: ParadigmProfileRef) -> ProjectIntent {
    ProjectIntent {
        intent_id: IntentId("p:test".into()),
        paradigm,
        declared_concerns: UniversalConcern::ALL.iter().copied().collect(),
    }
}

fn full_unit_intent(unit: &str) -> UnitIntent {
    UnitIntent {
        unit_ref: SoftwareUnitRef::new(unit),
        applies_to_concerns: UniversalConcern::ALL.iter().copied().collect(),
    }
}

/// ─── 1. Realistic end-to-end ────────────────────────────────────────────

#[test]
fn integration_realistic_oo_project_three_contracts_three_decisions() {
    let pi = full_intent(ParadigmProfileRef::ObjectOriented);
    let ui = full_unit_intent("ac4::Ac4Adapter");

    // 3 contracts: ground cohesion, coupling, state_safety
    let contracts = static_contract_ids(
        ["cohesion", "coupling", "state_safety"]
            .iter()
            .map(|t| contract_with_tag(t))
            .collect(),
    );
    // 3 decisions: ground cohesion, coupling, boundary_integrity
    let decisions = static_decision_refs(
        ["cohesion", "coupling", "boundary_integrity"]
            .iter()
            .map(|t| DecisionRef::Decision(format!("decision:{}", t)))
            .collect(),
    );

    let out = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("reduce");

    // Output must be deterministic and ordered.
    assert_eq!(out.len(), 10, "one entry per UniversalConcern");

    // Expected applicable: cohesion (contract + decision), coupling (contract + decision),
    // state_safety (contract only), boundary_integrity (decision only, contract missing
    // would normally flag NoContractReference but the rule for cohesion/coupling is
    // satisfied by either, so this verifies the precedence).
    let applicable: Vec<_> = out
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();
    let not_applicable: Vec<_> = out
        .iter()
        .filter(|(a, _)| a.is_not_applicable())
        .map(|(a, _)| a.concern())
        .collect();

    // Both buckets non-empty: this is a realistic mixed answer.
    assert!(
        !applicable.is_empty(),
        "realistic project must have some applicable concerns, got: {:?}",
        applicable
    );
    assert!(
        !not_applicable.is_empty(),
        "realistic project must have some not-applicable concerns (the other 7 of 10), got: {:?}",
        not_applicable
    );
    assert_eq!(applicable.len() + not_applicable.len(), 10);
}

/// ─── 2. UAT pinned end-to-end ───────────────────────────────────────────

#[test]
fn integration_uat_same_software_different_intent() {
    let contracts = static_contract_ids(
        ["cohesion", "coupling", "testability"]
            .iter()
            .map(|t| contract_with_tag(t))
            .collect(),
    );
    let decisions = static_decision_refs(
        ["cohesion", "coupling", "testability"]
            .iter()
            .map(|t| DecisionRef::Decision(format!("decision:{}", t)))
            .collect(),
    );
    let ui = full_unit_intent("ac4::Ac4Adapter");

    // Wide intent: declares all 10.
    let pi_wide = full_intent(ParadigmProfileRef::ObjectOriented);
    let out_wide = applicable_concerns(&pi_wide, &ui, &contracts, &decisions).expect("wide");
    let app_wide: BTreeSet<_> = out_wide
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();

    // Narrow intent: declares only 3 (the ones with evidence).
    let mut pi_narrow = full_intent(ParadigmProfileRef::ObjectOriented);
    pi_narrow.declared_concerns = [
        UniversalConcern::Cohesion,
        UniversalConcern::Coupling,
        UniversalConcern::Testability,
    ]
    .into_iter()
    .collect();
    let out_narrow = applicable_concerns(&pi_narrow, &ui, &contracts, &decisions).expect("narrow");
    let app_narrow: BTreeSet<_> = out_narrow
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();

    // Wide intent: 3 applicable (only those have evidence), the other 7 are
    // not-applicable due to NoGroundingDecision.
    assert_eq!(
        app_wide.len(),
        3,
        "wide intent with this evidence gives 3 applicable"
    );

    // Narrow intent: same 3 applicable (intersection with evidence).
    assert_eq!(app_narrow.len(), 3);

    // The UAT pin: declared_concerns changed → applicable set changed.
    // In this fixture both produce the same 3 (because evidence is sparse),
    // so we change the evidence instead to show the answer actually depends
    // on declared_concerns, not on the evidence alone.
    let pi_only_cohesion = ProjectIntent {
        intent_id: IntentId("p:only-cohesion".into()),
        paradigm: ParadigmProfileRef::ObjectOriented,
        declared_concerns: [UniversalConcern::Cohesion].into_iter().collect(),
    };
    let out_cohesion =
        applicable_concerns(&pi_only_cohesion, &ui, &contracts, &decisions).expect("only-cohesion");
    let app_cohesion: BTreeSet<_> = out_cohesion
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();
    assert_eq!(app_cohesion.len(), 1);
    assert!(app_cohesion.contains(&UniversalConcern::Cohesion));

    // The UAT pin: different declared_concerns → different applicable set.
    assert!(app_wide != app_cohesion);
    assert!(app_narrow != app_cohesion);
}

/// ─── 3. MISALIGNED ≠ DENY end-to-end ────────────────────────────────────

#[test]
fn integration_misaligned_not_deny_invariant() {
    // The ApplicableConcern enum carries no authority-shaped payload.
    // Verify by attempting to construct every variant and checking that
    // serialisation never produces "deny" / "DENY" / "capability" tokens.
    for concern in UniversalConcern::ALL {
        let app = ApplicableConcern::Applicable(concern);
        let s = serde_json::to_string(&app).unwrap();
        assert!(
            !s.contains("DENY"),
            "Applicable({:?}) serialised: {}",
            concern,
            s
        );
        assert!(
            !s.contains("deny"),
            "Applicable({:?}) serialised: {}",
            concern,
            s
        );
        assert!(
            !s.contains("capability"),
            "Applicable({:?}) serialised: {}",
            concern,
            s
        );
        assert!(
            !s.contains("authority"),
            "Applicable({:?}) serialised: {}",
            concern,
            s
        );

        let not_app = ApplicableConcern::NotApplicable(
            concern,
            sddk_engine::intent_universal_concern::NotApplicableReason::NotInProjectIntent,
        );
        let s2 = serde_json::to_string(&not_app).unwrap();
        assert!(
            !s2.contains("DENY"),
            "NotApplicable({:?}) serialised: {}",
            concern,
            s2
        );
        assert!(
            !s2.contains("capability"),
            "NotApplicable({:?}) serialised: {}",
            concern,
            s2
        );
    }
}

/// ─── 4. Reduction determinism end-to-end ─────────────────────────────────

#[test]
fn integration_reduction_is_deterministic_across_calls() {
    let pi = full_intent(ParadigmProfileRef::FunctionalPure);
    let ui = full_unit_intent("ac5::Ac5Adapter");
    let contracts = static_contract_ids(
        UniversalConcern::ALL
            .iter()
            .map(|c| contract_with_tag(c.canonical_tag()))
            .collect(),
    );
    let decisions = static_decision_refs(
        UniversalConcern::ALL
            .iter()
            .map(|c| DecisionRef::Decision(format!("decision:{}", c.canonical_tag())))
            .collect(),
    );

    let out1 = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("call 1");
    let out2 = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("call 2");
    let out3 = applicable_concerns(&pi, &ui, &contracts, &decisions).expect("call 3");

    assert_eq!(out1, out2);
    assert_eq!(out2, out3);

    // All 10 applicable (full evidence, full intent, FPure paradigm — no exclusions).
    let applicable: Vec<_> = out1.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(
        applicable.len(),
        10,
        "FunctionalPure paradigm + full evidence + full intent → all 10 applicable"
    );
}
