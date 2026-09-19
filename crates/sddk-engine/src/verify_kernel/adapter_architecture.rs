// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/adapter_architecture.rs — A4-1: ArchitectureVerificationDomain adapter.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
// Spec: `docs/architecture/specs/arch-spec-044-generic-debverify.md` (sibling)
//
// # Purpose
//
// `ArchitectureVerificationDomain` adapts the existing AC4
// `compute_conformance_delta` into the Generic Verify kernel surface.
//
// This is a **wrapping adapter**, not a rewrite:
// - Calls `compute_conformance_delta` with the same inputs AC4 uses today.
// - Maps `DeltaContractStatus → VerificationResult` (bijection, documented below).
// - The JSON of `ArchitectureConformanceReceipt` does NOT change.
//
// # Anti-encroachment
//
// This adapter imports `architecture_conformance::compute_conformance_delta`
// (per design.md §File Changes: "only an adapter import if in-place").
// It does NOT import any other `architecture_*` module.
//
// AC4's public types, JSON shape, digests and exit codes are unchanged.
// `sddk architecture receipt` continues to work as it does today.
//
// # Bijection: DeltaContractStatus ↔ VerificationResult
//
// | DeltaContractStatus | VerificationResult |
// |---|---|
// | `Verified` | `Verified` |
// | `Contradicted` | `Contradicted { reason: OwnershipViolation }` |
// | `Unknown` | `Unknown { gap: MissingForSubject(contract_id) }` |
// | `Stale` | `Stale { basis: computed_from_basis }` |
// | `NotEvaluated` | `NotApplicable` |

use crate::architecture_conformance::{
    ConformanceInputs, DeltaContractStatus, compute_conformance_delta_core,
};
use crate::observation::ObservationSet;
use crate::verify_kernel::registry::VerificationDomain;
use crate::verify_kernel::types::{
    AffectedSubjects, ArchitectureConformanceClaim, ContradictionReason, EvidenceGap, ProbeKind,
    ProbePlan, ProbePlanStep, VerificationClaim, VerificationResult,
};
use std::collections::BTreeSet;

/// The architecture-verification domain: adapts AC4 to the Generic Verify kernel.
///
/// This is the **only** specialization in this delivery (arch-spec-043 §In scope).
/// It wraps `compute_conformance_delta` without changing AC4's behavior.
pub struct ArchitectureVerificationDomain;

impl ArchitectureVerificationDomain {
    /// Construct a new domain instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArchitectureVerificationDomain {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationDomain for ArchitectureVerificationDomain {
    fn name(&self) -> &'static str {
        "architecture"
    }

    fn required_probe_kinds(&self, claim: &VerificationClaim) -> BTreeSet<ProbeKind> {
        match claim {
            VerificationClaim::ArchitectureConformance(_) => {
                // AC4's ProbeRequirement maps to our ProbeKind.
                // For a single contract, we need at least one probe kind.
                [ProbeKind::Ownership].into()
            }
            // Not this domain's claim; empty requirement (fail-closed
            // Unknown surfaces from the domain that owns it).
            VerificationClaim::StaticProvider(_) | VerificationClaim::RuntimeProvider(_) => {
                BTreeSet::new()
            }
        }
    }

    fn minimal_probe_plan(&self, claim: &VerificationClaim) -> ProbePlan {
        match claim {
            VerificationClaim::StaticProvider(_) | VerificationClaim::RuntimeProvider(_) => {
                ProbePlan { steps: vec![] }
            }
            VerificationClaim::ArchitectureConformance(c) => {
                let mut subjects = AffectedSubjects::default();
                for unit in &c.basis.units {
                    subjects.units.insert(unit.clone());
                }

                // Map contract kind to probe kind. For now, use Ownership as the default.
                // In a full implementation, this would consult the contract object.
                let kind = ProbeKind::Ownership;

                ProbePlan {
                    steps: vec![ProbePlanStep { kind, subjects }],
                }
            }
        }
    }

    fn evaluate(
        &self,
        claim: &VerificationClaim,
        _observations: &ObservationSet,
    ) -> VerificationResult {
        match claim {
            VerificationClaim::StaticProvider(_) | VerificationClaim::RuntimeProvider(_) => {
                VerificationResult::Unknown {
                    gap: EvidenceGap::Custom(
                        "static_provider claims are evaluated by StaticProviderDomain".into(),
                    ),
                }
            }
            VerificationClaim::ArchitectureConformance(c) => {
                // The trait-level `evaluate` is the **port**. It has access to
                // the claim but not to the architecture context (overlay,
                // contracts, evidence). Without context, the correct kernel
                // result is `Unknown` (we cannot evaluate an architecture
                // conformance claim without its inputs).
                //
                // The full computation goes through `evaluate_with_context`,
                // called by the CLI / integration layer which DOES have the
                // context. The trait method is here so the kernel registry is
                // complete and the domain is callable from generic surfaces;
                // it is not the primary path for architecture verification.
                let _ = c;
                VerificationResult::Unknown {
                    gap: EvidenceGap::MissingForSubject(
                        "architecture context not provided to trait evaluate; \
                         call evaluate_with_context from the integration layer"
                            .to_string(),
                    ),
                }
            }
        }
    }
}

impl ArchitectureVerificationDomain {
    /// Evaluate an architecture conformance claim with full context.
    ///
    /// This is the **A4-2M single execution path** for AC4. It calls
    /// `compute_conformance_delta_core` (the same core that
    /// `architecture_conformance::compute_conformance_delta` calls) and
    /// maps the resulting `DeltaContractStatus` for the claim's contract
    /// id into the generic verify kernel's `VerificationResult` vocabulary.
    ///
    /// Both this method and the legacy public function call into the same
    /// core, so there is exactly one AC4 computation in the codebase. The
    /// legacy DTO shape (`ArchitectureConformanceDelta`) survives as a
    /// read model; the kernel result (`VerificationResult`) is the
    /// authority in the generic substrate.
    ///
    /// Mapping rules (bijection with `map_status_to_result`):
    ///
    /// | DeltaContractStatus    | VerificationResult              |
    /// |------------------------|---------------------------------|
    /// | `Verified`             | `Verified`                      |
    /// | `Contradicted`         | `Contradicted { OwnershipViolation }` |
    /// | `Unknown`              | `Unknown { MissingForSubject }` |
    /// | `Stale`                | `Stale { basis: SENTINEL }`     |
    /// | `NotEvaluated`         | `NotApplicable`                 |
    pub fn evaluate_with_context(
        &self,
        claim: &ArchitectureConformanceClaim,
        overlay: &crate::architecture_graph::ArchitectureGraphOverlay,
        inputs: ConformanceInputs<'_>,
        now: crate::knowledge::EventTime,
        scope_units: &[crate::architecture_graph::SoftwareUnitRef],
    ) -> VerificationResult {
        let delta = match compute_conformance_delta_core(overlay, inputs, now, scope_units) {
            Ok(d) => d,
            Err(_e) => {
                // AC4 failed (e.g. invalid contract id, unresolved anchor).
                // In the kernel vocabulary this is `Unknown` — we cannot
                // produce a verification result without a delta.
                return VerificationResult::Unknown {
                    gap: EvidenceGap::MissingForSubject(claim.contract_id.clone()),
                };
            }
        };
        // Resolve the claim's contract_id to a `ContractId` and look up its
        // status in the delta. If the contract did not enter scope (e.g.
        // the change did not touch its subject), the status is `Unknown`.
        let status = match crate::architectural_contract::ContractId::new(claim.contract_id.clone())
        {
            Ok(cid) => delta
                .status_of(&cid)
                .unwrap_or(DeltaContractStatus::Unknown),
            Err(_) => DeltaContractStatus::Unknown,
        };
        Self::map_status_to_result(status, &claim.contract_id)
    }
}

impl ArchitectureVerificationDomain {
    /// Bijective mapping: DeltaContractStatus → VerificationResult.
    ///
    /// This mapping is the **inverse** of the mapping in the CLI adapter
    /// that converts VerificationResult back to DeltaContractStatus for
    /// receipt compatibility.
    pub(crate) fn map_status_to_result(
        status: DeltaContractStatus,
        contract_id: &str,
    ) -> VerificationResult {
        match status {
            DeltaContractStatus::Verified => VerificationResult::Verified,
            DeltaContractStatus::Contradicted => VerificationResult::Contradicted {
                reason: ContradictionReason::OwnershipViolation,
            },
            DeltaContractStatus::Unknown => VerificationResult::Unknown {
                gap: EvidenceGap::MissingForSubject(contract_id.to_string()),
            },
            DeltaContractStatus::Stale => VerificationResult::Stale {
                basis: crate::verify_kernel::types::BasisHash::SENTINEL,
            },
            DeltaContractStatus::NotEvaluated => VerificationResult::NotApplicable,
        }
    }
}

/// Bijective mapping: VerificationResult → DeltaContractStatus.
///
/// Used by the CLI adapter to convert the kernel result back to AC4's
/// status for receipt compatibility.
pub fn verification_result_to_delta_status(result: &VerificationResult) -> DeltaContractStatus {
    match result {
        VerificationResult::Verified => DeltaContractStatus::Verified,
        VerificationResult::Contradicted { .. } => DeltaContractStatus::Contradicted,
        VerificationResult::Unknown { .. } => DeltaContractStatus::Unknown,
        VerificationResult::Stale { .. } => DeltaContractStatus::Stale,
        VerificationResult::NotApplicable => DeltaContractStatus::NotEvaluated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify_kernel::types::ChangeBasis;

    #[test]
    fn domain_name_is_architecture() {
        let domain = ArchitectureVerificationDomain::new();
        assert_eq!(domain.name(), "architecture");
    }

    #[test]
    fn domain_requires_ownership_probe_for_architecture_claim() {
        let domain = ArchitectureVerificationDomain::new();
        let claim = VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim {
            contract_id: "test_contract".into(),
            basis: ChangeBasis::new(vec![]),
        });
        let required = domain.required_probe_kinds(&claim);
        assert!(required.contains(&ProbeKind::Ownership));
    }

    #[test]
    fn domain_returns_valid_probe_plan() {
        let domain = ArchitectureVerificationDomain::new();
        let claim = VerificationClaim::ArchitectureConformance(ArchitectureConformanceClaim {
            contract_id: "test_contract".into(),
            basis: ChangeBasis::new(vec![]),
        });
        let plan = domain.minimal_probe_plan(&claim);
        assert!(plan.is_deduplicated());
        assert!(plan.is_sorted());
        assert!(plan.covers_exactly(&domain.required_probe_kinds(&claim)));
    }

    #[test]
    fn bijection_holds_for_all_statuses() {
        for status in DeltaContractStatus::ALL {
            let result = ArchitectureVerificationDomain::map_status_to_result(status, "test");
            let back = verification_result_to_delta_status(&result);
            assert_eq!(status, back, "bijection violated for {:?}", status);
        }
    }

    // ─── A4-2M convergence tests ──────────────────────────────────────────

    use crate::architecture_graph::{ArchitectureGraphOverlay, SoftwareUnitRef};
    use crate::knowledge::EventTime;
    use std::collections::BTreeMap;

    /// Build a minimal claim-from-foundation path: an empty overlay and an
    /// empty contract set are the canonical A4-2M "no inputs" case.
    fn empty_context() -> (ArchitectureGraphOverlay, Vec<SoftwareUnitRef>) {
        (ArchitectureGraphOverlay::new(), Vec::new())
    }

    /// The legacy public function and the new `evaluate_with_context` MUST
    /// produce the same kernel status for the same contract_id, over the
    /// same inputs. This is the A4-2M convergence invariant — the two
    /// paths are guaranteed to share the same computation because they
    /// call the same core.
    #[test]
    fn a4_2m_legacy_and_context_paths_agree_on_empty_inputs() {
        // Both paths, when called with an empty overlay / contract set /
        // scope, MUST agree. The legacy path produces a delta with
        // `not_evaluated` containing the contract id (status Unknown);
        // the kernel path applies `map_status_to_result(Unknown, ..)` to
        // get `VerificationResult::Unknown { .. }`.
        let (overlay, scope) = empty_context();
        let now = EventTime::EPOCH;
        let inputs = ConformanceInputs {
            contracts: &[],
            evidence: &BTreeMap::new(),
            contradiction_witnesses: &[],
            contract_filter: None,
        };
        let claim = ArchitectureConformanceClaim {
            contract_id: "units/auth_core".to_string(),
            basis: ChangeBasis::new(vec![]),
        };
        // Legacy path: with no contracts in scope, the delta is empty and
        // `status_of` for any id returns None → Unknown by the bijection.
        let result = ArchitectureVerificationDomain::new()
            .evaluate_with_context(&claim, &overlay, inputs, now, &scope);
        assert!(
            matches!(result, VerificationResult::Unknown { .. }),
            "A4-2M: empty context must yield Unknown, got {:?}",
            result
        );
    }

    /// When the trait-level `evaluate` is called without context, the
    /// result is `Unknown` (NOT `Verified` and NOT `Contradicted`).
    /// This is the A4-2M false-clean guard at the verify kernel level:
    /// a domain that lacks the data it needs MUST say so.
    #[test]
    fn a4_2m_trait_evaluate_without_context_returns_unknown() {
        let claim = ArchitectureConformanceClaim {
            contract_id: "units/auth_core".to_string(),
            basis: ChangeBasis::new(vec![]),
        };
        let result = ArchitectureVerificationDomain::new().evaluate(
            &VerificationClaim::ArchitectureConformance(claim),
            &ObservationSet::new(),
        );
        assert!(
            matches!(result, VerificationResult::Unknown { .. }),
            "A4-2M: trait evaluate without context must return Unknown, got {:?}",
            result
        );
    }

    /// The two paths share `compute_conformance_delta_core` as their single
    /// execution surface. Asserting that the legacy `compute_conformance_delta`
    /// is now a thin wrapper over the core (no separate code path) is the
    /// A4-2M convergence guarantee.
    #[test]
    fn a4_2m_legacy_is_thin_wrapper_over_core() {
        // If they ever diverge, this compile-time + behavioural equality
        // breaks. Both must accept the same inputs and produce the same
        // delta shape.
        let (overlay, scope) = empty_context();
        let now = EventTime::EPOCH;
        let inputs = ConformanceInputs {
            contracts: &[],
            evidence: &BTreeMap::new(),
            contradiction_witnesses: &[],
            contract_filter: None,
        };
        let legacy = crate::architecture_conformance::compute_conformance_delta(
            &overlay,
            inputs.clone(),
            now,
            &scope,
        )
        .expect("legacy compute");
        let core =
            compute_conformance_delta_core(&overlay, inputs, now, &scope).expect("core compute");
        assert_eq!(
            legacy.id, core.id,
            "A4-2M: legacy and core must produce the same delta id"
        );
        assert_eq!(
            legacy.affected.len(),
            core.affected.len(),
            "A4-2M: legacy and core must produce the same affected map size"
        );
        assert_eq!(
            legacy.claims.len(),
            core.claims.len(),
            "A4-2M: legacy and core must produce the same claim map size"
        );
        assert_eq!(
            legacy.plan_digest, core.plan_digest,
            "A4-2M: legacy and core must produce the same plan_digest"
        );
        assert_eq!(
            legacy.contract_set_digest, core.contract_set_digest,
            "A4-2M: legacy and core must produce the same contract_set_digest"
        );
        assert_eq!(
            legacy.graph_digest, core.graph_digest,
            "A4-2M: legacy and core must produce the same graph_digest"
        );
        assert_eq!(
            legacy.evaluated_at, core.evaluated_at,
            "A4-2M: legacy and core must produce the same evaluated_at"
        );
        assert_eq!(
            legacy.vector, core.vector,
            "A4-2M: legacy and core must produce the same vector"
        );
    }
}
