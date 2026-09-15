// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_mutation/mod.rs — A3-S6 / AC6 public surface.
//
// Critical mutation probes: inject a bounded architectural violation into a
// disposable in-memory sandbox and require the expected guard to detect it.
//
// Cycle: `p-63676b11dc0ef88f/a3-6-ac6-mutation-probes` (A3-S6)
// Spec: `docs/architecture/specs/arch-spec-A3-S6-ac6-mutation-probes.md`
// Upstream: `arch-spec-038-architecture-mutation-and-counterfactual-probes`
//           (remains `proposed`); counterfactual parts are AC13.
//
// # State classes (per ADR-0095)
//
// - `MutationSandbox` — **EPHEMERAL**: an in-memory disposable copy. Never
//   persisted; no IO.
// - `MutationProbe`, `MutationSuiteReceipt`, `GuardHit`, `MutationSpec`,
//   `MutationGuard` — **EPHEMERAL** computed values.
// - `MutationKind`, `GuardCheck`, `MutationInjection`, `GuardId`,
//   `MutationId` — closed enums / typed newtypes.
//
// # Safety (AC-038-001)
//
// Mutations never land in the working tree. The runner clones the supplied
// sandbox, injects into the clone, evaluates the guard against the clone and
// discards it. This module performs no filesystem IO.
//
// # Anti-encroachment (REQ-AC6-019/020)
//
// This module does NOT `use`:
//   - `paradigm_profile` (AC3)
//   - `alignment` / `debverify` (AC5)
//   - `provider` / `host_sdk` / `agent_host`
//   - `effective_instructions` / `capability`
// It does NOT construct `ArchitectureClaim`, grants no capability, and uses no
// `std::fs` / `std::io::Write` / `File`.
//
// # AC4 seam
//
// `MutationSuiteReceipt::witnesses()` yields the contract ids of detected
// probes, directly consumable by AC4's
// `ConformanceInputs.contradiction_witnesses`.
//
// # Submodules
//
// - `types` — closed vocabularies and data shapes.
// - `sandbox` — the disposable sandbox + guard evaluation.
// - `run` — probe/suite runners and the four critical mutations.
// - `tests` — acceptance + anti-encroachment + boundary pins.

pub mod run;
pub mod sandbox;
pub mod types;

#[cfg(test)]
mod tests;

pub use run::{
    SUITE_DIGEST_DOMAIN, critical_guards, critical_mutations, run_critical_mutations,
    run_mutation_probe, run_mutation_suite, suite_digest,
};
pub use sandbox::{MutationSandbox, apply_injection, evaluate_guard};
pub use types::{
    GuardCheck, GuardHit, GuardId, GuardScope, MutationError, MutationGuard, MutationId,
    MutationInjection, MutationKind, MutationProbe, MutationSpec, MutationSuiteReceipt,
};
