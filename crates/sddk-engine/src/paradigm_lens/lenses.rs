// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/lenses.rs — A4-4M M4/M5: AC7 compatibility facade.
//
// Cycle: `p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`
// Matrix: docs/architecture/a4-4m-migration-matrix.md
//
// # M4 — compatibility facade, zero evaluation logic
//
// `evaluate_lens` keeps its historical signature (LEGACY_READ_COMPAT:
// consumers are the AC7 corpus tests) but contains NO evaluation logic
// of its own. The status is now a **projection** of the kernel-substrate
// posture:
//
// ```text
// legacy input (ParadigmLensKind, observations)
//   → intent-layer applicability gate (undeclared family → NotApplicable)
//   → M0 translation (typed observations → canonical substrate)
//   → substrate posture (resolve over translated set)
//   → projection (Supported→Aligned, Contradicted→Misaligned,
//                 Conflicted→Tension, Insufficient→Unknown)
// ```
//
// # M5 — single execution authority
//
// `status_from_polarities` is DELETED. It no longer exists as an
// evaluation motor anywhere. `inferred_lens_assessment` is DELETED
// (M0.4 verdict: DEAD — zero consumers). `LensProvenance` /
// `LensEvaluationBasis` remain only as part of the legacy
// `LensEvaluation` payload the corpus tests read; the generic kernel
// never produces them.
//
// Removal trigger for this facade: when the AC7 corpus tests
// (`crates/sddk-engine/tests/ac7_lens_over_ac3_profile.rs` and
// `crates/sddk-engine/tests/ac8_full_chain_receipt.rs`) migrate to
// call `AlignmentLensKernel` directly, delete this module.
// **A4-4C (2026-09-17) did NOT remove this facade** — A4-4C's scope
// is receipt/UAT only. The migration + removal is owned by a future
// cycle (next A4 series or A4-CLOSEOUT, depending on user green-light).
// See ADR-0125 §"Post-acceptance amendment" (3rd entry) for the
// acceptance log of this deferral.

use crate::architecture_graph::SoftwareUnitRef;
use crate::observation::ObservationTargetRef;
use crate::observation::posture::EvidencePosture;
use crate::observation::types::ObservationSet;
use crate::paradigm_profile::{LensAssessment, LensStatus, ParadigmAnchorRef, ParadigmLensKind};

use super::translation;
use super::types::{
    LENS_VERSION, LensEvaluationBasis, LensObservation, LensProvenance, family_for_kind,
};

/// The result of a deterministic lens evaluation: the AC3 assessment plus the
/// AC7-level basis/provenance that AC3's frozen vocabulary cannot express.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensEvaluation {
    /// The AC3-shaped assessment (status + basis + evidence).
    pub assessment: LensAssessment,
    /// AC7's own basis (`Deterministic` | `Inferred`).
    pub basis: LensEvaluationBasis,
    /// Provenance (always populated; AC-035-007).
    pub provenance: LensProvenance,
    /// The observations that contributed, in input order.
    pub used_observations: Vec<LensObservation>,
}

fn notes_for(family_tag: &str, used: usize, basis: LensEvaluationBasis) -> String {
    format!(
        "lens_version={LENS_VERSION}; family={family_tag}; basis={}; observations={used}",
        basis.canonical_tag()
    )
}

/// M0.3 compatibility projection: substrate posture → legacy status.
/// `NotApplicable` is unreachable here — it is resolved by the
/// intent-layer gate BEFORE this projection runs.
pub(crate) fn project_status(posture: &EvidencePosture<ObservationTargetRef>) -> LensStatus {
    match posture {
        EvidencePosture::Supported { .. } => LensStatus::Aligned,
        EvidencePosture::Contradicted { .. } => LensStatus::Misaligned,
        EvidencePosture::Conflicted { .. } => LensStatus::Tension,
        EvidencePosture::Insufficient { .. } => LensStatus::Unknown,
    }
}

/// Evaluate a lens through the canonical substrate (compatibility
/// facade; the execution authority is the A4-4 observation substrate).
///
/// `declared_kind` is the profile's declared lens kind (AC3 vocabulary);
/// `observations` is whatever the caller gathered (extra families are ignored).
pub fn evaluate_lens(
    declared_kind: ParadigmLensKind,
    anchor: ParadigmAnchorRef,
    observations: &[LensObservation],
    evaluated_at_ms: i64,
) -> LensEvaluation {
    let family = family_for_kind(declared_kind);
    let used: Vec<LensObservation> = match family {
        Some(f) => observations
            .iter()
            .copied()
            .filter(|o| o.family() == f)
            .collect(),
        None => Vec::new(),
    };

    // Intent-layer gate: undeclared family → NotApplicable (the ONLY
    // source of NotApplicable; never the kernel/substrate).
    let status = match family {
        None => LensStatus::NotApplicable,
        Some(_f) => {
            // Canonical substrate path (M2): translate the typed
            // observations and resolve the posture over them.
            let unit = SoftwareUnitRef(format!("ac7::{}", anchor.locator()));
            let kb = crate::knowledge::KnowledgeBasis::empty(crate::knowledge::EventTime(0))
                .basis_hash()
                .clone();
            let mut set = ObservationSet::new();
            translation::translate_batch(&used, &unit, &kb, "ac7-facade", &mut set);
            let posture =
                crate::observation::resolve_subject(&set, &ObservationTargetRef::Unit(unit));
            project_status(&posture)
        }
    };

    let basis = LensEvaluationBasis::Deterministic;
    let provenance = LensProvenance::deterministic("sddk.paradigm_lens.deterministic");
    let family_tag = family.map(|f| f.canonical_tag()).unwrap_or("none");

    let assessment = LensAssessment {
        lens: declared_kind,
        anchor,
        status,
        basis: basis.to_evidence_basis(),
        // Cite only the observations that were used.
        evidence_refs: used.iter().map(|o| o.evidence_ref()).collect(),
        notes: Some(notes_for(family_tag, used.len(), basis)),
        evaluated_at_ms,
    };

    LensEvaluation {
        assessment,
        basis,
        provenance,
        used_observations: used,
    }
}

/// REMOVED (A4-4M M5): `inferred_lens_assessment` — DEAD (zero
/// consumers, M0.4). REMOVED: `status_from_polarities` — the legacy
/// evaluation motor; the substrate posture + projection replace it.
///
/// Kept as compile-time documentation of the removal: any code that
/// still tries to call the removed functions fails to build.
#[deprecated(note = "removed in A4-4M M5: use alignment_lens::AlignmentLensKernel")]
#[doc(hidden)]
pub const REMOVED_IN_A4_4M: () = {
    // `inferred_lens_assessment` → DELETE (M0.4 DEAD).
    // `status_from_polarities`  → REPLACE (substrate posture).
    // Proof: no symbol references remain below.
    let _: fn(&EvidencePosture<ObservationTargetRef>) -> LensStatus = project_status;
};
