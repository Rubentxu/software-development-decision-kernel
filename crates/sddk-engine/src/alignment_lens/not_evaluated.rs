// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/not_evaluated.rs — A4-4b: typed gap vocabulary.
//
// `NotEvaluated` is **deliberately distinct** from every other gap in
// the substrate. The A4-4aR epistemic pin maps to:
//
// ```text
// NotApplicable           (A4-4a/R reducer; intent-only)
//   != NotEvaluated       (A4-4b lens kernel; registered-lens surface)
//      != Unknown         (a future vocabulary candidate — NOT introduced)
//         != Insufficient (observation substrate; EvidenceResolution)
//            != Ungrounded
//               != InsufficientEvidence
// ```
//
// Concretely: when the kernel runs `evaluate(...)` over an input whose
// `ApplicableConcern::concern()` has **no** registered lens, the result
// is `LensEvaluation { contributions: [], gaps: [NotEvaluated{...}] }`.
// It is **not** an `AlignmentFinding`, **not** an `AlignmentTension`,
// **not** a `LensContribution::evidence_resolution = Insufficient`. It
// is a typed gap that downstream wiring can decide what to do with
// (or leave for A4-5 to handle).

use serde::Serialize;

use crate::alignment_lens::types::LensId;
use crate::intent_universal_concern::{ApplicableConcern, UniversalConcern};

/// Typed gap: an `ApplicableConcern` that no lens is registered to
/// evaluate.
///
/// Constructed only by [`AlignmentLensKernel::evaluate`](super::AlignmentLensKernel::evaluate).
/// Lenses that fail to evaluate produce [`super::LensContribution`]s,
/// not `NotEvaluated`s.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct NotEvaluated {
    /// The applicable concern (with a typed `ApplicableConcern`) that
    /// had no registered lens. We carry the full
    /// `ApplicableConcern` (not just the bare concern) so downstream
    /// wiring can see *why* it was applicable.
    pub applicable: ApplicableConcern,
    /// `LensId`s that were registered but did NOT declare
    /// `supported_concerns.contains(this_concern)`. Typically this is
    /// empty if no lens exists at all, and contains entries only when
    /// other lenses exist but don't cover this concern. Useful for
    /// diagnostics; identity does not depend on it.
    pub candidate_lens_ids: Vec<LensId>,
    /// Typed reason. Closed enum: `NoRegisteredLens` is the only
    /// current variant; future variants may appear (e.g.
    /// `LensExistsButRefused`) without breaking the A4-4b
    /// determinism contract because each enum variant is typed
    /// rather than stringly.
    pub reason: NotEvaluatedReason,
}

/// Typed reason a [`NotEvaluated`] was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotEvaluatedReason {
    /// The registry contained no lens whose
    /// `supported_concerns.contains(applicable.concern())`.
    NoRegisteredLens,
    /// The registry contained only lenses that declared support for
    /// this concern but whose `LensRejected` evaluation short-circuited.
    /// Distinct from "no registered lens" because lenses DO exist for
    /// this concern — they just refused.
    LensExistsButRefused,
}

impl NotEvaluatedReason {
    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::NoRegisteredLens => "no_registered_lens",
            Self::LensExistsButRefused => "lens_exists_but_refused",
        }
    }
}

impl NotEvaluated {
    /// The bare concern this gap is about.
    pub fn concern(&self) -> UniversalConcern {
        self.applicable.concern()
    }
}

impl std::fmt::Display for NotEvaluated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NotEvaluated(concern={}, reason={})",
            self.concern().canonical_tag(),
            self.reason.canonical_tag()
        )
    }
}
