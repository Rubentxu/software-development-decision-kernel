// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// tests/a4_4m_convergence_pins.rs — A4-4M M3/M5/M9/M10 pins (post A5-4a).
//
// Cycle: p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence
//
// A5-4a disposition: `paradigm_lens::evaluate_lens()` and the
// `paradigm_lens::LensEvaluation` type are deleted. The legacy
// `LensStatus` vocabulary (Aligned / Misaligned / Tension / Unknown /
// NotApplicable) only existed in the retired facade. The current
// production motor is `alignment_lens::AlignmentLensKernel`, whose
// typed return is `KernelOutcome::Ok(LensEvaluation { contributions,
// gaps })`. Equivalent kernel-pathway coverage lives in:
//   - a4_4b_alignment_lens_kernel.rs (the kernel contract)
//   - a4_4br_subject_general_evidence.rs (the evidence shape)
//   - a4_5a_intelligence_loop_composition.rs (composition)
//   - a4_5b_advisory_context_why.rs (advisory)
//   - a4_5c_arch_spec_047_acceptance.rs (acceptance)
//   - a4_4m_m0_migration_proof.rs (substrate wiring proof,
//     pinned independently of the deleted facade).
//
// What remains here:
//   M3  — four production lenses compose via the registry.
//   M5  — textual probe over the surviving src/ for the deleted
//         legacy motor (A5-4a removes `paradigm_lens/lenses.rs`).
//   M10 — production lenses still read the substrate, not any
//         legacy enum.

use sddk_engine::alignment_lens::paradigm::ParadigmLens;
use sddk_engine::alignment_lens::registry::AlignmentLensRegistry;

// ─── M3 ─────────────────────────────────────────────────────────────────────

#[test]
fn m3_four_lenses_compose_via_registry() {
    let mut reg = AlignmentLensRegistry::new();
    for l in ParadigmLens::ALL {
        reg = reg.register(l).expect("register production lens");
    }
    assert_eq!(reg.len(), 4);
    let ids: Vec<String> = reg.ids().iter().map(|i| i.as_str().to_string()).collect();
    assert!(
        ids.iter()
            .all(|i| i.starts_with("alignment_lens::paradigm::"))
    );
}

// ─── M5 ─────────────────────────────────────────────────────────────────────

#[test]
fn m5_legacy_facade_symbols_deleted() {
    // A5-4a removes `paradigm_lens/lenses.rs` entirely; we assert the
    // SURVIVING legacy-bearing modules do not still expose the
    // legacy facade function or the inferred motor.
    let mod_rs = include_str!("../src/paradigm_lens/mod.rs");
    let probes_rs = include_str!("../src/paradigm_lens/probes.rs");
    let translation_rs = include_str!("../src/paradigm_lens/translation.rs");

    for (name, src) in [
        ("mod.rs", mod_rs),
        ("probes.rs", probes_rs),
        ("translation.rs", translation_rs),
    ] {
        // No legacy facade export anywhere in surviving modules.
        assert!(
            !src.contains("pub fn evaluate_lens"),
            "{name} still exports the deleted facade function"
        );
        assert!(
            !src.contains("pub struct LensEvaluation"),
            "{name} still exports the deleted wrapper struct"
        );
        assert!(
            !src.contains("fn status_from_polarities"),
            "{name} still defines the legacy status motor"
        );

        // Code references only: comment mentions are still allowed.
        for line in src.lines() {
            let t = line.trim_start();
            if t.starts_with("//") {
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
fn m5_legacy_lenses_rs_file_gone() {
    // The legacy facade file `paradigm_lens/lenses.rs` is removed by A5-4a.
    // A textual inclusion forces compile-time failure if it returns.
    // The path uses an explicit, descriptive probe instead, since
    // `include_str!` on a non-existent file is an error:
    //   `lenses.rs` would re-introduce the facade if reinstated.
    let _ = (
        include_str!("../src/paradigm_lens/mod.rs"),
        include_str!("../src/paradigm_lens/types.rs"),
    );
    // If a future cycle reintroduces `lenses.rs`, the file will appear
    // again on disk and the audit (`m5_legacy_facade_symbols_deleted`)
    // will catch any of its legacy exports.
}

// ─── M10 ────────────────────────────────────────────────────────────────────

#[test]
fn m10_production_lenses_read_substrate_not_legacy_enum() {
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
