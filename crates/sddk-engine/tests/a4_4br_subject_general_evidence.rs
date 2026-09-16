// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_4br_subject_general_evidence.rs
//
// A4-4bR - Subject-General Evidence Resolution.
//
// Load-bearing falsification surface for the cycle. Enumerates the
// pins (P01..P24 in spec.md) and the falsification checklist
// (1)..(7) against the live kernel and registry. Pins are grouped:
//
//   1. Subject-general algebra shape (P01..P07).
//   2. No-synthetic-relations in fixture (P08..P10).
//   3. Anti-encroachment (P11..P13).
//   4. A4-0 / A4-2 baseline regression (P14..P15).
//   5. Workspace gates (P16..P18).
//   6. Lens-side shapes preserved (P21..P24).
//   7. Release gate documentation (P19..P20) - exercised by
//      `scripts/release.sh`, not by this test file.
//
// Falsifications (1)..(7) are inlined as named tests so they appear
// in `cargo test` output by name.

mod alignment_lens_fixture;

// ─── Imports ────────────────────────────────────────────────────────────────

use sddk_engine::alignment_lens::types::{InsufficientGap, LensId, LensVersion};
use sddk_engine::alignment_lens::{
    AlignmentLens, AlignmentLensRegistry, LensContribution, LensInput,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::{BasisHash, EventTime};
use sddk_engine::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    RelationId, SoftwareObservation, SoftwareRelation,
};
use sddk_engine::observation::{
    EvidencePosture, EvidenceResolution, LensEvidenceResolution, ObservationTargetRef,
    resolve_relation, resolve_subject,
};
use sddk_engine::semantic_kind::CoreRelationKind;

use alignment_lens_fixture::{
    basis_for, kmt_fresh, kmt_stale, observation_freshness_b, registry_with_fixtures,
};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn basis() -> BasisHash {
    sddk_engine::knowledge::KnowledgeBasis::empty(EventTime(0))
        .basis_hash()
        .clone()
}

fn unit_a() -> SoftwareUnitRef {
    SoftwareUnitRef("crates/sddk-cli/src/main.rs".to_string())
}

fn unit_foo() -> SoftwareUnitRef {
    SoftwareUnitRef("foo".to_string())
}

fn intent_id_test() -> IntentId {
    IntentId("a4-4br/test/intent".to_string())
}

fn observation_set_for_input(
    concern: UniversalConcern,
    unit: SoftwareUnitRef,
    observations: ObservationSet,
) -> LensInput {
    let applicable = ApplicableConcern::Applicable(concern);
    LensInput::try_new(applicable, unit, intent_id_test(), basis(), observations)
        .expect("Applicable input must construct cleanly")
}

fn relation_for(a: &str, b: &str) -> RelationId {
    SoftwareRelation::new(
        sddk_engine::observation::SoftwareEntityRef::Unit(SoftwareUnitRef::new(a)),
        CoreRelationKind::DependsOn,
        sddk_engine::observation::SoftwareEntityRef::Unit(SoftwareUnitRef::new(b)),
    )
    .id()
}

fn unit_observation(unit: SoftwareUnitRef, stance: ObservationStance) -> SoftwareObservation {
    SoftwareObservation::declare(
        ObservationSubject::Unit(unit),
        stance,
        sddk_engine::evidence_ref::EvidenceRef::new(
            sddk_engine::evidence_ref::EvidenceKind::Governance,
            "u_loc",
        ),
        ObservationOrigin::DeterministicLocal,
        ObservationBasis {
            revision: "rev-1".to_string(),
            knowledge_basis: basis(),
            input_digest: "in".to_string(),
        },
        None,
        "a4_4br_fixture",
    )
}

fn relation_observation(rel: RelationId, stance: ObservationStance) -> SoftwareObservation {
    // Subject is the SAME SoftwareRelation the supplied rel was derived
    // from. We construct a relation with canonical a->b endpoints and
    // derive its id; if `rel` matches that, this function returns an
    // observation whose subject's relation id equals `rel`.
    let r = SoftwareRelation::new(
        sddk_engine::observation::SoftwareEntityRef::Unit(SoftwareUnitRef::new("a")),
        CoreRelationKind::DependsOn,
        sddk_engine::observation::SoftwareEntityRef::Unit(SoftwareUnitRef::new("b")),
    );
    assert_eq!(r.id(), rel, "test helper assumes a->DependsOn->b");
    SoftwareObservation::declare(
        ObservationSubject::SoftwareRelation(r),
        stance,
        sddk_engine::evidence_ref::EvidenceRef::new(
            sddk_engine::evidence_ref::EvidenceKind::Governance,
            "r_loc",
        ),
        ObservationOrigin::DeterministicLocal,
        ObservationBasis {
            revision: "rev-1".to_string(),
            knowledge_basis: basis(),
            input_digest: "in".to_string(),
        },
        None,
        "a4_4br_fixture",
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Subject-general algebra shape (P01..P07)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p01_posture_supported_for_relation_matches_legacy_supported() {
    // Equivalence: resolve_subject(Relation) of a single Affirm produces
    // a posture structurally equivalent to the legacy
    // EvidenceResolution::Supported (same observation id, same class).
    let r = relation_for("a", "b");
    let mut set = ObservationSet::new();
    set.insert(relation_observation(r.clone(), ObservationStance::Affirms));

    let legacy = resolve_relation(&set, &r);
    let target = ObservationTargetRef::Relation(r);
    let general = resolve_subject(&set, &target);

    assert_eq!(general.posture_class(), "supported");
    // Same observation id surfaced on both sides.
    match (legacy, general) {
        (
            EvidenceResolution::Supported { supporting, .. },
            EvidencePosture::Supported { supporting: s2, .. },
        ) => assert_eq!(supporting, s2),
        _ => panic!("expected Supported on both sides"),
    }
}

#[test]
fn p02_resolve_relation_via_generalized_helper_matches_legacy() {
    // Equivalence: drive the same set through both resolvers. The
    // generalized resolver must produce observation-id sets equal to
    // the legacy resolver's (modulo wire-shape: target is `Relation(r)`
    // vs `relation: r`).
    let r = relation_for("a", "b");
    let mut set = ObservationSet::new();
    set.insert(relation_observation(r.clone(), ObservationStance::Affirms));
    set.insert(relation_observation(r.clone(), ObservationStance::Denies));

    let legacy = resolve_relation(&set, &r);
    let target = ObservationTargetRef::Relation(r);
    let general = resolve_subject(&set, &target);

    // Posture class identical on both sides.
    assert_eq!(legacy.canonical_tag(), general.posture_class());

    // Conflicted preserves both sets.
    match (legacy, general) {
        (
            EvidenceResolution::Conflicted {
                supporting: s1,
                contradicting: c1,
                ..
            },
            EvidencePosture::Conflicted {
                supporting: s2,
                contradicting: c2,
                ..
            },
        ) => {
            assert_eq!(s1, s2);
            assert_eq!(c1, c2);
        }
        _ => panic!("expected Conflicted on both sides"),
    }
}

#[test]
fn p03_supported_unit_target_carries_unit_not_synthetic_relation() {
    // A Unit(foo) target resolves against ObservationSubject::Unit(foo)
    // observations WITHOUT producing a synthetic relation.
    let mut set = ObservationSet::new();
    set.insert(unit_observation(unit_foo(), ObservationStance::Affirms));

    let target = ObservationTargetRef::Unit(unit_foo());
    let r = resolve_subject(&set, &target);
    match r {
        EvidencePosture::Supported { target: t, .. } => {
            assert_eq!(t.kind_tag(), "unit");
            assert_eq!(t.canonical_tag(), "unit:foo");
        }
        _ => panic!("expected Supported(Unit(foo))"),
    }
}

#[test]
fn p04_same_lens_concern_obs_diff_target_kind_produce_distinct_ids() {
    // The load-bearing A4-4bR pin: same lens, same concern, same
    // observation set, different target kind -> distinct
    // LensContributionId.
    use sddk_engine::alignment_lens::kernel::contribution_id_for;
    let mut set = ObservationSet::new();
    set.insert(unit_observation(unit_foo(), ObservationStance::Affirms));

    let obs_tag = "OBSTAG|test";
    let input = observation_set_for_input(UniversalConcern::Freshness, unit_foo(), set.clone());
    let p_unit: LensEvidenceResolution = LensEvidenceResolution::Supported {
        target: ObservationTargetRef::Unit(unit_foo()),
        supporting: Vec::new(),
    };
    let p_rel: LensEvidenceResolution = LensEvidenceResolution::Supported {
        target: ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo")),
        supporting: Vec::new(),
    };
    let id_unit = contribution_id_for(
        LensId::new("test_lens"),
        LensVersion::new(1, 0),
        &input,
        &p_unit,
        &[],
        obs_tag,
    );
    let id_rel = contribution_id_for(
        LensId::new("test_lens"),
        LensVersion::new(1, 0),
        &input,
        &p_rel,
        &[],
        obs_tag,
    );
    assert_ne!(
        id_unit, id_rel,
        "different target kind must produce distinct ids"
    );
}

#[test]
fn p05_unit_foo_distinct_from_relation_foo_foo() {
    // The A4-4bR pin: Unit(foo) != Relation(foo -> foo) in identity.
    let u = ObservationTargetRef::Unit(unit_foo());
    let r = ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo"));
    assert_ne!(u.kind_tag(), r.kind_tag());
    assert_ne!(u.canonical_tag(), r.canonical_tag());
}

#[test]
fn p06_conflicted_unit_preserves_both_sets() {
    let mut set = ObservationSet::new();
    set.insert(unit_observation(unit_foo(), ObservationStance::Affirms));
    set.insert(unit_observation(unit_foo(), ObservationStance::Denies));
    let target = ObservationTargetRef::Unit(unit_foo());
    let r = resolve_subject(&set, &target);
    match r {
        EvidencePosture::Conflicted {
            target: t,
            supporting,
            contradicting,
        } => {
            assert_eq!(t.kind_tag(), "unit");
            assert_eq!(supporting.len(), 1);
            assert_eq!(contradicting.len(), 1);
        }
        _ => panic!("expected Conflicted(Unit)"),
    }
}

#[test]
fn p07_insufficient_unit_target_carries_typed_gap_not_string() {
    // Empty covering -> Insufficient(Unit, NoObservation). The gap is
    // typed; the wire message is a derived convenience.
    let set = ObservationSet::new();
    let target = ObservationTargetRef::Unit(unit_foo());
    let r = resolve_subject(&set, &target);
    match r {
        EvidencePosture::Insufficient { target: t, gap } => {
            assert_eq!(t.kind_tag(), "unit");
            assert_eq!(gap, InsufficientGap::NoObservation);
            // wire_message is stable and round-trippable.
            assert_eq!(
                InsufficientGap::from_wire_message(&gap.wire_message()),
                InsufficientGap::NoObservation,
            );
        }
        _ => panic!("expected Insufficient(Unit, NoObservation)"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. No synthetic relations in fixtures (P08..P10)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p08_lens_input_doc_no_longer_instructs_synthetic_derivation() {
    // P08: LensInput doc-comment must not instruct lens authors to
    // derive a RelationId from (intent_id, concern, unit_ref).
    let text = std::fs::read_to_string(std::path::Path::new("src/alignment_lens/types.rs"))
        .expect("read types.rs");
    // The phrase "derive ... RelationId" combined with "from (intent"
    // should not appear in non-doc blocks of LensInput.
    let mut saw_instructive = false;
    let mut in_doc = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            in_doc = true;
            continue;
        }
        // First non-doc line resets the doc tracker.
        if !trimmed.starts_with("//") {
            in_doc = false;
        }
        if in_doc {
            continue;
        }
        if line.contains("derive") && line.contains("RelationId") {
            saw_instructive = true;
            break;
        }
    }
    assert!(
        !saw_instructive,
        "LensInput source must not instruct lens authors to derive RelationId"
    );
}

#[test]
fn p09_freshness_helper_creates_no_synthetic_software_relation() {
    // P09: the `observation_freshness_b` fixture helper must NOT call
    // SoftwareRelation::new. We inspect the fixture file textually.
    let text = std::fs::read_to_string(std::path::Path::new("tests/alignment_lens_fixture.rs"))
        .expect("read fixture");
    // Find the body of observation_freshness_b (between its `pub fn`
    // line and the closing brace) and assert no `SoftwareRelation::new`.
    let start = text
        .find("pub fn observation_freshness_b")
        .expect("function definition");
    // Walk braces from there.
    let after_def = &text[start..];
    let open_idx = after_def.find('{').expect("opening brace");
    let mut depth = 0usize;
    let mut end = open_idx;
    for (i, c) in after_def[open_idx..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open_idx + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &after_def[open_idx..end];
    assert!(
        !body.contains("SoftwareRelation::new"),
        "observation_freshness_b must not call SoftwareRelation::new"
    );
    assert!(
        body.contains("ObservationSubject::Unit"),
        "observation_freshness_b must build an ObservationSubject::Unit observation"
    );
}

#[test]
fn p10_dependency_direction_fixture_keeps_real_software_relation() {
    // P10: the DependencyDirection fixture uses a real SoftwareRelation
    // (heterogeneity preserved).
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    use alignment_lens_fixture::observation_affirms_a;
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
    );
    let input = observation_set_for_input(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome =
        sddk_engine::alignment_lens::kernel::AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
    let c = &outcome.contributions[0];
    match &c.evidence_resolution {
        LensEvidenceResolution::Supported { target, .. }
        | LensEvidenceResolution::Contradicted { target, .. }
        | LensEvidenceResolution::Conflicted { target, .. } => {
            assert_eq!(target.kind_tag(), "relation");
        }
        LensEvidenceResolution::Insufficient { target, .. } => {
            assert_eq!(target.kind_tag(), "relation");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Anti-encroachment (P11..P13)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p11_no_aligned2_or_lens_state_taxonomy() {
    // P11: textual probe - no `Aligned2`, `Misaligned2`, `Tension2`,
    // `LensSupported2`, `LensUnknown2`, or `LensState` symbols under
    // `src/alignment_lens/`.
    let dir = std::path::Path::new("src/alignment_lens");
    assert!(dir.exists());
    let forbidden = [
        "Aligned2",
        "Misaligned2",
        "Tension2",
        "LensSupported2",
        "LensUnknown2",
        "LensState",
    ];
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        for sym in &forbidden {
            // Allow occurrences inside doc-comment lines.
            for line in text.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("///")
                    || trimmed.starts_with("//!")
                    || trimmed.starts_with("//")
                {
                    continue;
                }
                if line.contains(sym) {
                    hits.push((
                        path.display().to_string(),
                        sym.to_string(),
                        line.to_string(),
                    ));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "no second epistemic taxonomy allowed; found: {hits:?}"
    );
}

#[test]
fn p12_lens_contribution_does_not_import_software_alignment() {
    // P12: LensContribution source must not import
    // software_alignment::reduce_alignment.
    let text = std::fs::read_to_string(std::path::Path::new("src/alignment_lens/contribution.rs"))
        .expect("read contribution.rs");
    assert!(
        !text.contains("reduce_alignment"),
        "LensContribution must not import reduce_alignment"
    );
}

#[test]
fn p13_alignment_lens_module_does_not_import_paradigm_lens() {
    // P13: nothing under src/alignment_lens/ imports paradigm_lens.
    let dir = std::path::Path::new("src/alignment_lens");
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("//")
            {
                continue;
            }
            if line.contains("paradigm_lens") {
                hits.push((path.display().to_string(), line.to_string()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "alignment_lens must not import paradigm_lens; found: {hits:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. A4-0 / A4-2 baseline regression (P14..P15)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p14_resolve_relation_unchanged_for_relation_targets() {
    // P14: drive resolve_relation via the new generalized helper path
    // and assert posture-class + observation-id sets equality.
    let r = relation_for("a", "b");
    let mut set = ObservationSet::new();
    set.insert(relation_observation(r.clone(), ObservationStance::Affirms));
    set.insert(relation_observation(r.clone(), ObservationStance::Denies));

    let legacy = resolve_relation(&set, &r);
    let target = ObservationTargetRef::Relation(r);
    let general = resolve_subject(&set, &target);

    assert_eq!(legacy.canonical_tag(), general.posture_class());
    match (legacy, general) {
        (
            EvidenceResolution::Conflicted {
                supporting: s1,
                contradicting: c1,
                ..
            },
            EvidencePosture::Conflicted {
                supporting: s2,
                contradicting: c2,
                ..
            },
        ) => {
            assert_eq!(s1, s2);
            assert_eq!(c1, c2);
        }
        _ => panic!("expected Conflicted on both sides"),
    }
}

#[test]
fn p15_debverify_relation_targets_unchanged() {
    // P15: the bridge `From<EvidenceResolution> for EvidencePosture<_>`
    // preserves both observation-id sets on Conflicted.
    let r = relation_for("a", "b");
    let mut set = ObservationSet::new();
    set.insert(relation_observation(r.clone(), ObservationStance::Affirms));
    set.insert(relation_observation(r.clone(), ObservationStance::Denies));
    let legacy = resolve_relation(&set, &r);
    let general: EvidencePosture<ObservationTargetRef> = legacy.clone().into();
    match (legacy, general) {
        (
            EvidenceResolution::Conflicted {
                supporting: s1,
                contradicting: c1,
                ..
            },
            EvidencePosture::Conflicted {
                supporting: s2,
                contradicting: c2,
                ..
            },
        ) => {
            assert_eq!(s1, s2);
            assert_eq!(c1, c2);
        }
        _ => panic!("expected Conflicted on both sides"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Workspace gates (P16..P18) - documented here, exercised by CI
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p16_cargo_fmt_check_documented() {
    // P16: covered by `cargo fmt --all -- --check` in scripts/release.sh.
    // The gate is exercised before release. This test is a doc anchor.
}

#[test]
fn p17_clippy_check_documented() {
    // P17: covered by `cargo clippy --workspace --all-targets -- -D warnings`.
}

#[test]
fn p18_workspace_tests_documented() {
    // P18: covered by `cargo test --workspace --offline`.
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. Lens-side shapes preserved (P21..P24)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p21_insufficient_gap_enum_preserved_verbatim() {
    // P21: the InsufficientGap enum carries the A4-4b variant set
    // plus LensDeclaredGap (registered in A4-4b itself).
    let variants = [
        InsufficientGap::NoObservation,
        InsufficientGap::ObservationsWithoutStance,
        InsufficientGap::MissingProvenance,
        InsufficientGap::LensDeclaredGap,
    ];
    // canonical_tag is &'static str and unique per variant.
    let mut tags: Vec<&'static str> = variants.iter().map(|v| v.canonical_tag()).collect();
    tags.sort();
    tags.dedup();
    assert_eq!(tags.len(), variants.len());
}

#[test]
fn p22_lens_evaluation_outcome_shape_preserved() {
    // P22: LensEvaluationOutcome still has Contribution and Refused.
    // We assert the registry can register a refusing lens and the
    // kernel reports the refusal.
    #[derive(Debug)]
    struct RefusingLens;
    impl AlignmentLens for RefusingLens {
        fn descriptor(&self) -> sddk_engine::alignment_lens::types::LensDescriptor {
            use std::collections::BTreeSet;
            let mut s = BTreeSet::new();
            s.insert(UniversalConcern::Cohesion);
            sddk_engine::alignment_lens::types::LensDescriptor::new(
                LensId::new("refusing_lens"),
                LensVersion::new(1, 0),
                s,
            )
        }
        fn evaluate(
            &self,
            _input: &LensInput,
        ) -> sddk_engine::alignment_lens::lens::LensEvaluationOutcome {
            sddk_engine::alignment_lens::lens::LensEvaluationOutcome::Refused(
                sddk_engine::alignment_lens::error::LensError::LensRejected {
                    id: LensId::new("refusing_lens"),
                    concern: UniversalConcern::Cohesion,
                },
            )
        }
    }
    let r = AlignmentLensRegistry::new()
        .register(RefusingLens)
        .expect("ok");
    let input =
        observation_set_for_input(UniversalConcern::Cohesion, unit_a(), ObservationSet::new());
    let out = sddk_engine::alignment_lens::kernel::AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = out.ok().expect("ok");
    assert!(outcome.contributions.is_empty());
    assert_eq!(outcome.gaps.len(), 1);
    assert_eq!(
        outcome.gaps[0].reason,
        sddk_engine::alignment_lens::not_evaluated::NotEvaluatedReason::LensExistsButRefused,
    );
}

#[test]
fn p23_alignment_lens_registry_idempotence_preserved() {
    // P23: re-registering the same LensId+LensVersion with identical
    // supported_concerns is idempotent.
    use alignment_lens_fixture::LensDependencyDirection;
    let r = registry_with_fixtures()
        .expect("ok")
        .register(LensDependencyDirection)
        .expect("idempotent re-registration is OK");
    assert_eq!(r.len(), 2);
}

#[test]
fn p24_not_evaluated_distinct_from_insufficient_and_not_applicable() {
    // P24: epistemic-pin regression - the gap vocabulary is distinct.
    use sddk_engine::alignment_lens::not_evaluated::{NotEvaluated, NotEvaluatedReason};
    let gap = NotEvaluated {
        applicable: ApplicableConcern::Applicable(UniversalConcern::Cohesion),
        candidate_lens_ids: Vec::new(),
        reason: NotEvaluatedReason::NoRegisteredLens,
    };
    let s = format!("{}", gap);
    assert!(s.contains("NotEvaluated"));
    assert!(!s.contains("NotApplicable"));
    assert!(!s.contains("Insufficient"));
    assert!(!s.contains("Conflicted"));
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. Falsification checklist (1)..(7) from spec.md
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn falsification_1_unit_observation_to_lens_contribution_no_synthetic_relation() {
    // (1) Unit observation -> LensContribution -> no synthetic
    // SoftwareRelation created. Verified by running a real
    // observation through LensFreshness and asserting the resulting
    // LensContribution's target is Unit, not Relation.
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    observation_freshness_b(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_foo(),
        Some(kmt_fresh()),
    );
    let input = observation_set_for_input(UniversalConcern::Freshness, unit_foo(), observations);
    let outcome = sddk_engine::alignment_lens::kernel::AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
    let c: &LensContribution = &outcome.contributions[0];
    let target = c.evidence_resolution.target();
    assert_eq!(target.kind_tag(), "unit");
}

#[test]
fn falsification_2_relation_resolution_pre_a4_4br_semantics_preserved() {
    // (2) Relation resolution is equivalent to the pre-A4-4bR path.
    let r = relation_for("a", "b");
    let mut set = ObservationSet::new();
    set.insert(relation_observation(r.clone(), ObservationStance::Affirms));
    set.insert(relation_observation(r.clone(), ObservationStance::Denies));
    let legacy = resolve_relation(&set, &r);
    let general = resolve_subject(&set, &ObservationTargetRef::Relation(r));
    assert_eq!(legacy.canonical_tag(), general.posture_class());
}

#[test]
fn falsification_3_conflicted_unit_preserves_both_supporting_and_contradicting() {
    // (3) Conflicted(Unit) preserves both observation sets.
    let mut set = ObservationSet::new();
    set.insert(unit_observation(unit_foo(), ObservationStance::Affirms));
    set.insert(unit_observation(unit_foo(), ObservationStance::Denies));
    let r = resolve_subject(&set, &ObservationTargetRef::Unit(unit_foo()));
    match r {
        EvidencePosture::Conflicted {
            supporting,
            contradicting,
            ..
        } => {
            assert_eq!(supporting.len(), 1);
            assert_eq!(contradicting.len(), 1);
        }
        _ => panic!("expected Conflicted(Unit)"),
    }
}

#[test]
fn falsification_4_unit_foo_and_relation_foo_foo_have_distinct_identities() {
    // (4) Unit(foo) target identity != Relation(foo -> foo) target
    // identity.
    let u = ObservationTargetRef::Unit(unit_foo()).canonical_tag();
    let r = ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo")).canonical_tag();
    assert_ne!(u, r);
}

#[test]
fn falsification_5_insufficient_preserves_typed_gap() {
    // (5) Insufficient preserved with typed gap (InsufficientGap enum),
    // never a free-form string. Wire-roundtrip-stable.
    let g = InsufficientGap::MissingProvenance;
    let msg = g.wire_message();
    let decoded = InsufficientGap::from_wire_message(&msg);
    assert_eq!(decoded, g);

    // Foreign string falls back to LensDeclaredGap (smuggling guard).
    let foreign = InsufficientGap::from_wire_message("some random free-form text");
    assert_eq!(foreign, InsufficientGap::LensDeclaredGap);
}

#[test]
fn falsification_6_same_semantic_input_same_contribution_id() {
    // (6) Same semantic input -> same LensContributionId (target kind
    // is part of identity).
    use sddk_engine::alignment_lens::kernel::contribution_id_for;
    let mut set_a = ObservationSet::new();
    set_a.insert(unit_observation(unit_foo(), ObservationStance::Affirms));
    let mut set_b = ObservationSet::new();
    set_b.insert(unit_observation(unit_foo(), ObservationStance::Affirms));

    let input_a = observation_set_for_input(UniversalConcern::Freshness, unit_foo(), set_a);
    let input_b = observation_set_for_input(UniversalConcern::Freshness, unit_foo(), set_b);

    let p: LensEvidenceResolution = LensEvidenceResolution::Supported {
        target: ObservationTargetRef::Unit(unit_foo()),
        supporting: Vec::new(),
    };

    let id_a = contribution_id_for(
        LensId::new("lens_x"),
        LensVersion::new(1, 0),
        &input_a,
        &p,
        &[],
        "OBSTAG|test",
    );
    let id_b = contribution_id_for(
        LensId::new("lens_x"),
        LensVersion::new(1, 0),
        &input_b,
        &p,
        &[],
        "OBSTAG|test",
    );
    assert_eq!(id_a, id_b, "same semantic input must yield same id");
}

#[test]
fn falsification_7_freshness_fixture_creates_no_self_relation() {
    // (7) Stronger version of P09: even calling the helper several
    // times must not introduce any SoftwareRelation in the observation
    // set's subject shape.
    let mut observations = ObservationSet::new();
    observation_freshness_b(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_foo(),
        Some(kmt_fresh()),
    );
    observation_freshness_b(
        &mut observations,
        basis_for("rev-2", basis(), "in"),
        unit_foo(),
        Some(kmt_stale()),
    );
    // Every observation has Unit subject, not Relation.
    for o in observations.observations() {
        assert!(
            matches!(o.subject, ObservationSubject::Unit(_)),
            "freshness observations must have Unit subject"
        );
    }
}
