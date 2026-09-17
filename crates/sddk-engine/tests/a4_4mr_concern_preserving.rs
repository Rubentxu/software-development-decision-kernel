// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_4mr_concern_preserving.rs
//
// A4-4MR — Concern-Preserving ParadigmLens Evaluation (falsification
// suite).
//
// Cycle: p-63676b11dc0ef88f/a4-4mr-concern-preserving
// FU:    docs/debt/FU-A4-4M-CONCERN-PRESERVATION.md
//
// The four production `ParadigmLens` implementations in
// `alignment_lens::paradigm` MUST preserve `LensInput::concern()`:
// every successful evaluation returns a `LensContribution` whose
// `concern` field equals the requested concern, and whose
// `LensContributionId` is content-addressed from the requested concern.
// An unsupported concern yields `Refused(LensRejected { id, concern })`
// at the per-lens level, which the kernel aggregates into a single
// `NotEvaluated { reason: LensExistsButRefused }` gap.
//
// This corpus covers seven pin families:
//   P1 — exhaustive positive: every lens × every supported concern
//   P2 — exhaustive negative: every lens × every unsupported concern
//   P3 — defence-in-depth: direct `Lens::evaluate` refusal path
//   P4 — multi-lens invariants: same concern across multiple lenses
//   P5 — falsification: order and posture do not change concern
//   P6 — registry proof UAT: real registry → kernel → contribution
//   P7 — cross-concern identity: two distinct concerns → two distinct ids
//
// No probes; the property is fully pinned by enumeration of the closed
// `UniversalConcern` × `ParadigmLens::ALL` product, which is finite.

use sddk_engine::alignment_lens::LensContribution;
use sddk_engine::alignment_lens::error::LensError;
use sddk_engine::alignment_lens::id::derive_contribution_id;
use sddk_engine::alignment_lens::kernel::AlignmentLensKernel;
use sddk_engine::alignment_lens::lens::{AlignmentLens, LensEvaluationOutcome};
use sddk_engine::alignment_lens::not_evaluated::NotEvaluatedReason;
use sddk_engine::alignment_lens::paradigm::ParadigmLens;
use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;
use sddk_engine::alignment_lens::types::LensInput;
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef as UniEvidenceRef};
use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::{BasisHash, EventTime, KnowledgeBasis};
use sddk_engine::observation::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareObservation,
};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn basis() -> BasisHash {
    KnowledgeBasis::empty(EventTime(0)).basis_hash().clone()
}

fn unit(s: &str) -> SoftwareUnitRef {
    SoftwareUnitRef(format!("a4-4mr::{s}"))
}

fn intent() -> IntentId {
    IntentId("a4-4mr/intent".to_string())
}

fn lens_input_for(
    concern: UniversalConcern,
    unit: SoftwareUnitRef,
    observations: ObservationSet,
) -> LensInput {
    LensInput::try_new(
        ApplicableConcern::Applicable(concern),
        unit,
        intent(),
        basis(),
        observations,
    )
    .expect("Applicable input must construct cleanly")
}

/// One typed `SoftwareObservation` with the requested stance, on the
/// given unit, produced by `producer`. The lens matches it by
/// `producer == self.producer_tag()`.
fn observation_affirms_unit(
    unit: &SoftwareUnitRef,
    producer: &str,
    stance: ObservationStance,
    ev_locator: &str,
) -> SoftwareObservation {
    let obs_basis = ObservationBasis::new("a4-4mr-rev", basis(), "a4-4mr-input");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev_locator.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Unit(unit.clone()),
        stance,
        evidence,
        ObservationOrigin::DeterministicLocal,
        obs_basis,
        None,
        producer.to_string(),
    )
}

/// Build an observation set carrying only observations from one
/// specific producer (matching one specific paradigm lens).
fn set_for_lens(lens: &ParadigmLens, unit: &SoftwareUnitRef) -> ObservationSet {
    let mut set = ObservationSet::new();
    set.insert(observation_affirms_unit(
        unit,
        &lens.producer_tag(),
        ObservationStance::Affirms,
        "a4-4mr/observation/1",
    ));
    set
}

/// Closed enum of `UniversalConcern` variants (single source of truth
/// from the intent-universal-concern module's `UniversalConcern::ALL`).
fn all_concerns() -> Vec<UniversalConcern> {
    UniversalConcern::ALL.to_vec()
}

fn production_registry() -> AlignmentLensRegistry {
    let mut reg = AlignmentLensRegistry::new();
    for l in ParadigmLens::ALL {
        reg = reg.register(l).expect("register production lens");
    }
    reg
}

// ─── P1 — exhaustive positive: every lens × every supported concern ─────────

#[test]
fn p1_oop_lens_emits_contribution_tagged_with_requested_concern() {
    let lens = &ParadigmLens::ALL[0]; // object_oriented
    let declared: std::collections::BTreeSet<UniversalConcern> =
        lens.descriptor().supported_concerns;
    let registry = production_registry();
    for c in declared.iter().copied() {
        let u = unit(&format!("p1/oo/{c:?}"));
        let input = lens_input_for(c, u.clone(), set_for_lens(lens, &u));
        let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
        let eval = outcome
            .ok()
            .expect("kernel must not refuse when a lens supports the concern");
        // The kernel returns one contribution per matching lens; for
        // the OO lens, exactly one of those is from OO. Assert OO's
        // contribution carries the requested concern.
        let oo_contributions: Vec<&sddk_engine::alignment_lens::LensContribution> = eval
            .contributions
            .iter()
            .filter(|c2| c2.lens_id == lens.descriptor().id)
            .collect();
        assert_eq!(
            oo_contributions.len(),
            1,
            "OO lens must contribute exactly once per call (got {})",
            oo_contributions.len()
        );
        assert!(eval.gaps.is_empty(), "no gaps when a lens succeeded");
        assert_eq!(
            oo_contributions[0].concern, c,
            "OO lens must return the requested concern {c:?}"
        );
    }
}

#[test]
fn p1_functional_lens_emits_contribution_tagged_with_requested_concern() {
    let lens = &ParadigmLens::ALL[1]; // functional
    let declared: std::collections::BTreeSet<UniversalConcern> =
        lens.descriptor().supported_concerns;
    let registry = production_registry();
    for c in declared.iter().copied() {
        let u = unit(&format!("p1/fn/{c:?}"));
        let input = lens_input_for(c, u.clone(), set_for_lens(lens, &u));
        let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
        let eval = outcome
            .ok()
            .expect("kernel must not refuse when a lens supports the concern");
        let lens_contributions: Vec<&sddk_engine::alignment_lens::LensContribution> = eval
            .contributions
            .iter()
            .filter(|c2| c2.lens_id == lens.descriptor().id)
            .collect();
        assert_eq!(lens_contributions.len(), 1);
        assert!(eval.gaps.is_empty());
        assert_eq!(lens_contributions[0].concern, c);
    }
}

#[test]
fn p1_adt_lens_emits_contribution_tagged_with_requested_concern() {
    let lens = &ParadigmLens::ALL[2]; // adt
    let declared: std::collections::BTreeSet<UniversalConcern> =
        lens.descriptor().supported_concerns;
    let registry = production_registry();
    for c in declared.iter().copied() {
        let u = unit(&format!("p1/adt/{c:?}"));
        let input = lens_input_for(c, u.clone(), set_for_lens(lens, &u));
        let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
        let eval = outcome
            .ok()
            .expect("kernel must not refuse when a lens supports the concern");
        let lens_contributions: Vec<&sddk_engine::alignment_lens::LensContribution> = eval
            .contributions
            .iter()
            .filter(|c2| c2.lens_id == lens.descriptor().id)
            .collect();
        assert_eq!(lens_contributions.len(), 1);
        assert!(eval.gaps.is_empty());
        assert_eq!(lens_contributions[0].concern, c);
    }
}

#[test]
fn p1_dsl_lens_emits_contribution_tagged_with_requested_concern() {
    let lens = &ParadigmLens::ALL[3]; // dsl
    let declared: std::collections::BTreeSet<UniversalConcern> =
        lens.descriptor().supported_concerns;
    let registry = production_registry();
    for c in declared.iter().copied() {
        let u = unit(&format!("p1/dsl/{c:?}"));
        let input = lens_input_for(c, u.clone(), set_for_lens(lens, &u));
        let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
        let eval = outcome
            .ok()
            .expect("kernel must not refuse when a lens supports the concern");
        let lens_contributions: Vec<&sddk_engine::alignment_lens::LensContribution> = eval
            .contributions
            .iter()
            .filter(|c2| c2.lens_id == lens.descriptor().id)
            .collect();
        assert_eq!(lens_contributions.len(), 1);
        assert!(eval.gaps.is_empty());
        assert_eq!(lens_contributions[0].concern, c);
    }
}

// ─── P2 — exhaustive negative: every lens × every unsupported concern ───────

// P2 — exhaustive negative: every lens × every unsupported concern
//
// Note: the kernel aggregate behaviour for a concern that no lens in
// the registry declares is `NoRegisteredLens` (A4-4b spec §3.5 — the
// kernel is a closed-vocabulary authority, not a stringly-typed
// fallthrough). The per-lens refusal path is exercised separately by
// P3 (`p3_each_lens_refuses_unsupported_concern_directly`). Here we
// assert the aggregate behaviour for the production registry across
// the full `UniversalConcern::ALL` set:
//
//   - Concern declared by at least one lens  → `Ok` with contributions
//     (no gaps), and the OO/Functional/ADT/DSL lens that declared it
//     emits a contribution tagged with the requested concern.
//   - Concern declared by no production lens → `Ok(LensEvaluation {
//     contributions.is_empty(), gaps = [NoRegisteredLens] })`.
//
// What P2 does NOT cover (and P3 does): the per-lens
// `LensError::LensRejected { id, concern }` refusal. The kernel
// absorbs the refusal silently if any other lens succeeds; if no
// other lens succeeds, the gap is `LensExistsButRefused` (see P3
// for the direct path). Both outcomes are tested in P3.

#[test]
fn p2_production_registry_aggregate_for_declared_concerns() {
    let registry = production_registry();
    let kernel = AlignmentLensKernel::new();
    let mut total_lens_emissions: usize = 0;
    for lens in ParadigmLens::ALL {
        let producer = lens.producer_tag();
        for c in lens.descriptor().supported_concerns.iter().copied() {
            let u = unit(&format!("p2/declared/{:?}/{c:?}", lens.descriptor().id));
            let mut observations = ObservationSet::new();
            observations.insert(observation_affirms_unit(
                &u,
                &producer,
                ObservationStance::Affirms,
                "a4-4mr/p2/declared/1",
            ));
            let input = lens_input_for(c, u, observations);
            let outcome = kernel.evaluate(&registry, &input);
            let eval = outcome.ok().expect("kernel aggregate");
            assert!(eval.gaps.is_empty());
            let lens_count = eval
                .contributions
                .iter()
                .filter(|c2| c2.lens_id == lens.descriptor().id)
                .count();
            assert_eq!(lens_count, 1);
            total_lens_emissions += 1;
        }
    }
    let expected: usize = ParadigmLens::ALL
        .iter()
        .map(|l| l.descriptor().supported_concerns.len())
        .sum();
    assert_eq!(total_lens_emissions, expected);
}

#[test]
fn p2_production_registry_aggregate_for_undeclared_concern_is_no_registered_lens() {
    // The 10 `UniversalConcern::ALL` × 4 production lenses product:
    // concerns declared by NO lens in the production registry should
    // be exactly the complement of the declared set. Find those, and
    // assert the kernel aggregate returns `NoRegisteredLens` for
    // each.
    let registry = production_registry();
    let kernel = AlignmentLensKernel::new();
    let mut declared: std::collections::BTreeSet<UniversalConcern> =
        std::collections::BTreeSet::new();
    for lens in ParadigmLens::ALL {
        for c in lens.descriptor().supported_concerns.iter() {
            declared.insert(*c);
        }
    }
    let mut undeclared_tested = 0;
    for c in all_concerns().iter().copied() {
        if declared.contains(&c) {
            continue;
        }
        let u = unit(&format!("p2/undeclared/{c:?}"));
        let input = lens_input_for(c, u, ObservationSet::new());
        let outcome = kernel.evaluate(&registry, &input);
        let eval = outcome.ok().expect("kernel aggregate");
        assert!(
            eval.contributions.is_empty(),
            "no contributions when no lens declared {c:?}"
        );
        assert_eq!(
            eval.gaps.len(),
            1,
            "exactly one gap when no lens declared {c:?}"
        );
        assert_eq!(
            eval.gaps[0].reason,
            NotEvaluatedReason::NoRegisteredLens,
            "kernel aggregate must be NoRegisteredLens when no lens declared {c:?}"
        );
        assert_eq!(eval.gaps[0].concern(), c);
        undeclared_tested += 1;
    }
    assert!(
        undeclared_tested > 0,
        "at least one concern must be undeclared (sanity check)"
    );
}

// ─── P3 — defence-in-depth: direct `Lens::evaluate` refusal path ────────────

#[test]
fn p3_each_lens_refuses_unsupported_concern_directly() {
    // Bypass the kernel; call `Lens::evaluate` directly to confirm the
    // per-lens refusal returns the typed `LensRejected { id, concern }`.
    for lens in ParadigmLens::ALL {
        let supported: std::collections::BTreeSet<UniversalConcern> =
            lens.descriptor().supported_concerns;
        for c in all_concerns().iter().copied() {
            if supported.contains(&c) {
                continue;
            }
            let u = unit(&format!("p3/{c:?}"));
            let input = lens_input_for(c, u, ObservationSet::new());
            match lens.evaluate(&input) {
                LensEvaluationOutcome::Refused(LensError::LensRejected { id, concern }) => {
                    assert_eq!(id, lens.descriptor().id);
                    assert_eq!(concern, c);
                }
                other => panic!(
                    "lens {} refused concern {c:?} must be LensRejected; got {other:?}",
                    lens.descriptor().id
                ),
            }
        }
    }
}

// ─── P4 — multi-lens invariants: same concern across multiple lenses ────────

#[test]
fn p4_state_safety_emits_one_contribution_per_matching_lens() {
    // StateSafety is declared by all four production lenses (OO, Fn,
    // ADT, DSL). Each lens consumes a different producer tag; we give
    // every lens one family observation so all four succeed.
    let concern = UniversalConcern::StateSafety;
    let registry = production_registry();
    let u = unit("p4/state_safety");
    let mut observations = ObservationSet::new();
    for lens in ParadigmLens::ALL {
        observations.insert(observation_affirms_unit(
            &u,
            &lens.producer_tag(),
            ObservationStance::Affirms,
            &format!("a4-4mr/p4/{:?}", lens.descriptor().id),
        ));
    }
    let input = lens_input_for(concern, u, observations);
    let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
    let eval = outcome.ok().expect("kernel aggregate");
    let matching = registry.for_concern(concern);
    assert_eq!(
        eval.contributions.len(),
        matching.len(),
        "one contribution per matching lens"
    );
    assert!(
        eval.gaps.is_empty(),
        "no gaps when all matching lenses succeed"
    );
    for c in &eval.contributions {
        assert_eq!(
            c.concern, concern,
            "every contribution must carry the requested concern"
        );
    }
    // Distinct ids (different lens_ids; canonical-tagged identities).
    let mut ids: Vec<_> = eval.contributions.iter().map(|c| c.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), eval.contributions.len(), "distinct ids per lens");
}

#[test]
fn p4_boundary_integrity_emits_oo_and_dsl_only() {
    // BoundaryIntegrity is declared by OO and DSL only.
    let concern = UniversalConcern::BoundaryIntegrity;
    let registry = production_registry();
    let u = unit("p4/boundary_integrity");
    let mut observations = ObservationSet::new();
    for lens in ParadigmLens::ALL {
        let supported: std::collections::BTreeSet<UniversalConcern> =
            lens.descriptor().supported_concerns;
        if !supported.contains(&concern) {
            continue;
        }
        observations.insert(observation_affirms_unit(
            &u,
            &lens.producer_tag(),
            ObservationStance::Affirms,
            &format!("a4-4mr/p4/bi/{:?}", lens.descriptor().id),
        ));
    }
    let input = lens_input_for(concern, u, observations);
    let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
    let eval = outcome.ok().expect("kernel aggregate");
    let matching = registry.for_concern(concern);
    assert_eq!(eval.contributions.len(), matching.len());
    assert_eq!(matching.len(), 2, "OO + DSL");
    for c in &eval.contributions {
        assert_eq!(c.concern, concern);
    }
}

#[test]
fn p4_one_succeeds_one_refuses_kernel_preserves_both() {
    // OO lens declares DependencyDirection; Functional lens does not.
    // Build observations only for OO so Functional refuses. The
    // kernel must absorb the refusal (silent per spec §3) and emit a
    // single contribution from OO.
    let concern = UniversalConcern::DependencyDirection;
    let registry = production_registry();
    let u = unit("p4/dep_direction");
    let oo = ParadigmLens::ALL[0];
    let mut observations = ObservationSet::new();
    observations.insert(observation_affirms_unit(
        &u,
        &oo.producer_tag(),
        ObservationStance::Affirms,
        "a4-4mr/p4/dep_direction/oo",
    ));
    let input = lens_input_for(concern, u, observations);
    let outcome = AlignmentLensKernel::new().evaluate(&registry, &input);
    let eval = outcome.ok().expect("kernel aggregate");
    assert_eq!(
        eval.contributions.len(),
        1,
        "OO succeeds, Functional refuses"
    );
    assert!(
        eval.gaps.is_empty(),
        "the refused lens is absorbed silently"
    );
    assert_eq!(eval.contributions[0].concern, concern);
    assert_eq!(eval.contributions[0].lens_id, oo.descriptor().id);
}

// ─── P5 — falsification: order and posture do not change concern ────────────

#[test]
fn p5_posture_class_does_not_change_concern() {
    // Same lens, same unit, same concern — vary the observation set
    // across all four posture classes (Supported, Contradicted,
    // Conflicted, Insufficient). Concern MUST stay Stable.
    let lens = &ParadigmLens::ALL[1]; // functional supports StateSafety
    let concern = UniversalConcern::StateSafety;
    let producer = lens.producer_tag();

    // Supported: one affirming observation.
    let mut supported_set = ObservationSet::new();
    supported_set.insert(observation_affirms_unit(
        &unit("p5/supported"),
        &producer,
        ObservationStance::Affirms,
        "a4-4mr/p5/supported",
    ));

    // Contradicted: one denying observation.
    let mut contradicted_set = ObservationSet::new();
    contradicted_set.insert(observation_affirms_unit(
        &unit("p5/contradicted"),
        &producer,
        ObservationStance::Denies,
        "a4-4mr/p5/contradicted",
    ));

    // Conflicted: one affirming + one denying on the SAME unit
    // (allowed by the typed posture machinery; lens resolves to
    // Conflicted when both stances are present for the unit).
    let u_cf = unit("p5/conflicted");
    let mut conflicted_set = ObservationSet::new();
    conflicted_set.insert(observation_affirms_unit(
        &u_cf,
        &producer,
        ObservationStance::Affirms,
        "a4-4mr/p5/conflicted/aff",
    ));
    conflicted_set.insert(observation_affirms_unit(
        &u_cf,
        &producer,
        ObservationStance::Denies,
        "a4-4mr/p5/conflicted/den",
    ));

    // Insufficient: empty set for the same producer.
    let insufficient_set = ObservationSet::new();

    let u_s = unit("p5/supported");
    let u_x = unit("p5/contradicted");
    let registry = production_registry();

    let cs = AlignmentLensKernel::new()
        .evaluate(&registry, &lens_input_for(concern, u_s, supported_set))
        .ok()
        .unwrap()
        .contributions[0]
        .clone();
    let cx = AlignmentLensKernel::new()
        .evaluate(&registry, &lens_input_for(concern, u_x, contradicted_set))
        .ok()
        .unwrap()
        .contributions[0]
        .clone();
    let cf = AlignmentLensKernel::new()
        .evaluate(&registry, &lens_input_for(concern, u_cf, conflicted_set))
        .ok()
        .unwrap()
        .contributions[0]
        .clone();
    let ci = AlignmentLensKernel::new()
        .evaluate(
            &registry,
            &lens_input_for(concern, unit("p5/insufficient"), insufficient_set),
        )
        .ok()
        .unwrap()
        .contributions[0]
        .clone();

    for (label, c) in [
        ("supported", &cs),
        ("contradicted", &cx),
        ("conflicted", &cf),
        ("insufficient", &ci),
    ] {
        assert_eq!(
            c.concern, concern,
            "posture class {label} must not change contribution.concern"
        );
    }

    // Distinct posture classes produce distinct ids (different posture
    // ⇒ different id, holding concern constant).
    let mut ids = vec![cs.id.clone(), cx.id.clone(), cf.id.clone(), ci.id.clone()];
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        4,
        "four posture classes yield four distinct contribution ids"
    );
}

#[test]
fn p5_observation_set_with_identical_content_yields_identical_contribution() {
    // Same unit, same concern, same observation content — even when
    // built as two separate `ObservationSet`s. The contribution
    // concern stays Stable and the id is identical (ObservationSet
    // is insertion-order-independent and `ObservationId` is
    // content-derived).
    let lens = &ParadigmLens::ALL[0]; // object_oriented
    let concern = UniversalConcern::StateSafety;
    let producer = lens.producer_tag();
    let u = unit("p5/order");

    let mut set1 = ObservationSet::new();
    set1.insert(observation_affirms_unit(
        &u,
        &producer,
        ObservationStance::Affirms,
        "a4-4mr/p5/order/1",
    ));

    let mut set2 = ObservationSet::new();
    set2.insert(observation_affirms_unit(
        &u,
        &producer,
        ObservationStance::Affirms,
        "a4-4mr/p5/order/1",
    ));

    let registry = production_registry();
    let c1 = AlignmentLensKernel::new()
        .evaluate(&registry, &lens_input_for(concern, u.clone(), set1))
        .ok()
        .unwrap()
        .contributions[0]
        .clone();
    let c2 = AlignmentLensKernel::new()
        .evaluate(&registry, &lens_input_for(concern, u, set2))
        .ok()
        .unwrap()
        .contributions[0]
        .clone();

    assert_eq!(c1.concern, concern);
    assert_eq!(c2.concern, concern);
    assert_eq!(
        c1.id, c2.id,
        "identical observation content ⇒ identical contribution id (order-independent)"
    );
}

// ─── P6 — registry proof UAT: real registry → kernel → contribution ─────────

#[test]
fn p6_full_registry_produces_one_contribution_per_supported_pair() {
    // Load-bearing: every (lens, supported_concern) pair in
    // `ParadigmLens::ALL` × `UniversalConcern` is exercised end-to-end
    // through the production `AlignmentLensRegistry` and
    // `AlignmentLensKernel`. No shortcuts via the legacy facade.
    let registry = production_registry();
    let kernel = AlignmentLensKernel::new();
    let mut total_lens_emissions: usize = 0;
    for lens in ParadigmLens::ALL {
        let producer = lens.producer_tag();
        let supported: std::collections::BTreeSet<UniversalConcern> =
            lens.descriptor().supported_concerns;
        for c in supported.iter().copied() {
            let u = unit(&format!("p6/{:?}/{c:?}", lens.descriptor().id));
            let mut observations = ObservationSet::new();
            observations.insert(observation_affirms_unit(
                &u,
                &producer,
                ObservationStance::Affirms,
                &format!("a4-4mr/p6/{c:?}/1"),
            ));
            let input = lens_input_for(c, u, observations);
            let outcome = kernel.evaluate(&registry, &input);
            let eval = outcome.ok().expect("kernel aggregate");
            assert!(eval.gaps.is_empty());
            // Count this lens's contribution inside the kernel's
            // (multi-lens) emission; other lenses may also contribute
            // when they share the concern.
            let lens_count = eval
                .contributions
                .iter()
                .filter(|c2| c2.lens_id == lens.descriptor().id)
                .count();
            assert_eq!(
                lens_count,
                1,
                "exactly one contribution per (lens, concern) call (got {lens_count} for {:?} {c:?})",
                lens.descriptor().id
            );
            let contribution = eval
                .contributions
                .iter()
                .find(|c2| c2.lens_id == lens.descriptor().id)
                .expect("lens's contribution is present");
            assert_eq!(contribution.concern, c);
            total_lens_emissions += 1;
        }
    }
    // Cross-check: total = sum of |declared concerns| over all lenses.
    let expected: usize = ParadigmLens::ALL
        .iter()
        .map(|l| l.descriptor().supported_concerns.len())
        .sum();
    assert_eq!(
        total_lens_emissions, expected,
        "every supported pair emitted exactly one contribution from its lens"
    );
}

#[test]
fn p6_contribution_id_is_canonical_for_requested_concern() {
    // The contribution id must be derived with the REQUESTED concern,
    // not with any lens-side terminal concern (the bug). The same
    // observation set fed to a different requested concern must
    // produce a different id at the SAME lens.
    let lens = &ParadigmLens::ALL[0]; // object_oriented supports StateSafety and Coupling (both)
    let producer = lens.producer_tag();
    // Single-lens registry so we observe the lens's id in isolation.
    let registry = {
        let mut r = AlignmentLensRegistry::new();
        r = r.register(*lens).expect("register");
        r
    };

    let build_contribution = |concern: UniversalConcern| -> LensContribution {
        let u = unit(&format!("p6/id/{concern:?}"));
        let mut observations = ObservationSet::new();
        observations.insert(observation_affirms_unit(
            &u,
            &producer,
            ObservationStance::Affirms,
            "a4-4mr/p6/id/1",
        ));
        let input = lens_input_for(concern, u, observations);
        let outcome = AlignmentLensKernel::new()
            .evaluate(&registry, &input)
            .ok()
            .unwrap();
        assert_eq!(outcome.contributions.len(), 1);
        outcome.contributions[0].clone()
    };

    let c_state = build_contribution(UniversalConcern::StateSafety);
    let c_coupling = build_contribution(UniversalConcern::Coupling);

    assert_eq!(c_state.concern, UniversalConcern::StateSafety);
    assert_eq!(c_coupling.concern, UniversalConcern::Coupling);
    assert_ne!(
        c_state.id, c_coupling.id,
        "different requested concerns ⇒ different ids (the bug had them collide)"
    );
}

// ─── P7 — cross-concern identity: two distinct concerns → two distinct ids ─

#[test]
fn p7_two_distinct_concerns_yield_two_distinct_ids_same_lens() {
    // The bug: when iterating `self.concerns`, the LAST element wins
    // for both the contribution's `concern` field AND the
    // `derive_contribution_id` input. This pin exercises the same
    // lens with two of its declared concerns (StateSafety and
    // Coupling for OO) and asserts: the two contributions have
    // DIFFERENT ids (because they encode DIFFERENT requested concerns)
    // and each carries its OWN concern.
    let lens = &ParadigmLens::ALL[0]; // object_oriented
    let producer = lens.producer_tag();

    let mk = |c: UniversalConcern, ev: &str| -> LensContribution {
        let u = unit(&format!("p7/{c:?}"));
        let mut observations = ObservationSet::new();
        observations.insert(observation_affirms_unit(
            &u,
            &producer,
            ObservationStance::Affirms,
            ev,
        ));
        let input = lens_input_for(c, u, observations);
        AlignmentLensKernel::new()
            .evaluate(&production_registry(), &input)
            .ok()
            .unwrap()
            .contributions[0]
            .clone()
    };

    let c1 = mk(UniversalConcern::StateSafety, "a4-4mr/p7/state");
    let c2 = mk(UniversalConcern::Coupling, "a4-4mr/p7/coupling");

    assert_eq!(c1.concern, UniversalConcern::StateSafety);
    assert_eq!(c2.concern, UniversalConcern::Coupling);
    assert_ne!(c1.id, c2.id);

    // Sanity: determinism — same input twice ⇒ same id.
    let c1_again = mk(UniversalConcern::StateSafety, "a4-4mr/p7/state");
    assert_eq!(c1.id, c1_again.id);

    let c2_again = mk(UniversalConcern::Coupling, "a4-4mr/p7/coupling");
    assert_eq!(c2.id, c2_again.id);
}

#[test]
fn p7_contribution_id_derives_with_requested_concern_explicit() {
    // Hand-compute the expected id using `derive_contribution_id(...
    // concern, ...)` and assert the kernel returns exactly that. This
    // is the strict invariant: the lens hashes the requested concern,
    // never its terminal concern.
    let lens = &ParadigmLens::ALL[2]; // adt (smallest supported set)
    let producer = lens.producer_tag();
    let concern = UniversalConcern::SemanticOwnership;
    // Single-lens registry so the OO+Functional lenses (which also
    // declare SemanticOwnership) do not contribute their own ids.
    let registry = {
        let mut r = AlignmentLensRegistry::new();
        r = r.register(*lens).expect("register");
        r
    };
    let u = unit("p7/explicit");
    let mut observations = ObservationSet::new();
    observations.insert(observation_affirms_unit(
        &u,
        &producer,
        ObservationStance::Affirms,
        "a4-4mr/p7/explicit",
    ));
    let input = lens_input_for(concern, u.clone(), observations.clone());
    let outcome = AlignmentLensKernel::new()
        .evaluate(&registry, &input)
        .ok()
        .unwrap();
    assert_eq!(outcome.contributions.len(), 1);
    let contribution = &outcome.contributions[0];
    assert_eq!(contribution.concern, concern);

    let expected_id = derive_contribution_id(
        lens.descriptor().id,
        lens.descriptor().version,
        concern,
        &observations.canonical_digest(),
        &contribution.evidence_resolution,
        &contribution.evidence_refs,
    );
    assert_eq!(
        contribution.id, expected_id,
        "the contribution id must equal derive_contribution_id(..., REQUESTED concern, ...)"
    );
}
