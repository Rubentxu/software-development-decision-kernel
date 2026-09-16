// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/registry.rs — A4-1: domain registry for the Generic Verify kernel.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
//
// # Purpose
//
// `VerifyDomainRegistry` maps domain names to `&'static dyn VerificationDomain`.
// Domains are registered explicitly at boot; there is no `inventory` / `linkme`
// hack. Boot failure on a missing domain is fail-closed and obvious.
//
// # Design notes
//
// - Q1 decision (design.md §Decision Q1): Closed ADT core + trait-bounded
//   extension. Domains register as `impl VerificationDomain` for one concrete
//   claim type and are looked up by name.
// - No global mutable state; the registry is constructed at boot and passed
//   to `VerifyKernel::evaluate`.

use crate::observation::ObservationSet;
use crate::verify_kernel::types::{ProbeKind, ProbePlan, VerificationClaim, VerificationResult};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Error returned when a domain is not found in the registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainNotFoundError {
    pub domain: String,
}

impl std::fmt::Display for DomainNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "verification domain not found: {}", self.domain)
    }
}

impl std::error::Error for DomainNotFoundError {}

/// A verification domain: specializes the kernel for one domain.
///
/// Q1 decision (design.md §Decision Q1): trait-bounded extension behind the
/// closed `VerificationClaim` enum. Each domain implements this trait for one
/// concrete claim type (via `From<VerificationClaim>`).
///
/// The trait is intentionally minimal:
/// - `required_probe_kinds`: declares the probe kinds needed for a claim.
/// - `minimal_probe_plan`: derives the minimal probe plan (kernel asserts minimality).
/// - `evaluate`: runs the domain-specific evaluation.
pub trait VerificationDomain: Send + Sync {
    /// The domain name (used for lookup).
    fn name(&self) -> &'static str;

    /// Declare the probe kinds required for a claim.
    ///
    /// Q2 decision (design.md §Decision Q2): structural minimality. The kernel
    /// asserts that the plan's probe kinds equal this set.
    fn required_probe_kinds(&self, claim: &VerificationClaim) -> BTreeSet<ProbeKind>;

    /// Derive the minimal probe plan for a claim.
    ///
    /// The kernel asserts that this plan's probe kinds equal `required_probe_kinds`.
    fn minimal_probe_plan(&self, claim: &VerificationClaim) -> ProbePlan;

    /// Evaluate a claim against observations.
    ///
    /// This is the domain-specific specialization. The generic kernel handles
    /// probe-plan minimality and observation resolution; this method produces
    /// the domain-specific result.
    fn evaluate(
        &self,
        claim: &VerificationClaim,
        observations: &ObservationSet,
    ) -> VerificationResult;
}

/// A registry of verification domains, indexed by name.
pub struct VerifyDomainRegistry {
    domains: BTreeMap<&'static str, &'static dyn VerificationDomain>,
}

impl VerifyDomainRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
        }
    }

    /// Register a domain.
    ///
    /// Panics if a domain with the same name is already registered.
    pub fn register(mut self, domain: &'static dyn VerificationDomain) -> Self {
        let name = domain.name();
        if self.domains.contains_key(name) {
            panic!("verification domain already registered: {name}");
        }
        self.domains.insert(name, domain);
        self
    }

    /// Look up a domain by name.
    pub fn lookup(
        &self,
        name: &str,
    ) -> Result<&'static dyn VerificationDomain, DomainNotFoundError> {
        self.domains.get(name).copied().ok_or(DomainNotFoundError {
            domain: name.to_string(),
        })
    }

    /// Returns all registered domain names.
    pub fn all_names(&self) -> Vec<&'static str> {
        self.domains.keys().copied().collect()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.domains.is_empty()
    }
}

impl Default for VerifyDomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify_kernel::types::{ArchitectureConformanceClaim, ChangeBasis};

    /// A test domain that always returns `Verified`.
    struct TestDomain;

    impl VerificationDomain for TestDomain {
        fn name(&self) -> &'static str {
            "test"
        }

        fn required_probe_kinds(&self, _claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
            [ProbeKind::Ownership].into()
        }

        fn minimal_probe_plan(&self, _claim: &VerificationClaim) -> ProbePlan {
            ProbePlan {
                steps: vec![crate::verify_kernel::types::ProbePlanStep {
                    kind: ProbeKind::Ownership,
                    subjects: crate::verify_kernel::types::AffectedSubjects::default(),
                }],
            }
        }

        fn evaluate(
            &self,
            _claim: &VerificationClaim,
            _observations: &ObservationSet,
        ) -> VerificationResult {
            VerificationResult::Verified
        }
    }

    #[test]
    fn registry_registers_and_looks_up_domain() {
        let registry = VerifyDomainRegistry::new().register(&TestDomain as &dyn VerificationDomain);

        let domain = registry.lookup("test").unwrap();
        assert_eq!(domain.name(), "test");
    }

    #[test]
    fn registry_returns_error_for_unknown_domain() {
        let registry = VerifyDomainRegistry::new();
        let result = registry.lookup("unknown");
        match result {
            Err(e) => assert_eq!(e.domain, "unknown"),
            Ok(_) => panic!("expected error for unknown domain"),
        }
    }

    #[test]
    fn registry_all_names_returns_registered() {
        let registry = VerifyDomainRegistry::new().register(&TestDomain as &dyn VerificationDomain);
        assert_eq!(registry.all_names(), vec!["test"]);
    }
}
