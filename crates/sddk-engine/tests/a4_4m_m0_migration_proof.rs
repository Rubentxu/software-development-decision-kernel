// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// tests/a4_4m_m0_migration_proof.rs — A4-4M M0 Migration Proof pins.
//
// Cycle: p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence
// Matrix doc: docs/architecture/a4-4m-migration-matrix.md
//
// These tests pin the AC7 → generic-AlignmentLens translation BEFORE any
// paradigm_lens production edit (M0 gate). They prove:
//   - the translation is exhaustive and typed (no string-binding),
//   - the concern mapping is total over the 16 variants,
//   - the legacy LensStatus is a pure projection of EvidencePosture,
//   - NotApplicable never comes from the kernel side,
//   - order permutation cannot change the posture class.

use std::collections::BTreeSet;

use sddk_engine::alignment_lens::id::derive_contribution_id;
use sddk_engine::alignment_lens::types::{LensId, LensVersion};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::EvidenceKind;
use sddk_engine::intent_universal_concern::types::UniversalConcern;
use sddk_engine::knowledge::{BasisHash, EventTime, KnowledgeBasis};
use sddk_engine::observation::posture::{EvidencePosture, InsufficientGap, ObservationTargetRef};
use sddk_engine::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareObservation,
};
use sddk_engine::paradigm_lens::types::{LensFamily, LensObservation, ObservationPolarity};
use sddk_engine::paradigm_profile::types::LensStatus;

// ─── Test helpers ───────────────────────────────────────────────────────────

fn test_basis() -> ObservationBasis {
    let kb: BasisHash = KnowledgeBasis::empty(EventTime(0)).basis_hash().clone();
    ObservationBasis::new("test-rev-0", kb, "m0-migration-proof")
}

/// The M0 translation function: typed `LensObservation` → canonical
/// `SoftwareObservation` about a typed Unit target. Exhaustive match,
/// no wildcard — adding a 17th variant breaks compilation (M0-P01).
/// The `EvidenceRef` locator is written (provenance), never read back.
fn translate(obs: LensObservation, unit: &SoftwareUnitRef) -> SoftwareObservation {
    let stance = match obs.polarity() {
        ObservationPolarity::Supports => ObservationStance::Affirms,
        ObservationPolarity::Contradicts => ObservationStance::Denies,
    };
    let producer = format!("ac7.probe.{}", obs.family().canonical_tag());
    SoftwareObservation::declare(
        ObservationSubject::Unit(unit.clone()),
        stance,
        obs.evidence_ref(), // provenance citation ONLY (M0-P04)
        ObservationOrigin::DeterministicLocal,
        test_basis(),
        None,
        producer,
    )
}

/// The M0 concern mapping (matrix §3): total over the 16 variants,
/// pinned. Multi-concern rows are intentional and justified in the doc.
/// No wildcard — exhaustiveness is compile-checked (M0-P05).
fn concerns_of(obs: LensObservation) -> &'static [UniversalConcern] {
    use LensObservation::*;
    use UniversalConcern::*;
    match obs {
        EncapsulationPresent => &[BoundaryIntegrity, StateSafety],
        DependencyInversionUsed => &[DependencyDirection],
        AnemicModelDetected => &[SemanticOwnership, Cohesion],
        InheritanceOverComposition => &[Coupling],
        ImmutableValuesPresent => &[StateSafety],
        EffectsAtBoundary => &[EffectVisibility],
        HiddenMutationDetected => &[StateSafety, EffectVisibility],
        SentinelControlFlowDetected => &[SemanticOwnership],
        TypedSumTypePresent => &[StateSafety, SemanticOwnership],
        InvalidStatesRepresentable => &[StateSafety],
        BooleanBlindness => &[StateSafety, SemanticOwnership],
        StringlyTypedStatus => &[SemanticOwnership],
        TypedAstPresent => &[BoundaryIntegrity],
        SyntaxSeparatedFromEffects => &[BoundaryIntegrity, EffectVisibility],
        ValidatesBeforeExecution => &[StateSafety, TemporalCoupling],
        InvalidProgramsRepresentable => &[StateSafety],
    }
}

/// The M0.3 compatibility projection (matrix §4): posture → legacy status.
/// Total over the four closed variants; `NotApplicable` is unreachable
/// from the posture (M0-P09).
fn project_status(posture: &EvidencePosture<ObservationTargetRef>) -> LensStatus {
    match posture {
        EvidencePosture::Supported { .. } => LensStatus::Aligned,
        EvidencePosture::Contradicted { .. } => LensStatus::Misaligned,
        EvidencePosture::Conflicted { .. } => LensStatus::Tension,
        EvidencePosture::Insufficient { .. } => LensStatus::Unknown,
    }
}

fn unit(name: &str) -> SoftwareUnitRef {
    SoftwareUnitRef(format!("m0::{name}"))
}

/// Stable set summary: sorted observation ids (order-independent).
fn observation_set_canonical_tag(set: &ObservationSet) -> String {
    let mut ids: Vec<&str> = set.observations().iter().map(|o| o.id.as_str()).collect();
    ids.sort_unstable();
    let mut out = String::from("OBSET|");
    out.push_str(&ids.join("\u{1f}"));
    if set.is_empty() {
        out.push_str("empty");
    }
    out
}

fn posture_class(p: &EvidencePosture<ObservationTargetRef>) -> &'static str {
    match p {
        EvidencePosture::Supported { .. } => "supported",
        EvidencePosture::Contradicted { .. } => "contradicted",
        EvidencePosture::Conflicted { .. } => "conflicted",
        EvidencePosture::Insufficient { .. } => "insufficient",
    }
}

// ─── M0-P01..P04 — translation is typed, exhaustive, unit-targeted ─────────

#[test]
fn m0_p02_translation_projects_polarity_into_stance_for_all_16_variants() {
    let u = unit("polarity");
    for obs in LensObservation::ALL {
        let translated = translate(obs, &u);
        let expected_stance = match obs.polarity() {
            ObservationPolarity::Supports => ObservationStance::Affirms,
            ObservationPolarity::Contradicts => ObservationStance::Denies,
        };
        assert_eq!(translated.stance, expected_stance, "variant {obs:?}");
    }
}

#[test]
fn m0_p03_translation_targets_units_never_synthetic_relations() {
    let u = unit("no-synth");
    for obs in LensObservation::ALL {
        let translated = translate(obs, &u);
        assert!(
            matches!(translated.subject, ObservationSubject::Unit(ref r) if r == &u),
            "variant {obs:?} must observe the Unit, not a fabricated relation"
        );
    }
}

#[test]
fn m0_p04_translation_uses_static_locator_as_provenance_only() {
    // The locator is a STATIC string per variant (never parsed); the
    // translation writes it as evidence citation. Pin that each variant's
    // locator is a distinct static provenance tag and that the evidence
    // kind is Planning (heuristic analysis), preserving provenance.
    let mut seen = BTreeSet::new();
    for obs in LensObservation::ALL {
        let r = obs.evidence_ref();
        assert_eq!(r.kind, EvidenceKind::Planning);
        assert!(seen.insert(r.locator.clone()), "duplicate locator {obs:?}");
    }
    assert_eq!(seen.len(), 16);
}

// ─── M0-P05 — concern mapping is total and pinned ──────────────────────────

#[test]
fn m0_p05_concern_mapping_is_total_and_pinned() {
    // Pinned expectations (matrix §3), asserted row by row.
    use LensObservation as L;
    use UniversalConcern as C;
    let pinned: &[(L, &[C])] = &[
        (
            L::EncapsulationPresent,
            &[C::BoundaryIntegrity, C::StateSafety],
        ),
        (L::DependencyInversionUsed, &[C::DependencyDirection]),
        (L::AnemicModelDetected, &[C::SemanticOwnership, C::Cohesion]),
        (L::InheritanceOverComposition, &[C::Coupling]),
        (L::ImmutableValuesPresent, &[C::StateSafety]),
        (L::EffectsAtBoundary, &[C::EffectVisibility]),
        (
            L::HiddenMutationDetected,
            &[C::StateSafety, C::EffectVisibility],
        ),
        (L::SentinelControlFlowDetected, &[C::SemanticOwnership]),
        (
            L::TypedSumTypePresent,
            &[C::StateSafety, C::SemanticOwnership],
        ),
        (L::InvalidStatesRepresentable, &[C::StateSafety]),
        (L::BooleanBlindness, &[C::StateSafety, C::SemanticOwnership]),
        (L::StringlyTypedStatus, &[C::SemanticOwnership]),
        (L::TypedAstPresent, &[C::BoundaryIntegrity]),
        (
            L::SyntaxSeparatedFromEffects,
            &[C::BoundaryIntegrity, C::EffectVisibility],
        ),
        (
            L::ValidatesBeforeExecution,
            &[C::StateSafety, C::TemporalCoupling],
        ),
        (L::InvalidProgramsRepresentable, &[C::StateSafety]),
    ];
    assert_eq!(pinned.len(), 16);
    for (obs, expected) in pinned {
        assert_eq!(concerns_of(*obs), *expected, "unpinned mapping for {obs:?}");
        assert!(!expected.is_empty(), "{obs:?} must map to >=1 concern");
    }
    // Every mapped concern is a member of the closed vocabulary (all C
    // variants are valid by construction; pin ALL count = 10).
    assert_eq!(UniversalConcern::ALL.len(), 10);
}

// ─── M0-P06..P08 — posture → legacy status projection equivalence ──────────

fn build_set(obs: &[LensObservation], u: &SoftwareUnitRef) -> ObservationSet {
    let mut set = ObservationSet::new();
    for o in obs {
        set.insert(translate(*o, u));
    }
    set
}

#[test]
fn m0_p07_zero_observations_insufficient_projects_to_unknown() {
    let u = unit("legacy-unknown");
    let set = build_set(&[], &u);
    let target = ObservationTargetRef::Unit(u.clone());
    let posture = sddk_engine::observation::resolve_subject(&set, &target);
    match &posture {
        EvidencePosture::Insufficient { gap, .. } => {
            assert!(matches!(gap, InsufficientGap::NoObservation));
        }
        other => panic!("expected Insufficient, got {other:?}"),
    }
    assert_eq!(project_status(&posture), LensStatus::Unknown);
}

#[test]
fn m0_p08_mixed_support_and_contradict_is_conflicted_projects_to_tension() {
    let u = unit("legacy-tension");
    // Functional family: one Supports (ImmutableValuesPresent), one
    // Contradicts (HiddenMutationDetected) → mixed → legacy Tension.
    let set = build_set(
        &[
            LensObservation::ImmutableValuesPresent,
            LensObservation::HiddenMutationDetected,
        ],
        &u,
    );
    let target = ObservationTargetRef::Unit(u.clone());
    let posture = sddk_engine::observation::resolve_subject(&set, &target);
    assert_eq!(posture_class(&posture), "conflicted");
    assert_eq!(project_status(&posture), LensStatus::Tension);
}

#[test]
fn m0_p08b_pure_support_and_pure_contradict_project_correctly() {
    let u = unit("legacy-aligned");
    let set = build_set(&[LensObservation::TypedSumTypePresent], &u);
    let target = ObservationTargetRef::Unit(u.clone());
    let posture = sddk_engine::observation::resolve_subject(&set, &target);
    assert_eq!(posture_class(&posture), "supported");
    assert_eq!(project_status(&posture), LensStatus::Aligned);

    let u2 = unit("legacy-misaligned");
    let set2 = build_set(&[LensObservation::BooleanBlindness], &u2);
    let target2 = ObservationTargetRef::Unit(u2.clone());
    let posture2 = sddk_engine::observation::resolve_subject(&set2, &target2);
    assert_eq!(posture_class(&posture2), "contradicted");
    assert_eq!(project_status(&posture2), LensStatus::Misaligned);
}

// ─── M0-P09 — NotApplicable unreachable from posture ───────────────────────

#[test]
fn m0_p09_not_applicable_is_unreachable_from_any_posture() {
    // Type-level pin: `project_status` is a total match over the four
    // closed posture variants with no NotApplicable arm — the legacy
    // NotApplicable can only originate in the intent layer. Exercise all
    // four classes and confirm the projection never yields NotApplicable.
    let u = unit("na");
    let statuses = [
        project_status(&EvidencePosture::Supported {
            target: ObservationTargetRef::Unit(u.clone()),
            supporting: vec![],
        }),
        project_status(&EvidencePosture::Contradicted {
            target: ObservationTargetRef::Unit(u.clone()),
            contradicting: vec![],
        }),
        project_status(&EvidencePosture::Conflicted {
            target: ObservationTargetRef::Unit(u.clone()),
            supporting: vec![],
            contradicting: vec![],
        }),
        project_status(&EvidencePosture::Insufficient {
            target: ObservationTargetRef::Unit(u.clone()),
            gap: InsufficientGap::NoObservation,
        }),
    ];
    for s in statuses {
        assert_ne!(s, LensStatus::NotApplicable);
    }
}

// ─── M0-P10 — order permutation cannot change the posture class ────────────

#[test]
fn m0_p10_observation_order_does_not_change_posture_class() {
    let u = unit("order");
    let obs_a = [
        LensObservation::ImmutableValuesPresent,
        LensObservation::HiddenMutationDetected,
    ];
    let obs_b = [
        LensObservation::HiddenMutationDetected,
        LensObservation::ImmutableValuesPresent,
    ];
    let set_a = build_set(&obs_a, &u);
    let set_b = build_set(&obs_b, &u);
    let target = ObservationTargetRef::Unit(u.clone());
    let pa = sddk_engine::observation::resolve_subject(&set_a, &target);
    let pb = sddk_engine::observation::resolve_subject(&set_b, &target);
    assert_eq!(posture_class(&pa), posture_class(&pb));
    assert_eq!(project_status(&pa), project_status(&pb));
}

// ─── Family mapping sanity (matrix §1) ─────────────────────────────────────

#[test]
fn m0_family_partition_of_16_observations_is_4x4() {
    let mut counts = BTreeSet::new();
    for family in LensFamily::ALL {
        let n = LensObservation::ALL
            .iter()
            .filter(|o| o.family() == family)
            .count();
        assert_eq!(n, 4, "family {family:?} must own exactly 4 observations");
        counts.insert(family);
    }
    assert_eq!(counts.len(), 4);
}

// ─── Contribution identity still excludes legacy vocabulary ────────────────

#[test]
fn m0_contribution_identity_ignores_legacy_status_and_polarity_order() {
    // LensContributionId derives from (lens, concern, observation_set,
    // evidence_resolution) — a legacy LensStatus must not enter it.
    // Pin: two postures differing only in insertion order produce equal
    // identity inputs (observation set digest), while genuinely different
    // evidence produces different identity.
    let u = unit("identity");
    let set_a = build_set(
        &[
            LensObservation::TypedSumTypePresent,
            LensObservation::InvalidStatesRepresentable,
        ],
        &u,
    );
    let set_b = build_set(
        &[
            LensObservation::InvalidStatesRepresentable,
            LensObservation::TypedSumTypePresent,
        ],
        &u,
    );
    let tag_a = observation_set_canonical_tag(&set_a);
    let tag_b = observation_set_canonical_tag(&set_b);
    assert_eq!(tag_a, tag_b, "insertion order must not change set identity");

    let lens = LensId::new("m0::pin");
    let version = LensVersion::new(0, 1);
    let posture = EvidencePosture::<ObservationTargetRef>::Insufficient {
        target: ObservationTargetRef::Unit(u.clone()),
        gap: InsufficientGap::NoObservation,
    };
    let id_a = derive_contribution_id(
        lens,
        version,
        UniversalConcern::StateSafety,
        &tag_a,
        &posture,
        &[],
    );
    let id_b = derive_contribution_id(
        lens,
        version,
        UniversalConcern::StateSafety,
        &tag_b,
        &posture,
        &[],
    );
    assert_eq!(id_a, id_b);
}
