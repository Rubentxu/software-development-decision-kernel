//! Static-provider verification domain — consumes provider
//! observations from the `ObservationSet`.
//!
//! Authority: `arch-spec-021` (IPB-008: providers are evidence,
//! never authority), AIW-S1.
//!
//! This is the first VerifyKernel domain whose `evaluate` actually
//! reads the observation set produced by a `CodeIntelligencePort`
//! provider (UAT A05). The claim vocabulary is deliberately tiny:
//! a `StaticProviderClaim` names a subject; the set is searched
//! for affirm/deny observations about that subject.

use std::collections::BTreeSet;

use crate::observation::types::{ObservationSet, ObservationStance};
use crate::verify_kernel::registry::VerificationDomain;
use crate::verify_kernel::types::{EvidenceGap, ProbeKind, VerificationClaim};
use crate::verify_kernel::types::{ProbePlan, VerificationResult};

/// A claim evaluated purely against provider observations.
///
/// `subject_tag` is the canonical tag of the observation subject
/// (e.g. `unit:crates/sddk-engine/src/lib.rs` or a relation id).
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct StaticProviderClaim {
    /// Canonical tag of the observed subject.
    pub subject_tag: String,
    /// Human-readable contract id carried into results/receipts.
    pub contract_id: String,
}

/// Verification result variants are shared with the kernel; this
/// domain maps:
///
/// - affirm present, no deny  → `Verified`
/// - deny present, no affirm  → `Contradicted`
/// - both present             → `Contradicted` (contradiction is
///   surfaced, never averaged away)
/// - neither                  → `Unknown { MissingForSubject }`
pub struct StaticProviderDomain;

impl VerificationDomain for StaticProviderDomain {
    fn name(&self) -> &'static str {
        "static_provider"
    }

    fn required_probe_kinds(&self, claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
        match claim {
            VerificationClaim::StaticProvider(_) => [ProbeKind::ProviderBoundary].into(),
            VerificationClaim::ArchitectureConformance(_) => BTreeSet::new(),
        }
    }

    fn minimal_probe_plan(&self, claim: &VerificationClaim) -> ProbePlan {
        match claim {
            VerificationClaim::StaticProvider(_) => ProbePlan {
                steps: vec![crate::verify_kernel::types::ProbePlanStep {
                    kind: ProbeKind::ProviderBoundary,
                    subjects: Default::default(),
                }],
            },
            VerificationClaim::ArchitectureConformance(_) => ProbePlan { steps: vec![] },
        }
    }

    fn evaluate(
        &self,
        claim: &VerificationClaim,
        observations: &ObservationSet,
    ) -> VerificationResult {
        let VerificationClaim::StaticProvider(c) = claim else {
            return VerificationResult::Unknown {
                gap: EvidenceGap::Custom(
                    "static_provider domain cannot evaluate this claim type".into(),
                ),
            };
        };
        let mut affirm = false;
        let mut deny = false;
        for obs in observations.observations() {
            if obs.subject.canonical_tag() != c.subject_tag {
                continue;
            }
            match obs.stance {
                ObservationStance::Affirms => affirm = true,
                ObservationStance::Denies => deny = true,
            }
        }
        match (affirm, deny) {
            (true, false) => VerificationResult::Verified,
            (_, true) => VerificationResult::Contradicted {
                reason: crate::verify_kernel::types::ContradictionReason::ProviderBoundaryViolation,
            },
            (false, false) => VerificationResult::Unknown {
                gap: EvidenceGap::MissingForSubject(c.contract_id.clone()),
            },
        }
    }
}
