// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// A4-S15R — VerifiedBy Evidence Provenance — corpus.
//
// Cycle: p-63676b11dc0ef88f-a4-s15r-verifiedby-evidence-provenance
// Spec:  arch-spec-016/017 + ADR-0127
//
// Pin families (per §15 of the cycle scope contract):
//   §17 Evidence identity falsification
//   §18 Multi-evidence cardinality
//   §19 A3-S15 regression (SpecifiedBy, architecture_why)
//   §20 A4 regression (overlay unchanged + no coupling)
//
// Anti-encroachment pins ensure the cycle does NOT depend on AdvisoryContext,
// ContextCompiler, InstructionCompiler, AuthorityEngine, Capability,
// Governance, Alignment reducer, IntelligenceLoop composition, providers
// (CogniCode, Chronos, JCode).

use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::architecture_graph::overlay::ArchitectureGraphOverlay;
use sddk_engine::architecture_graph::types::{
    ArchitectureOverlayNodeKind, ArchitectureOverlayRelation, ArchitectureOverlayRelationKind,
    OverlayNodeRef, SoftwareUnit, SoftwareUnitRef, UnitKind, evidence_overlay_node_ref,
};
use sddk_engine::canonical_event_log::CasRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::knowledge::EventTime;
use sddk_engine::semantic_graph::SemanticGraphProjection;
use sddk_engine::semantic_kind::NodeKind;
use sddk_engine::semantic_node::NodeId;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_contract(id: &str, component: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        ContractId::new(id).expect("valid id"),
        ComponentRef::new(component).expect("valid component"),
        DecisionRef::Decision(format!("decision:{id}")),
        SpecRef::Spec(format!("spec:{id}")),
        Revision::new(format!("rev:{id}")).expect("valid revision"),
        EventTime(1_700_000_000),
    )
    .expect("declare_single_authority should succeed")
}

fn ev_planning(loc: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::Planning, loc)
}

fn ev_authority(loc: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::Authority, loc)
}

fn ev_decision_memory(loc: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::DecisionMemory, loc)
}

fn ev_governance(loc: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::Governance, loc)
}

fn ev_with_cas(kind: EvidenceKind, loc: &str, cas_bytes: &[u8]) -> EvidenceRef {
    EvidenceRef::new(kind, loc).with_cas(CasRef::from_bytes(cas_bytes))
}

fn overlay_with_metadata(
    contract: &ArchitecturalContract,
    verified_by: &[EvidenceRef],
) -> ArchitectureGraphOverlay {
    let mut g = ArchitectureGraphOverlay::new();
    g.add_contract_metadata(
        contract,
        contract.decided_by(),
        contract.specified_by(),
        verified_by,
    );
    g
}

fn verified_by_edges(overlay: &ArchitectureGraphOverlay) -> Vec<(NodeId, NodeId)> {
    let wanted = ArchitectureOverlayRelationKind::VerifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    overlay
        .projection()
        .relations()
        .into_iter()
        .filter(|r| r.kind == wanted)
        .map(|r| (r.from.clone(), r.to.clone()))
        .collect()
}

fn specified_by_edges(overlay: &ArchitectureGraphOverlay) -> Vec<(NodeId, NodeId)> {
    let wanted = ArchitectureOverlayRelationKind::SpecifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    overlay
        .projection()
        .relations()
        .into_iter()
        .filter(|r| r.kind == wanted)
        .map(|r| (r.from.clone(), r.to.clone()))
        .collect()
}

fn decided_by_edges(overlay: &ArchitectureGraphOverlay) -> Vec<(NodeId, NodeId)> {
    let wanted = ArchitectureOverlayRelationKind::DecidedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    overlay
        .projection()
        .relations()
        .into_iter()
        .filter(|r| r.kind == wanted)
        .map(|r| (r.from.clone(), r.to.clone()))
        .collect()
}

/// Look up the NodeKind of the node that owns this NodeId. Panics if
/// missing — the projection must always be self-consistent for a single
/// overlay construction, so a missing id is a real bug.
fn kind_of(overlay: &ArchitectureGraphOverlay, id: &NodeId) -> NodeKind {
    overlay
        .projection()
        .nodes()
        .into_iter()
        .find(|n| n.id == *id)
        .unwrap_or_else(|| panic!("missing node for id={}", id.as_str()))
        .kind
}

fn evidence_kind_tag() -> NodeKind {
    NodeKind::parse(ArchitectureOverlayNodeKind::EvidenceRef.domain_tag())
        .expect("static tag is well-formed")
}

fn spec_kind_tag() -> NodeKind {
    NodeKind::parse(ArchitectureOverlayNodeKind::SpecRef.domain_tag())
        .expect("static tag is well-formed")
}

fn contract_anchor_id(contract: &ArchitecturalContract) -> NodeId {
    let kind = NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
        .expect("static tag is well-formed");
    NodeId::new(&kind, &format!("contract:{}", contract.id().as_str()))
}

// ─────────────────────────────────────────────────────────────────────────────
// §17 Evidence identity falsification
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s17_same_evidence_ref_yields_one_node() {
    // Two EvidenceRefs with identical (kind, locator, cas) MUST project to
    // the same NodeId. Inserting the same ref twice via the same contract
    // must produce exactly one VerifiedBy edge and one evidence node.
    let contract = make_contract("c-identity-1", "comp:a");
    let e = ev_planning("workitem/W-1");
    // Intentional duplication: this slice is the dedup target of the test.
    #[allow(clippy::cloned_ref_to_slice_refs)]
    let g = overlay_with_metadata(&contract, &[e.clone(), e.clone()]);
    let edges = verified_by_edges(&g);
    assert_eq!(
        edges.len(),
        1,
        "duplicate EvidenceRef must dedup to one edge"
    );

    let from = contract_anchor_id(&contract);
    let (_, to_id) = &edges[0];
    assert_eq!(kind_of(&g, to_id), evidence_kind_tag());
    assert!(
        g.projection().nodes().into_iter().any(|n| n.id == *to_id),
        "the evidence node must exist in the projection"
    );
    assert_eq!(
        edges[0].0, from,
        "VerifiedBy must originate at the contract anchor"
    );
}

#[test]
fn s17_different_evidence_kind_yields_different_nodes() {
    // Planning("x") vs Authority("x"): same locator, different kind →
    // distinct nodes, distinct VerifiedBy edges.
    let contract = make_contract("c-kind", "comp:a");
    let planning = ev_planning("shared-locator");
    let authority = ev_authority("shared-locator");
    let g = overlay_with_metadata(&contract, &[planning, authority]);
    let edges = verified_by_edges(&g);
    assert_eq!(
        edges.len(),
        2,
        "two distinct EvidenceRefs must yield two edges"
    );

    let from = contract_anchor_id(&contract);
    let mut to_ids: Vec<NodeId> = edges.iter().map(|(_, to)| to.clone()).collect();
    to_ids.sort();
    to_ids.dedup();
    assert_eq!(
        to_ids.len(),
        2,
        "the two evidence nodes must have distinct ids"
    );

    for edge in &edges {
        assert_eq!(
            edge.0, from,
            "VerifiedBy must originate at the contract anchor"
        );
        assert_eq!(kind_of(&g, &edge.1), evidence_kind_tag());
    }
}

#[test]
fn s17_different_cas_yields_different_nodes() {
    // Planning("x", cas-A) vs Planning("x", cas-B): same kind+locator,
    // different CAS → distinct nodes.
    let contract = make_contract("c-cas", "comp:a");
    let with_cas_a = ev_with_cas(EvidenceKind::Planning, "shared", b"payload-A");
    let with_cas_b = ev_with_cas(EvidenceKind::Planning, "shared", b"payload-B");
    let g = overlay_with_metadata(&contract, &[with_cas_a, with_cas_b]);
    let edges = verified_by_edges(&g);
    assert_eq!(
        edges.len(),
        2,
        "distinct CAS must yield distinct nodes/edges"
    );

    let mut to_ids = edges.iter().map(|(_, to)| to.clone()).collect::<Vec<_>>();
    to_ids.sort();
    to_ids.dedup();
    assert_eq!(to_ids.len(), 2);
}

#[test]
fn s17_insertion_order_is_irrelevant_for_digest() {
    // The canonical_bytes digest must be identical regardless of insertion
    // order. This is the canonicalisation pin.
    let contract = make_contract("c-order", "comp:a");
    let e1 = ev_planning("plan-1");
    let e2 = ev_authority("auth-1");
    let e3 = ev_decision_memory("dm-1");

    let mut g_a = ArchitectureGraphOverlay::new();
    g_a.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[e1.clone(), e2.clone(), e3.clone()],
    );

    let mut g_b = ArchitectureGraphOverlay::new();
    g_b.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[e3.clone(), e1.clone(), e2.clone()],
    );

    let mut g_c = ArchitectureGraphOverlay::new();
    g_c.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[e2.clone(), e3.clone(), e1.clone()],
    );

    assert_eq!(
        g_a.digest(),
        g_b.digest(),
        "digest must be insertion-order-independent"
    );
    assert_eq!(
        g_a.digest(),
        g_c.digest(),
        "digest must be insertion-order-independent"
    );

    let edges_a = verified_by_edges(&g_a);
    let edges_b = verified_by_edges(&g_b);
    let mut a_to: Vec<String> = edges_a
        .iter()
        .map(|(_, t)| t.as_str().to_string())
        .collect();
    let mut b_to: Vec<String> = edges_b
        .iter()
        .map(|(_, t)| t.as_str().to_string())
        .collect();
    a_to.sort();
    b_to.sort();
    assert_eq!(a_to, b_to, "edge `to` ids must match modulo order");
    assert_eq!(edges_a.len(), 3);
}

// ─────────────────────────────────────────────────────────────────────────────
// §18 Multi-evidence cardinality
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s18_zero_evidence_emits_no_verified_by_edge() {
    // Pre-A4-S15R this was the only observable state (all real producers
    // passed empty evidence). Pin: empty verified_by ⇒ no VerifiedBy edge.
    let contract = make_contract("c-zero", "comp:a");
    let g = overlay_with_metadata(&contract, &[]);
    let edges = verified_by_edges(&g);
    assert!(
        edges.is_empty(),
        "empty verified_by must emit no VerifiedBy edge"
    );
    // But SpecifiedBy still emitted (A3-S15 regression).
    assert_eq!(specified_by_edges(&g).len(), 1);
    assert_eq!(decided_by_edges(&g).len(), 1);
}

#[test]
fn s18_one_evidence_emits_exactly_one_verified_by_edge() {
    let contract = make_contract("c-one", "comp:a");
    let g = overlay_with_metadata(&contract, &[ev_planning("p1")]);
    let edges = verified_by_edges(&g);
    assert_eq!(edges.len(), 1);
}

#[test]
fn s18_n_evidence_emits_n_verified_by_edges() {
    let contract = make_contract("c-n", "comp:a");
    let refs = vec![
        ev_planning("p1"),
        ev_authority("a1"),
        ev_decision_memory("d1"),
        ev_governance("g1"),
        ev_planning("p2"),
    ];
    let g = overlay_with_metadata(&contract, &refs);
    let edges = verified_by_edges(&g);
    assert_eq!(edges.len(), 5);
}

#[test]
fn s18_duplicate_typed_ref_dedups_to_one_edge() {
    let contract = make_contract("c-dup", "comp:a");
    let r = ev_planning("p1");
    let g = overlay_with_metadata(&contract, &[r.clone(), r.clone(), r.clone()]);
    let edges = verified_by_edges(&g);
    assert_eq!(edges.len(), 1, "typed-equal EvidenceRefs must dedup");
}

#[test]
fn s18_no_verified_by_targets_spec_node() {
    // Falsification: a VerifiedBy edge MUST NOT terminate at the spec node.
    // SpecifiedBy targets spec; VerifiedBy targets evidence. Two distinct
    // destination kinds.
    let contract = make_contract("c-distinct", "comp:a");
    let g = overlay_with_metadata(&contract, &[ev_planning("p1")]);
    let spec_kind = spec_kind_tag();
    let ev_kind = evidence_kind_tag();

    let wanted_v = ArchitectureOverlayRelationKind::VerifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    let wanted_s = ArchitectureOverlayRelationKind::SpecifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");

    let relations: Vec<_> = g.projection().relations();
    for rel in &relations {
        if rel.kind == wanted_v {
            assert_eq!(
                kind_of(&g, &rel.to),
                ev_kind.clone(),
                "VerifiedBy must terminate at an evidence node"
            );
            assert_ne!(
                kind_of(&g, &rel.to),
                spec_kind.clone(),
                "VerifiedBy must NOT terminate at a spec node (A4-S15R correction)"
            );
        } else if rel.kind == wanted_s {
            assert_eq!(
                kind_of(&g, &rel.to),
                spec_kind.clone(),
                "SpecifiedBy must terminate at the spec node"
            );
            assert_ne!(
                kind_of(&g, &rel.to),
                ev_kind.clone(),
                "SpecifiedBy must NOT terminate at an evidence node"
            );
        }
    }
}

#[test]
fn s18_evidence_node_carries_typed_identity_props() {
    // The evidence node must carry the typed identity on its props so a
    // future WHY traversal can recover the EvidenceRef without re-parsing
    // the locator.
    let contract = make_contract("c-props", "comp:a");
    let e = ev_with_cas(EvidenceKind::Planning, "workitem/W-9", b"content-bytes");
    let g = overlay_with_metadata(&contract, std::slice::from_ref(&e));

    let ev_kind = evidence_kind_tag();
    let target_locator = format!("evidence:{}", e.ordering_key());
    let target_id = NodeId::new(&ev_kind, &target_locator);
    let node = g
        .projection()
        .nodes()
        .into_iter()
        .find(|n| n.id == target_id)
        .expect("evidence node must exist");

    assert_eq!(
        node.props_inline.get("evidence_kind").map(|s| s.as_str()),
        Some(EvidenceKind::Planning.domain_tag())
    );
    assert_eq!(
        node.props_inline
            .get("evidence_locator")
            .map(|s| s.as_str()),
        Some("workitem/W-9")
    );
    assert!(node.props_inline.contains_key("evidence_cas"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §19 A3-S15 regression
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s19_specified_by_survives_with_empty_evidence() {
    // A3-S15 regression: SpecifiedBy (contract → spec) is emitted even when
    // there is no verification evidence. The whole point of A3-S15 was to
    // make this edge unconditional.
    let contract = make_contract("c-reg-1", "comp:a");
    let g = overlay_with_metadata(&contract, &[]);
    let edges = specified_by_edges(&g);
    assert_eq!(
        edges.len(),
        1,
        "SpecifiedBy must be emitted unconditionally"
    );

    let from = contract_anchor_id(&contract);
    let (_, to) = &edges[0];
    assert_eq!(edges[0].0, from);
    assert_eq!(kind_of(&g, to), spec_kind_tag());
}

#[test]
fn s19_specified_by_survives_with_evidence() {
    // Both SpecifiedBy and VerifiedBy must coexist on the same contract.
    let contract = make_contract("c-reg-2", "comp:a");
    let g = overlay_with_metadata(&contract, &[ev_planning("p1")]);
    assert_eq!(specified_by_edges(&g).len(), 1);
    assert_eq!(verified_by_edges(&g).len(), 1);
}

#[test]
fn s19_two_provenance_axes_are_distinct_destinations() {
    // SpecifiedBy → spec node; VerifiedBy → evidence node. Distinct.
    let contract = make_contract("c-axes", "comp:a");
    let g = overlay_with_metadata(&contract, &[ev_planning("p1")]);
    let wanted_s = ArchitectureOverlayRelationKind::SpecifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    let wanted_v = ArchitectureOverlayRelationKind::VerifiedBy
        .as_relation_kind()
        .expect("static tag is well-formed");
    let from = contract_anchor_id(&contract);

    let mut s_to: Option<NodeId> = None;
    let mut v_to: Option<NodeId> = None;
    for rel in g.projection().relations() {
        if rel.from != from {
            continue;
        }
        if rel.kind == wanted_s {
            s_to = Some(rel.to.clone());
        } else if rel.kind == wanted_v {
            v_to = Some(rel.to.clone());
        }
    }
    let s_to = s_to.expect("SpecifiedBy from contract must exist");
    let v_to = v_to.expect("VerifiedBy from contract must exist");
    assert_ne!(s_to, v_to, "the two axes must target distinct nodes");
    assert_eq!(kind_of(&g, &s_to), spec_kind_tag());
    assert_eq!(kind_of(&g, &v_to), evidence_kind_tag());
}

#[test]
fn s19_provenance_reachability_contract_to_evidence() {
    // Future WHY reachability: from the contract anchor, traverse the
    // VerifiedBy edge to a typed evidence node. This demonstrates the
    // projection contains sufficient provenance even though no WHY engine
    // is built in this cycle.
    let contract = make_contract("c-reach", "comp:a");
    let e = ev_with_cas(EvidenceKind::Authority, "auth/RC-7", b"receipt-payload");
    let g = overlay_with_metadata(&contract, std::slice::from_ref(&e));

    let from = contract_anchor_id(&contract);
    let ev_kind = evidence_kind_tag();
    let target_id = NodeId::new(&ev_kind, &format!("evidence:{}", e.ordering_key()));

    let hit = g
        .projection()
        .relations()
        .into_iter()
        .find(|r| {
            r.from == from
                && r.kind
                    == ArchitectureOverlayRelationKind::VerifiedBy
                        .as_relation_kind()
                        .unwrap()
                && r.to == target_id
        })
        .expect("must reach the evidence node via a single VerifiedBy edge");
    assert_eq!(kind_of(&g, &hit.to), ev_kind);
}

#[test]
fn s19_evidence_overlay_node_ref_helper_agrees_with_overlay_kind() {
    // Sanity: the public helper for the typed overlay node ref must agree
    // with the closed enum used by add_contract_metadata. If they diverge
    // in a future refactor, this fails closed at compile-time.
    let e = ev_planning("p1");
    let r = evidence_overlay_node_ref(&e);
    assert!(matches!(r, OverlayNodeRef::Evidence(_)));
}

// ─────────────────────────────────────────────────────────────────────────────
// §20 A4 regression: overlay is unchanged in semantics for callers that
// don't pass evidence, and no coupling to Authority/Capability/Advisory
// subsystems.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s20_no_advisory_context_dependency_in_overlay() {
    // Anti-encroachment: the overlay crate MUST NOT pull in AdvisoryContext
    // or any A4-5b surface. We assert that the overlay types module does
    // not contain references to those names.
    //
    // This is a compile-time + grep-time pin: any future refactor that
    // imports AdvisoryContext into the overlay will fail this test.
    let overlay_rs = include_str!("../src/architecture_graph/overlay.rs");
    let types_rs = include_str!("../src/architecture_graph/types.rs");
    for forbidden in [
        "AdvisoryContext",
        "ContextCompiler",
        "InstructionCompiler",
        "AuthorityEngine",
        "IntelligenceLoop",
        "compose_intelligence_loop",
    ] {
        assert!(
            !overlay_rs.contains(forbidden),
            "overlay.rs must not depend on {forbidden} (A4-S15R anti-encroachment)"
        );
        assert!(
            !types_rs.contains(forbidden),
            "types.rs must not depend on {forbidden} (A4-S15R anti-encroachment)"
        );
    }
}

#[test]
fn s20_digest_is_stable_across_repeated_construction() {
    // Determinism pin: building the overlay twice from the same inputs must
    // yield identical canonical bytes.
    let contract = make_contract("c-det", "comp:a");
    let refs = vec![
        ev_planning("p1"),
        ev_authority("a1"),
        ev_decision_memory("dm1"),
    ];

    let mut a = ArchitectureGraphOverlay::new();
    a.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &refs,
    );

    let mut b = ArchitectureGraphOverlay::new();
    b.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &refs,
    );

    assert_eq!(
        a.digest(),
        b.digest(),
        "two constructions over identical inputs must be identical"
    );
    assert_eq!(a.graph_revision(), b.graph_revision());
}

#[test]
fn s20_overlay_node_kind_includes_evidence_ref() {
    // Pin the enum extension: ALL grows from 8 to 9 (A4-S15R).
    let mut tags: Vec<&str> = ArchitectureOverlayNodeKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    tags.sort();
    tags.dedup();
    assert_eq!(
        tags.len(),
        ArchitectureOverlayNodeKind::ALL.len(),
        "all overlay node kinds must have unique tags"
    );
    assert!(ArchitectureOverlayNodeKind::ALL.contains(&ArchitectureOverlayNodeKind::EvidenceRef));
}

#[test]
fn s20_clear_then_reproject_is_idempotent() {
    // Clearing and rebuilding the overlay must yield the same digest.
    let contract = make_contract("c-clear", "comp:a");
    let refs = vec![ev_planning("p1"), ev_authority("a1")];

    let mut g = ArchitectureGraphOverlay::new();
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &refs,
    );
    let baseline = g.digest();
    g.clear();
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &refs,
    );
    assert_eq!(
        g.digest(),
        baseline,
        "clear + re-project must yield the same digest"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §15 ArchitectureClaimedBy ≠ VerifiedBy
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s15_architecture_claimed_by_and_verified_by_are_distinct() {
    // A claim observes/evaluates a contract or software subject;
    // evidence backs a verification. The two relations must stay distinct:
    //   ArchitectureClaimedBy: claim node → unit node
    //   VerifiedBy:            contract anchor → evidence node
    let contract = make_contract("c-claim", "comp:test");
    let e = ev_planning("p1");
    let mut g = ArchitectureGraphOverlay::new();
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        std::slice::from_ref(&e),
    );

    // Build a real Verified claim and attach it.
    let claim = sddk_engine::architectural_contract::ContractEvaluation::evaluate(
        &contract,
        vec![sddk_engine::architectural_contract::EvidenceRef::new("static", "w").unwrap()],
        EventTime(1_700_000_001),
        sddk_engine::architectural_contract::EvaluatorRef::new("test:a4s15r").unwrap(),
        None,
    );
    let claim_id = g.add_claim(&claim);
    let unit_ref = SoftwareUnitRef::new("unit:static");
    g.add_unit(&SoftwareUnit::new(
        unit_ref.clone(),
        UnitKind::Module,
        "src/static.rs",
    ));
    g.attach_claim_to_unit(&claim, &claim_id, &unit_ref);

    let wanted_claimed = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
        .as_relation_kind()
        .expect("static tag");
    let wanted_verified = ArchitectureOverlayRelationKind::VerifiedBy
        .as_relation_kind()
        .expect("static tag");
    assert_ne!(
        wanted_claimed, wanted_verified,
        "the two relations must have distinct kind tags"
    );

    let relations: Vec<_> = g.projection().relations();
    let mut saw_claimed = false;
    let mut saw_verified = false;
    for rel in &relations {
        if rel.kind == wanted_verified {
            saw_verified = true;
            assert_eq!(
                kind_of(&g, &rel.to),
                evidence_kind_tag(),
                "VerifiedBy must terminate at an evidence node"
            );
        }
        if rel.kind == wanted_claimed {
            saw_claimed = true;
            let to_kind = kind_of(&g, &rel.to);
            assert_ne!(
                to_kind,
                evidence_kind_tag(),
                "ArchitectureClaimedBy must NOT terminate at an evidence node"
            );
        }
    }
    assert!(
        saw_claimed,
        "the claim must have produced an ArchitectureClaimedBy edge"
    );
    assert!(
        saw_verified,
        "the evidence must have produced a VerifiedBy edge"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §16 Rebuild: stale VerifiedBy(contract→spec) cannot survive clear+rebuild
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s16_stale_verified_by_contract_to_spec_cannot_survive_rebuild() {
    // The overlay is a PROJECTION — no persistent migration. Construct the
    // pre-A4-S15R legacy shape by hand (contract → spec, VerifiedBy),
    // mutate via clear(), then re-project with the correct producer. The
    // stale edge must not survive.
    let contract = make_contract("c-stale", "comp:a");
    let e = ev_planning("p1");

    let mut g = ArchitectureGraphOverlay::new();
    // First, emit the correct shape once to materialise the contract/spec nodes.
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[],
    );

    // Hand-inject the stale legacy edge: VerifiedBy(contract → spec).
    let stale_rel = ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new(format!(
            "contract:{}",
            contract.id().as_str()
        ))),
        to: OverlayNodeRef::Spec(contract.specified_by().clone()),
        kind: ArchitectureOverlayRelationKind::VerifiedBy,
        evidence: Vec::new(),
    };
    g.add_relation(&stale_rel);
    let stale = verified_by_edges(&g);
    assert_eq!(
        stale.len(),
        1,
        "the hand-injected legacy edge must be present"
    );
    assert_eq!(kind_of(&g, &stale[0].1), spec_kind_tag());

    // Rebuild: clear + re-project with the corrected producer.
    g.clear();
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        std::slice::from_ref(&e),
    );

    let rebuilt = verified_by_edges(&g);
    assert_eq!(rebuilt.len(), 1, "one evidence edge after rebuild");
    assert_eq!(
        kind_of(&g, &rebuilt[0].1),
        evidence_kind_tag(),
        "after rebuild VerifiedBy must terminate at an evidence node"
    );
    for (_, to) in &rebuilt {
        assert_ne!(
            kind_of(&g, to),
            spec_kind_tag(),
            "no stale VerifiedBy(contract → spec) may survive clear+rebuild"
        );
    }
}
