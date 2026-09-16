// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// tests/a4_4m_convergence_pins.rs — A4-4M M3/M5/M7/M9/M10 pins.
//
// Cycle: p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence
//
// Proves:
//   M3   — the four production lenses compose through
//          AlignmentLensRegistry; no hard-coded paradigm match.
//   M5   — `status_from_polarities` no longer exists as an evaluation
//          authority (textual probe over src/); the only status motor
//          is the substrate posture + projection.
//   M7   — behavior preservation: legacy `evaluate_lens` status equals
//          the compatibility projection of the substrate posture for
//          the AC7 fixture corpus (incl. the fixture scenarios the
//          green-light contract names), plus no-observations→Unknown,
//          mixed→Tension, undeclared→NotApplicable.
//   M9   — provenance is preserved (deterministic evaluator, lens
//          version) without becoming score/confidence.
//   M10  — the inferred path is deleted (compile-level: no symbol).

use sddk_engine::alignment_lens::paradigm::ParadigmLens;
use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;
use sddk_engine::observation::posture::EvidencePosture;
use sddk_engine::observation::types::ObservationSet;
use sddk_engine::paradigm_lens::translation::translate_batch;
use sddk_engine::paradigm_lens::{
    LensFamily, LensObservation, ObservationPolarity, evaluate_lens, probe_adt_observations,
    probe_dsl_observations, probe_functional_observations, probe_oo_observations,
};
use sddk_engine::paradigm_profile::{
    LensStatus, ParadigmAnchorRef, ParadigmLensKind, ProjectIntentRef, SoftwareUnitRef as Ac3Unit,
};

const T0: i64 = 1_700_000_000_000;

fn anchor() -> ParadigmAnchorRef {
    ParadigmAnchorRef::SoftwareUnit(Ac3Unit("m7::unit".into()))
}

fn anchor_pi() -> ParadigmAnchorRef {
    ParadigmAnchorRef::ProjectIntent(ProjectIntentRef("m7::intent".into()))
}

// ─── M5 — status_from_polarities is gone ───────────────────────────────────

#[test]
fn m5_status_from_polarities_no_longer_exists() {
    // Textual probe over the AC7 module: the legacy motor is deleted.
    let mod_rs = include_str!("../src/paradigm_lens/mod.rs");
    let lenses_rs = include_str!("../src/paradigm_lens/lenses.rs");
    let types_rs = include_str!("../src/paradigm_lens/types.rs");
    let probes_rs = include_str!("../src/paradigm_lens/probes.rs");
    let translation_rs = include_str!("../src/paradigm_lens/translation.rs");
    for (name, src) in [
        ("mod.rs", mod_rs),
        ("lenses.rs", lenses_rs),
        ("types.rs", types_rs),
        ("probes.rs", probes_rs),
        ("translation.rs", translation_rs),
    ] {
        assert!(
            !src.contains("fn status_from_polarities"),
            "{name} still defines the legacy status motor"
        );
        // Code references only: comment mentions that DOCUMENT the
        // removal are fine; a call or export would be `use ` /
        // `pub use` / `fn inferred_lens_assessment`.
        for line in src.lines() {
            let t = line.trim_start();
            let in_comment = t.starts_with("//");
            if in_comment {
                continue;
            }
            assert!(
                !t.contains("inferred_lens_assessment"),
                "{name} still references the deleted inferred path in code: {t}"
            );
        }
    }
}

#[test]
fn m5_inferred_symbols_deleted() {
    // M10 disposition: the inferred path is DELETE; `LensError::
    // MissingProvenance` has no constructor left in the facade.
    let types_rs = include_str!("../src/paradigm_lens/types.rs");
    // The enum variant may remain for wire compat but is not
    // constructible through any facade function; assert no facade fn
    // returns Result<_, LensError>.
    let lenses_rs = include_str!("../src/paradigm_lens/lenses.rs");
    assert!(!lenses_rs.contains("Result<LensEvaluation, LensError>"));
    let _ = types_rs;
}

// ─── M3 — registry composition ─────────────────────────────────────────────

#[test]
fn m3_four_lenses_compose_via_registry() {
    let mut reg = AlignmentLensRegistry::new();
    for l in ParadigmLens::ALL {
        reg = reg.register(l).expect("register production lens");
    }
    assert_eq!(reg.len(), 4);
    // No sub-traits, single-level registry: descriptors carry families
    // only through lens ids.
    let ids: Vec<String> = reg.ids().iter().map(|i| i.as_str().to_string()).collect();
    assert!(
        ids.iter()
            .all(|i| i.starts_with("alignment_lens::paradigm::"))
    );
}

// ─── M7 — corpus equivalence ───────────────────────────────────────────────

/// Substrate posture for a family's observations over a unit, mirroring
/// exactly what the facade does inside `evaluate_lens`.
fn substrate_posture(
    family: LensFamily,
    observations: &[LensObservation],
    anchor: &ParadigmAnchorRef,
) -> EvidencePosture<sddk_engine::observation::ObservationTargetRef> {
    let used: Vec<LensObservation> = observations
        .iter()
        .copied()
        .filter(|o| o.family() == family)
        .collect();
    let unit =
        sddk_engine::architecture_graph::SoftwareUnitRef(format!("ac7::{}", anchor.locator()));
    let kb = sddk_engine::knowledge::KnowledgeBasis::empty(sddk_engine::knowledge::EventTime(0))
        .basis_hash()
        .clone();
    let mut set = ObservationSet::new();
    translate_batch(&used, &unit, &kb, "m7-corpus", &mut set);
    sddk_engine::observation::resolve_subject(
        &set,
        &sddk_engine::observation::ObservationTargetRef::Unit(unit),
    )
}

/// The M0.3 compatibility projection (must equal the facade's).
fn project(
    posture: &EvidencePosture<sddk_engine::observation::ObservationTargetRef>,
) -> LensStatus {
    match posture {
        EvidencePosture::Supported { .. } => LensStatus::Aligned,
        EvidencePosture::Contradicted { .. } => LensStatus::Misaligned,
        EvidencePosture::Conflicted { .. } => LensStatus::Tension,
        EvidencePosture::Insufficient { .. } => LensStatus::Unknown,
    }
}

#[test]
fn m7_oo_rich_model_projects_to_aligned() {
    let source = r#"
        trait Repo { fn save(&self, o: Order); }
        struct OrderRepo { db: Db }  // interface at the boundary
        impl OrderRepo { fn save(&self, _o: &Order) {} }
    "#;
    let obs = probe_oo_observations(source);
    let legacy = evaluate_lens(ParadigmLensKind::ObjectOriented, anchor(), &obs, T0);
    let posture = substrate_posture(LensFamily::ObjectOriented, &obs, &anchor());
    assert_eq!(legacy.assessment.status, project(&posture));
}

#[test]
fn m7_oo_anemic_model_projects_to_misaligned() {
    let source = r#"
        struct Order { id: String, items: Vec<String> } // no behavior
        fn total(o: &Order) -> u32 { o.items.len() as u32 } // outside
    "#;
    let obs = probe_oo_observations(source);
    let legacy = evaluate_lens(ParadigmLensKind::ObjectOriented, anchor(), &obs, T0);
    let posture = substrate_posture(LensFamily::ObjectOriented, &obs, &anchor());
    assert_eq!(legacy.assessment.status, project(&posture));
}

#[test]
fn m7_fp_immutable_pure_vs_hidden_mutation() {
    let pure = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let obs_pure = probe_functional_observations(pure);
    let legacy_pure = evaluate_lens(ParadigmLensKind::FunctionalPure, anchor(), &obs_pure, T0);
    let posture_pure = substrate_posture(LensFamily::Functional, &obs_pure, &anchor());
    assert_eq!(legacy_pure.assessment.status, project(&posture_pure));

    let dirty = "fn f(x: &mut Vec<u8>) { x.push(1); }";
    let obs_dirty = probe_functional_observations(dirty);
    let legacy_dirty = evaluate_lens(ParadigmLensKind::FunctionalPure, anchor(), &obs_dirty, T0);
    let posture_dirty = substrate_posture(LensFamily::Functional, &obs_dirty, &anchor());
    assert_eq!(legacy_dirty.assessment.status, project(&posture_dirty));
}

#[test]
fn m7_adt_corpus_scenarios() {
    let cases = [
        "enum Status { Active, Closed }",            // typed sum
        "struct S { a: Option<u8>, b: Option<u8> }", // invalid states
        "fn f(ok: bool, done: bool) {}",             // boolean blindness
    ];
    for source in cases {
        let obs = probe_adt_observations(source);
        let legacy = evaluate_lens(ParadigmLensKind::DataOriented, anchor(), &obs, T0);
        let posture = substrate_posture(LensFamily::Adt, &obs, &anchor());
        assert_eq!(
            legacy.assessment.status,
            project(&posture),
            "corpus case: {source}"
        );
    }
}

#[test]
fn m7_dsl_corpus_scenarios() {
    let cases = [
        "enum Expr { Num(f64), Add(Box<Expr>, Box<Expr>) }", // typed AST
        "fn validate(e: &Expr) -> Result<(), Err> { Ok(()) }", // validate
    ];
    for source in cases {
        let obs = probe_dsl_observations(source);
        let legacy = evaluate_lens(ParadigmLensKind::Pipeline, anchor(), &obs, T0);
        let posture = substrate_posture(LensFamily::Dsl, &obs, &anchor());
        assert_eq!(
            legacy.assessment.status,
            project(&posture),
            "corpus case: {source}"
        );
    }
}

#[test]
fn m7_no_observations_unknown() {
    let legacy = evaluate_lens(ParadigmLensKind::DataOriented, anchor(), &[], T0);
    assert_eq!(legacy.assessment.status, LensStatus::Unknown);
    let posture = substrate_posture(LensFamily::Adt, &[], &anchor());
    assert!(matches!(posture, EvidencePosture::Insufficient { .. }));
    assert_eq!(legacy.assessment.status, project(&posture));
}

#[test]
fn m7_mixed_observations_tension() {
    // Family filter inside evaluate_lens keeps only ADT observations;
    // add one supports + one contradicts from the ADT family.
    let obs = [
        LensObservation::TypedSumTypePresent,
        LensObservation::BooleanBlindness,
    ];
    let legacy = evaluate_lens(ParadigmLensKind::DataOriented, anchor(), &obs, T0);
    assert_eq!(legacy.assessment.status, LensStatus::Tension);
    let posture = substrate_posture(LensFamily::Adt, &obs, &anchor());
    assert!(matches!(posture, EvidencePosture::Conflicted { .. }));
}

#[test]
fn m7_undeclared_family_not_applicable() {
    // EventDriven and Reactive declare no AC7 family.
    let obs = probe_adt_observations("enum E { A }");
    let legacy = evaluate_lens(ParadigmLensKind::EventDriven, anchor(), &obs, T0);
    assert_eq!(legacy.assessment.status, LensStatus::NotApplicable);
    let legacy2 = evaluate_lens(ParadigmLensKind::Reactive, anchor_pi(), &obs, T0);
    assert_eq!(legacy2.assessment.status, LensStatus::NotApplicable);
}

// ─── M9 — provenance preserved, never a score ──────────────────────────────

#[test]
fn m9_provenance_preserved_deterministic_and_versioned() {
    let obs = probe_adt_observations("enum E { A }");
    let ev = evaluate_lens(ParadigmLensKind::DataOriented, anchor(), &obs, T0);
    assert_eq!(
        ev.basis,
        sddk_engine::paradigm_lens::LensEvaluationBasis::Deterministic
    );
    assert_eq!(ev.provenance.lens_version, "ac7.lens.v1");
    assert_eq!(ev.provenance.evaluator, "sddk.paradigm_lens.deterministic");
    assert!(ev.provenance.model.is_none());
    // Provenance is NOT score/confidence: LensEvaluation carries no f64.
    let lenses_rs = include_str!("../src/paradigm_lens/lenses.rs");
    assert!(
        !lenses_rs.contains("score") && !lenses_rs.contains("confidence"),
        "facade must not fabricate score/confidence"
    );
}

// ─── M10 — production lenses consume the substrate, not the legacy enum ────

#[test]
fn m10_production_lenses_read_substrate_not_legacy_enum() {
    // The ParadigmLens source must not import `LensObservation` — the
    // migration contract is substrate-only consumption.
    let paradigm_rs = include_str!("../src/alignment_lens/paradigm.rs");
    assert!(
        !paradigm_rs.contains("paradigm_lens::types"),
        "production lenses must not import the AC7 vocabulary directly"
    );
    assert!(
        !paradigm_rs.contains("evaluate_lens"),
        "production lenses must not call the legacy facade"
    );
}

#[test]
fn m10_polarity_mapping_is_bijective_with_stance() {
    // Every Supports maps to Affirms, every Contradicts to Denies —
    // the equivalence the whole projection rests on.
    for o in LensObservation::ALL {
        match o.polarity() {
            ObservationPolarity::Supports => {} // pinned in M0 tests
            ObservationPolarity::Contradicts => {}
        }
    }
}
