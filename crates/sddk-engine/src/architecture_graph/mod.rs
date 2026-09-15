// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/mod.rs — A3-S3 / AC2 public surface.
//
// ArchitectureGraphOverlay: typed projection of ArchitecturalContract +
// ArchitectureClaim + SoftwareUnit + DecisionRef/SpecRef/TestRef/UatRef
// inputs into the one canonical SemanticGraphProjection (arch-spec-005).
//
// # State classes (per ADR-0095)
//
// - `ArchitectureGraphOverlay` — **PROJECTION** (delegates to canonical
//   graph; no second persistence authority).
// - `SoftwareUnit`, `SoftwareUnitRef`, `DecisionRef`, `SpecRef`, `TestRef`,
//   `UatRef`, `CompatibilityPathRef` — typed newtypes / PROJECTION inputs
//   (the overlay never owns bytes for these).
// - `ArchitectureOverlayNodeKind`, `ArchitectureOverlayRelationKind`,
//   `ArchitectureOverlayRelation` — closed enums.
// - `ArchitectureClaimId` — typed newtype derived from
//   `(contract_id, outcome, evaluated_at)` (sha256 with domain prefix
//   `sddk.architecture_graph.claim_id.v1|`).
//
// # Submodules
//
// - `types` — overlay kinds, refs, claim identity.
// - `overlay` — the `ArchitectureGraphOverlay` struct, queries, claim
//   attachment.
// - `rebuild` — `rebuild(overlay, inputs)` for delete-and-rebuild equivalence.
// - `tests` — 16 tests covering REQ-AC2-001..022 + AC-033-001..007.

pub mod card;
pub mod overlay;
pub mod rebuild;
pub mod types;

#[cfg(test)]
mod tests;

pub use card::{
    CardDependency, CardKnowledgeStatus, CardProvenance, SoftwareUnitCard, card_for_unit,
};
pub use overlay::{ArchitectureGraphOverlay, Query};
pub use rebuild::{RebuildInputs, rebuild};
pub use types::{
    ArchitectureClaimId, ArchitectureOverlayNodeKind, ArchitectureOverlayRelation,
    ArchitectureOverlayRelationKind, CompatibilityPathRef, OverlayDecisionRef, OverlayNodeRef,
    OverlaySpecRef, SoftwareUnit, SoftwareUnitRef, TestRef, UatRef, UnitKind, build_unit_props,
    contract_overlay_node_id, convert_claim_evidence_to_universal,
};
