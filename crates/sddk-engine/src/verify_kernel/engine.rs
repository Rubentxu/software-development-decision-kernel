// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/engine.rs — A4-1: the Generic Verify kernel orchestrator.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
//
// # Purpose
//
// `VerifyKernel::evaluate` is the single entry point for all verification.
// It owns the structural probe-plan minimality pin (Q2 decision).
//
// # Data flow
//
// ```
// VerifyKernel::evaluate(claim, observations, domain) -> VerificationResult
//   ├─ domain.required_probe_kinds(&claim)      → required: BTreeSet<ProbeKind>
//   ├─ domain.minimal_probe_plan(&claim)        → plan: ProbePlan
//   │   └─ assert dedup + sorted + covers req   ← Q2 structural pin
//   ├─ observation::resolve_relation(...)       ← subcontracted to A4-0
//   └─ domain.evaluate(&claim, &observations)    ← specialization
// ```
//
// # Constraints
//
// - `evaluate(claim, observations, domain) -> VerificationResult` is pure and
//   deterministic once the evidence set is fixed (arch-spec-043 §Constraints).
// - Missing evidence = `Unknown`/`NOT_EVALUATED`, never a pass (arch-spec-043).
// - No universal quality score (arch-spec-043).

use crate::observation::ObservationSet;
use crate::verify_kernel::registry::{VerificationDomain, VerifyDomainRegistry};
use crate::verify_kernel::types::{VerificationClaim, VerificationResult};
use std::collections::BTreeSet;

/// Error returned when probe-plan minimality is violated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbePlanMinimalityError {
    pub claim: String,
    pub required: Vec<String>,
    pub provided: Vec<String>,
}

impl std::fmt::Display for ProbePlanMinimalityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "probe plan minimality violated for claim '{}': required {:?}, provided {:?}",
            self.claim, self.required, self.provided
        )
    }
}

impl std::error::Error for ProbePlanMinimalityError {}

/// The Generic Verify kernel.
///
/// Orchestrates verification: declares required probe kinds, derives the minimal
/// probe plan (with structural pin), and delegates to the domain for evaluation.
pub struct VerifyKernel;

impl VerifyKernel {
    /// Evaluate a claim against an observation set using a domain.
    ///
    /// This is the **only** entry point for verification. All domains are
    /// driven through this method.
    ///
    /// # Structural pin (Q2)
    ///
    /// The kernel asserts that:
    /// 1. `plan.is_deduplicated()` — no duplicate probe steps.
    /// 2. `plan.is_sorted()` — steps in canonical order.
    /// 3. `plan.probe_kinds() == domain.required_probe_kinds(claim)` — minimality.
    ///
    /// If any assertion fails, the kernel returns `Unknown` with a gap indicating
    /// the minimality violation. This is fail-closed at the kernel boundary.
    ///
    /// # Errors
    ///
    /// Returns `Unknown` if the plan violates minimality; the kernel never panics
    /// on bad adapter output (fail-closed, not fail-hard).
    pub fn evaluate(
        claim: &VerificationClaim,
        observations: &ObservationSet,
        domain: &dyn VerificationDomain,
    ) -> VerificationResult {
        // 1. Declare required probe kinds.
        let required = domain.required_probe_kinds(claim);

        // 2. Derive minimal probe plan.
        let plan = domain.minimal_probe_plan(claim);

        // 3. Structural pin: assert minimality (Q2 decision).
        if !Self::check_minimality(claim, &required, &plan) {
            // Fail-closed: return Unknown with gap indicating minimality violation.
            return VerificationResult::Unknown {
                gap: crate::verify_kernel::types::EvidenceGap::Custom(format!(
                    "probe_plan_minimality_violated: required {:?}, plan {:?}",
                    required
                        .iter()
                        .map(|k| format!("{:?}", k))
                        .collect::<Vec<_>>(),
                    plan.steps
                        .iter()
                        .map(|s| format!("{:?}", s.kind))
                        .collect::<Vec<_>>()
                )),
            };
        }

        // 4. Delegate to domain for evaluation.
        domain.evaluate(claim, observations)
    }

    /// Check that the probe plan satisfies minimality invariants.
    ///
    /// Returns `true` if the plan is valid:
    /// - Deduplicated
    /// - Sorted in canonical order
    /// - Probe kinds equal required
    fn check_minimality(
        _claim: &VerificationClaim,
        required: &BTreeSet<crate::verify_kernel::types::ProbeKind>,
        plan: &crate::verify_kernel::types::ProbePlan,
    ) -> bool {
        // Dedup check.
        if !plan.is_deduplicated() {
            return false;
        }

        // Sort check.
        if !plan.is_sorted() {
            return false;
        }

        // Coverage check: plan covers exactly the required kinds.
        if !plan.covers_exactly(required) {
            return false;
        }

        true
    }

    /// Evaluate a claim using a named domain from a registry.
    ///
    /// This is a convenience method that looks up the domain by name.
    /// Returns `Unknown` if the domain is not found.
    pub fn evaluate_with_registry(
        claim: &VerificationClaim,
        observations: &ObservationSet,
        registry: &VerifyDomainRegistry,
        domain_name: &str,
    ) -> VerificationResult {
        match registry.lookup(domain_name) {
            Ok(domain) => Self::evaluate(claim, observations, domain),
            Err(_) => VerificationResult::Unknown {
                gap: crate::verify_kernel::types::EvidenceGap::Custom(format!(
                    "domain_not_found: {}",
                    domain_name
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify_kernel::types::{
        AffectedSubjects, ArchitectureConformanceClaim, ChangeBasis, ProbeKind, ProbePlan,
        ProbePlanStep, VerificationClaim,
    };
    use std::collections::BTreeMap;

    /// A test domain that returns `Verified` for all claims.
    struct AlwaysVerifiedDomain;

    impl crate::verify_kernel::registry::VerificationDomain for AlwaysVerifiedDomain {
        fn name(&self) -> &'static str {
            "always_verified"
        }

        fn required_probe_kinds(&self, _claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
            [ProbeKind::Ownership].into()
        }

        fn minimal_probe_plan(&self, _claim: &VerificationClaim) -> ProbePlan {
            ProbePlan {
                steps: vec![ProbePlanStep {
                    kind: ProbeKind::Ownership,
                    subjects: AffectedSubjects::default(),
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

    /// A test domain that produces a non-minimal plan (violates Q2).
    struct NonMinimalDomain;

    impl crate::verify_kernel::registry::VerificationDomain for NonMinimalDomain {
        fn name(&self) -> &'static str {
            "non_minimal"
        }

        fn required_probe_kinds(&self, _claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
            [ProbeKind::Ownership].into()
        }

        fn minimal_probe_plan(&self, _claim: &VerificationClaim) -> ProbePlan {
            // Returns TWO probe kinds when only ONE is required.
            ProbePlan {
                steps: vec![
                    ProbePlanStep {
                        kind: ProbeKind::Ownership,
                        subjects: AffectedSubjects::default(),
                    },
                    ProbePlanStep {
                        kind: ProbeKind::Dependency,
                        subjects: AffectedSubjects::default(),
                    },
                ],
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

    fn empty_observations() -> ObservationSet {
        ObservationSet::new()
    }

    #[test]
    fn evaluate_with_valid_domain_returns_verified() {
        let claim = VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim {
            contract_id: "test_contract".into(),
            basis: ChangeBasis::new(vec![]),
        });
        let domain = AlwaysVerifiedDomain;
        let result = VerifyKernel::evaluate(&claim, &empty_observations(), &domain);
        assert!(matches!(result, VerificationResult::Verified));
    }

    #[test]
    fn evaluate_with_non_minimal_domain_returns_unknown() {
        let claim = VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim {
            contract_id: "test_contract".into(),
            basis: ChangeBasis::new(vec![]),
        });
        let domain = NonMinimalDomain;
        let result = VerifyKernel::evaluate(&claim, &empty_observations(), &domain);
        assert!(matches!(result, VerificationResult::Unknown { .. }));
    }
}
