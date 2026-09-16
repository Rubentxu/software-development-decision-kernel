// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/mod.rs — A4-4b: Generic AlignmentLens kernel public surface.
//
// Cycle: `p-63676b11dc0ef88f/a4-4b-alignment-lens-kernel`
// Spec: `.sddk/cycles/.../spec.md` (gitignored)
// ADR:  ADR-0125 (accepted, 2026-09-16).
//        <https://github.com/Rubentxu/software-development-decision-kernel/blob/main/docs/architecture/adrs/ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY.md>
//        The architectural decision recorded there is `AlignmentLens` as
//        a *trait* (documented again in `lens.rs` and in the cycle's
//        `archive-manifest.md`). AC7 paradigm_lens migration is owned
//        by A4-4M (blocked by A4-4bR — Subject-General Evidence Resolution).
//
// # Purpose
//
// This module is the **generic kernel** that owns the shape of "a lens
// evaluating an applicable concern from evidence". It is intentionally
// abstract: the kernel does not know what OO/FP/ADT/DSL means. It only
// knows that an `AlignmentLens` accepts a typed `LensInput` (canonical
// types: `ApplicableConcern`, `ObservationSet`, declared intent refs) and
// returns a typed `LensContribution`. The registry is deterministic.
// The kernel is pure.
//
// ```text
// UniversalConcern
//       +
// ApplicableConcern
//       +
// Evidence / Observations
//       ↓
// AlignmentLensRegistry      (deterministic; no IO; no global mutable state)
//       ↓
// matching lenses             (every candidate lens runs; no first-wins;
//                               no dedup by rendered message)
//       ↓
// AlignmentLensKernel         (pure: input → LensContribution[])
//       ↓
// LensContribution[]          (advisory; identity excludes wall clock,
//                               message, registration order, vector order)
// ```
//
// # Epistemic discipline
//
// The lens kernel **reuses** `observation::EvidenceResolution::{Supported,
// Contradicted, Conflicted, Insufficient}` instead of inventing a parallel
// `LensSupported/LensContradicted/LensConflict/LensUnknown` vocabulary.
// `Insufficient` stays `Insufficient`; `Conflicted` stays `Conflicted`.
// The lens does not synthesize MISALIGNED / Aligned — that is A4-5's job.
//
// A missing lens is **typed as `NotEvaluated`** (a gap), not
// `NotApplicable` and not `InsufficientEvidence`. `ApplicableConcern` is
// a property of declared intent (A4-4a/A4-4aR); `NotEvaluated` is a
// property of the lens registry surface (A4-4b). Keeping them distinct
// is the A4-4aR epistemic pin:
//
// ```text
// NotApplicable != NotEvaluated != Unknown != Ungrounded != InsufficientEvidence
// ```
//
// # Subject-General Evidence (A4-4bR)
//
// A4-4bR generalizes the epistemic algebra from `EvidenceResolution`
// (relation-only) into `EvidencePosture<Target>` parameterized over the
// observed subject (`Relation | Unit | Contract | Knowledge`). The lens
// kernel keeps the four-variant shape and never invents a parallel
// `LensSupported2/LensUnknown2/Aligned2/Misaligned2/Tension2/LensState`
// vocabulary. Lenses consume real targets/relations from the
// `ObservationSet`; the kernel MUST NOT derive a synthetic `RelationId`
// from `(intent_id, concern, unit_ref)`.
//
// # Determinism contract
//
// - Duplicate `LensId` → `Err(LensError::DuplicateLensId)` (NOT replace;
//   divergence from `debverify_kernel::ChallengeStrategySet::register`
//   which replaces — see the doc-comment in `registry.rs` for the
//   rationale).
// - Insertion order has no semantic effect. Lookup is by `LensId` and by
//   `UniversalConcern`; the secondary index is `BTreeMap<UniversalConcern,
//   BTreeSet<LensId>>`, so `for_concern(...)` returns a canonical
//   order regardless of how lenses were registered.
// - No IO during evaluation. No `SystemTime::now`, no filesystem, no
//   network. No global mutable state.
// - Identity excludes wall clock, message text, registration order, and
//   vector insertion order. See `id.rs::derive_contribution_id`.
//
// # Anti-encroachment (load-bearing — see `tests.rs` for pin cases)
//
// - `paradigm_lens/**` is **untouched** during A4-4b. The lens kernel
//   compiles and runs without depending on `paradigm_lens` at all.
// - `software_alignment::reduce_alignment` is **untouched** during A4-4b.
//   The lens kernel produces `LensContribution[]` only; it never
//   constructs `AlignmentAssessment`, `ContractViolation`,
//   `AlignmentTension`, or `ImprovementOpportunity`. (Those belong to
//   A4-3's domain reducer and to A4-5 integration.)
// - The two reference/test lenses (`TestLensA` / `TestLensB`) live
//   inside the `tests` tree (`crates/sddk-engine/tests/
//   alignment_lens_fixture.rs`) and are **never re-exported** through
//   `pub use` below. They are not a production surface; they are public
//   only for tests.
//
// # Boot-time registry
//
// `AlignmentLensRegistry::default()` is an empty registry — A4-4b does
// not register a built-in lens (that would be production strategy, which
// is A4-4M territory). Tests instantiate `TestLensA`/`TestLensB` into
// the registry to demonstrate generic dispatch.
//
// # State classes (per ADR-0095)
//
// All types in this module are **EPHEMERAL** (no durable identity; no
// persistence authority). The only durable artifact they might produce
// is a CLI receipt, which is the CLI's job, not this module's.
//
// # Public surface
//
// - `LensId`, `LensVersion` — typed identity and version of one lens.
// - `LensDescriptor { id, version, supported_concerns }` — what a lens
//   declares about itself.
// - `LensInput` — typed input handed to a lens.
// - `LensContribution` — typed advisory output.
// - `LensContributionId` — content-addressed identity of a contribution.
// - `InsufficientGap` — typed reason for `Insufficient`; replaces the
//   upstream `gap: String` for identity purposes (the upstream string
//   is still produced for serialization, but is derived from the enum).
// - `AlignmentLens` (trait) — the extensibility seam.
// - `AlignmentLensRegistry` — deterministic registry.
// - `AlignmentLensKernel` — pure evaluation entry point.
// - `LensEvaluation` — aggregate answer from the kernel.
// - `NotEvaluated`, `NotEvaluatedReason` — typed gap vocabulary.
// - `LensError`, `KernelError` — typed error vocabulary.

pub mod contribution;
pub mod error;
pub mod id;
pub mod kernel;
pub mod lens;
pub mod not_evaluated;
pub mod paradigm;
pub mod registry;
pub mod types;

pub use paradigm::ParadigmLens;

pub use contribution::LensContribution;
pub use error::{KernelError, LensError};
pub use id::{LensContributionId, derive_contribution_id};
pub use kernel::{AlignmentLensKernel, LensEvaluation};
pub use lens::{AlignmentLens, LensEvaluationOutcome};
pub use not_evaluated::{NotEvaluated, NotEvaluatedReason};
pub use registry::AlignmentLensRegistry;
pub use types::{InsufficientGap, LensDescriptor, LensId, LensInput, LensVersion};

#[cfg(test)]
mod tests;
