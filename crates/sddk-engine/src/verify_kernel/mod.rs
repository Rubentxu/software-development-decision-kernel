// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/mod.rs — A4-1: Generic Verify kernel public surface.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
// ADR: `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md`
//
// # Public surface
//
// - `VerificationResult` — closed 5-variant result vocabulary.
// - `VerificationClaim` — closed claim taxonomy.
// - `VerifyKernel::evaluate` — single entry point.
// - `VerificationDomain` trait — domain specialization.
// - `VerifyDomainRegistry` — boot-time domain lookup.
// - `EvidenceSource` — thin shared substrate trait.
// - `ArchitectureVerificationDomain` — only specialization in this delivery.
//
// # Boot-time domain registry
//
// `default_registry() -> VerifyDomainRegistry` registers all known domains.
// Currently only `ArchitectureVerificationDomain` is registered.
//
// # State classes (per ADR-0095)
//
// All types in this module are **EPHEMERAL** (no durable identity, no
// persistence authority). The only durable artifact is the CLI's receipt
// output, which is produced by the CLI adapter (not this module).

pub mod adapter_architecture;
pub mod adapter_runtime_provider;
pub mod adapter_static_provider;
pub mod engine;
pub mod evidence_source;
pub mod registry;
pub mod tests;
pub mod types;

pub use adapter_architecture::{
    ArchitectureVerificationDomain, verification_result_to_delta_status,
};
pub use engine::VerifyKernel;
pub use evidence_source::EvidenceSource;
pub use registry::{DomainNotFoundError, VerificationDomain, VerifyDomainRegistry};
pub use types::{
    AffectedSubjects, ArchitectureConformanceClaim, BasisHash, ChangeBasis, ContradictionReason,
    EvidenceGap, ProbeKind, ProbePlan, ProbePlanStep, VerificationClaim, VerificationResult,
};

/// Construct the default domain registry with all known domains.
///
/// Currently registers:
/// - `architecture` → `ArchitectureVerificationDomain`
///
/// To add a new domain:
/// 1. Create the adapter in `adapter_<name>.rs`.
/// 2. Implement `VerificationDomain` for your adapter.
/// 3. Register it in this function.
pub fn default_registry() -> VerifyDomainRegistry {
    static ARCH_DOMAIN: ArchitectureVerificationDomain = ArchitectureVerificationDomain;
    VerifyDomainRegistry::new().register(&ARCH_DOMAIN as &dyn VerificationDomain)
}
