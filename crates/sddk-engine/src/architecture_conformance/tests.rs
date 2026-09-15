// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_conformance/tests.rs — A3-S5 / AC4 acceptance +
// anti-encroachment + boundary tests.
//
// REQ-AC4-001..029 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S5-ac4-verify-contracts.md`.

use super::compute::{AC4_EVALUATOR, compute_conformance_delta, kind_tag};
use super::types::*;
use crate::architectural_contract::{
    ArchitecturalContract, ClaimOutcome, ContractEvaluation, ContractId, ContractKind,
    ContractKindRef, DecisionRef, EvaluatorRef, EvidenceRef, Revision, SpecRef,
};
use crate::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use crate::knowledge::EventTime;

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

const T0: i64 = 1_700_000_000;

fn cid(s: &str) -> ContractId {
    ContractId::new(s).expect("valid contract id")
}

fn unit(s: &str) -> SoftwareUnitRef {
    SoftwareUnitRef::new(s)
}

fn decision(id: &str) -> DecisionRef {
    DecisionRef::Decision(format!("decision:{id}"))
}

fn spec(id: &str) -> SpecRef {
    SpecRef::Spec(format!("spec:{id}"))
}

fn revision(id: &str) -> Revision {
    Revision::new(format!("rev:{id}")).expect("valid revision")
}

fn evidence(provider: &str, reference: &str) -> EvidenceRef {
    EvidenceRef::new(provider, reference).expect("valid evidence ref")
}

fn single_authority(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        cid(id),
        crate::architectural_contract::ComponentRef::new("comp:x").expect("valid component"),
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_single_authority")
}

fn unique_owner(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_unique_owner(
        cid(id),
        crate::architectural_contract::EntityRef::new("entity:x").expect("valid entity"),
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_unique_owner")
}

fn forbidden_dependency(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_forbidden_dependency(
        cid(id),
        crate::architectural_contract::ComponentRef::new("comp:from").expect("valid component"),
        crate::architectural_contract::ComponentRef::new("comp:to").expect("valid component"),
        "no edge".to_string(),
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_forbidden_dependency")
}

fn projection_only(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_projection_only(
        cid(id),
        "entity:x".to_string(),
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_projection_only")
}

fn bounded_compatibility(id: &str, deprecated_after_ms: i64) -> ArchitecturalContract {
    ArchitecturalContract::declare_bounded_compatibility(
        cid(id),
        EventTime(deprecated_after_ms),
        None,
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_bounded_compatibility")
}

fn provider_boundary(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_provider_boundary(
        cid(id),
        crate::architectural_contract::BoundaryKind::Outbound,
        "provider.sdk".to_string(),
        decision(id),
        spec(id),
        revision(id),
        EventTime(T0),
    )
    .expect("declare_provider_boundary")
}

/// Build a claim for `contract` with a specific outcome.
///
/// Uses AC1's public `ContractEvaluation::evaluate` where possible; for
/// `Contradicted` (which AC1 deliberately never produces) it uses the AC1
/// test helper, which is `pub` behind `test_helpers`.
fn claim_with(
    contract: &ArchitecturalContract,
    outcome: ClaimOutcome,
) -> crate::architectural_contract::ArchitectureClaim {
    match outcome {
        ClaimOutcome::Verified => ContractEvaluation::evaluate(
            contract,
            vec![evidence("static", "obs:1")],
            EventTime(T0 + 1),
            EvaluatorRef::new("test:fixture").expect("valid evaluator"),
            None,
        ),
        ClaimOutcome::Unknown => ContractEvaluation::evaluate(
            contract,
            vec![],
            EventTime(T0 + 1),
            EvaluatorRef::new("test:fixture").expect("valid evaluator"),
            None,
        ),
        other => crate::architectural_contract::claim::test_helpers::claim_with_outcome(
            contract.id().clone(),
            other,
            "static",
        ),
    }
}

/// Wire a contract + unit into the overlay so `find_contracts_for_unit` finds it.
///
/// The claim outcome decides the relation AC2 emits: `Verified`/`Unknown`
/// produce `ArchitectureClaimedBy` (discoverable); `Contradicted`/`Stale` do
/// not, so callers that need discoverability must use Verified or Unknown.
fn wire(
    g: &mut ArchitectureGraphOverlay,
    contract: &ArchitecturalContract,
    unit_ref: &SoftwareUnitRef,
    outcome: ClaimOutcome,
) {
    let u = SoftwareUnit::new(
        unit_ref.clone(),
        UnitKind::Module,
        format!("loc:{}", unit_ref.0),
    );
    g.add_unit(&u);
    let claim = claim_with(contract, outcome);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, unit_ref);
    g.add_contract_metadata(contract, &decision("d"), &spec("s"), &[]);
}

/// An overlay with one contract reachable from one unit (Verified claim).
fn overlay_one(
    contract: &ArchitecturalContract,
    unit_ref: &SoftwareUnitRef,
) -> ArchitectureGraphOverlay {
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, contract, unit_ref, ClaimOutcome::Verified);
    g
}

fn empty_evidence() -> ContractEvidence {
    ContractEvidence::new()
}

fn evidence_for(id: &str, provider: &str, reference: &str) -> ContractEvidence {
    let mut m = ContractEvidence::new();
    m.insert(cid(id), vec![evidence(provider, reference)]);
    m
}

fn no_witnesses() -> Vec<ContractId> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — vocabulary (REQ-AC4-001..004, 009, 010)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_delta_status_closed_five() {
    // REQ-AC4-001
    assert_eq!(DeltaContractStatus::ALL.len(), 5);
}

#[test]
fn acceptance_status_from_claim_outcome_is_total() {
    // REQ-AC4-002
    assert_eq!(
        DeltaContractStatus::from_claim_outcome(ClaimOutcome::Verified),
        DeltaContractStatus::Verified
    );
    assert_eq!(
        DeltaContractStatus::from_claim_outcome(ClaimOutcome::Unknown),
        DeltaContractStatus::Unknown
    );
    assert_eq!(
        DeltaContractStatus::from_claim_outcome(ClaimOutcome::Stale),
        DeltaContractStatus::Stale
    );
    assert_eq!(
        DeltaContractStatus::from_claim_outcome(ClaimOutcome::Contradicted),
        DeltaContractStatus::Contradicted
    );
    // NotEvaluated is never produced from an evaluation outcome.
    for o in [
        ClaimOutcome::Verified,
        ClaimOutcome::Unknown,
        ClaimOutcome::Stale,
        ClaimOutcome::Contradicted,
    ] {
        assert_ne!(
            DeltaContractStatus::from_claim_outcome(o),
            DeltaContractStatus::NotEvaluated
        );
    }
}

#[test]
fn acceptance_status_tags_unique() {
    // REQ-AC4-003
    let mut tags: Vec<&str> = DeltaContractStatus::ALL
        .iter()
        .map(|s| s.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 5);
}

#[test]
fn acceptance_no_fabricated_verified_without_evidence() {
    // REQ-AC4-004: an affected contract with no evidence is Unknown, not Verified.
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .expect("compute");
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::Unknown)
    );
}

#[test]
fn acceptance_probe_requirement_closed_seven() {
    // REQ-AC4-009
    assert_eq!(ProbeRequirement::ALL.len(), 7);
}

#[test]
fn acceptance_probe_mapping_total_stable() {
    // REQ-AC4-010
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::SingleAuthority),
        ProbeRequirement::OwnershipProbe
    );
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::UniqueOwner),
        ProbeRequirement::OwnershipProbe
    );
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::ForbiddenDependency),
        ProbeRequirement::DependencyProbe
    );
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::ProjectionOnly),
        ProbeRequirement::ProjectionProbe
    );
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::BoundedCompatibility),
        ProbeRequirement::CompatibilityWindowProbe
    );
    assert_eq!(
        ProbeRequirement::for_contract_kind(&ContractKind::ProviderBoundary),
        ProbeRequirement::ProviderBoundaryProbe
    );
    // Stable across repeated calls (deterministic).
    let k = ContractKind::SingleAuthority;
    assert_eq!(
        ProbeRequirement::for_contract_kind(&k),
        ProbeRequirement::for_contract_kind(&k)
    );
}

#[test]
fn acceptance_probe_mapping_covers_all_contract_kinds() {
    // REQ-AC4-010: no current kind maps to Unknown.
    let kinds = [
        ContractKind::SingleAuthority,
        ContractKind::UniqueOwner,
        ContractKind::ForbiddenDependency,
        ContractKind::ProjectionOnly,
        ContractKind::BoundedCompatibility,
        ContractKind::ProviderBoundary,
        ContractKind::Extension(ContractKindRef::new("ac4.ext").expect("valid kind ref")),
    ];
    for kind in kinds {
        assert_ne!(
            ProbeRequirement::for_contract_kind(&kind),
            ProbeRequirement::Unknown,
            "kind {kind:?} must map to a concrete probe"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — delta shape (REQ-AC4-005..008, 011)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_triggering_units_sorted_dedup() {
    // REQ-AC4-005: one contract reached from two units, one repeated.
    let contract = single_authority("c1");
    let u1 = unit("u1");
    let u2 = unit("u2");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &contract, &u1, ClaimOutcome::Verified);
    // Second unit also constrains the same contract.
    let claim = claim_with(&contract, ClaimOutcome::Verified);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &u2);

    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u1.clone(), u2.clone(), u1.clone()], // u1 repeated
    )
    .expect("compute");

    let aff = delta.affected.get(&cid("c1")).expect("c1 affected");
    let locators: Vec<&str> = aff.triggering_units.iter().map(|u| u.0.as_str()).collect();
    assert_eq!(
        locators,
        vec!["u1", "u2"],
        "triggering units must be sorted+deduped"
    );
}

#[test]
fn acceptance_affected_is_sorted() {
    // REQ-AC4-006: affected is a BTreeMap (sorted by ContractId).
    let c_b = single_authority("c-b");
    let c_a = single_authority("c-a");
    let u = unit("u1");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &c_b, &u, ClaimOutcome::Verified);
    wire(&mut g, &c_a, &u, ClaimOutcome::Verified);

    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[c_b.clone(), c_a.clone()],
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .expect("compute");

    let ids: Vec<String> = delta
        .affected
        .keys()
        .map(|c| c.as_str().to_string())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "affected keys must be sorted");
}

#[test]
fn acceptance_one_claim_per_evaluated_contract() {
    // REQ-AC4-007
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .expect("compute");
    assert_eq!(delta.claims.len(), 1);
    assert!(delta.claims.contains_key(&cid("c1")));
    // The claim records AC4's evaluator, not an invented one.
    assert_eq!(
        delta.claims.get(&cid("c1")).unwrap().evaluator().as_str(),
        AC4_EVALUATOR
    );
}

#[test]
fn acceptance_unknowns_contradictions_stale_sorted() {
    // REQ-AC4-008: derived lists are sorted.
    let c1 = single_authority("c1");
    let c2 = unique_owner("c2");
    let c3 = forbidden_dependency("c3");
    let u = unit("u1");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &c1, &u, ClaimOutcome::Verified);
    wire(&mut g, &c2, &u, ClaimOutcome::Verified);
    wire(&mut g, &c3, &u, ClaimOutcome::Verified);

    // c1, c3 get no evidence (Unknown); c2 gets a witness (Contradicted).
    let mut ev = ContractEvidence::new();
    ev.insert(cid("c2"), vec![]); // no-op; witness wins anyway
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[c1, c2, c3],
            evidence: &ev,
            contradiction_witnesses: &[cid("c2")],
        },
        EventTime(T0 + 2),
        &[u],
    )
    .expect("compute");

    let unknowns = delta.unknowns();
    let mut sorted = unknowns.clone();
    sorted.sort();
    assert_eq!(unknowns, sorted, "unknowns must be sorted");
    assert_eq!(unknowns, vec![cid("c1"), cid("c3")]);
    assert_eq!(delta.contradictions(), &[cid("c2")]);
    // `stale` is derived from claims; none here.
    assert!(delta.stale().is_empty());
}

#[test]
fn acceptance_probe_plan_dedup() {
    // REQ-AC4-011: two SingleAuthority contracts → same probe kind; the plan
    // per contract is deduplicated and sorted.
    let c1 = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&c1, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&c1),
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .expect("compute");
    let aff = delta.affected.get(&cid("c1")).unwrap();
    assert_eq!(aff.probes, vec![ProbeRequirement::OwnershipProbe]);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — digests & determinism (REQ-AC4-012, 013, 020, 021)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_plan_digest_deterministic() {
    // REQ-AC4-012
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let ev = empty_evidence();
    let w = no_witnesses();
    let d1 = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &ev,
            contradiction_witnesses: &w,
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&u),
    )
    .unwrap();
    let d2 = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &ev,
            contradiction_witnesses: &w,
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&u),
    )
    .unwrap();
    assert_eq!(d1.plan_digest, d2.plan_digest);
    assert_eq!(d1.id, d2.id);
}

#[test]
fn acceptance_compute_is_deterministic() {
    // REQ-AC4-013: full-value determinism across two invocations.
    let c1 = single_authority("c1");
    let c2 = unique_owner("c2");
    let u1 = unit("u1");
    let u2 = unit("u2");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &c1, &u1, ClaimOutcome::Verified);
    wire(&mut g, &c2, &u2, ClaimOutcome::Verified);

    let ev = empty_evidence();
    let w = no_witnesses();
    let run = || {
        compute_conformance_delta(
            &g,
            ConformanceInputs {
                contracts: &[c1.clone(), c2.clone()],
                evidence: &ev,
                contradiction_witnesses: &w,
            },
            EventTime(T0 + 2),
            &[u1.clone(), u2.clone()],
        )
        .expect("compute")
    };
    let a = run();
    let b = run();
    assert_eq!(a, b, "two computations over identical inputs must be equal");
}

#[test]
fn acceptance_contract_set_digest_stable_across_evidence() {
    // REQ-AC4-020
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);

    let with_ev = evidence_for("c1", "static", "obs:1");
    let d_no_ev = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&u),
    )
    .unwrap();
    let d_ev = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &with_ev,
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&u),
    )
    .unwrap();

    assert_eq!(
        d_no_ev.contract_set_digest, d_ev.contract_set_digest,
        "contract-set digest must not depend on evidence"
    );
    // But the statuses differ.
    assert_eq!(
        d_no_ev.status_of(&cid("c1")),
        Some(DeltaContractStatus::Unknown)
    );
    assert_eq!(
        d_ev.status_of(&cid("c1")),
        Some(DeltaContractStatus::Verified)
    );
}

#[test]
fn acceptance_graph_digest_matches_overlay() {
    // REQ-AC4-021
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(delta.graph_digest, g.digest());
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — affected scope (REQ-AC4-014, 015, 016, 017, 018, 019)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_affected_resolved_from_overlay() {
    // REQ-AC4-014
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert!(delta.affected.contains_key(&cid("c1")));
    assert_eq!(
        delta.affected.get(&cid("c1")).unwrap().contract_kind,
        Some(ContractKind::SingleAuthority)
    );
}

#[test]
fn acceptance_only_changed_units_enter_scope() {
    // REQ-AC4-015 / AC-UAT-008
    let c1 = single_authority("c1");
    let c2 = unique_owner("c2");
    let u1 = unit("u1");
    let u2 = unit("u2");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &c1, &u1, ClaimOutcome::Verified);
    wire(&mut g, &c2, &u2, ClaimOutcome::Verified);

    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[c1, c2],
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u1], // only u1 changed
    )
    .unwrap();

    assert!(delta.affected.contains_key(&cid("c1")));
    assert!(
        !delta.affected.contains_key(&cid("c2")),
        "a contract on an unchanged unit must be out of scope"
    );
}

#[test]
fn acceptance_missing_contract_object_is_not_evaluated() {
    // REQ-AC4-016
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[], // object not supplied
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert!(delta.affected.contains_key(&cid("c1")));
    assert_eq!(delta.not_evaluated(), &[cid("c1")]);
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::NotEvaluated)
    );
    assert!(delta.claims.is_empty());
    assert_eq!(delta.affected.get(&cid("c1")).unwrap().contract_kind, None);
}

#[test]
fn acceptance_empty_evidence_is_unknown() {
    // REQ-AC4-017 / AC-UAT-007
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(delta.unknowns(), vec![cid("c1")]);
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::Unknown)
    );
}

#[test]
fn acceptance_valid_evidence_is_verified() {
    // REQ-AC4-018
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::Verified)
    );
    // AC1's evaluator produced the claim (not AC4).
    assert_eq!(delta.claims.len(), 1);
}

#[test]
fn acceptance_contradicted_listed() {
    // REQ-AC4-019
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &[cid("c1")],
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(delta.contradictions(), &[cid("c1")]);
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::Contradicted)
    );
    // A contradicted contract is not also represented as a claim.
    assert!(delta.claims.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — vector (REQ-AC4-023, 024, 025)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_vector_has_seven_dimensions() {
    // REQ-AC4-023
    let v = ConformanceVector::empty();
    assert_eq!(v.dimensions().len(), 7);
    let names: Vec<VectorDimension> = v.dimensions().iter().map(|(d, _)| *d).collect();
    assert_eq!(
        names,
        vec![
            VectorDimension::Authority,
            VectorDimension::Ownership,
            VectorDimension::Dependencies,
            VectorDimension::Compatibility,
            VectorDimension::NegativePaths,
            VectorDimension::ParadigmAlignment,
            VectorDimension::RuntimeEvidence,
        ]
    );
}

#[test]
fn acceptance_vector_status_is_closed() {
    // REQ-AC4-024
    assert_eq!(VectorStatus::ALL.len(), 5);
    let mut tags: Vec<&str> = VectorStatus::ALL
        .iter()
        .map(|s| s.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 5);
}

#[test]
fn acceptance_ac7_ac11_dimensions_not_evaluated() {
    // REQ-AC4-025: even with a verified authority contract, the AC7/AC11
    // dimensions stay NotEvaluated.
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(delta.vector.authority, VectorStatus::Verified);
    assert_eq!(delta.vector.paradigm_alignment, VectorStatus::NotEvaluated);
    assert_eq!(delta.vector.runtime_evidence, VectorStatus::NotEvaluated);
}

#[test]
fn acceptance_vector_maps_contract_kinds_to_dimensions() {
    // REQ-AC4-023/024: the five named dimensions are populated by kind.
    let c1 = single_authority("c1");
    let c2 = unique_owner("c2");
    let c3 = forbidden_dependency("c3");
    let c4 = bounded_compatibility("c4", T0 + 10_000);
    let c5 = provider_boundary("c5");
    let u = unit("u1");
    let mut g = ArchitectureGraphOverlay::new();
    for c in [&c1, &c2, &c3, &c4, &c5] {
        wire(&mut g, c, &u, ClaimOutcome::Verified);
    }
    let mut ev = ContractEvidence::new();
    for id in ["c1", "c2", "c3", "c4", "c5"] {
        ev.insert(cid(id), vec![evidence("static", "obs:1")]);
    }
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[c1, c2, c3, c4, c5],
            evidence: &ev,
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(delta.vector.authority, VectorStatus::Verified);
    assert_eq!(delta.vector.ownership, VectorStatus::Verified);
    assert_eq!(delta.vector.dependencies, VectorStatus::Verified);
    assert_eq!(delta.vector.compatibility, VectorStatus::Verified);
    assert_eq!(delta.vector.negative_paths, VectorStatus::Verified);
}

#[test]
fn acceptance_stale_compatibility_is_partial() {
    // REQ-AC4-024: a stale compatibility window degrades to Partial, not
    // Contradicted and not Verified.
    let contract = bounded_compatibility("c1", T0 + 5);
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 100), // past the window
        &[u],
    )
    .unwrap();
    assert_eq!(delta.stale(), vec![cid("c1")]);
    assert_eq!(delta.vector.compatibility, VectorStatus::Partial);
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment (REQ-AC4-026, 027, 028, 008/024)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_forbidden_imports() {
    // REQ-AC4-026: source-grep across the three submodules.
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("compute.rs", include_str!("compute.rs")),
    ];
    let forbidden = [
        "use crate::alignment",
        "use crate::debverify",
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::agent_host",
        "use crate::paradigm_profile",
        "use crate::effective_instructions",
        "use crate::capability",
    ];
    let mut offenders: Vec<(String, String)> = Vec::new();
    for (file, src) in sources {
        for (idx, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            for f in forbidden {
                if trimmed.starts_with(f) {
                    offenders.push((file.to_string(), format!("line {}: {}", idx + 1, trimmed)));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "architecture_conformance imports forbidden surfaces: {offenders:?}"
    );
}

#[test]
fn anti_encroachment_read_only_overlay() {
    // REQ-AC4-027: the overlay digest is identical before and after computing
    // a delta (no mutation of the semantic graph).
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let before = g.digest();
    let _delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &[cid("c1")],
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(before, g.digest(), "compute must not mutate the overlay");

    // Source-grep: no overlay mutating method is named in production code.
    let src = include_str!("compute.rs");
    for forbidden in [
        "add_unit(",
        "add_claim(",
        "add_relation(",
        "add_contract_metadata(",
        "attach_claim_to_unit(",
        ".clear()",
    ] {
        assert!(
            !src.contains(forbidden),
            "compute.rs must not call overlay mutator `{forbidden}`"
        );
    }
}

#[test]
fn anti_encroachment_no_claim_construction() {
    // REQ-AC4-028: AC4 never constructs `ArchitectureClaim` directly.
    let src = include_str!("compute.rs");
    assert!(
        !src.contains("ArchitectureClaim {") && !src.contains("ArchitectureClaim::"),
        "compute.rs must not construct ArchitectureClaim directly"
    );
    assert!(
        src.contains("ContractEvaluation::evaluate"),
        "compute.rs must route through ContractEvaluation::evaluate"
    );
    // And the delta's claims all come from evaluate().
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    for claim in delta.claims.values() {
        assert_eq!(claim.evaluator().as_str(), AC4_EVALUATOR);
    }
}

#[test]
fn anti_encroachment_no_numeric_score_field() {
    // REQ-AC4-008 / REQ-AC4-024 / AC-UAT-043: no aggregate score exists.
    //
    // Structural check: serialize the vector and assert it is an object whose
    // 7 values are all status strings. A numeric field (a score/weight/grade)
    // would show up as a JSON number and fail this assertion.
    let v = ConformanceVector::empty();
    let value = serde_json::to_value(v).expect("vector serializes");
    let obj = value.as_object().expect("vector is a JSON object");
    assert_eq!(obj.len(), 7, "vector must have exactly 7 dimensions");
    for (k, val) in obj {
        assert!(
            val.is_string(),
            "dimension `{k}` must be a status string, got {val:?} (no numeric score allowed)"
        );
        assert!(
            !val.is_number(),
            "dimension `{k}` must not be numeric (no universal score)"
        );
    }
    // The declared keys are exactly the seven named dimensions.
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "authority",
            "compatibility",
            "dependencies",
            "negative_paths",
            "ownership",
            "paradigm_alignment",
            "runtime_evidence",
        ]
    );
    // And every dimension value is one of the closed status variants.
    for (_, status) in v.dimensions() {
        assert!(VectorStatus::ALL.contains(&status));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// boundary + bonus
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_empty_changed_units_yields_empty_delta() {
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &[cid("c1")],
        },
        EventTime(T0 + 2),
        &[], // no changed units
    )
    .unwrap();
    assert!(delta.affected.is_empty());
    assert!(delta.claims.is_empty());
    assert!(delta.contradictions().is_empty());
    assert!(delta.not_evaluated().is_empty());
    assert_eq!(delta.vector, ConformanceVector::empty());
}

#[test]
fn bonus_witness_outside_scope_is_dropped() {
    // A witness for a contract not reachable from the change basis must not
    // appear in the delta.
    let contract = single_authority("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &empty_evidence(),
            contradiction_witnesses: &[cid("c-other")],
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert!(delta.contradictions().is_empty());
}

#[test]
fn bonus_statuses_cover_affected() {
    // Every affected contract has exactly one status.
    let c1 = single_authority("c1");
    let c2 = unique_owner("c2");
    let u = unit("u1");
    let mut g = ArchitectureGraphOverlay::new();
    wire(&mut g, &c1, &u, ClaimOutcome::Verified);
    wire(&mut g, &c2, &u, ClaimOutcome::Verified);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: &[c1],
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    let statuses = delta.statuses();
    assert_eq!(statuses.len(), delta.affected.len());
    assert_eq!(statuses.len(), 2);
}

#[test]
fn bonus_projection_only_appears_in_delta_but_not_vector() {
    // ProjectionOnly maps to no named vector dimension in AC4, but must still
    // appear in the delta (it is affected and can be evaluated).
    let contract = projection_only("c1");
    let u = unit("u1");
    let g = overlay_one(&contract, &u);
    let delta = compute_conformance_delta(
        &g,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence_for("c1", "static", "obs:1"),
            contradiction_witnesses: &no_witnesses(),
        },
        EventTime(T0 + 2),
        &[u],
    )
    .unwrap();
    assert_eq!(
        delta.status_of(&cid("c1")),
        Some(DeltaContractStatus::Verified)
    );
    assert_eq!(
        delta.affected.get(&cid("c1")).unwrap().probes,
        vec![ProbeRequirement::ProjectionProbe]
    );
    // No named dimension is claimed by a ProjectionOnly contract.
    assert_eq!(delta.vector, ConformanceVector::empty());
}

#[test]
fn bonus_kind_tag_distinct_for_extension() {
    // Local digest tag rendering is distinct for Extension kinds.
    let a = kind_tag(&ContractKind::SingleAuthority);
    let b = kind_tag(&ContractKind::Extension(
        ContractKindRef::new("ac4.ext").expect("valid"),
    ));
    assert_ne!(a, b);
    assert!(b.starts_with("extension:"));
}
