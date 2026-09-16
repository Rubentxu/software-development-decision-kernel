//! A4-4aR downstream-consumer integration test.
//!
//! Lives at the crate-root `tests/` directory so it exercises the public
//! API exactly as a downstream consumer would. The in-module `tests.rs`
//! is unit-level; this file is integration-level.
//!
//! Pins four end-to-end behaviors under A4-4aR semantics:
//!
//! 1. A realistic OO project + full unit intent produces all 10 applicable
//!    concerns (Applicability no longer depends on contracts/decisions;
//!    ground truth lives outside Applicability in A4-4b).
//!
//! 2. The UAT (same software + different declared intent =
//!    different applicable concerns) holds.
//!
//! 3. The A4-3 MISALIGNED ≠ DENY invariant: `ApplicableConcern` cannot
//!    carry authority-shaped payloads.
//!
//! 4. Determinism across calls.
//!
//! 5. A4-4aR falsification: applicability is independent of any external
//!    decisions/contracts (the reducer takes only `(pi, ui)`).

use std::collections::BTreeSet;

use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{
    ApplicableConcern, ParadigmProfileRef, ProjectIntent, UnitIntent, UniversalConcern,
    applicable_concerns,
};

fn full_intent(paradigm: ParadigmProfileRef) -> ProjectIntent {
    ProjectIntent {
        intent_id: IntentId("p:test".into()),
        paradigm,
        declared_concerns: UniversalConcern::ALL.iter().copied().collect(),
        excluded_concerns: BTreeSet::new(),
    }
}

fn full_unit_intent(unit: &str) -> UnitIntent {
    UnitIntent {
        unit_ref: sddk_engine::architecture_graph::SoftwareUnitRef::new(unit),
        applies_to_concerns: UniversalConcern::ALL.iter().copied().collect(),
        excluded_concerns: BTreeSet::new(),
    }
}

/// ─── 1. Realistic end-to-end ────────────────────────────────────────────

#[test]
fn integration_full_oo_intent_emits_ten_applicable() {
    let pi = full_intent(ParadigmProfileRef::ObjectOriented);
    let ui = full_unit_intent("ac4::Ac4Adapter");

    // A4-4aR: contracts and decisions are NOT inputs to the function.
    // The reducer takes only (pi, ui).
    let out = applicable_concerns(&pi, &ui).expect("reduce");

    assert_eq!(out.len(), 10, "one entry per UniversalConcern");
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

    // Pre-A4-4aR this was mixed based on evidence; now all 10 are
    // Applicable because intent declares them all.
    assert_eq!(applicable.len(), 10, "all 10 declared → all 10 applicable");
    assert_eq!(not_applicable.len(), 0, "no NotApplicable rows");

    // Every Applicable row carries reason `ProjectAndUnitIntent`.
    for (a, r) in &out {
        if a.is_applicable() {
            assert_eq!(
                *r,
                Some(sddk_engine::intent_universal_concern::ApplicableReason::ProjectAndUnitIntent)
            );
        }
    }
}

/// ─── 2. UAT pinned end-to-end ───────────────────────────────────────────

#[test]
fn integration_uat_same_software_different_intent() {
    let ui = full_unit_intent("ac4::Ac4Adapter");

    // Wide intent: declares all 10.
    let pi_wide = full_intent(ParadigmProfileRef::ObjectOriented);
    let out_wide = applicable_concerns(&pi_wide, &ui).expect("wide");
    let app_wide: BTreeSet<_> = out_wide
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();
    assert_eq!(app_wide.len(), 10);

    // Narrow intent: declares only Cohesion.
    let mut pi_narrow = full_intent(ParadigmProfileRef::ObjectOriented);
    pi_narrow.declared_concerns = [UniversalConcern::Cohesion].into_iter().collect();
    let out_narrow = applicable_concerns(&pi_narrow, &ui).expect("narrow");
    let app_narrow: BTreeSet<_> = out_narrow
        .iter()
        .filter(|(a, _)| a.is_applicable())
        .map(|(a, _)| a.concern())
        .collect();
    assert_eq!(app_narrow.len(), 1);
    assert!(app_narrow.contains(&UniversalConcern::Cohesion));

    // UAT pin: different declared_concerns → different applicable set.
    assert!(app_wide != app_narrow);
}

/// ─── 3. MISALIGNED ≠ DENY end-to-end ────────────────────────────────────

#[test]
fn integration_misaligned_not_deny_invariant() {
    // The ApplicableConcern enum carries no authority-shaped payload.
    // Verify by serialising every variant and checking no "deny" /
    // "DENY" / "capability" / "authority" tokens appear.
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

    let out1 = applicable_concerns(&pi, &ui).expect("call 1");
    let out2 = applicable_concerns(&pi, &ui).expect("call 2");
    let out3 = applicable_concerns(&pi, &ui).expect("call 3");

    assert_eq!(out1, out2);
    assert_eq!(out2, out3);

    // FunctionalPure paradigm + full intent → all 10 applicable.
    // (Pre-A4-4aR this row would have triggered ParadigmIrrelevant for
    // Freshness. A4-4aR removes that heuristic exclusion.)
    let applicable: Vec<_> = out1.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(
        applicable.len(),
        10,
        "FunctionalPure paradigm + full intent → all 10 applicable"
    );
}

/// ─── 5. A4-4aR falsification: applicability ignores evidence ──────────

#[test]
fn integration_applicability_independent_of_external_evidence() {
    // The reducer takes only (pi, ui). Building two PI/UI pairs with the
    // SAME intent but DIFFERENT external-context (contract/decision
    // fixtures) is moot for applicability — the function does not see
    // them. This test calls the function with only (pi, ui) and verifies
    // the answer is intent-only.
    let pi = full_intent(ParadigmProfileRef::ObjectOriented);
    let ui = full_unit_intent("ac4::Ac4Adapter");

    // Two calls with the same inputs.
    let out1 = applicable_concerns(&pi, &ui).expect("a");
    let out2 = applicable_concerns(&pi, &ui).expect("b");
    assert_eq!(out1, out2);

    // All 10 applicable; none carry legacy string-grounding artifacts.
    let applicable: Vec<_> = out1.iter().filter(|(a, _)| a.is_applicable()).collect();
    assert_eq!(applicable.len(), 10);

    // Spot-check: no row carries ParadigmIrrelevant / NoGroundingDecision /
    // NoContractReference. The NotApplicable set must be empty.
    let na: Vec<_> = out1
        .iter()
        .filter(|(a, _)| a.is_not_applicable())
        .map(|(a, _)| a.concern())
        .collect();
    assert!(
        na.is_empty(),
        "expected no NotApplicable rows, got: {:?}",
        na
    );
}
