// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/contribution.rs — A4-4b: advisory output of one lens.
//
// `LensContribution` is **advisory** (not final): it carries the lens's
// typed answer to "what does the evidence say about this concern?",
// not an `AlignmentFinding` or any wider A4-3 / A4-5 vocabulary.
//
// Constraints pinned by spec §3.X:
// - **No score.** **No confidence.** **No priority.** **No weight.**
// - **No `AlignmentState`.** **No `Capability`.** **No
//   `AuthorityDecision`.** **No `InstructionSource`.**
// - **No `AlignmentAssessment` / `ContractViolation` /
//   `AlignmentTension` / `ImprovementOpportunity`** (those are A4-3 /
//   A4-5 territory).
// - **Identity excludes wall clock, message text, registration
//   order, vector insertion order.** See `id.rs`.
//
// `LensContribution` is **construct-only**. The kernel calls the lens's
// `evaluate` method, the lens returns one of these via
// [`LensEvaluationOutcome::Contribution`], the kernel attaches the
// derived id, and the result is read-only for the rest of its life.

use serde::Serialize;

use crate::evidence_ref::EvidenceRef;
use crate::observation::EvidenceResolution;

use super::id::LensContributionId;
use super::types::{LensId, LensVersion};
use crate::intent_universal_concern::UniversalConcern;

/// Provenance tag for a contribution: who/what produced it (a tool
/// name, an analyzer id, a human id) and **only** content-stable data.
///
/// A `LensContribution`'s provenance is intentionally distinct from
/// `ObservationOrigin` (used inside the observation substrate): an
/// observation's origin is "how was this observed?", while the
/// contribution's provenance is "what lens emitted this contribution?"
/// The former answers an empirical question; the latter answers a
/// model-question.
///
/// Format is a `String` because analyzer identifiers in production are
/// not yet a closed enum; this may be tightened in A4-5 once the
/// analyzer-shape surface is closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ProvenanceTag {
    /// Stable identifier of the lens that emitted the contribution.
    pub lens_id: LensId,
    /// Stable identifier of the lens version that emitted the
    /// contribution.
    pub lens_version: LensVersion,
}

impl ProvenanceTag {
    /// Construct a provenance tag.
    pub fn new(lens_id: LensId, lens_version: LensVersion) -> Self {
        Self {
            lens_id,
            lens_version,
        }
    }
}

/// One lens's advisory answer to "what does the evidence say about
/// this concern?".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LensContribution {
    /// Identity, content-addressed from identity-defining inputs.
    pub id: LensContributionId,
    /// Stable identifier of the lens.
    pub lens_id: LensId,
    /// Stable semantic version of the lens.
    pub lens_version: LensVersion,
    /// The concern being evaluated.
    pub concern: UniversalConcern,
    /// The evidence posture, **reused** from the canonical
    /// `EvidenceResolution` (A4-0 substrate). Do not introduce a
    /// parallel vocabulary.
    pub evidence_resolution: EvidenceResolution,
    /// Semantic references the lens claims it consulted. Sorted by
    /// [`EvidenceRef::ordering_key`] before being attached (the kernel
    /// does this; lenses passing unsorted refs are accepted, sorted,
    /// and the sorted version is hashed into identity). Empty means
    /// "the lens reasoned without naming an evidence ref", which is
    /// legal (e.g. sufficiency-from-shape reasoning); identity handles
    /// this case.
    pub evidence_refs: Vec<EvidenceRef>,
    /// Provenance tag.
    pub provenance: ProvenanceTag,
}

impl LensContribution {
    /// Construct a contribution. The kernel wraps lens output in this;
    /// do **not** call from lenses (it bypasses the sorted-refs
    /// invariant).
    ///
    /// `evidence_refs` is sorted by [`EvidenceRef::ordering_key`] in
    /// place before the contribution is returned. The caller must
    /// supply the corresponding `observation_set_canonical_tag` (the
    /// kernel computes this and passes it in).
    #[allow(clippy::too_many_arguments)]
    pub fn assemble(
        id: LensContributionId,
        lens_id: LensId,
        lens_version: LensVersion,
        concern: UniversalConcern,
        evidence_resolution: EvidenceResolution,
        mut evidence_refs: Vec<EvidenceRef>,
    ) -> Self {
        // Sort refs for identity determinism.
        evidence_refs.sort();
        Self {
            id,
            lens_id,
            lens_version,
            concern,
            evidence_resolution,
            evidence_refs,
            provenance: ProvenanceTag::new(lens_id, lens_version),
        }
    }
}
