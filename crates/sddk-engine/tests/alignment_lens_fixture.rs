// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/fixtures/alignment_lens/mod.rs
//
// Reference/test lenses for A4-4b. **NOT** part of the engine's public
// API. These lenses exist to demonstrate the kernel's genericity
// across two heterogeneous shapes, per A4-4b spec §2.M9 + §6.
//
// Lens A (DependencyDirection): examines relation-kind-tagged
//   observations on a synthetic `depends_on` relation derived from the
//   input's `(unit_ref, concern)`. Returns `Supported` (Affirms),
//   `Contradicted` (Denies), `Conflicted` (both), or `Insufficient`
//   (none) per stance.
// Lens B (Freshness): examines per-observation `KmtStatus` inside
//   `ObservationBasis` for observations about the same unit. Returns
//   `Supported` if any covering observation has a fresh basis, else
//   `Insufficient` if no covering observation carries a basis at all.
//   Heterogeneity is structural: Lens B reads `basis.freshness`, which
//   Lens A never touches.
//
// Neither lens reaches out of the typed substrate. Both lenses return
// `LensEvaluationOutcome::Contribution(_)` for the happy path; both
// return `Refused(LensError::LensRejected)` when the input's unit_ref
// matches a fenced "rejection" marker (so the integration test can
// exercise the kernel's refusal path).

use std::collections::BTreeSet;

use sddk_engine::alignment_lens::error::LensError;
use sddk_engine::alignment_lens::kernel::contribution_id_for;
use sddk_engine::alignment_lens::lens::{AlignmentLens, LensEvaluationOutcome};
use sddk_engine::alignment_lens::types::{InsufficientGap, LensDescriptor, LensId, LensVersion};
use sddk_engine::alignment_lens::{AlignmentLensRegistry, LensContribution, LensInput};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::intent_universal_concern::UniversalConcern;
use sddk_engine::knowledge::BasisHash;
use sddk_engine::knowledge::evaluate_freshness;
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::types::{
    ObservationBasis, ObservationId, ObservationOrigin, ObservationStance, ObservationSubject,
    RelationId, SoftwareObservation, SoftwareRelation,
};
use sddk_engine::observation::{EvidenceResolution, ObservationSet, SoftwareEntityRef};
use sddk_engine::semantic_kind::CoreRelationKind;

// ─── Helpers (typed, no string parsing) ──────────────────────────────────────

/// Canonical tag for an `ObservationSet` membership, deterministic by
/// `BTreeSet<ObservationId>` (the set already sorts on insert).
pub fn observation_set_canonical_tag(set: &ObservationSet) -> String {
    let mut out = String::from("OBSET|");
    for (i, o) in set.observations().iter().enumerate() {
        if i > 0 {
            out.push('\u{1f}');
        }
        out.push_str(o.id.as_str());
    }
    if set.is_empty() {
        out.push_str("empty");
    }
    out
}

/// Derive the canonical relation id for Lens A. Lens A's relation is
/// `(unit_ref --depends_on--> unit_ref)` — a stable identity bridge
/// from the lens id into the relation substrate. Intentionally
/// different from Lens B's relation.
fn relation_for_lens_a(input: &LensInput) -> RelationId {
    let from = SoftwareEntityRef::Unit(input.unit_ref.clone());
    let to = SoftwareEntityRef::Unit(input.unit_ref.clone());
    let rel = SoftwareRelation::new(from, CoreRelationKind::DependsOn, to);
    rel.id()
}

/// Derive Lens B's relation. Lens B is "freshness-of-unit"; the
/// relation is `(unit_ref --measures_freshness_of--> unit_ref)`.
fn relation_for_lens_b(input: &LensInput) -> RelationId {
    let from = SoftwareEntityRef::Unit(input.unit_ref.clone());
    let to = SoftwareEntityRef::Unit(input.unit_ref.clone());
    let rel = SoftwareRelation::new(from, CoreRelationKind::Verifies, to);
    rel.id()
}

/// Sort and dedup observation ids.
fn sort_ids(ids: &mut Vec<ObservationId>) {
    ids.sort();
    ids.dedup();
}

/// Build a `KmtStatus::Fresh` for tests via the public
/// `evaluate_freshness` constructor (which is the only way external
/// code can obtain a `Fresh` variant — its field is `pub(crate)`).
pub fn kmt_fresh() -> sddk_engine::knowledge::KmtStatus {
    let basis = KnowledgeBasis::empty(EventTime(0));
    evaluate_freshness(&basis, &basis, EventTime(0))
}

/// Build a `KmtStatus::Stale` for tests via `evaluate_freshness` with
/// different `revised_at` (the only way external code can produce a
/// divergent basis from the canonical constructor surface).
pub fn kmt_stale() -> sddk_engine::knowledge::KmtStatus {
    let observed = KnowledgeBasis::empty(EventTime(0));
    let mut expected = KnowledgeBasis::empty(EventTime(0));
    // Revise the expected basis to a strictly greater time -> divergent hash.
    expected = expected.revise(EventTime(1)).expect("future revise");
    evaluate_freshness(&observed, &expected, EventTime(0))
}

/// Build an `ObservationBasis` for tests. `ObservationBasis` itself
/// has no `freshness` field — freshness is part of
/// `SoftwareObservation`, exposed via `SoftwareObservation::declare`.
pub fn basis_for(
    revision: &str,
    knowledge_basis: BasisHash,
    input_digest: &str,
) -> ObservationBasis {
    ObservationBasis {
        revision: revision.to_string(),
        knowledge_basis,
        input_digest: input_digest.to_string(),
    }
}

// ─── Lens A — DependencyDirection ──────────────────────────────────────────

/// Stable lens id for Lens A.
pub const LENS_A_ID: LensId = LensId::new("alignment_lens_fixture::dependency_direction");
/// Stable lens version for Lens A.
pub const LENS_A_VERSION: LensVersion = LensVersion::new(1, 0);

#[derive(Debug, Clone)]
pub struct LensDependencyDirection;

impl AlignmentLens for LensDependencyDirection {
    fn descriptor(&self) -> LensDescriptor {
        let mut concerns = BTreeSet::new();
        concerns.insert(UniversalConcern::DependencyDirection);
        LensDescriptor::new(LENS_A_ID, LENS_A_VERSION, concerns)
    }

    fn evaluate(&self, input: &LensInput) -> LensEvaluationOutcome {
        if input.unit_ref.as_str().starts_with("__reject__") {
            return LensEvaluationOutcome::Refused(LensError::LensRejected {
                id: LENS_A_ID,
                concern: input.concern(),
            });
        }

        let relation = relation_for_lens_a(input);
        let covering = input.observations.for_relation(&relation);
        if covering.is_empty() {
            // Insufficient — no observation covers Lens A's relation.
            let gap = InsufficientGap::NoObservation.wire_message();
            let resolution = EvidenceResolution::Insufficient { relation, gap };
            let obs_tag = observation_set_canonical_tag(&input.observations);
            let id =
                contribution_id_for(LENS_A_ID, LENS_A_VERSION, input, &resolution, &[], &obs_tag);
            return LensEvaluationOutcome::Contribution(LensContribution::assemble(
                id,
                LENS_A_ID,
                LENS_A_VERSION,
                input.concern(),
                resolution,
                Vec::new(),
            ));
        }
        let mut supporting: Vec<ObservationId> = Vec::new();
        let mut contradicting: Vec<ObservationId> = Vec::new();
        let mut refs: Vec<EvidenceRef> = Vec::new();
        for o in &covering {
            match o.stance {
                ObservationStance::Affirms => supporting.push(o.id.clone()),
                ObservationStance::Denies => contradicting.push(o.id.clone()),
            }
            refs.push(o.evidence.clone());
        }
        sort_ids(&mut supporting);
        sort_ids(&mut contradicting);
        let resolution = if !supporting.is_empty() && contradicting.is_empty() {
            EvidenceResolution::Supported {
                relation,
                supporting,
            }
        } else if supporting.is_empty() && !contradicting.is_empty() {
            EvidenceResolution::Contradicted {
                relation,
                contradicting,
            }
        } else if !supporting.is_empty() && !contradicting.is_empty() {
            EvidenceResolution::Conflicted {
                relation,
                supporting,
                contradicting,
            }
        } else {
            EvidenceResolution::Insufficient {
                relation,
                gap: InsufficientGap::ObservationsWithoutStance.wire_message(),
            }
        };
        let obs_tag = observation_set_canonical_tag(&input.observations);
        let id = contribution_id_for(
            LENS_A_ID,
            LENS_A_VERSION,
            input,
            &resolution,
            &refs,
            &obs_tag,
        );
        LensEvaluationOutcome::Contribution(LensContribution::assemble(
            id,
            LENS_A_ID,
            LENS_A_VERSION,
            input.concern(),
            resolution,
            refs,
        ))
    }
}

// ─── Lens B — Freshness ─────────────────────────────────────────────────────

pub const LENS_B_ID: LensId = LensId::new("alignment_lens_fixture::freshness");
pub const LENS_B_VERSION: LensVersion = LensVersion::new(1, 0);

#[derive(Debug, Clone)]
pub struct LensFreshness;

impl AlignmentLens for LensFreshness {
    fn descriptor(&self) -> LensDescriptor {
        let mut concerns = BTreeSet::new();
        concerns.insert(UniversalConcern::Freshness);
        LensDescriptor::new(LENS_B_ID, LENS_B_VERSION, concerns)
    }

    fn evaluate(&self, input: &LensInput) -> LensEvaluationOutcome {
        if input.unit_ref.as_str().starts_with("__reject__") {
            return LensEvaluationOutcome::Refused(LensError::LensRejected {
                id: LENS_B_ID,
                concern: input.concern(),
            });
        }
        let relation = relation_for_lens_b(input);
        let covering = input.observations.for_relation(&relation);
        if covering.is_empty() {
            let gap = InsufficientGap::NoObservation.wire_message();
            let resolution = EvidenceResolution::Insufficient { relation, gap };
            let obs_tag = observation_set_canonical_tag(&input.observations);
            let id =
                contribution_id_for(LENS_B_ID, LENS_B_VERSION, input, &resolution, &[], &obs_tag);
            return LensEvaluationOutcome::Contribution(LensContribution::assemble(
                id,
                LENS_B_ID,
                LENS_B_VERSION,
                input.concern(),
                resolution,
                Vec::new(),
            ));
        }
        // Freshness posture — heterogeneous from Lens A: we look at
        // `basis.freshness: Option<KmtStatus>`. None means
        // "observation exists but its freshness is not evaluated";
        // treat as evidence of *no* measured freshness.
        let mut has_fresh = false;
        let mut has_stale = false;
        let mut refs: Vec<EvidenceRef> = Vec::new();
        for o in &covering {
            refs.push(o.evidence.clone());
            match &o.freshness {
                Some(sddk_engine::knowledge::KmtStatus::Fresh { .. }) => {
                    has_fresh = true;
                }
                Some(sddk_engine::knowledge::KmtStatus::Stale { .. }) => {
                    has_stale = true;
                }
                Some(_) | None => {}
            }
        }
        let supporting: Vec<ObservationId> = covering
            .iter()
            .filter(|o| o.stance == ObservationStance::Affirms)
            .map(|o| o.id.clone())
            .collect();
        let contradicting: Vec<ObservationId> = covering
            .iter()
            .filter(|o| o.stance == ObservationStance::Denies)
            .map(|o| o.id.clone())
            .collect();
        let mut supporting = supporting;
        let mut contradicting = contradicting;
        sort_ids(&mut supporting);
        sort_ids(&mut contradicting);
        let resolution = if has_stale && has_fresh {
            EvidenceResolution::Conflicted {
                relation,
                supporting,
                contradicting,
            }
        } else if has_stale {
            EvidenceResolution::Contradicted {
                relation,
                contradicting,
            }
        } else if has_fresh {
            EvidenceResolution::Supported {
                relation,
                supporting,
            }
        } else {
            EvidenceResolution::Insufficient {
                relation,
                gap: InsufficientGap::MissingProvenance.wire_message(),
            }
        };
        let obs_tag = observation_set_canonical_tag(&input.observations);
        let id = contribution_id_for(
            LENS_B_ID,
            LENS_B_VERSION,
            input,
            &resolution,
            &refs,
            &obs_tag,
        );
        LensEvaluationOutcome::Contribution(LensContribution::assemble(
            id,
            LENS_B_ID,
            LENS_B_VERSION,
            input.concern(),
            resolution,
            refs,
        ))
    }
}

/// Register both lenses into a registry. Pure helper used by tests.
pub fn registry_with_fixtures() -> Result<AlignmentLensRegistry, LensError> {
    AlignmentLensRegistry::new()
        .register(LensDependencyDirection)?
        .register(LensFreshness)
}

// ─── Observation-building helpers ───────────────────────────────────────────

/// Build a typed observation of Lens A's relation with the given
/// stance. Returns the new `ObservationSet` (consumed by tests).
pub fn observation_affirms_a(
    set: &mut ObservationSet,
    basis: ObservationBasis,
    unit: SoftwareUnitRef,
) {
    let relation = SoftwareRelation::new(
        SoftwareEntityRef::Unit(unit.clone()),
        CoreRelationKind::DependsOn,
        SoftwareEntityRef::Unit(unit),
    );
    let subject = ObservationSubject::SoftwareRelation(relation);
    let evidence = EvidenceRef::new(EvidenceKind::Governance, "lens_a_affirms");
    let obs = SoftwareObservation::declare(
        subject,
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "lens_a_fixture",
    );
    set.insert(obs);
}

/// Same as `observation_affirms_a` but stance `Denies`.
pub fn observation_denies_a(
    set: &mut ObservationSet,
    basis: ObservationBasis,
    unit: SoftwareUnitRef,
) {
    let relation = SoftwareRelation::new(
        SoftwareEntityRef::Unit(unit.clone()),
        CoreRelationKind::DependsOn,
        SoftwareEntityRef::Unit(unit),
    );
    let subject = ObservationSubject::SoftwareRelation(relation);
    let evidence = EvidenceRef::new(EvidenceKind::Governance, "lens_a_denies");
    let obs = SoftwareObservation::declare(
        subject,
        ObservationStance::Denies,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "lens_a_fixture",
    );
    set.insert(obs);
}

/// Build a typed observation of Lens B's relation with the given
/// `KmtStatus` (freshness) and a corresponding stance. The status is
/// baked into the observation's basis via the typed constructor (the
/// observation substrate does not expose `with_freshness` as a builder
/// method, so this fixture holds the construction seam).
///
/// For "no freshness" semantics, pass `None`-style: the caller can
/// wrap a `KmtStatus` in `Some(...)` if a non-empty status is desired;
/// the public surface accepts `Option<KmtStatus>` so the test passes
/// `Some(status)` for fresh/stale/invalidated cases and `None` when
/// the observation's basis intentionally has no measured freshness.
pub fn observation_freshness_b(
    set: &mut ObservationSet,
    basis: ObservationBasis,
    unit: SoftwareUnitRef,
    freshness: Option<sddk_engine::knowledge::KmtStatus>,
) {
    let relation = SoftwareRelation::new(
        SoftwareEntityRef::Unit(unit.clone()),
        CoreRelationKind::Verifies,
        SoftwareEntityRef::Unit(unit),
    );
    let subject = ObservationSubject::SoftwareRelation(relation);
    let evidence = EvidenceRef::new(EvidenceKind::Adhoc, "lens_b_freshness");
    let obs = SoftwareObservation::declare(
        subject,
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        freshness,
        "lens_b_fixture",
    );
    set.insert(obs);
}

#[allow(dead_code)]
fn _basis_helper_assert(b: BasisHash) -> BasisHash {
    b
}
