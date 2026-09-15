// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_why/mod.rs — A3-S15 public surface.
//
// READ / EXPLAIN over the architecture substrate. Answers "why does SDDK claim
// this about the architecture?" by traversing provenance that AC1–AC5 already
// computed. It performs **no evaluation** and mutates nothing.
//
// Cycle: `p-63676b11dc0ef88f/a3-15-why-architecture` (A3-S15)
// Spec:  `docs/architecture/specs/arch-spec-A3-S15-why-architecture.md`
// ADR:   `docs/architecture/adrs/ADR-0121-ARCHITECTURE-WHY-TRAVERSAL.md`
//
// # Target chain
//
// ```text
// finding → claim assessment → contract → decision/spec → evidence → software unit
// ```
//
// See `types.rs` for the model and `traverse.rs` for what the substrate can and
// cannot supply. It cannot supply `evidence → observes → software relation`, and
// that leg is reported in `ArchitectureWhy::unresolved_edges` rather than inferred.
//
// # Why this is not `graph why`
//
// `sddk graph why` traverses the **event-ledger** reactive graph (runtime facts).
// The architecture substrate is a different projection: the AC2 overlay is rebuilt
// from the declaration on every run (ADR-0120 keeps the declaration as input and
// the AC1 objects as authority), so it is not in that graph and cannot be reached
// from it.

pub mod traverse;
pub mod types;

#[cfg(test)]
mod tests;

pub use traverse::{WhyInput, explain};
pub use types::{
    ArchitectureWhy, WhyAssessment, WhyBasis, WhyContract, WhyEvidence, WhyFinding, WhyIntent,
    WhyNotReason, WhyResolvedAs, WhyUnresolved,
};
