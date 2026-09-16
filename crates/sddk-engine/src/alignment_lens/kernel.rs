// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/kernel.rs — A4-4b: pure alignment-lens kernel.
//
// ## What the kernel does
//
// `AlignmentLensKernel::evaluate(&registry, &input)` produces a
// `KernelOutcome`:
//
// - If `input` could not be constructed (e.g. `NotApplicable` was
//   passed), the outcome is `Refused(KernelError::InputRefused(...))`.
// - Otherwise the kernel:
//     1. Looks up every lens whose `descriptor.supported_concerns`
//        contains `input.concern()` (registry lookup is deterministic).
//     2. If the set is empty, produces one `NotEvaluated{ reason:
//        NoRegisteredLens }`.
//     3. Otherwise calls each lens's `evaluate(&input)`. Each lens
//        returns `Contribution(c)` or `Refused(e)`.
//     4. Builds `Vec<LensContribution>` for the contributions. Two
//        `Contribution`s from different lenses are KEPT distinct, even
//        if their `(lens_id, lens_version, concern, observations,
//        evidence_resolution)` would otherwise collide (which they
//        cannot because `lens_id` differs).
//     5. For each lens that returned `Refused`, the kernel checks
//        whether any other lens registered for the concern produced a
//        contribution. If **all** lenses refused, the kernel emits one
//        `NotEvaluated{ reason: LensExistsButRefused }`. Otherwise the
//        successful contributions stand and the refused-lens attempts
//        are silently absorbed (we do not leak `LensError` past the
//        kernel surface, by spec §3: the kernel is advisory-only).
//     6. Asserts determinism post-construction: no duplicate
//        `LensContributionId` inside the produced contributions. A
//        duplicate is a kernel-impossibility probe; if observed, the
//        outcome is `Refused(DeterminismProbeFailed)`. Lenses cannot
//        produce duplicate ids for the same `(lens_id, lens_version,
//        concern, observations, evidence_resolution)` because the
//        content address is built from those fields.
//
// ## Purity
//
// `AlignmentLensKernel` holds only a registry reference (via `&'a
// AlignmentLensRegistry`). It does **not** read wall clock, do IO, or
// read process state. Same `(registry_state, input)` ⟹ same outcome.
// Concrete validation: the integration tests run `evaluate` twice on
// the same input and assert identical contributions and identical ids.

use std::collections::BTreeMap;

use super::contribution::LensContribution;
use super::error::KernelError;
use super::id::derive_contribution_id;
use super::lens::LensEvaluationOutcome;
use super::not_evaluated::{NotEvaluated, NotEvaluatedReason};
use super::registry::AlignmentLensRegistry;
use super::types::LensInput;

/// Aggregate evaluation result: contributions from lenses that ran,
/// plus typed gaps for concerns no lens covered.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LensEvaluation {
    /// Advisory contributions produced by registered lenses, sorted by
    /// `LensContributionId`.
    pub contributions: Vec<LensContribution>,
    /// Typed gaps for concerns no lens was able to evaluate.
    pub gaps: Vec<NotEvaluated>,
}

impl LensEvaluation {
    /// Construct an empty `LensEvaluation`.
    pub fn new() -> Self {
        Self::default()
    }
}

/// What a kernel evaluation produced, distinguishing success from
/// structural refusal.
///
/// On success the caller has both `contributions: Vec<LensContribution>`
/// and `gaps: Vec<NotEvaluated>`: contributions from lenses that DID run,
/// gaps from concerns that had no registered lens (typed `NotEvaluated`,
/// NOT `NotApplicable` and NOT `Insufficient`).
///
/// On refusal the caller has the typed `KernelError`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelOutcome {
    Ok(LensEvaluation),
    Refused(KernelError),
}

impl KernelOutcome {
    /// True iff the outcome is `Ok`.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// Result-like accessor: `Ok` -> `Some(LensEvaluation)`,
    /// `Refused` -> `None`. Lets callers chain `.ok().expect(...)` in tests
    /// and pipelines.
    pub fn ok(self) -> Option<LensEvaluation> {
        match self {
            Self::Ok(eval) => Some(eval),
            Self::Refused(_) => None,
        }
    }

    /// Borrow the contributions (empty if refused).
    pub fn contributions(&self) -> &[LensContribution] {
        match self {
            Self::Ok(eval) => &eval.contributions,
            Self::Refused(_) => &[],
        }
    }

    /// Borrow the gaps (empty if refused).
    pub fn gaps(&self) -> &[NotEvaluated] {
        match self {
            Self::Ok(eval) => &eval.gaps,
            Self::Refused(_) => &[],
        }
    }
}

/// The pure alignment-lens kernel.
///
/// A zero-sized type: construction is just
/// `AlignmentLensKernel` (no fields needed; the registry is passed at
/// evaluation time).
#[derive(Clone, Copy, Debug, Default)]
pub struct AlignmentLensKernel;

impl AlignmentLensKernel {
    /// Construct a kernel.
    pub const fn new() -> Self {
        Self
    }

    /// Evaluate one `LensInput` against a registry.
    ///
    /// This is the **single entry point** of the kernel. It is total:
    /// - Successful evaluations return `KernelOutcome::Ok(LensEvaluation { ... })`.
    /// - Structural refusals return `KernelOutcome::Refused(...)`.
    pub fn evaluate(&self, registry: &AlignmentLensRegistry, input: &LensInput) -> KernelOutcome {
        // Step 1 — gather matching lenses (deterministic).
        let matching = registry.for_concern(input.concern());

        // Step 2 — empty registry slice → gap.
        if matching.is_empty() {
            return KernelOutcome::Ok(LensEvaluation {
                contributions: Vec::new(),
                gaps: vec![NotEvaluated {
                    applicable: input.applicable.clone(),
                    candidate_lens_ids: Vec::new(),
                    reason: NotEvaluatedReason::NoRegisteredLens,
                }],
            });
        }

        // Step 3 — run each lens. Collect contributions and refused ids.
        let mut contributions: Vec<LensContribution> = Vec::new();
        let mut refused_lens_ids: Vec<super::types::LensId> = Vec::new();
        for lens_id in &matching {
            let lens = match registry.lookup(*lens_id) {
                Some(l) => l,
                None => {
                    // This is impossible: `for_concern` returned an id
                    // whose `lookup` is empty. Determinism would be
                    // violated. Treat as a kernel error.
                    return KernelOutcome::Refused(KernelError::DeterminismProbeFailed {
                        duplicate_id_count: 0,
                    });
                }
            };
            match lens.evaluate(input) {
                LensEvaluationOutcome::Contribution(c) => contributions.push(c),
                LensEvaluationOutcome::Refused(_err) => {
                    refused_lens_ids.push(*lens_id);
                }
            }
        }

        // Sort contributions deterministically by `LensContributionId`
        // (sha256 ascending).
        contributions.sort_by(|a, b| a.id.cmp(&b.id));

        // Determinism probe: no duplicate ids.
        let mut seen: BTreeMap<&super::id::LensContributionId, usize> = BTreeMap::new();
        for c in &contributions {
            *seen.entry(&c.id).or_insert(0usize) += 1;
        }
        let dups: usize = seen.values().filter(|&&n| n > 1).count();
        if dups > 0 {
            return KernelOutcome::Refused(KernelError::DeterminismProbeFailed {
                duplicate_id_count: dups,
            });
        }

        // Build gaps: if every lens refused, emit ONE LensExistsButRefused
        // gap. If some succeeded, the refused ones are absorbed silently.
        let gaps: Vec<NotEvaluated> = if contributions.is_empty() && !refused_lens_ids.is_empty() {
            vec![NotEvaluated {
                applicable: input.applicable.clone(),
                candidate_lens_ids: refused_lens_ids,
                reason: NotEvaluatedReason::LensExistsButRefused,
            }]
        } else {
            Vec::new()
        };

        KernelOutcome::Ok(LensEvaluation {
            contributions,
            gaps,
        })
    }
}

/// Helper used by lenses: derive the contribution id from a typed
/// bundle. The kernel calls this on every lens output before
/// assembling the final `LensContribution` (so the lens author
/// doesn't have to hash).
pub fn contribution_id_for(
    lens_id: super::types::LensId,
    lens_version: super::types::LensVersion,
    input: &LensInput,
    resolution: &crate::observation::LensEvidenceResolution,
    evidence_refs: &[crate::evidence_ref::EvidenceRef],
    observation_set_canonical_tag: &str,
) -> super::id::LensContributionId {
    derive_contribution_id(
        lens_id,
        lens_version,
        input.concern(),
        observation_set_canonical_tag,
        resolution,
        evidence_refs,
    )
}
