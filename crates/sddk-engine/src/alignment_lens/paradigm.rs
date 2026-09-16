// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/paradigm.rs — A4-4M M1: production AC7-family lenses.
//
// Cycle: `p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`
// Matrix: docs/architecture/a4-4m-migration-matrix.md (M0 GO)
// ADR:    ADR-0125 (amended for A4-4bR/M)
//
// # What this module is
//
// The four production `AlignmentLens` implementations that replace the
// AC7 `paradigm_lens` evaluation motor (A3-S8): ObjectOriented,
// Functional, Adt, Dsl. Every lens implements the SAME
// `alignment_lens::AlignmentLens` trait. There are no sub-traits per
// paradigm and no parallel registry (M1/M3 contract).
//
// # How a lens decides
//
// Each lens consumes the **typed** `SoftwareObservation`s already
// projected into the canonical `ObservationSet` (M2 contract: the AC7
// probes keep producing typed observations; their output lands in the
// canonical substrate via the M0 translation — see
// `paradigm_lens::translation`). A lens:
//
// 1. reads the observations covering `input.unit_ref` (typed
//    `ObservationSubject::Unit` — never a synthetic relation),
// 2. filters by its family's dimension tags carried on the typed
//    observation provenance (`producer` = `ac7.probe.<family>` and the
//    evidence locator = the static per-variant tag written by the
//    translation, never parsed for semantics — the polarity rides the
//    typed `ObservationStance`),
// 3. resolves the posture with the kernel substrate
//    (`observation::resolve_subject`) and assembles ONE
//    `LensContribution` per supported concern.
//
// A lens with no family observations for the unit emits the honest
// `Insufficient` posture (identity-stable); the legacy `Unknown` is a
// pure projection of it (M0.3 pins). The lens NEVER emits
// `NotApplicable`, alignment state, scores, or confidence.
//
// # Concern mapping
//
// The per-variant `UniversalConcern` mapping is the one pinned by
// `tests/a4_4m_m0_migration_proof.rs` (matrix §3). A lens emits one
// contribution per (concern, family observations) pair, all carrying
// the same evidence resolution — the concerns are views over the same
// posture, not independent judgments.

use std::collections::BTreeSet;

use crate::alignment_lens::contribution::LensContribution;
use crate::alignment_lens::error::LensError;
use crate::alignment_lens::id::derive_contribution_id;
use crate::alignment_lens::lens::{AlignmentLens, LensEvaluationOutcome};
use crate::alignment_lens::types::{LensDescriptor, LensId, LensInput, LensVersion};
use crate::evidence_ref::EvidenceRef;
use crate::intent_universal_concern::UniversalConcern;
use crate::observation::posture::{EvidencePosture, ObservationTargetRef};
use crate::observation::types::{ObservationStance, ObservationSubject};

/// Stable lens ids (content-only; never renumbered).
pub mod ids {
    use super::LensId;
    pub const OBJECT_ORIENTED: LensId = LensId::new("alignment_lens::paradigm::object_oriented");
    pub const FUNCTIONAL: LensId = LensId::new("alignment_lens::paradigm::functional");
    pub const ADT: LensId = LensId::new("alignment_lens::paradigm::adt");
    pub const DSL: LensId = LensId::new("alignment_lens::paradigm::dsl");
}

/// Lens versions: one bump per family stays possible; all start 1.0.
mod versions {
    use super::LensVersion;
    pub const V1: LensVersion = LensVersion::new(1, 0);
}

/// The producer tag prefix written by the M0 translation for AC7 probe
/// observations. A production lens recognizes its family's observations
/// by this tag (typed provenance), never by parsing locators for
/// meaning.
pub const PRODUCER_PREFIX: &str = "ac7.probe.";

/// Shared implementation: one struct per family, one code path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParadigmLens {
    id: LensId,
    family_tag: &'static str,
    concerns: &'static [UniversalConcern],
}

impl ParadigmLens {
    /// The four production lenses, in canonical family order.
    pub const ALL: [ParadigmLens; 4] = [
        ParadigmLens {
            id: ids::OBJECT_ORIENTED,
            family_tag: "object_oriented",
            concerns: &[
                UniversalConcern::BoundaryIntegrity,
                UniversalConcern::StateSafety,
                UniversalConcern::DependencyDirection,
                UniversalConcern::SemanticOwnership,
                UniversalConcern::Cohesion,
                UniversalConcern::Coupling,
            ],
        },
        ParadigmLens {
            id: ids::FUNCTIONAL,
            family_tag: "functional",
            concerns: &[
                UniversalConcern::StateSafety,
                UniversalConcern::EffectVisibility,
                UniversalConcern::SemanticOwnership,
            ],
        },
        ParadigmLens {
            id: ids::ADT,
            family_tag: "adt",
            concerns: &[
                UniversalConcern::StateSafety,
                UniversalConcern::SemanticOwnership,
            ],
        },
        ParadigmLens {
            id: ids::DSL,
            family_tag: "dsl",
            concerns: &[
                UniversalConcern::BoundaryIntegrity,
                UniversalConcern::EffectVisibility,
                UniversalConcern::StateSafety,
                UniversalConcern::TemporalCoupling,
            ],
        },
    ];

    /// The `ac7.probe.<family>` producer tag this lens consumes.
    pub fn producer_tag(&self) -> String {
        format!("{PRODUCER_PREFIX}{}", self.family_tag)
    }
}

impl AlignmentLens for ParadigmLens {
    fn descriptor(&self) -> LensDescriptor {
        let mut concerns = BTreeSet::new();
        for c in self.concerns {
            concerns.insert(*c);
        }
        LensDescriptor::new(self.id, versions::V1, concerns)
    }

    fn evaluate(&self, input: &LensInput) -> LensEvaluationOutcome {
        // Typed, family-scoped observations about THIS unit only.
        let producer = self.producer_tag();
        let family_obs: Vec<_> = input
            .observations
            .for_subject(&ObservationTargetRef::Unit(input.unit_ref.clone()))
            .into_iter()
            .filter(|o| {
                matches!(o.subject, ObservationSubject::Unit(ref u) if u == &input.unit_ref)
                    && o.producer == producer
            })
            .collect();

        // Posture over the family's observations (supporting vs
        // contradicting by TYPED stance).
        let target = ObservationTargetRef::Unit(input.unit_ref.clone());
        let resolution = if family_obs.is_empty() {
            EvidencePosture::Insufficient {
                target,
                gap: crate::observation::posture::InsufficientGap::NoObservation,
            }
        } else {
            let supporting: Vec<_> = family_obs
                .iter()
                .filter(|o| o.stance == ObservationStance::Affirms)
                .map(|o| o.id.clone())
                .collect();
            let contradicting: Vec<_> = family_obs
                .iter()
                .filter(|o| o.stance == ObservationStance::Denies)
                .map(|o| o.id.clone())
                .collect();
            match (supporting.is_empty(), contradicting.is_empty()) {
                (false, true) => EvidencePosture::Supported { target, supporting },
                (true, false) => EvidencePosture::Contradicted {
                    target,
                    contradicting,
                },
                // Both present: preserve BOTH sets (A4-4b pin).
                (false, false) => EvidencePosture::Conflicted {
                    target,
                    supporting,
                    contradicting,
                },
                // Both empty cannot happen: family_obs is non-empty and
                // every stance is one of the two closed variants.
                (true, true) => unreachable!("non-empty observations with no stance"),
            }
        };

        // Evidence refs = the observations the lens actually depended on.
        let evidence_refs: Vec<EvidenceRef> =
            family_obs.iter().map(|o| o.evidence.clone()).collect();

        // One contribution per supported concern — views over the same
        // posture. Identity is content-addressed per (lens, concern,
        // set, posture), so distinct concerns yield distinct ids while
        // remaining deterministic.
        let set_digest = input.observations.canonical_digest();
        let mut out: Option<LensContribution> = None;
        for concern in self.concerns {
            let id = derive_contribution_id(
                self.id,
                versions::V1,
                *concern,
                &set_digest,
                &resolution,
                &evidence_refs,
            );
            out = Some(LensContribution::assemble(
                id,
                self.id,
                versions::V1,
                *concern,
                resolution.clone(),
                evidence_refs.clone(),
            ));
        }

        match out {
            Some(c) => LensEvaluationOutcome::Contribution(c),
            // Unreachable: `concerns` is a non-empty static slice.
            None => LensEvaluationOutcome::Refused(LensError::LensRejected {
                id: self.id,
                concern: *self.concerns.first().expect("non-empty concern list"),
            }),
        }
    }
}

#[cfg(test)]
mod tests;
