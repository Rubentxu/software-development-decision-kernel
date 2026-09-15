// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_conformance/mod.rs — A3-S5 / AC4 public surface.
//
// Verify contracts: map a change basis (changed software units) to affected
// architectural contracts, derive the minimum deterministic probe plan, and
// produce an evidence-backed `ArchitectureConformanceDelta`.
//
// Cycle: `p-63676b11dc0ef88f/a3-5-ac4-verify-contracts` (A3-S5)
// Spec: `docs/architecture/specs/arch-spec-A3-S5-ac4-verify-contracts.md`
// Upstream: `arch-spec-034-architecture-conformance-verification` (proposed)
//
// # State classes (per ADR-0095)
//
// - `ArchitectureConformanceDelta` — **PROJECTION**: derived, reconstructible
//   from (graph revision, contract set, evidence, now). No durable identity;
//   no persistence authority.
// - `AffectedContract`, `ProbeRequirement`, `ConformanceVector`,
//   `VectorStatus`, `DeltaContractStatus` — **EPHEMERAL** computation values.
// - `ArchitecturalContract` (AC1) — **OBJECT**; AC4 reads, never writes.
// - `ArchitectureClaim` (AC1) — **PROJECTION**; AC4 reads, never constructs.
//
// # Determinism (REQ-AC4-012/013/020/021)
//
// Two `compute_conformance_delta` calls over identical inputs yield identical
// `plan_digest`, `contract_set_digest`, `graph_digest`, ordering and key sets.
//
// # Anti-encroachment (REQ-AC4-026..028)
//
// This module does NOT `use` any of:
//   - `paradigm_profile` (AC3 lens evaluation is AC7)
//   - `alignment` / `debverify` (AC5 surfaces)
//   - `provider` / `host_sdk` / `agent_host` (AC-034-006)
//   - `effective_instructions` / `capability` (A2 surfaces)
//
// It NEVER mutates the semantic graph (read-only consumer of AC2) and NEVER
// constructs an `ArchitectureClaim` directly (always routes through
// `ContractEvaluation::evaluate`). No numeric aggregate score exists
// (AC-034-008, AC-UAT-043).
//
// # Submodules
//
// - `types` — closed vocabularies and data shapes.
// - `compute` — the pure `compute_conformance_delta` function and digests.
// - `tests` — acceptance + anti-encroachment pins.

pub mod compute;
pub mod types;

#[cfg(test)]
mod tests;

pub use compute::{
    AC4_EVALUATOR, CONTRACT_SET_DIGEST_DOMAIN, PLAN_DIGEST_DOMAIN, compute_conformance_delta,
    contract_set_digest,
};
pub use types::{
    AffectedContract, ArchitectureConformanceDelta, ArchitectureConformanceDeltaId,
    ConformanceError, ConformanceInputs, ConformanceVector, ContractEvidence, DeltaContractStatus,
    ProbeRequirement, VectorDimension, VectorStatus,
};
