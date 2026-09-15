// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// ac7_lens_over_ac3_profile.rs — A3-S8 cross-module integration test.
//
// AC3 declares the paradigm intent; AC7 evaluates it. This test exercises both
// modules through their public APIs (no mocks): the declared profile is read
// from an AC3 `ParadigmProfileOverlay`, then AC7's lens produces an assessment
// scoped to exactly that anchor.

use sddk_engine::paradigm_lens::family_for_kind;
use sddk_engine::paradigm_lens::{
    LensFamily, LensObservation, evaluate_lens, probe_functional_observations,
    probe_oo_observations,
};
use sddk_engine::paradigm_profile::{
    LensStatus, ParadigmAnchorRef, ParadigmLensKind, ParadigmProfileEdge, ParadigmProfileKind,
    ParadigmProfileOverlay, ProjectIntentRef,
};

const T0: i64 = 1_700_000_000;

const PURE_FIXTURE: &str = "pub fn sum(xs: &[u64]) -> u64 { xs.iter().sum() }\n";
const MUTATING_FIXTURE: &str =
    "pub struct C { v: Cell<u64> }\nimpl C { pub fn bump(&mut self) { let mut n = 1; n += 1; } }\n";

#[test]
fn ac7_evaluates_the_profile_ac3_declared() {
    // 1. AC3: declare a ProjectIntent that uses FunctionalPure.
    let project = ProjectIntentRef::new("p-63676b11dc0ef88f");
    let anchor = ParadigmAnchorRef::ProjectIntent(project.clone());
    let mut overlay = ParadigmProfileOverlay::new();
    overlay.add_project(&project);
    overlay.add_profile(&ParadigmProfileEdge {
        anchor: anchor.clone(),
        profile: ParadigmProfileKind::FunctionalPure,
        rationale: Some("pure core".to_string()),
    });

    // 2. Read the declared profile back from AC3.
    let declared = overlay.find_profiles_for_anchor(&anchor);
    assert_eq!(declared, vec![ParadigmProfileKind::FunctionalPure]);

    // The AC3 profile kind and the AC7 lens kind correspond 1:1.
    let lens_kind = ParadigmLensKind::FunctionalPure;
    assert_eq!(lens_kind.to_profile_kind(), declared[0]);
    assert_eq!(
        sddk_engine::paradigm_lens::family_for_kind(lens_kind),
        Some(LensFamily::Functional)
    );

    // 3. AC7: evaluate the declared lens against observed evidence.
    //    (a) a genuinely pure unit -> Aligned
    let clean = probe_functional_observations(PURE_FIXTURE);
    let ev_clean = evaluate_lens(lens_kind, anchor.clone(), &clean, T0);
    assert_eq!(ev_clean.assessment.status, LensStatus::Aligned);
    assert_eq!(ev_clean.assessment.anchor, anchor);

    //    (b) hidden mutation contradicts the declared pure profile -> Misaligned
    let dirty = probe_functional_observations(MUTATING_FIXTURE);
    assert!(dirty.contains(&LensObservation::HiddenMutationDetected));
    let ev_dirty = evaluate_lens(lens_kind, anchor.clone(), &dirty, T0);
    assert_eq!(ev_dirty.assessment.status, LensStatus::Misaligned);
    assert_eq!(ev_dirty.assessment.anchor, anchor);
    // AC-035-007: the assessment cites the concrete observation.
    assert!(
        ev_dirty
            .assessment
            .evidence_refs
            .iter()
            .any(|e| e.locator == "lens.fp.hidden_mutation")
    );
    // AC3's basis vocabulary is used, not invented.
    assert_eq!(
        ev_dirty.assessment.basis,
        sddk_engine::paradigm_profile::EvidenceBasis::Observed
    );
}

#[test]
fn ac7_not_applicable_applies_to_kinds_without_a_lens() {
    // AC3 declares ObjectOriented on the anchor.
    let project = ProjectIntentRef::new("p-63676b11dc0ef88f");
    let anchor = ParadigmAnchorRef::ProjectIntent(project.clone());
    let mut overlay = ParadigmProfileOverlay::new();
    overlay.add_project(&project);
    overlay.add_profile(&ParadigmProfileEdge {
        anchor: anchor.clone(),
        profile: ParadigmProfileKind::ObjectOriented,
        rationale: None,
    });
    let declared = overlay.find_profiles_for_anchor(&anchor);
    assert_eq!(declared, vec![ParadigmProfileKind::ObjectOriented]);

    // AC7 is stateless with respect to the overlay: the caller passes the lens
    // kind. A kind AC7 has a lens for evaluates to a real verdict.
    let oo_obs = probe_oo_observations("pub struct O { pub id: u64 }\n");
    let ev_oo = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor.clone(),
        &oo_obs,
        T0,
    );
    assert_eq!(ev_oo.assessment.status, LensStatus::Misaligned);
    assert_eq!(ev_oo.assessment.lens, ParadigmLensKind::ObjectOriented);

    // A kind with no AC7 lens family yields NOT_APPLICABLE even when given
    // observations that contradict *some* paradigm (AC-035-004: absent intent
    // is never MISALIGNED).
    let adt_obs = vec![LensObservation::StringlyTypedStatus];
    let ev_na = evaluate_lens(ParadigmLensKind::EventDriven, anchor.clone(), &adt_obs, T0);
    assert_eq!(ev_na.assessment.status, LensStatus::NotApplicable);

    // As does an anchor whose declared kind is unsupported.
    let ev_na2 = evaluate_lens(ParadigmLensKind::Reactive, anchor, &adt_obs, T0);
    assert_eq!(ev_na2.assessment.status, LensStatus::NotApplicable);

    // Whereas the ADT lens, given ADT observations, does evaluate (Misaligned
    // because the observations contradict the ADT paradigm).
    let ev_adt = evaluate_lens(ParadigmLensKind::DataOriented, anchor_at("p"), &adt_obs, T0);
    assert_eq!(ev_adt.assessment.status, LensStatus::Misaligned);
}

fn anchor_at(id: &str) -> ParadigmAnchorRef {
    ParadigmAnchorRef::ProjectIntent(ProjectIntentRef::new(id))
}

#[test]
fn ac7_family_helper_covers_every_ac3_lens_kind() {
    // Every AC3 `ParadigmLensKind` is classified by AC7 (either into one of the
    // four families or explicitly as having no AC7 lens).
    for k in ParadigmLensKind::ALL {
        let family = family_for_kind(k);
        match k {
            ParadigmLensKind::ObjectOriented => {
                assert_eq!(family, Some(LensFamily::ObjectOriented))
            }
            ParadigmLensKind::Functional | ParadigmLensKind::FunctionalPure => {
                assert_eq!(family, Some(LensFamily::Functional))
            }
            ParadigmLensKind::DataOriented => assert_eq!(family, Some(LensFamily::Adt)),
            ParadigmLensKind::Pipeline | ParadigmLensKind::Custom => {
                assert_eq!(family, Some(LensFamily::Dsl))
            }
            // Kinds with no AC7 lens: they must yield NotApplicable, not a verdict.
            _ => assert_eq!(family, None, "kind {k:?} unexpectedly mapped"),
        }
    }
}
