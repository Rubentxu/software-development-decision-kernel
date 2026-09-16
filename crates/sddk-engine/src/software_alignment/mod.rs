// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// software_alignment/mod.rs — A4-3: Software Alignment domain.
//
// Implements arch-spec-045. The domain is **advisory** and
// **epistemologically honest**: it computes a delta between an
// ArchitecturalIntentSnapshot and the observed reality, and reports
// the result in a closed state vocabulary. There is no operational
// authority in this module.
//
// State classes (per ADR-0095):
//   - `AlignmentAssessment`              — PROJECTION (derived from inputs).
//   - `ArchitecturalIntentSnapshot`      — INPUT     (read by the reducer).
//   - `ArchitecturalContract`            — OBJECT    (read-only via AC1).
//   - `AcceptedDecision`                 — INPUT     (read by the reducer).
//   - `KnowledgeBasis`                   — INPUT     (identity only).

pub mod reducer;
pub mod types;

#[cfg(test)]
mod tests;

pub use reducer::{ReductionError, reduce_alignment};
pub use types::{
    AcceptedDecision, AlignmentAssessment, AlignmentAssessmentId, AlignmentFinding,
    AlignmentFindingKind, AlignmentScope, AlignmentState, ArchitecturalIntentSnapshot,
    ContradictionMarker, FindingLocation, MustDirection, RevisitTrigger,
};
