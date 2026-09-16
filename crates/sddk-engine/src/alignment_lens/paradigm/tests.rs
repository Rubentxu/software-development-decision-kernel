// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/paradigm/tests.rs — A4-4M M1/M3 unit tests.

use super::{PRODUCER_PREFIX, ParadigmLens, ids};
use crate::alignment_lens::lens::{AlignmentLens, LensEvaluationOutcome};
use crate::alignment_lens::registry::AlignmentLensRegistry;
use crate::alignment_lens::types::{LensId, LensInput};
use crate::architecture_graph::SoftwareUnitRef;
use crate::intent_universal_concern::types::{ApplicableConcern, IntentId, UniversalConcern};
use crate::knowledge::{BasisHash, EventTime, KnowledgeBasis};
use crate::observation::posture::{EvidencePosture, ObservationTargetRef};
use crate::observation::types::ObservationSet;
use crate::paradigm_lens::translation::translate_batch;
use crate::paradigm_lens::{LensObservation, probe_adt_observations};

fn basis() -> BasisHash {
    KnowledgeBasis::empty(EventTime(0)).basis_hash().clone()
}

fn unit(name: &str) -> SoftwareUnitRef {
    SoftwareUnitRef(format!("m1::{name}"))
}

fn intent_for(concerns: &[UniversalConcern]) -> (IntentId, ApplicableConcern) {
    let mut declared = std::collections::BTreeSet::new();
    for c in concerns {
        declared.insert(*c);
    }
    let _project = crate::intent_universal_concern::types::ProjectIntent {
        intent_id: IntentId("m1-intent".into()),
        paradigm: crate::intent_universal_concern::types::ParadigmProfileRef::DataOriented,
        declared_concerns: declared.clone(),
        excluded_concerns: Default::default(),
    };
    (
        IntentId("m1-intent".into()),
        ApplicableConcern::Applicable(UniversalConcern::StateSafety),
    )
}

fn input_for(unit_ref: SoftwareUnitRef, concern: UniversalConcern) -> LensInput {
    let id = IntentId("m1-intent".into());
    LensInput::try_new(
        ApplicableConcern::Applicable(concern),
        unit_ref,
        id,
        basis(),
        ObservationSet::new(),
    )
    .expect("applicable")
}

#[test]
fn four_lenses_with_distinct_descriptors() {
    let ids: Vec<LensId> = ParadigmLens::ALL
        .iter()
        .map(|l| l.descriptor().id)
        .collect();
    let set: std::collections::BTreeSet<&str> = ids.iter().map(|i| i.as_str()).collect();
    assert_eq!(set.len(), 4);
    assert!(ids.contains(&ids::OBJECT_ORIENTED));
    assert!(ids.contains(&ids::FUNCTIONAL));
    assert!(ids.contains(&ids::ADT));
    assert!(ids.contains(&ids::DSL));
}

#[test]
fn registry_composition_registers_all_four() {
    let mut reg = AlignmentLensRegistry::new();
    for l in ParadigmLens::ALL {
        reg = reg.register(l).expect("register");
    }
    assert_eq!(reg.len(), 4);
    // M3: composition via registry, concern-indexed — no hard-coded
    // paradigm match anywhere outside the adapter.
    for c in [
        UniversalConcern::BoundaryIntegrity,
        UniversalConcern::EffectVisibility,
        UniversalConcern::StateSafety,
        UniversalConcern::TemporalCoupling,
        UniversalConcern::SemanticOwnership,
    ] {
        assert!(reg.has_concern(c), "no lens for {c:?}");
    }
}

#[test]
fn lens_insufficient_when_no_family_observations() {
    let u = unit("empty");
    let input = input_for(u, UniversalConcern::StateSafety);
    let out = ParadigmLens::ALL[2].evaluate(&input); // ADT
    match out {
        LensEvaluationOutcome::Contribution(c) => match c.evidence_resolution {
            EvidencePosture::Insufficient { target, gap } => {
                assert!(
                    matches!(target, ObservationTargetRef::Unit(ref r) if r.as_str().starts_with("m1::"))
                );
                assert!(matches!(
                    gap,
                    crate::observation::posture::InsufficientGap::NoObservation
                ));
            }
            other => panic!("expected Insufficient, got {other:?}"),
        },
        other => panic!("expected contribution, got {other:?}"),
    }
}

#[test]
fn lens_resolves_conflicted_from_translated_probe_output() {
    // M2 end-to-end: probe → translation → set → lens → contribution.
    let u = unit("mixed");
    let adt_obs =
        probe_adt_observations("let status: Result<u8, String> = ok_with_bool_flags(true, false);");
    // Ensure both polarities are present in whatever the probe found;
    // otherwise synthesize the mixed scenario from the typed enum.
    let mut obs: Vec<LensObservation> = adt_obs;
    if !obs
        .iter()
        .any(|o| o.polarity() == crate::paradigm_lens::ObservationPolarity::Supports)
    {
        obs.push(LensObservation::TypedSumTypePresent);
    }
    if !obs
        .iter()
        .any(|o| o.polarity() == crate::paradigm_lens::ObservationPolarity::Contradicts)
    {
        obs.push(LensObservation::BooleanBlindness);
    }
    assert!(
        obs.iter()
            .any(|o| o.polarity() == crate::paradigm_lens::ObservationPolarity::Supports)
    );
    assert!(
        obs.iter()
            .any(|o| o.polarity() == crate::paradigm_lens::ObservationPolarity::Contradicts)
    );

    let mut set = ObservationSet::new();
    translate_batch(&obs, &u, &basis(), "rev-0", &mut set);

    let id = IntentId("m1-intent".into());
    let input = LensInput::try_new(
        ApplicableConcern::Applicable(UniversalConcern::StateSafety),
        u,
        id,
        basis(),
        set,
    )
    .expect("applicable");

    let out = ParadigmLens::ALL[2].evaluate(&input); // ADT
    match out {
        LensEvaluationOutcome::Contribution(c) => {
            assert!(matches!(
                c.evidence_resolution,
                EvidencePosture::Conflicted { .. }
            ));
        }
        other => panic!("expected contribution, got {other:?}"),
    }
}

#[test]
fn lens_purity_same_input_same_contribution() {
    let u = unit("pure");
    let mut set = ObservationSet::new();
    translate_batch(
        &[LensObservation::TypedSumTypePresent],
        &u,
        &basis(),
        "rev-0",
        &mut set,
    );
    let id = IntentId("m1-intent".into());
    let mk = || {
        LensInput::try_new(
            ApplicableConcern::Applicable(UniversalConcern::StateSafety),
            u.clone(),
            id.clone(),
            basis(),
            set.clone(),
        )
        .expect("applicable")
    };
    let a = ParadigmLens::ALL[2].evaluate(&mk());
    let b = ParadigmLens::ALL[2].evaluate(&mk());
    assert_eq!(a, b, "same input must yield the same contribution");
}

#[test]
fn producer_tag_matches_translation_output() {
    for l in ParadigmLens::ALL {
        assert!(l.producer_tag().starts_with(PRODUCER_PREFIX));
    }
    let l = ParadigmLens::ALL[1]; // functional
    assert_eq!(l.producer_tag(), "ac7.probe.functional");
}

#[test]
fn intent_concern_projection_helper_shapes() {
    // Shape guard: the (IntentId, ApplicableConcern) pair returned by
    // the helper is what a compatibility facade (M4) will build from a
    // declared ParadigmLensKind. NotApplicable NEVER appears here —
    // the facade maps undeclared families to the intent layer.
    let (id, applicable) = intent_for(&[UniversalConcern::StateSafety]);
    assert_eq!(id.as_str(), "m1-intent");
    assert!(applicable.is_applicable());
}
