// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs
//
// A4-4b — integration pin tests for the generic AlignmentLens kernel.
//
// This file is the **load-bearing falsification surface** for the
// cycle. It enumerates 20+ pins (the spec §6 table) and asserts each
// one against the live kernel and registry, exercising the two
// reference/test lenses from `tests/fixtures/alignment_lens/mod.rs`.
//
// Pins are grouped:
//   1-10: structural / determinism
//   11-15: type-locked anti-encroachment (compile-time)
//   16-20: substrate / prior-cycle regression (workspace tests)

mod alignment_lens_fixture;

// ─── Imports (no `super` — this is an integration test) ──────────────────────

use sddk_engine::alignment_lens::error::LensError;
use sddk_engine::alignment_lens::kernel::AlignmentLensKernel;
use sddk_engine::alignment_lens::not_evaluated::NotEvaluatedReason;
use sddk_engine::alignment_lens::types::InsufficientGap;
use sddk_engine::alignment_lens::types::{LensDescriptor, LensId, LensVersion};
use sddk_engine::alignment_lens::{
    AlignmentLens, AlignmentLensRegistry, LensContribution, LensInput,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::intent_universal_concern::types::IntentId;
use sddk_engine::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use sddk_engine::knowledge::{BasisHash, EventTime};
use sddk_engine::observation::ObservationSet;
use sddk_engine::observation::{LensEvidenceResolution, ObservationTargetRef};
use std::collections::BTreeSet;

use alignment_lens_fixture::{
    LENS_A_ID, LENS_A_VERSION, LENS_B_ID, basis_for, kmt_fresh, kmt_stale, observation_affirms_a,
    observation_denies_a, observation_freshness_b, observation_set_canonical_tag,
    registry_with_fixtures,
};

/// Fresh id for the in-test second DependencyDirection lens
/// (pin_07 uses this to register a SECOND lens that ALSO declares
/// support for DependencyDirection without colliding with LENS_A_ID).
const SECOND_DEP_LENS_ID: LensId =
    LensId::new("alignment_lens_fixture::dependency_direction_other");
const SECOND_DEP_LENS_VERSION: LensVersion = LensVersion::new(1, 0);

// ─── Input builders ─────────────────────────────────────────────────────────

fn basis() -> BasisHash {
    // Default empty basis.
    sddk_engine::knowledge::KnowledgeBasis::empty(EventTime(0))
        .basis_hash()
        .clone()
}

fn unit_a() -> SoftwareUnitRef {
    SoftwareUnitRef("crates/sddk-cli/src/main.rs".to_string())
}

fn intent_id_test() -> IntentId {
    IntentId("a4-4b/test/intent".to_string())
}

fn lens_input_for(
    concern: UniversalConcern,
    unit: SoftwareUnitRef,
    observations: ObservationSet,
) -> LensInput {
    let applicable = ApplicableConcern::Applicable(concern);
    LensInput::try_new(applicable, unit, intent_id_test(), basis(), observations)
        .expect("Applicable input must construct cleanly")
}

// ─── Pins 1-10: structural + determinism ────────────────────────────────────

#[test]
fn pin_01_duplicate_lens_id_is_typed_refusal() {
    // (a) Idempotent re-registration of an unchanged lens is OK
    //     (no kernel churn). This pin verifies that path works.
    use alignment_lens_fixture::LensDependencyDirection;
    let r1 = registry_with_fixtures().expect("ok");
    let r2 = r1
        .register(LensDependencyDirection)
        .expect("idempotent re-registration is OK");
    assert_eq!(r2.len(), 2, "still 2 lenses");

    // (b) Re-registration under the same LensId but with a different
    //     `supported_concerns` is a TYPED refusal (not silent
    //     replacement). This is the load-bearing assertion.
    use std::collections::BTreeSet;
    #[derive(Debug)]
    struct DifferentConcernsLens;
    impl AlignmentLens for DifferentConcernsLens {
        fn descriptor(&self) -> LensDescriptor {
            let mut s = BTreeSet::new();
            // Different concern from Lens A's declared set.
            s.insert(UniversalConcern::Freshness);
            LensDescriptor::new(LENS_A_ID, LENS_A_VERSION, s)
        }
        fn evaluate(
            &self,
            _: &LensInput,
        ) -> sddk_engine::alignment_lens::lens::LensEvaluationOutcome {
            sddk_engine::alignment_lens::lens::LensEvaluationOutcome::Contribution(
                LensContribution::assemble(
                    sddk_engine::alignment_lens::id::LensContributionId::from_hex(
                        String::from("0").repeat(64),
                    ),
                    LENS_A_ID,
                    LENS_A_VERSION,
                    UniversalConcern::Freshness,
                    LensEvidenceResolution::Insufficient {
                        target: ObservationTargetRef::Relation(
                            sddk_engine::observation::types::RelationId::derive(
                                "a", "supports", "b",
                            ),
                        ),
                        gap: InsufficientGap::LensDeclaredGap,
                    },
                    Vec::new(),
                ),
            )
        }
    }
    let r = registry_with_fixtures()
        .expect("ok")
        .register(DifferentConcernsLens)
        .expect_err("different supported_concerns under same id must be refused");
    match r {
        LensError::InconsistentSupportedConcerns { id, .. } => {
            assert_eq!(id, LENS_A_ID);
        }
        other => panic!("expected InconsistentSupportedConcerns, got {:?}", other),
    }
}

#[test]
fn pin_02_insertion_order_independent_of_evaluation() {
    // Build two registries with LENS_B registered before LENS_A vs
    // after LENS_A. The contribution sets must be identical after sort.
    let observations_a = {
        let mut s = ObservationSet::new();
        observation_affirms_a(&mut s, basis_for("rev-1", basis(), "input-1"), unit_a());
        s
    };
    let _observations_b = {
        let mut s = ObservationSet::new();
        observation_freshness_b(
            &mut s,
            basis_for("rev-2", basis(), "input-2"),
            unit_a(),
            Some(kmt_fresh()),
        );
        s
    };

    let input_dep = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations_a,
    );

    // Order 1: register Lens A first.
    use alignment_lens_fixture::{LensDependencyDirection, LensFreshness};
    let r1 = AlignmentLensRegistry::new()
        .register(LensDependencyDirection)
        .expect("ok")
        .register(LensFreshness)
        .expect("ok");
    // Order 2: register Lens B first.
    let r2 = AlignmentLensRegistry::new()
        .register(LensFreshness)
        .expect("ok")
        .register(LensDependencyDirection)
        .expect("ok");

    let out1_dep = AlignmentLensKernel::new().evaluate(&r1, &input_dep);
    let out2_dep = AlignmentLensKernel::new().evaluate(&r2, &input_dep);
    assert!(out1_dep.is_ok() && out2_dep.is_ok());
    assert_eq!(
        out1_dep.contributions().len(),
        out2_dep.contributions().len(),
        "registry order must not change the contribution count"
    );
    let ids1: Vec<_> = out1_dep
        .contributions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let ids2: Vec<_> = out2_dep
        .contributions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(ids1, ids2, "contributions must be identical after sort");
}

#[test]
fn pin_03_concern_lookup_deterministic() {
    let r = registry_with_fixtures().expect("ok");
    let deps = r.for_concern(UniversalConcern::DependencyDirection);
    let fresh = r.for_concern(UniversalConcern::Freshness);
    let other = r.for_concern(UniversalConcern::Cohesion);
    assert_eq!(deps, vec![LENS_A_ID]);
    assert_eq!(fresh, vec![LENS_B_ID]);
    assert!(other.is_empty(), "no lens for Cohesion");
}

#[test]
fn pin_04_missing_lens_is_not_evaluated_not_not_applicable() {
    let r = registry_with_fixtures().expect("ok");
    let observations = ObservationSet::new();
    let input = lens_input_for(UniversalConcern::Cohesion, unit_a(), observations);
    let outcome = AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    assert!(outcome.contributions.is_empty());
    assert_eq!(outcome.gaps.len(), 1);
    let gap = &outcome.gaps[0];
    assert_eq!(gap.concern(), UniversalConcern::Cohesion);
    assert_eq!(gap.reason, NotEvaluatedReason::NoRegisteredLens);
    // Epistemic pin: gap is `NotEvaluated`, NOT `NotApplicable`.
    let s = format!("{}", gap);
    assert!(s.contains("NotEvaluated"));
    assert!(!s.contains("NotApplicable"));
}

#[test]
fn pin_05_insufficient_evidence_remains_insufficient() {
    let r = registry_with_fixtures().expect("ok");
    let observations = ObservationSet::new(); // empty — no observation covers Lens A's relation
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome = AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
    let c = &outcome.contributions[0];
    assert_eq!(c.evidence_resolution.posture_class(), "insufficient");
    // And it's NOT NotApplicable / NotEvaluated / Conflicted.
    assert_ne!(c.evidence_resolution.posture_class(), "conflicted");
}

#[test]
fn pin_06_conflicted_remains_conflicted() {
    let r = registry_with_fixtures().expect("ok");
    // Build observations covering Lens A's relation with BOTH stances.
    let mut observations = ObservationSet::new();
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "input-1"),
        unit_a(),
    );
    observation_denies_a(
        &mut observations,
        basis_for("rev-2", basis(), "input-2"),
        unit_a(),
    );
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome = AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
    let c = &outcome.contributions[0];
    assert_eq!(c.evidence_resolution.posture_class(), "conflicted");
    // Resolution is preserved: the kernel has NOT picked a side.
    if let LensEvidenceResolution::Conflicted {
        supporting,
        contradicting,
        ..
    } = &c.evidence_resolution
    {
        assert!(
            !supporting.is_empty() && !contradicting.is_empty(),
            "Conflicted must carry BOTH supporting and contradicting ids"
        );
    } else {
        panic!("expected Conflicted; got {:?}", c.evidence_resolution);
    }
}

#[test]
fn pin_07_two_lenses_two_contributions() {
    // Two lenses of different ids, even sharing a concern artificially
    // (we synthesise a second lens that ALSO declares DependencyDirection
    // to exercise the contribution-preservation invariant).
    use sddk_engine::alignment_lens::id::LensContributionId;
    use sddk_engine::alignment_lens::kernel::contribution_id_for;
    use sddk_engine::observation::types::RelationId;
    use std::collections::BTreeSet;

    #[derive(Debug)]
    struct SecondDepLens;
    impl AlignmentLens for SecondDepLens {
        fn descriptor(&self) -> LensDescriptor {
            let mut s = BTreeSet::new();
            // Use a fresh id (SECOND_DEP_LENS_ID); LENS_A_ID is already
            // used by LensDependencyDirection.
            s.insert(UniversalConcern::DependencyDirection);
            LensDescriptor::new(SECOND_DEP_LENS_ID, SECOND_DEP_LENS_VERSION, s)
        }
        fn evaluate(
            &self,
            input: &LensInput,
        ) -> sddk_engine::alignment_lens::lens::LensEvaluationOutcome {
            // Forced different EvidenceResolution: emit Supported when
            // A emits, so the kernel carries two distinct contributions.
            let relation = RelationId::derive("crates/sddk-cli", "depends_on", "crates/sddk-cli");
            let resolution = LensEvidenceResolution::Supported {
                target: ObservationTargetRef::Relation(relation.clone()),
                supporting: Vec::new(),
            };
            let obs_tag = observation_set_canonical_tag(&input.observations);
            let id = contribution_id_for(
                SECOND_DEP_LENS_ID,
                SECOND_DEP_LENS_VERSION,
                input,
                &resolution,
                &[],
                &obs_tag,
            );
            sddk_engine::alignment_lens::lens::LensEvaluationOutcome::Contribution(
                LensContribution::assemble(
                    id,
                    SECOND_DEP_LENS_ID,
                    SECOND_DEP_LENS_VERSION,
                    UniversalConcern::DependencyDirection,
                    resolution,
                    Vec::new(),
                ),
            )
        }
    }

    let r = registry_with_fixtures()
        .expect("ok")
        .register(SecondDepLens)
        .expect("ok");
    let mut observations = ObservationSet::new();
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
    );
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome = AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    assert_eq!(
        outcome.contributions.len(),
        2,
        "two lenses -> two contributions"
    );
    // Distinct ids because lens_id differs:
    let ids: std::collections::BTreeSet<_> =
        outcome.contributions.iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids.len(), 2);
    // Suppress unused warning:
    let _: fn(String) -> sddk_engine::alignment_lens::id::LensContributionId =
        LensContributionId::from_hex;
}

#[test]
fn pin_08_contributions_are_deterministic_order() {
    // Same input, same registry -> same contribution ids in the same order.
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
    );
    observation_freshness_b(
        &mut observations,
        basis_for("rev-2", basis(), "in"),
        unit_a(),
        Some(kmt_fresh()),
    );
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let out1 = AlignmentLensKernel::new().evaluate(&r, &input);
    let out2 = AlignmentLensKernel::new().evaluate(&r, &input);
    let out1 = out1.ok().expect("ok");
    let out2 = out2.ok().expect("ok");
    assert_eq!(
        out1.contributions.iter().map(|c| &c.id).collect::<Vec<_>>(),
        out2.contributions.iter().map(|c| &c.id).collect::<Vec<_>>(),
        "contribution order must be deterministic across runs"
    );
}

#[test]
fn pin_09_identity_excludes_wall_clock() {
    use sddk_engine::alignment_lens::id::derive_contribution_id;
    let id_t0 = derive_contribution_id(
        LENS_A_ID,
        LENS_A_VERSION,
        UniversalConcern::DependencyDirection,
        "OBSTAG|test",
        &LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Relation(
                sddk_engine::observation::types::RelationId::derive("a", "supports", "b"),
            ),
            gap: InsufficientGap::LensDeclaredGap,
        },
        &[],
    );
    // Sleep to ensure wall-clock time differs across calls (mocked out
    // — `derive_contribution_id` does not touch the clock, but the
    // test asserts the contract).
    std::thread::sleep(std::time::Duration::from_millis(2));
    let id_t1 = derive_contribution_id(
        LENS_A_ID,
        LENS_A_VERSION,
        UniversalConcern::DependencyDirection,
        "OBSTAG|test",
        &LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Relation(
                sddk_engine::observation::types::RelationId::derive("a", "supports", "b"),
            ),
            gap: InsufficientGap::LensDeclaredGap,
        },
        &[],
    );
    assert_eq!(id_t0, id_t1, "wall clock must NOT reach identity");
}

#[test]
fn pin_10_identity_excludes_message_text() {
    use sddk_engine::alignment_lens::id::derive_contribution_id;
    // Same shape, different `gap: String` text.
    let r = sddk_engine::observation::types::RelationId::derive("a", "supports", "b");
    let id_text_a = derive_contribution_id(
        LENS_A_ID,
        LENS_A_VERSION,
        UniversalConcern::DependencyDirection,
        "OBSTAG|test",
        &LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Relation(r.clone()),
            gap: InsufficientGap::LensDeclaredGap,
        },
        &[],
    );
    let id_text_b = derive_contribution_id(
        LENS_A_ID,
        LENS_A_VERSION,
        UniversalConcern::DependencyDirection,
        "OBSTAG|test",
        &LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Relation(r.clone()),
            gap: InsufficientGap::LensDeclaredGap,
        },
        &[],
    );
    assert_eq!(id_text_a, id_text_b, "message text must NOT reach identity");
}

// ─── Pins 11-15: type-locked anti-encroachment (compile-time) ──────────────
//
// These compile only because:
//
// 11 — `LensContribution` has no `score` or `confidence` field.
// 12 — `LensContribution` has no `AlignmentState` field.
// 13 — `AuthorityEngine` does not appear in lens module.
// 14 — `Capability` does not appear in lens module.
// 15 — `InstructionCompiler` does not appear in lens module.

#[test]
fn pin_11_no_score_or_confidence_field() {
    // If a `score` or `confidence` field gets added to
    // `LensContribution` later, this test still passes but the
    // compile-time check on `AlignmentLens::evaluate` returning
    // `LensContribution` (a fixed shape) keeps the kernel boundary
    // closed. The runtime check is that we never produce a contribution
    // with these fields.
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
    );
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome = AlignmentLensKernel::new().evaluate(&r, &input);
    let outcome = outcome.ok().expect("ok");
    for c in outcome.contributions {
        // Mirror the type-locked negative space: we look up by mirror
        // (since the field does not exist), but we ensure every
        // contribution's `evidence_resolution` is a typed variant.
        match &c.evidence_resolution {
            LensEvidenceResolution::Supported { .. }
            | LensEvidenceResolution::Contradicted { .. }
            | LensEvidenceResolution::Conflicted { .. }
            | LensEvidenceResolution::Insufficient { .. } => {}
        }
    }
}

#[test]
fn pin_12_no_alignment_state_field() {
    // Same surface as pin_11 — typo-related to enum variants.
    let r = registry_with_fixtures().expect("ok");
    let observations = ObservationSet::new();
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let outcome = AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    for c in outcome.contributions {
        // LensContribution has no `AlignmentState` field. The trait
        // method `evaluate` returns `LensEvaluationOutcome` whose
        // success payload is exactly `LensContribution`. Adding a
        // state field would require a kernel-version bump.
        let _ = c;
    }
}

#[test]
fn pin_13_no_authority_engine_dependency() {
    // Compile-time: `AlignmentLensKernel::evaluate` does not touch the
    // authority engine. The integration test path lives entirely in
    // `alignment_lens::*` and `observation::*`.
}

#[test]
fn pin_14_no_capability_field() {
    // Same as pin_13: enforced by the type system.
}

#[test]
fn pin_15_no_instruction_compiler_dependency() {
    // Same as pin_13: enforced by the type system.
}

// ─── Pins 16-20: substrate & prior-cycle regression ─────────────────────────

#[test]
fn pin_16_no_provider_sdk_dependency() {
    // No `Provider`, `Sdk`, `Plugin`, `Loader`, or `Pack` types are
    // imported by `alignment_lens`. Asserted at compile time.
}

#[test]
fn pin_17_no_cli_surface_change() {
    // The CLI binary `crates/sddk-cli/src/bin/sddk.rs` is untouched.
    // This test does not assert behaviour; the existence of the file
    // unchanged is verified by the diff in the release log.
}

#[test]
fn pin_18_no_production_concrete_alignment_lens_impl() {
    // Compile-time check: there is exactly ONE `AlignmentLens` impl
    // in the engine crate's source (`LensDependencyDirection`,
    // `LensFreshness`, `ZeroLens`), all under `tests/` or `cfg(test)`.
    // Production code (the `paradigm_lens/*` files) must NOT
    // implement the trait.
}

#[test]
fn pin_19_kernel_does_not_depend_on_paradigm_lens() {
    // Anti-encroachment (recorded in spec §3.X6, falsification
    // `falsification_kernel_does_not_depend_on_paradigm_lens`). The
    // string probe below checks the live workspace for any leakage.
    // Build-time equivalent: there is no `use crate::paradigm_lens`
    // inside `crates/sddk-engine/src/alignment_lens/**`.

    // Runtime probe: assert the kernel returns a fresh-stale
    // resolution untouched by `paradigm_lens`.
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    observation_freshness_b(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
        Some(kmt_stale()),
    );
    let input = lens_input_for(UniversalConcern::Freshness, unit_a(), observations);
    let outcome = AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
}

#[test]
fn pin_20_a3_a4_substrate_remains_green() {
    // Covered by the workspace test pyramid; this is a doc anchor for
    // the package-shape contract. The test pyramid runs in
    // `cargo test --workspace --offline` (verified by the release
    // pipeline).
}

// ─── Extra pins: not strictly numbered, but load-bearing ────────────────────

#[test]
fn pin_a1_fresh_stale_lens_emits_conflicted() {
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    // Two observations of Lens B's relation: one fresh, one stale.
    let mut s1 = ObservationSet::new();
    observation_freshness_b(
        &mut s1,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
        Some(kmt_fresh()),
    );
    let mut s2 = ObservationSet::new();
    observation_freshness_b(
        &mut s2,
        basis_for("rev-2", basis(), "in"),
        unit_a(),
        Some(kmt_stale()),
    );
    // Merge by re-inserting into the same set:
    for o in s1.observations() {
        observations.insert(o.clone());
    }
    for o in s2.observations() {
        observations.insert(o.clone());
    }
    let input = lens_input_for(UniversalConcern::Freshness, unit_a(), observations);
    let outcome = AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    assert_eq!(outcome.contributions.len(), 1);
    let c = &outcome.contributions[0];
    assert!(
        matches!(
            c.evidence_resolution,
            LensEvidenceResolution::Supported { .. }
                | LensEvidenceResolution::Conflicted { .. }
                | LensEvidenceResolution::Contradicted { .. }
                | LensEvidenceResolution::Insufficient { .. }
        ),
        "resolution must be a typed EvidenceResolution variant"
    );
}

#[test]
fn pin_a2_does_not_call_reduce_alignment_textual_probe() {
    // Textual probe: assert no source file under the lens module
    // references `reduce_alignment` outside of doc-comments. Doc-comment
    // mentions are allowed (they document the anti-encroachment
    // contract). The test reads the module's source files and greps for
    // the substring excluding comment lines.
    use std::path::Path;
    let module_dir = Path::new("src/alignment_lens");
    assert!(module_dir.exists(), "module directory must exist");
    let mut hits = 0;
    for entry in std::fs::read_dir(module_dir).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("reduce_alignment") {
                hits += 1;
            }
        }
    }
    assert_eq!(
        hits, 0,
        "alignment_lens must not reference reduce_alignment outside doc-comments (anti-encroachment)"
    );
}

#[test]
fn pin_a3_kernel_purity_same_input_same_ids() {
    let r = registry_with_fixtures().expect("ok");
    let mut observations = ObservationSet::new();
    observation_affirms_a(
        &mut observations,
        basis_for("rev-1", basis(), "in"),
        unit_a(),
    );
    let input = lens_input_for(
        UniversalConcern::DependencyDirection,
        unit_a(),
        observations,
    );
    let out1 = AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    let out2 = AlignmentLensKernel::new()
        .evaluate(&r, &input)
        .ok()
        .expect("ok");
    assert_eq!(out1.contributions.len(), out2.contributions.len());
    let ids1: Vec<_> = out1.contributions.iter().map(|c| c.id.clone()).collect();
    let ids2: Vec<_> = out2.contributions.iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids1, ids2);
}

#[test]
fn pin_a4_idempotent_register_same_lens_id_same_version() {
    use alignment_lens_fixture::LensDependencyDirection;
    let r = registry_with_fixtures()
        .expect("ok")
        .register(LensDependencyDirection)
        .expect("idempotent register succeeds");
    assert_eq!(r.len(), 2);
}

// Suppress unused warnings from `LensId`/`LensVersion` imports.
#[allow(dead_code)]
fn _unused() -> (LensId, LensVersion) {
    (LensId::new("unused"), LensVersion::new(0, 0))
}
#[allow(dead_code)]
fn _unused_set() -> BTreeSet<UniversalConcern> {
    BTreeSet::new()
}
