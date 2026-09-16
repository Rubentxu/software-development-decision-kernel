// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/translation.rs — A4-4M M2: probes feed the canonical
// substrate.
//
// Cycle: `p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`
// Matrix: docs/architecture/a4-4m-migration-matrix.md (M0 translation)
//
// The M0-proven translation: typed `LensObservation` → canonical
// `SoftwareObservation` about a typed `Unit` target. This is the ONLY
// bridge between the AC7 probe vocabulary and the A4-4 observation
// substrate. Properties (pinned by `tests/a4_4m_m0_migration_proof.rs`):
//
// - exhaustive over the closed 16-variant enum (no wildcard match),
// - no `EvidenceRef.locator` parsing (the locator is written as
//   provenance, never read back for meaning),
// - no synthetic `SoftwareRelation` (the subject is always the Unit),
// - polarity rides the typed `ObservationStance`.

use crate::architecture_graph::SoftwareUnitRef;
use crate::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareObservation,
};
use crate::paradigm_lens::types::{LensObservation, ObservationPolarity};

/// The producer tag a translated AC7 observation carries. Production
/// paradigm lenses (`alignment_lens::paradigm`) consume observations by
/// this typed provenance tag.
pub fn producer_tag(family_tag: &str) -> String {
    format!("ac7.probe.{family_tag}")
}

/// Translate one typed AC7 observation into a canonical observation
/// about `unit`. Pure; deterministic identity.
pub fn translate(
    obs: LensObservation,
    unit: &SoftwareUnitRef,
    knowledge_basis: &crate::knowledge::BasisHash,
    revision: &str,
) -> SoftwareObservation {
    let stance = match obs.polarity() {
        ObservationPolarity::Supports => ObservationStance::Affirms,
        ObservationPolarity::Contradicts => ObservationStance::Denies,
    };
    SoftwareObservation::declare(
        ObservationSubject::Unit(unit.clone()),
        stance,
        obs.evidence_ref(), // provenance citation only
        ObservationOrigin::DeterministicLocal,
        ObservationBasis::new(revision, knowledge_basis.clone(), "ac7.probe.v1"),
        None, // freshness never fabricated
        producer_tag(obs.family().canonical_tag()),
    )
}

/// Translate a probe output batch into the canonical set (M2 contract:
/// probes feed ONE substrate; there is no parallel AC7 observation DB).
pub fn translate_batch(
    obs: &[LensObservation],
    unit: &SoftwareUnitRef,
    knowledge_basis: &crate::knowledge::BasisHash,
    revision: &str,
    set: &mut ObservationSet,
) {
    for o in obs {
        set.insert(translate(*o, unit, knowledge_basis, revision));
    }
}
