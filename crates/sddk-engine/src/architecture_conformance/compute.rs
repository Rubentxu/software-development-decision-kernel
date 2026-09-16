// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_conformance/compute.rs — A3-S5 / AC4 pure computation.
//
// `compute_conformance_delta` is a pure function: no IO, no clock reads, no
// provider/LLM calls (AC-034-004, AC-034-006). It maps a change basis
// (changed software units) to affected contracts through the AC2 overlay
// public API, derives the minimum deterministic probe plan, and evaluates
// each affected contract through AC1's `ContractEvaluation::evaluate`.

use crate::architectural_contract::{
    ArchitecturalContract, ClaimOutcome, ContractEvaluation, ContractId, ContractKind, EvaluatorRef,
};
use crate::architecture_graph::{ArchitectureGraphOverlay, SoftwareUnitRef};
use crate::knowledge::EventTime;
use crate::semantic_graph::SemanticGraphProjection;
use crate::semantic_node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    AffectedContract, ArchitectureConformanceDelta, ArchitectureConformanceDeltaId,
    ConformanceError, ConformanceInputs, ConformanceVector, ProbeRequirement, VectorStatus,
};

/// Domain prefix for the probe-plan digest.
pub const PLAN_DIGEST_DOMAIN: &str = "sddk.architecture_conformance.plan.v1|";
/// Domain prefix for the contract-set digest.
pub const CONTRACT_SET_DIGEST_DOMAIN: &str = "sddk.architecture_conformance.contract_set.v1|";
/// The evaluator identifier AC4 records on every claim it causes.
pub const AC4_EVALUATOR: &str = "sddk.architecture_conformance";

/// Compute the conformance delta for a change basis.
///
/// Pure: identical inputs yield byte-identical digests and identical ordering
/// (REQ-AC4-013). `now` is data, not a clock read.
///
/// A4-2M convergence: this is a thin wrapper around
/// [`compute_conformance_delta_core`]. The core is the **single execution
/// surface** of AC4's computation; both the public function and the
/// architecture verification domain adapter (via
/// `ArchitectureVerificationDomain::evaluate_with_context`) call it.
pub fn compute_conformance_delta(
    overlay: &ArchitectureGraphOverlay,
    inputs: ConformanceInputs<'_>,
    now: EventTime,
    scope_units: &[SoftwareUnitRef],
) -> Result<ArchitectureConformanceDelta, ConformanceError> {
    compute_conformance_delta_core(overlay, inputs, now, scope_units)
}

/// Core AC4 computation. Returns the delta as-is.
///
/// This is the **single source of truth** for AC4's conformance
/// computation. Both [`compute_conformance_delta`] (legacy DTO shape) and
/// [`crate::verify_kernel::ArchitectureVerificationDomain::evaluate_with_context`]
/// (generic verify kernel shape) call into here, ensuring the two paths
/// share exactly one execution.
pub fn compute_conformance_delta_core(
    overlay: &ArchitectureGraphOverlay,
    inputs: ConformanceInputs<'_>,
    now: EventTime,
    scope_units: &[SoftwareUnitRef],
) -> Result<ArchitectureConformanceDelta, ConformanceError> {
    // 1. Contract object index (sorted by id via BTreeMap).
    let mut contract_index: BTreeMap<ContractId, &ArchitecturalContract> = BTreeMap::new();
    for c in inputs.contracts {
        contract_index.insert(c.id().clone(), c);
    }

    // Contradiction witnesses: normalized, sorted, deduplicated.
    let mut witnesses: BTreeSet<ContractId> =
        inputs.contradiction_witnesses.iter().cloned().collect();

    // 2. Affected contracts: only those reachable from the scope units
    //    (AC-UAT-008). Iterate units in sorted order for determinism.
    //
    //    `scope_units` is the set of units whose contracts are being asked
    //    about. Its usual source is the diff (`--changed`), but a caller may
    //    instead name a subject explicitly (`--contract X`, which scopes to X's
    //    subject unit). The delta does not care which: it reports what it was
    //    asked about, and `contract_filter` narrows further.
    let scope: BTreeSet<&SoftwareUnitRef> = scope_units.iter().collect();
    let mut triggering: BTreeMap<ContractId, BTreeSet<SoftwareUnitRef>> = BTreeMap::new();
    for unit in scope {
        for anchor in overlay.find_contracts_for_unit(unit) {
            let cid = resolve_contract_id(overlay, &anchor)?;
            triggering.entry(cid).or_default().insert(unit.clone());
        }
    }

    // Narrow to the requested contract, if any. Applied here rather than
    // downstream so the delta stays the single authority for its own scope:
    // `claim_results`, `unknowns`, `contradictions` and `class_coverage` are
    // all derived from `affected` and follow without a second filtering pass.
    //
    // A filter can only remove; it never adds a contract the change did not
    // reach. `--contract X --changed` is therefore the intersection, and an
    // empty result means "X was not touched", which the receipt makes
    // attributable by recording the filter.
    if let Some(filter) = inputs.contract_filter {
        triggering.retain(|cid, _| cid == filter);
    }

    // A contradiction witness is only meaningful for an affected contract;
    // keep only the witnesses that actually entered scope.
    witnesses.retain(|cid| triggering.contains_key(cid));

    // 3. Affected map with deterministic probe plans.
    let mut affected: BTreeMap<ContractId, AffectedContract> = BTreeMap::new();
    for (cid, units) in &triggering {
        let (contract_kind, probes) = match contract_index.get(cid) {
            Some(c) => {
                let kind = c.kind().clone();
                let mut probes = vec![ProbeRequirement::for_contract_kind(&kind)];
                probes.sort_unstable();
                probes.dedup();
                (Some(kind), probes)
            }
            // Affected but no object supplied: kind unknown, probe unknown.
            // The status will be `NotEvaluated`.
            None => (None, vec![ProbeRequirement::Unknown]),
        };
        affected.insert(
            cid.clone(),
            AffectedContract {
                contract_kind,
                triggering_units: units.iter().cloned().collect(),
                probes,
            },
        );
    }

    // 4. Evaluate each affected contract that has an object and no witness
    //    (REQ-AC4-016, REQ-AC4-019).
    let mut claims = BTreeMap::new();
    let mut not_evaluated: Vec<ContractId> = Vec::new();
    for cid in affected.keys() {
        if witnesses.contains(cid) {
            // A probe observed a violation; AC1's evaluator does not produce
            // `Contradicted`, so it is recorded as a delta-level contradiction
            // (never a fabricated claim).
            continue;
        }
        match contract_index.get(cid) {
            Some(contract) => {
                let refs = inputs.evidence.get(cid).cloned().unwrap_or_default();
                let evaluator = EvaluatorRef::new(AC4_EVALUATOR).expect("non-empty literal");
                let claim = ContractEvaluation::evaluate(contract, refs, now, evaluator, None);
                claims.insert(cid.clone(), claim);
            }
            None => not_evaluated.push(cid.clone()),
        }
    }
    not_evaluated.sort();
    let contradictions: Vec<ContractId> = witnesses.into_iter().collect();

    // 5. Vector (statuses only; no aggregate).
    let vector = derive_vector(&affected, &claims, &contradictions, &not_evaluated);

    // 6. Digests.
    let plan_digest = compute_plan_digest(&affected);
    let contract_set_digest = compute_contract_set_digest(&contract_index);
    let graph_digest = overlay.digest();
    let id =
        ArchitectureConformanceDeltaId::derive(&plan_digest, &contract_set_digest, &graph_digest);

    Ok(ArchitectureConformanceDelta {
        affected,
        claims,
        contradictions,
        not_evaluated,
        plan_digest,
        contract_set_digest,
        graph_digest,
        evaluated_at: now,
        vector,
        id,
    })
}

/// Resolve an overlay anchor node to its `ContractId`.
///
/// AC2's `add_contract_metadata` writes `props_inline["contract_id"]` on the
/// `contract:<id>` anchor node. If the anchor is absent or lacks the prop, the
/// AC2 convention was violated and we fail loudly (design note R2).
fn resolve_contract_id(
    overlay: &ArchitectureGraphOverlay,
    anchor: &NodeId,
) -> Result<ContractId, ConformanceError> {
    for node in overlay.projection().nodes() {
        if &node.id == anchor {
            return match node.props_inline.get("contract_id") {
                Some(raw) => ContractId::new(raw.clone())
                    .map_err(|_| ConformanceError::InvalidContractId { raw: raw.clone() }),
                None => Err(ConformanceError::UnresolvedContractAnchor {
                    anchor: anchor.as_str().to_string(),
                }),
            };
        }
    }
    Err(ConformanceError::UnresolvedContractAnchor {
        anchor: anchor.as_str().to_string(),
    })
}

/// Local kind tag for digests. AC1's `ContractKind::canonical_tag` is
/// `pub(super)`; AC4 keeps its own domain-separated rendering.
pub(crate) fn kind_tag(kind: &ContractKind) -> String {
    match kind {
        ContractKind::SingleAuthority => "single_authority".into(),
        ContractKind::UniqueOwner => "unique_owner".into(),
        ContractKind::ForbiddenDependency => "forbidden_dependency".into(),
        ContractKind::ProjectionOnly => "projection_only".into(),
        ContractKind::BoundedCompatibility => "bounded_compatibility".into(),
        ContractKind::ProviderBoundary => "provider_boundary".into(),
        ContractKind::Extension(k) => format!("extension:{}", k.as_str()),
    }
}

/// Derive the conformance vector from the delta signals (deterministic, by kind).
fn derive_vector(
    affected: &BTreeMap<ContractId, AffectedContract>,
    claims: &BTreeMap<ContractId, crate::architectural_contract::ArchitectureClaim>,
    contradictions: &[ContractId],
    not_evaluated: &[ContractId],
) -> ConformanceVector {
    use VectorStatus::*;

    // Dimension index: 0 authority, 1 ownership, 2 dependencies,
    //                  3 compatibility, 4 negative_paths.
    let mut dims: [Option<VectorStatus>; 5] = [None; 5];

    for (cid, aff) in affected {
        let dim = match &aff.contract_kind {
            Some(ContractKind::SingleAuthority) => 0,
            Some(ContractKind::UniqueOwner) => 1,
            Some(ContractKind::ForbiddenDependency) => 2,
            Some(ContractKind::BoundedCompatibility) => 3,
            Some(ContractKind::ProviderBoundary) => 4,
            // ProjectionOnly / Extension / unresolved do not map to a named
            // dimension in AC4. They still appear in the delta.
            _ => continue,
        };
        let status = if contradictions.contains(cid) {
            Contradicted
        } else if not_evaluated.contains(cid) {
            NotEvaluated
        } else {
            match claims.get(cid).map(|c| c.outcome()) {
                Some(ClaimOutcome::Verified) => Verified,
                Some(ClaimOutcome::Contradicted) => Contradicted,
                Some(ClaimOutcome::Stale) => Partial,
                Some(ClaimOutcome::Unknown) => Unknown,
                None => NotEvaluated,
            }
        };
        dims[dim] = Some(match dims[dim] {
            Some(current) => current.combine(status),
            None => status,
        });
    }

    let d = |i: usize| dims[i].unwrap_or(VectorStatus::NotEvaluated);
    ConformanceVector {
        authority: d(0),
        ownership: d(1),
        dependencies: d(2),
        compatibility: d(3),
        negative_paths: d(4),
        // Owned by AC7 / AC11; never evaluated in AC4.
        paradigm_alignment: VectorStatus::NotEvaluated,
        runtime_evidence: VectorStatus::NotEvaluated,
    }
}

/// sha256 over the canonical probe plan (REQ-AC4-012).
fn compute_plan_digest(affected: &BTreeMap<ContractId, AffectedContract>) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(PLAN_DIGEST_DOMAIN.as_bytes());
    for (cid, aff) in affected {
        h.update(cid.as_str().as_bytes());
        h.update(b"|kind|");
        match &aff.contract_kind {
            Some(k) => h.update(kind_tag(k).as_bytes()),
            None => h.update(b"<unresolved>"),
        }
        h.update(b"|units|");
        for u in &aff.triggering_units {
            h.update(u.0.as_bytes());
            h.update(b",");
        }
        h.update(b"|probes|");
        for p in &aff.probes {
            h.update(p.canonical_tag().as_bytes());
            h.update(b",");
        }
        h.update(b";");
    }
    h.finalize().into()
}

/// sha256 over the sorted supplied contract basis hashes (REQ-AC4-020).
///
/// Public since A3-S15: `FindingId`'s basis needs the same digest, and the
/// surfaces that print finding ids (`architecture findings`) deliberately compute
/// no AC4 delta. Exposing one function keeps AC4 and the finding basis from
/// drifting into two answers for one question.
pub fn contract_set_digest(contracts: &[ArchitecturalContract]) -> [u8; 32] {
    let mut index: BTreeMap<ContractId, &ArchitecturalContract> = BTreeMap::new();
    for c in contracts {
        index.insert(c.id().clone(), c);
    }
    compute_contract_set_digest(&index)
}

fn compute_contract_set_digest(
    contract_index: &BTreeMap<ContractId, &ArchitecturalContract>,
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(CONTRACT_SET_DIGEST_DOMAIN.as_bytes());
    for (cid, c) in contract_index {
        h.update(cid.as_str().as_bytes());
        h.update(b"|");
        h.update(c.basis_hash().to_hex().as_bytes());
        h.update(b";");
    }
    h.finalize().into()
}
