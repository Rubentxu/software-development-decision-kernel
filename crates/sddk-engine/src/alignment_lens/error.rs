// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/error.rs — A4-4b: typed error vocabulary.
//
// Two error families, both closed:
// - `LensError` — registration / input-shape refusals (structurally
//   typed; never free-form strings).
// - `KernelError` — kernel-level refusals (per-lens evaluation failures,
//   determinism violations, etc.).

use serde::Serialize;

use crate::alignment_lens::types::{LensId, LensVersion};
use crate::intent_universal_concern::{NotApplicableReason, UniversalConcern};

// ─── LensError ──────────────────────────────────────────────────────────────

/// Typed errors raised by the lens kernel and registry.
///
/// All variants are enums of typed data; there is **no** free-form
/// `String`. This keeps the error surface content-stable and
/// serializable. Display impls derive only from the typed data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum LensError {
    /// A `LensId` was registered twice (same id, version ignored).
    DuplicateLensId(LensId),
    /// A `LensId` was re-registered with a different `LensVersion`.
    /// First registration wins; the kernel refuses silent supersession.
    InconsistentLensVersion {
        id: LensId,
        first: LensVersion,
        subsequent: LensVersion,
    },
    /// A lens's `supported_concerns` set disagrees with the version
    /// already registered under the same `LensId`.
    InconsistentSupportedConcerns {
        id: LensId,
        first: u32,
        subsequent: u32,
    },
    /// A `LensInput` was constructed with `Applicable(c)` but the
    /// observer passed a `NotApplicable(c, _)` — applicability-layer
    /// refusal. **Not** the same as a missing lens (that is
    /// `NotEvaluated`, not an error).
    NotApplicableInput {
        concern: UniversalConcern,
        reason: NotApplicableReason,
    },
    /// A lens rejected its own input (e.g. asked to evaluate a concern
    /// it does not declare support for). The lens author typed a
    /// refusal.
    LensRejected {
        id: LensId,
        concern: UniversalConcern,
    },
}

impl std::fmt::Display for LensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateLensId(id) => {
                write!(f, "alignment_lens: duplicate LensId registered: {}", id)
            }
            Self::InconsistentLensVersion {
                id,
                first,
                subsequent,
            } => write!(
                f,
                "alignment_lens: LensId {} re-registered; first version was {}, got {}",
                id, first, subsequent
            ),
            Self::InconsistentSupportedConcerns {
                id,
                first,
                subsequent,
            } => write!(
                f,
                "alignment_lens: LensId {} re-registered; first supported_concerns had {} entries, got {}",
                id, first, subsequent
            ),
            Self::NotApplicableInput { concern, reason } => write!(
                f,
                "alignment_lens: LensInput requires Applicable(...), got NotApplicable({}, {})",
                concern.canonical_tag(),
                reason.canonical_tag()
            ),
            Self::LensRejected { id, concern } => write!(
                f,
                "alignment_lens: lens {} refused to evaluate concern {}",
                id,
                concern.canonical_tag()
            ),
        }
    }
}

impl std::error::Error for LensError {}

// ─── KernelError ────────────────────────────────────────────────────────────

/// Typed errors raised by [`AlignmentLensKernel::evaluate`](super::AlignmentLensKernel::evaluate).
///
/// The kernel is total: it produces an aggregate `KernelOutcome`
/// regardless of individual lens outcomes. `KernelError` is reserved
/// for kernel-level (not lens-level) failures: input-shape refusals
/// not already covered by `LensError`, or determinism-violations
/// detected post-evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum KernelError {
    /// An input failed to construct before reaching the kernel.
    InputRefused(LensError),
    /// Post-evaluation determinism probe failed (e.g. duplicate
    /// `LensContributionId` inside one evaluation result). This should
    /// be impossible if lenses are pure; the kernel asserts it.
    DeterminismProbeFailed { duplicate_id_count: usize },
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputRefused(e) => write!(f, "alignment_lens: kernel input refused: {}", e),
            Self::DeterminismProbeFailed { duplicate_id_count } => write!(
                f,
                "alignment_lens: {} duplicate LensContributionId values observed inside one evaluation; kernel purity invariant violated",
                duplicate_id_count
            ),
        }
    }
}

impl std::error::Error for KernelError {}
