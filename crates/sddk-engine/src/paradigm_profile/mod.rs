// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_profile/mod.rs — A3-S4 / AC3 public surface.
//
// ParadigmProfileOverlay: typed projection of ParadigmProfileKind +
// ParadigmLensKind + LensAssessment inputs into the one canonical
// SemanticGraphProjection (arch-spec-005, ADR-0098).
//
// # State classes (per ADR-0095)
//
// - `ParadigmProfileOverlay` — **PROJECTION** (delegates to canonical graph;
//   no second persistence authority).
// - `ProjectIntentRef`, `BoundedContextRef`, `SoftwareUnitRef` — typed
//   newtypes (no persistence).
// - `ParadigmProfileKind`, `ParadigmLensKind`, `LensStatus`, `EvidenceBasis`,
//   `ParadigmAnchorKind`, `ParadigmOverlayRelationKind` — closed enums.
// - `LensAssessment`, `ParadigmProfileEdge`, `ParadigmLensEdge` — data shapes.
//
// # Anti-encroachment (REQ-AC3-020..023)
//
// This module does NOT `use` any of:
//   - alignment / verify / debverify (A4 surfaces)
//   - authority_engine / capability / EffectiveInstructions (A2 surfaces)
//   - provider / host_sdk / agent_host (A7 surfaces)
//   - architecture_graph type reexports (shared substrate only via the canonical
//     SemanticGraphProjection)
//
// AC3 is **data-only**: assessment status defaults to `LensStatus::Unknown`
// with `EvidenceBasis::Declared`; lens evaluation belongs to AC7.
//
// # Submodules
//
// - `types` — overlay kinds, refs, edge/assessment data shapes.
// - `overlay` — the `ParadigmProfileOverlay` struct, query ADT.
// - `rebuild` — `rebuild(overlay, inputs)` for delete-and-rebuild equivalence.
// - `tests` — acceptance + anti-encroachment pins.

pub mod overlay;
pub mod rebuild;
pub mod types;

#[cfg(test)]
mod tests;

pub use overlay::{ParadigmProfileOverlay, Query, QueryResult};
pub use rebuild::rebuild;
pub use types::{
    BoundedContextRef, EvidenceBasis, LensAssessment, LensAssessmentId, LensStatus,
    ParadigmAnchorKind, ParadigmAnchorRef, ParadigmLensEdge, ParadigmLensKind,
    ParadigmOverlayRelationKind, ParadigmProfileEdge, ParadigmProfileKind, ProjectIntentRef,
    RebuildInputs, SoftwareUnitRef,
};
