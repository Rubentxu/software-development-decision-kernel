//! Runtime-provider verification domain — consumes runtime capture
//! observations from an `ObservationSet` produced by a
//! `RuntimeEvidencePort` provider.
//!
//! Authority: AIW-S5 (A7 first vertical chain). Mirrors
//! `adapter_static_provider` (AIW-S1) but is distinct by origin: it
//! only accepts observations whose origin is `RuntimeProvider`, so a
//! static-provider affirmation can never satisfy a runtime claim.
//!
//! Claim vocabulary: `RuntimeProviderClaim` names a subject tag
//! (e.g. `runtime:/path/to/bin`) plus a contract id.

use std::collections::BTreeSet;

use crate::observation::types::{ObservationOrigin, ObservationSet, ObservationStance};
use crate::verify_kernel::registry::VerificationDomain;
use crate::verify_kernel::types::{EvidenceGap, ProbeKind, VerificationClaim};
use crate::verify_kernel::types::{ProbePlan, VerificationResult};

/// A claim evaluated against runtime-provider observations.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct RuntimeProviderClaim {
    /// Canonical tag of the observed runtime subject
    /// (`runtime:<program>`).
    pub subject_tag: String,
    /// Human-readable contract id carried into results/receipts.
    pub contract_id: String,
    /// Minimum number of captured events required for the claim to
    /// be evaluable (0 = any non-empty capture). A capture with
    /// fewer events leaves the result `Unknown`, never `Verified`.
    pub min_events: u64,
}

/// Mapping:
/// - runtime-origin affirm present, total events >= min_events, no
///   deny → `Verified`
/// - deny present (any origin matching the subject) → `Contradicted`
/// - affirm but events below threshold → `Unknown { Custom }`
///   (insufficient capture; NOT a pass)
/// - neither → `Unknown { MissingForSubject }`
pub struct RuntimeProviderDomain;

impl VerificationDomain for RuntimeProviderDomain {
    fn name(&self) -> &'static str {
        "runtime_provider"
    }

    fn required_probe_kinds(&self, claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
        match claim {
            VerificationClaim::RuntimeProvider(_) => [ProbeKind::ProviderBoundary].into(),
            _ => BTreeSet::new(),
        }
    }

    fn minimal_probe_plan(&self, claim: &VerificationClaim) -> ProbePlan {
        match claim {
            VerificationClaim::RuntimeProvider(_) => ProbePlan {
                steps: vec![crate::verify_kernel::types::ProbePlanStep {
                    kind: ProbeKind::ProviderBoundary,
                    subjects: Default::default(),
                }],
            },
            _ => ProbePlan { steps: vec![] },
        }
    }

    fn evaluate(
        &self,
        claim: &VerificationClaim,
        observations: &ObservationSet,
    ) -> VerificationResult {
        let VerificationClaim::RuntimeProvider(c) = claim else {
            return VerificationResult::Unknown {
                gap: EvidenceGap::Custom(
                    "runtime_provider domain cannot evaluate this claim type".into(),
                ),
            };
        };
        let mut affirm = false;
        let mut deny = false;
        let mut runtime_events: Option<u64> = None;
        for obs in observations.observations() {
            if obs.subject.canonical_tag() != c.subject_tag {
                continue;
            }
            // Only runtime-origin observations carry the event-count
            // evidence; static affirmations on the same tag do not
            // satisfy a runtime claim.
            if obs.origin == ObservationOrigin::RuntimeProvider {
                match obs.stance {
                    ObservationStance::Affirms => affirm = true,
                    ObservationStance::Denies => deny = true,
                }
                if let Some(n) = parse_events_from_locator(&obs.evidence.locator) {
                    runtime_events = Some(runtime_events.unwrap_or(0) + n);
                }
            }
        }
        match (affirm, deny) {
            (_, true) => VerificationResult::Contradicted {
                reason: crate::verify_kernel::types::ContradictionReason::ProviderBoundaryViolation,
            },
            (true, false) => {
                let captured = runtime_events.unwrap_or(0);
                if captured >= c.min_events {
                    VerificationResult::Verified
                } else {
                    VerificationResult::Unknown {
                        gap: EvidenceGap::Custom(format!(
                            "insufficient_runtime_capture: {} events captured, {} required",
                            captured, c.min_events
                        )),
                    }
                }
            }
            (false, false) => VerificationResult::Unknown {
                gap: EvidenceGap::MissingForSubject(c.contract_id.clone()),
            },
        }
    }
}

/// Parse `#events=<n>` out of an evidence locator. Returns None when
/// the locator carries no count (the caller then treats the capture
/// as zero-event for threshold purposes).
fn parse_events_from_locator(locator: &str) -> Option<u64> {
    let idx = locator.find("#events=")?;
    locator[idx + "#events=".len()..].parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture_graph::SoftwareUnitRef;
    use crate::evidence_ref::{EvidenceKind, EvidenceRef};
    use crate::observation::types::ObservationSet as CanonicalObservationSet;
    use crate::observation::types::{ObservationBasis, ObservationSubject, SoftwareObservation};

    fn runtime_claim(min_events: u64) -> VerificationClaim {
        VerificationClaim::RuntimeProvider(RuntimeProviderClaim {
            subject_tag: "unit:runtime:/tmp/t".to_owned(),
            contract_id: "program-completes".to_owned(),
            min_events,
        })
    }

    #[test]
    fn non_runtime_claim_is_unknown() {
        let other = VerificationClaim::StaticProvider(
            crate::verify_kernel::adapter_static_provider::StaticProviderClaim {
                subject_tag: "runtime:/tmp/t".into(),
                contract_id: "x".into(),
            },
        );
        let set = ObservationSet::new();
        let d = RuntimeProviderDomain;
        assert!(matches!(
            d.evaluate(&other, &set),
            VerificationResult::Unknown { .. }
        ));
    }

    #[test]
    fn missing_subject_is_unknown() {
        let set = ObservationSet::new();
        let d = RuntimeProviderDomain;
        assert!(matches!(
            d.evaluate(&runtime_claim(0), &set),
            VerificationResult::Unknown { .. }
        ));
    }

    #[test]
    fn static_origin_never_satisfies_runtime_claim() {
        let mut set = CanonicalObservationSet::new();
        let subject = ObservationSubject::Unit(SoftwareUnitRef::new("runtime:/tmp/t".to_string()));
        set.insert(SoftwareObservation::declare(
            subject,
            ObservationStance::Affirms,
            EvidenceRef::new(EvidenceKind::Adhoc, "static://x".to_string()),
            ObservationOrigin::StaticProvider,
            ObservationBasis::for_provider_result("rev", "digest"),
            None,
            "static",
        ));
        let d = RuntimeProviderDomain;
        assert!(matches!(
            d.evaluate(&runtime_claim(0), &set),
            VerificationResult::Unknown { .. }
        ));
    }

    #[test]
    fn events_below_threshold_is_unknown_not_verified() {
        let mut set = CanonicalObservationSet::new();
        let subject = ObservationSubject::Unit(SoftwareUnitRef::new("runtime:/tmp/t".to_string()));
        set.insert(SoftwareObservation::declare(
            subject,
            ObservationStance::Affirms,
            EvidenceRef::new(
                EvidenceKind::Adhoc,
                "chronos-mcp://session/s#events=3".to_string(),
            ),
            ObservationOrigin::RuntimeProvider,
            ObservationBasis::for_provider_result("rev", "digest"),
            None,
            "chronos-mcp",
        ));
        let d = RuntimeProviderDomain;
        assert!(matches!(
            d.evaluate(&runtime_claim(10), &set),
            VerificationResult::Unknown { .. }
        ));
        assert!(matches!(
            d.evaluate(&runtime_claim(3), &set),
            VerificationResult::Verified
        ));
    }
}
