// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/lens.rs — A4-4b: the `AlignmentLens` trait.
//
// ## Why a trait, not an ADT (recorded design decision)
//
// A4-4b's only architectural decision (spec §2.M13 + this header) is
// the choice of `trait AlignmentLens` over a closed ADT + explicit
// dispatch. Both were considered. The trait was chosen because:
//
// 1. **Built-in lenses (A4-4M)** and **packs** (future, A4+) need to
//    be registered without kernel churn. A trait + a `&dyn`-compatible
//    layout gets us that for free. An ADT would have required a
//    `Box<dyn Strategy>` inside the enum anyway, at which point the
//    trait is the leaner vocabulary.
//
// 2. **Type-system lock against score/confidence.** A trait method
//    `fn evaluate(&self, &LensInput) -> Result<LensContribution, LensError>`
//    fixes the contribution shape at the return type. Adding score or
//    confidence to `LensContribution` later is a breaking change,
//    which makes the "no score / no confidence" promise structural
//    rather than policy.
//
// 3. **Provider-backed observations (post-A4-4b).** Provider adapters
//    (CogniCode, Chronos, runtime probes) will produce lenses. A trait
//    is the natural Rust idiom for that; an ADT would force every
//    new adapter to live in the engine crate.
//
// The trait is **object-safe** (`&dyn AlignmentLens` works). It has
// no default methods. Sub-traits are forbidden during A4-4b so the
// registry surface stays single-level.
//
// ## Contract
//
// - `descriptor()` returns a stable, content-only description. Lenses
//   MUST NOT include wall clock, addresses, or message text here.
// - `evaluate()` is **pure**: same input → same contribution (modulo
//   identity-based sorting of `evidence_refs`). The kernel validates
//   this property at registration time by construction (lenses cannot
//   compare against hidden state if their struct is `pub`). It also
//   asserts it post-evaluation by detecting duplicate `LensContributionId`s.
// - Lenses MUST NOT touch filesystem, clock, or network.
// - Lenses MUST NOT construct any `AlignmentAssessment`,
//   `ContractViolation`, `AlignmentTension`, or `ImprovementOpportunity`.

use std::fmt::Debug;

use crate::alignment_lens::contribution::LensContribution;
use crate::alignment_lens::error::LensError;
use crate::alignment_lens::types::{LensDescriptor, LensId, LensInput, LensVersion};
use crate::intent_universal_concern::UniversalConcern;

/// The result a lens hands back to the kernel.
///
/// Splitting `Contribution` from `Refused` gives the kernel a closed
/// vocabulary for what happened — there is no ambiguous "evaluation
/// returned a free-form error string". `LensError` itself is also
/// closed (see `error.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensEvaluationOutcome {
    /// The lens produced one advisory contribution.
    Contribution(LensContribution),
    /// The lens refuses to evaluate this input. The kernel attaches
    /// the lens id and moves on.
    Refused(LensError),
}

/// The extensibility seam for the lens kernel.
///
/// Implemented by every concrete `AlignmentLens`. Trait methods are
/// object-safe; lenses are stored as `Arc<dyn AlignmentLens>` in the
/// registry.
pub trait AlignmentLens: Debug + Send + Sync {
    /// Stable, content-only descriptor of the lens.
    fn descriptor(&self) -> LensDescriptor;

    /// Evaluate the input and return one of
    /// [`LensEvaluationOutcome::Contribution`] or
    /// [`LensEvaluationOutcome::Refused`].
    ///
    /// MUST be pure. MUST NOT touch filesystem, clock, or network.
    /// MUST NOT construct any A4-3 / A4-5 alignment state.
    fn evaluate(&self, input: &LensInput) -> LensEvaluationOutcome;
}

/// Convenience accessors so callers don't have to reach into the
/// descriptor when they only need one field.
pub trait AlignmentLensExt: AlignmentLens {
    /// The lens's stable identifier.
    fn id(&self) -> LensId {
        self.descriptor().id
    }
    /// The lens's stable semantic version.
    fn version(&self) -> LensVersion {
        self.descriptor().version
    }
    /// True iff this lens declares support for `c`.
    fn supports(&self, c: UniversalConcern) -> bool {
        self.descriptor().supports(c)
    }
}

impl<T: AlignmentLens + ?Sized> AlignmentLensExt for T {}
