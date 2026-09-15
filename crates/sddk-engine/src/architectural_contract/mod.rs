// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/mod.rs — Typed Architectural Contracts and
// Architecture Claims for SDDK.
//
// Cycle: `p-63676b11dc0ef88f/a3-2-architectural-contract` (A3-S2)
// Spec: `docs/architecture/specs/arch-spec-A3-S2-architectural-contracts.md`
//
// # State classes
//
// - [`ArchitecturalContract`] — **OBJECT**: a durable, revisioned, basis-addressed
//   claim about a closed architectural invariant (single authority, forbidden
//   dependency, projection-only, etc.).
// - [`ArchitectureClaim`] — **PROJECTION**: a pure, rebuildable evaluation of
//   an [`ArchitecturalContract`] against a set of [`EvidenceRef`]s at a given
//   point in time. Not durable.
// - [`ContractEvaluation`] / [`ContractEvaluation::evaluate`] — **EPHEMERAL**:
//   pure transformations over inputs, no IO, no clock reads.
// - [`ContractId`], [`ContractKind`], [`ContractPayload`], [`DecisionRef`],
//   [`SpecRef`], [`Revision`], [`ClaimOutcome`], [`EvaluatorRef`],
//   [`EvidenceRef`] — **variant enums and typed newtypes**, exhaustive.
//
// # Determinism
//
// All hash derivations are pure functions of the input bytes plus the declared
// metadata. [`ArchitecturalContract::basis_hash`] reuses the
// [`BasisHash`](crate::knowledge::BasisHash) from A3-S1 with a distinct domain
// prefix (`sddk.architectural_contract.v1\n`) so identity collisions across
// substrates are impossible.
//
// # Anti-encroachment (compile-time pinned as `#[test]`)
//   REQ-A3S2-021: no A4 (`alignment`, `verification`, `governance`) or provider
//   SDK imports allowed in this module.
//   REQ-A3S2-022: no Markdown parsing methods. Markdown is authoring surface,
//   not runtime authority.

#![allow(missing_docs)]

pub(crate) mod claim;
mod contract;
mod error;
mod hashing;
mod payload;
mod types;

#[cfg(test)]
mod tests;

// ─────────────────────────────────────────────────────────────────────────────
// Re-exports (single point of public surface)
// ─────────────────────────────────────────────────────────────────────────────

pub use claim::{ArchitectureClaim, ClaimOutcome, ContractEvaluation, EvaluatorRef, EvidenceRef};
pub use contract::ArchitecturalContract;
pub use error::ContractError;
pub use payload::{BoundaryKind, ContractExtensionValue, ContractKind, ContractPayload};
pub use types::{
    ComponentRef, ContractId, ContractKindRef, DecisionRef, EntityRef, Revision, SpecRef,
};
