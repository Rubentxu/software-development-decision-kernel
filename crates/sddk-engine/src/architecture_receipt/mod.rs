// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_receipt/mod.rs — A3-S9 / AC8 public surface.
//
// The `ArchitectureConformanceReceipt`: the named Base-mode artefact that
// aggregates the native architecture-conformance capabilities and judges
// whether unresolved MUST findings block a PASS.
//
// Cycle: `p-63676b11dc0ef88f/a3-9-ac8-self-audit-receipt` (A3-S9)
// Spec: `docs/architecture/specs/arch-spec-A3-S9-ac8-self-audit.md`
// Upstream: `arch-spec-041-sddk-architecture-self-audit` (remains `proposed`)
// ADR: `ADR-0119-ARCHITECTURE-CONFORMANCE-RECEIPT`
//
// # What AC8 is
//
// AC8 is an **aggregator + judge**, not a detector. Every historical A0/A1
// class is already reproducible natively:
//
// | Historical class | Native reproducer |
// |---|---|
// | duplicate/shadow authority | AC5 `ShadowAuthority` |
// | dependency boundary | AC5 `AuthorityBypass` |
// | bounded compatibility | AC5 `StaleCompatibility` / `MissingOwner` |
// | projection/authority confusion | AC5 `Contradiction` |
// | missing negative evidence | AC6 mutation suite |
//
// The receipt composes AC2 (graph digest), AC4 (delta), AC5 (audit),
// AC6 (mutations) and AC7 (lenses) into the upstream-defined shape.
//
// # Base mode (AC-041-005)
//
// No CogniCode, no Chronos, no LLM, no provider: `provider_basis` is empty by
// construction and this module imports no provider surface. Enhanced providers
// may add `provider_basis` entries; the verdict rules are unchanged
// (AC-041-006).
//
// # State classes (ADR-0095)
//
// - `ArchitectureConformanceReceipt` — **PROJECTION** (derived, reconstructible,
//   no persistence authority; the module performs no IO).
// - `ReceiptBasis`, `ClassCoverage`, `UnresolvedFinding`, `ClaimResult`,
//   `MutationResult`, `LensResult` — **EPHEMERAL** computed values.
// - `ReceiptVerdict`, `HistoricalClass` — closed enums.
//
// # Submodules
//
// - `types` — the receipt shape and closed vocabularies.
// - `compose` — `compose_receipt` + verdict rules.
// - `self_audit` — historical-class coverage.
// - `tests` — acceptance (AC-UAT-016) + anti-encroachment pins.

pub mod compose;
pub mod self_audit;
pub mod types;

#[cfg(test)]
mod tests;

pub use compose::{ReceiptInputs, compose_receipt};
pub use self_audit::evaluate_class_coverage;
pub use types::{
    ArchitectureConformanceReceipt, ChangeBasis, ClaimResult, ClassCoverage, CompatibilityEntry,
    HistoricalClass, LensResult, MutationResult, ReceiptBasis, ReceiptId, ReceiptVerdict,
    UnresolvedFinding,
};
