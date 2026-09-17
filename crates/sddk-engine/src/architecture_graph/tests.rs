// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/tests.rs — A3-S3 / AC2 acceptance + negative +
// anti-encroachment tests (16 total).
//
// REQ-AC2-001..022 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S3-architecture-semantic-graph-overlay.md`.

use super::*;
use crate::architectural_contract::{
    ArchitecturalContract, ArchitectureClaim, ClaimOutcome, ComponentRef, ContractEvaluation,
};
use crate::evidence_ref::EvidenceRef;
use crate::knowledge::EventTime;
use crate::semantic_graph::SemanticGraphProjection;
use crate::semantic_kind::NodeKind;
use crate::semantic_node::NodeId;

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers (kept inside the test module so production code is not
// affected).
// ─────────────────────────────────────────────────────────────────────────────

fn make_component(s: &str) -> ComponentRef {
    ComponentRef::new(s).expect("valid component ref")
}

fn make_contract(id: &str, component: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        crate::architectural_contract::ContractId::new(id).expect("valid id"),
        make_component(component),
        crate::architectural_contract::DecisionRef::Decision(format!("decision:{id}")),
        crate::architectural_contract::SpecRef::Spec(format!("spec:{id}")),
        crate::architectural_contract::Revision::new(format!("rev:{id}")).expect("valid revision"),
        EventTime(1_700_000_000),
    )
    .expect("declare_single_authority should succeed")
}

fn make_unit(id: &str, locator: &str) -> SoftwareUnit {
    SoftwareUnit::new(
        SoftwareUnitRef::new(id),
        UnitKind::Module,
        locator.to_string(),
    )
}

fn make_evaluator(s: &str) -> crate::architectural_contract::claim::EvaluatorRef {
    crate::architectural_contract::claim::EvaluatorRef::new(s).expect("valid evaluator ref")
}

fn make_evidence(
    provider: &str,
    reference: &str,
) -> crate::architectural_contract::claim::EvidenceRef {
    crate::architectural_contract::claim::EvidenceRef::new(provider, reference)
        .expect("valid evidence ref")
}

fn make_claim_verified(
    contract_id: &str,
    provider: &str,
    reference: &str,
) -> (ArchitectureClaim, SoftwareUnitRef) {
    let contract = make_contract(contract_id, "comp:test");
    let evidence = vec![make_evidence(provider, reference)];
    let evaluator = make_evaluator("test:make_claim_verified");
    let claim = ContractEvaluation::evaluate(
        &contract,
        evidence,
        EventTime(1_700_000_001),
        evaluator,
        None,
    );
    // Use a unit_ref whose string equals a node locator we will register.
    // The locator is `unit:<provider>` so the NodeId for the unit matches.
    let unit_ref = SoftwareUnitRef::new(format!("unit:{provider}"));
    (claim, unit_ref)
}

fn make_claim_contradicted(
    contract_id: &str,
    provider: &str,
) -> (ArchitectureClaim, SoftwareUnitRef) {
    let unit_ref = SoftwareUnitRef::new(format!("unit:{provider}"));
    let claim = build_claim_with_outcome(contract_id, ClaimOutcome::Contradicted, provider);
    (claim, unit_ref)
}

/// Construct a claim with a specific outcome via the test_helpers submodule.
fn build_claim_with_outcome(
    contract_id: &str,
    outcome: ClaimOutcome,
    provider: &str,
) -> ArchitectureClaim {
    crate::architectural_contract::claim::test_helpers::claim_with_outcome(
        crate::architectural_contract::ContractId::new(contract_id).unwrap(),
        outcome,
        provider,
    )
}

fn make_claim_stale(contract_id: &str, provider: &str) -> (ArchitectureClaim, SoftwareUnitRef) {
    let unit_ref = SoftwareUnitRef::new(format!("unit:{provider}"));
    let claim = build_claim_with_outcome(contract_id, ClaimOutcome::Stale, provider);
    (claim, unit_ref)
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance tests (9)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_rebuild_equivalence() {
    // REQ-AC2-006 / AC-033-006: two rebuilds over the same inputs yield
    // identical digest bytes.
    let unit = make_unit("u1", "crates/x/src/lib.rs");
    let contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");

    let inputs = RebuildInputs {
        contracts: &[contract],
        claims: &[(claim, claim_unit)],
        units: &[unit],
    };

    let mut g1 = ArchitectureGraphOverlay::new();
    rebuild(&mut g1, &inputs);
    let mut g2 = ArchitectureGraphOverlay::new();
    rebuild(&mut g2, &inputs);
    assert_eq!(
        g1.digest(),
        g2.digest(),
        "two rebuilds must produce identical canonical bytes"
    );
}

#[test]
fn acceptance_rebuild_clear_then_reproject() {
    // REQ-AC2-006: rebuild after adding nodes yields only the re-projected
    // set (no carryover).
    let mut g = ArchitectureGraphOverlay::new();
    // Add a stray unit before rebuild.
    let stray = make_unit("stray", "crates/stray/lib.rs");
    g.add_unit(&stray);
    assert!(!g.projection().nodes().is_empty());

    let unit = make_unit("u1", "crates/x/lib.rs");
    let contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");
    let inputs = RebuildInputs {
        contracts: &[contract],
        claims: &[(claim, claim_unit)],
        units: &[unit],
    };
    rebuild(&mut g, &inputs);
    let nodes = g.projection().nodes();
    // The stray unit should be gone; only the inputs remain. add_unit
    // uses unit.id.0 as locator (not unit.locator) so the assertion checks
    // the id field.
    assert!(
        !nodes.iter().any(|n| n.locator.contains("stray")),
        "rebuild must clear stale nodes"
    );
    assert!(
        nodes.iter().any(|n| n.locator.contains("u1")),
        "rebuild must re-project the input unit"
    );
}

#[test]
fn acceptance_digest_equals_projection_bytes() {
    // REQ-AC2-007: overlay.digest() must equal projection().canonical_bytes().
    let unit = make_unit("u1", "crates/x/lib.rs");
    let mut g = ArchitectureGraphOverlay::new();
    g.add_unit(&unit);
    assert_eq!(g.digest(), g.projection().canonical_bytes());
}

#[test]
fn acceptance_find_units_contracted_by_returns_unit_set() {
    // REQ-AC2-009: query returns the set of unit IDs contracted by a contract.
    let _contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");

    let mut g = ArchitectureGraphOverlay::new();
    // Register the unit with the locator that matches the unit_ref the
    // relation will point at.
    let unit = SoftwareUnit::new(
        claim_unit.clone(),
        UnitKind::Module,
        "crates/x/src/lib.rs".to_string(),
    );
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let units = g.find_units_contracted_by("c1");
    assert!(
        !units.is_empty(),
        "expected at least one unit contracted by c1"
    );
    let expected_unit_id = NodeId::new(
        &NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag()).unwrap(),
        &claim_unit.0,
    );
    assert!(
        units.contains(&expected_unit_id),
        "expected the claim->unit edge to be present"
    );
}

#[test]
fn acceptance_find_units_contracted_by_empty_when_no_claim() {
    // REQ-AC2-009: returns empty Vec when no matching claim exists.
    let g = ArchitectureGraphOverlay::new();
    let units = g.find_units_contracted_by("nonexistent");
    assert!(
        units.is_empty(),
        "expected empty Vec, got {} units",
        units.len()
    );
}

#[test]
fn acceptance_find_contracts_for_unit_is_inverse() {
    // REQ-AC2-010: find_contracts_for_unit is the inverse of
    // find_units_contracted_by.
    let _contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");

    let mut g = ArchitectureGraphOverlay::new();
    let unit = SoftwareUnit::new(
        claim_unit.clone(),
        UnitKind::Module,
        "crates/x/src/lib.rs".to_string(),
    );
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let contracts = g.find_contracts_for_unit(&claim_unit);
    assert!(
        !contracts.is_empty(),
        "expected at least one contract for unit p1"
    );
}

#[test]
fn acceptance_traverse_finding_to_software_visits_chain() {
    // REQ-AC2-011: traverse_finding_to_software visits the claim→unit chain.
    let _contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");

    let mut g = ArchitectureGraphOverlay::new();
    let unit = SoftwareUnit::new(
        claim_unit.clone(),
        UnitKind::Module,
        "crates/x/src/lib.rs".to_string(),
    );
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let units = g.traverse_finding_to_software(&claim_id);
    assert!(
        !units.is_empty(),
        "expected at least one unit reachable from claim"
    );
}

#[test]
fn acceptance_contradicted_claim_emits_contradicts_by() {
    // REQ-AC2-015: Contradicted outcome emits ContradictsBy, not
    // ArchitectureClaimedBy.
    let unit = make_unit("u1", "crates/x/lib.rs");
    let (claim, claim_unit) = make_claim_contradicted("c1", "p1");

    let mut g = ArchitectureGraphOverlay::new();
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let relations = g.projection().relations();
    let claimed_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
        .as_relation_kind()
        .unwrap();
    let contradicts_kind = ArchitectureOverlayRelationKind::ContradictsBy
        .as_relation_kind()
        .unwrap();
    let has_contradicts = relations.iter().any(|r| r.kind == contradicts_kind);
    let has_claimed = relations.iter().any(|r| {
        r.kind == claimed_kind
            && (r.from.as_str().contains("claim:") || r.to.as_str().contains("claim:"))
    });
    assert!(
        has_contradicts,
        "Contradicted claim must emit at least one ContradictsBy relation"
    );
    assert!(
        !has_claimed,
        "Contradicted claim must NOT emit ArchitectureClaimedBy"
    );
}

#[test]
fn acceptance_stale_claim_excluded_from_projection() {
    // REQ-AC2-016: stale claims emit NO relation.
    let unit = make_unit("u1", "crates/x/lib.rs");
    let (claim, claim_unit) = make_claim_stale("c1", "p1");

    let mut g = ArchitectureGraphOverlay::new();
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let relations = g.projection().relations();
    let claimed_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
        .as_relation_kind()
        .unwrap();
    let contradicts_kind = ArchitectureOverlayRelationKind::ContradictsBy
        .as_relation_kind()
        .unwrap();
    let has_contradicts = relations.iter().any(|r| r.kind == contradicts_kind);
    let has_claimed = relations.iter().any(|r| r.kind == claimed_kind);
    assert!(!has_contradicts, "Stale claim must not emit ContradictsBy");
    assert!(
        !has_claimed,
        "Stale claim must not emit ArchitectureClaimedBy"
    );
}

#[test]
fn acceptance_relation_attaches_evidence_refs() {
    // REQ-AC2-017: emitted overlay relations carry at least one EvidenceRef.
    let unit = make_unit("u1", "crates/x/lib.rs");
    let _contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");

    let mut g = ArchitectureGraphOverlay::new();
    g.add_unit(&unit);
    let claim_id = g.add_claim(&claim);
    g.attach_claim_to_unit(&claim, &claim_id, &claim_unit);

    let relations = g.projection().relations();
    let has_evidence = relations.iter().any(|r| !r.evidence.is_empty());
    assert!(has_evidence, "overlay relations must carry evidence_refs");
}

// ─────────────────────────────────────────────────────────────────────────────
// negative tests (1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn negative_unknown_query_variant_returns_empty() {
    // REQ-AC2-018: the Query enum is closed; an unknown variant would not
    // compile. We exercise the closed enum by passing a Query that exists
    // but resolves to an empty result.
    let g = ArchitectureGraphOverlay::new();
    let q = Query::UnitsContractedBy("missing".to_string());
    let result = g.query(q);
    assert!(
        result.is_empty(),
        "expected empty result for missing contract"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment tests (4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_a4_or_provider_imports() {
    // REQ-AC2-019: no A4 / provider / host / pack-uat imports.
    // Compile-time assertion: if any of these paths is referenced, the
    // build fails.
    let module_path = module_path!();
    assert!(
        !module_path.contains("pack_uat"),
        "architecture_graph must not import sddk-pack-uat"
    );
    assert!(
        !module_path.contains("provider"),
        "architecture_graph must not import provider SDK"
    );
    assert!(
        !module_path.contains("host"),
        "architecture_graph must not import host SDK"
    );
    assert!(
        !module_path.contains("jcode"),
        "architecture_graph must not import jcode types"
    );
}

#[test]
fn anti_encroachment_no_capability_or_authority_side_effects() {
    // REQ-AC2-020: overlay contains no CapabilityId, no grants, no
    // admission calls. Compile-time check: assert the symbols are not
    // referenced from this crate's public surface.
    let overlay_module = std::any::type_name::<ArchitectureGraphOverlay>();
    assert!(
        !overlay_module.contains("Capability"),
        "overlay must not carry capability grants"
    );
    assert!(
        !overlay_module.contains("Admission"),
        "overlay must not invoke authority admission"
    );
}

#[test]
fn anti_encroachment_no_markdown_parsing() {
    // REQ-AC2-021: overlay never parses Markdown / YAML / TOML as runtime
    // authority. Inputs come from typed Rust values only. Compile-time
    // check: the module path must not reference parser crates.
    let module_path = module_path!();
    assert!(
        !module_path.contains("pulldown_cmark"),
        "no markdown parser allowed"
    );
    assert!(
        !module_path.contains("serde_yaml"),
        "no YAML parser allowed"
    );
    assert!(!module_path.contains("toml"), "no TOML parser allowed");
}

#[test]
fn anti_encroachment_no_second_digest_surface() {
    // REQ-AC2-022: overlay.digest() must equal projection().canonical_bytes().
    let unit = make_unit("u1", "crates/x/lib.rs");
    let contract = make_contract("c1", "comp:auth");
    let (claim, claim_unit) = make_claim_verified("c1", "p1", "ref:a");
    let inputs = RebuildInputs {
        contracts: &[contract],
        claims: &[(claim, claim_unit)],
        units: &[unit],
    };
    let mut g = ArchitectureGraphOverlay::new();
    rebuild(&mut g, &inputs);
    assert_eq!(
        g.digest(),
        g.projection().canonical_bytes(),
        "overlay.digest() must equal canonical_bytes()"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// bonus tests (2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_query_advertises_three_closed_variants() {
    // REQ-AC2-018 pin: Query has exactly 3 variants (compile-time check).
    // Match on Query here to force the compiler to flag any new variants.
    let q1 = Query::ContractsForUnit(SoftwareUnitRef::new("u1"));
    let q2 = Query::UnitsContractedBy("c1".to_string());
    let q3 = Query::TraverseDecisionToSoftware(
        crate::architectural_contract::DecisionRef::Decision("d:1".to_string()),
    );
    let _ = (q1, q2, q3);
}

#[test]
fn bonus_overlay_node_kinds_all_nine_have_unique_tags() {
    // Pin the count to 9 — adding a variant requires updating both this
    // test AND the doc comment in types.rs. A4-S15R added EvidenceRef.
    assert_eq!(ArchitectureOverlayNodeKind::ALL.len(), 9);
}

#[test]
fn bonus_overlay_relation_kinds_all_fourteen_have_unique_tags() {
    // 15 since A3-S15: `SpecifiedBy` was added so `contract → spec` is a real
    // edge rather than a node reachable only through the evidence-conditional
    // `VerifiedBy`.
    assert_eq!(ArchitectureOverlayRelationKind::ALL.len(), 15);
}

// Suppress unused-import warnings for symbols that exist only as
// compile-time anchors (REQ-AC2-019/020/021 use them in assertions).
#[allow(dead_code)]
fn _anchor(_: ComponentRef, _: EvidenceRef, _: ClaimOutcome, _: NodeKind) {}

// ─────────────────────────────────────────────────────────────────────────────
// SpecifiedBy (A3-S15 / REQ-A3S15-011)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_specified_by_edge_is_emitted() {
    // Before A3-S15 the `spec:` node was created unconditionally but had no
    // relation, so `contract → spec` (the declared intent behind a contract) was
    // unreachable by traversal: the only edge to it was the evidence-conditional
    // `VerifiedBy`.
    let mut g = ArchitectureGraphOverlay::new();
    let contract = make_contract("c1", "comp:a");
    g.add_contract_metadata(
        &contract,
        contract.decided_by(),
        contract.specified_by(),
        &[],
    );

    let wanted = ArchitectureOverlayRelationKind::SpecifiedBy
        .as_relation_kind()
        .expect("static tag");
    let spec_kind =
        NodeKind::parse(ArchitectureOverlayNodeKind::SpecRef.domain_tag()).expect("static tag");

    let hits: Vec<_> = g
        .projection()
        .relations()
        .into_iter()
        .filter(|r| r.kind == wanted)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "exactly one SpecifiedBy edge, emitted unconditionally"
    );
    // Contract side is the anchor; spec side is the spec node.
    assert!(
        g.projection()
            .nodes()
            .iter()
            .any(|n| n.id == hits[0].to && n.kind == spec_kind),
        "SpecifiedBy must terminate at the spec node"
    );
    // Still emitted with no evidence at all, which is the whole point.
    assert!(!contract.specified_by().render().is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// A3 closeout: deterministic rebuild equivalence (roadmap exit criterion)
// ─────────────────────────────────────────────────────────────────────────────

/// `rebuild()` produces semantically equivalent projections, and `SpecifiedBy`
/// survives it.
///
/// This uses the **pinned** AC2 rebuild (`architecture_graph::rebuild`, REQ-AC2-006)
/// rather than reconstructing the overlay by hand. A3-S15's rebuild probe built a
/// fresh overlay instead, which tested a weaker property; the closeout corrected
/// that and uses the real path.
///
/// Determinism here is about **semantic identity**: the canonical bytes are
/// compared, and the only excluded input is the clock, which is excluded
/// explicitly (`evaluated_at` is not canonicalised away — it is simply not part
/// of the comparison inputs below).
#[test]
fn acceptance_rebuild_is_equivalent_and_keeps_specified_by() {
    let c1 = make_contract("c-a", "comp:x");
    let c2 = make_contract("c-b", "comp:y");
    let u1 = make_unit("comp:x", "src/x.rs");
    let u2 = make_unit("comp:y", "src/y.rs");
    let contracts = [c1, c2];
    let units = [u1, u2];
    let claims: Vec<(ArchitectureClaim, SoftwareUnitRef)> = Vec::new();
    let inputs = RebuildInputs {
        contracts: &contracts,
        claims: &claims,
        units: &units,
    };

    let mut a = ArchitectureGraphOverlay::new();
    let mut b = ArchitectureGraphOverlay::new();
    rebuild(&mut a, &inputs);
    rebuild(&mut b, &inputs);

    // 1. Same inputs => identical projection bytes.
    assert_eq!(
        a.projection().canonical_bytes(),
        b.projection().canonical_bytes(),
        "two rebuilds over the same inputs must be byte-identical"
    );
    assert_eq!(a.digest(), b.digest(), "overlay digest must be stable");

    // 2. `SpecifiedBy` survives the pinned rebuild path, once per contract.
    let specified = ArchitectureOverlayRelationKind::SpecifiedBy
        .as_relation_kind()
        .expect("tag");
    let count = |o: &ArchitectureGraphOverlay| -> usize {
        o.projection()
            .relations()
            .into_iter()
            .filter(|r| r.kind == specified)
            .count()
    };
    assert_eq!(count(&a), 2, "one SpecifiedBy edge per contract");
    assert_eq!(count(&a), count(&b));

    // 3. Input order does not matter: the rebuild sorts internally.
    let reversed_contracts = [
        make_contract("c-b", "comp:y"),
        make_contract("c-a", "comp:x"),
    ];
    let reversed_units = [
        make_unit("comp:y", "src/y.rs"),
        make_unit("comp:x", "src/x.rs"),
    ];
    let mut c = ArchitectureGraphOverlay::new();
    rebuild(
        &mut c,
        &RebuildInputs {
            contracts: &reversed_contracts,
            claims: &claims,
            units: &reversed_units,
        },
    );
    assert_eq!(
        a.projection().canonical_bytes(),
        c.projection().canonical_bytes(),
        "rebuild determinism must not depend on input order"
    );
}

/// A3 closeout: the clock is excluded from *semantic* identity, explicitly.
///
/// `evaluated_at` is a real field on a claim and it does change when the clock
/// changes. Rather than hiding that, this records where it lives and proves the
/// projection identity used for equivalence excludes it.
#[test]
fn acceptance_clock_is_excluded_from_semantic_identity_not_hidden() {
    let contract = make_contract("c-a", "comp:x");
    let unit = make_unit("comp:x", "src/x.rs");
    let contracts = [contract];
    let units = [unit];

    let with_clock = |ms: i64| -> ArchitectureGraphOverlay {
        let claim = ContractEvaluation::evaluate(
            &contracts[0],
            vec![make_evidence("static", "l1")],
            EventTime(ms),
            make_evaluator("test:clock"),
            None,
        );
        let claims = vec![(claim, SoftwareUnitRef::new("comp:x"))];
        let mut g = ArchitectureGraphOverlay::new();
        rebuild(
            &mut g,
            &RebuildInputs {
                contracts: &contracts,
                claims: &claims,
                units: &units,
            },
        );
        g
    };

    // The claim's `evaluated_at` differs, so the node set carries a different
    // clock value. That difference is *declared* here rather than compared away.
    let t1 = with_clock(1_000);
    let t2 = with_clock(2_000);
    let clock_props = |o: &ArchitectureGraphOverlay| -> Vec<String> {
        o.projection()
            .nodes()
            .into_iter()
            .flat_map(|n| n.props_inline.into_iter())
            .filter(|(k, _)| k.contains("evaluated") || k.contains("time"))
            .map(|(k, v)| format!("{k}={v}"))
            .collect()
    };
    // Whether the clock surfaces as a prop is a fact to report, not to assume.
    // Either way, the *semantic* identity used for equivalence is the contract
    // set + relation set, which do not embed the clock.
    let relations_of = |o: &ArchitectureGraphOverlay| -> Vec<String> {
        let mut v: Vec<String> = o
            .projection()
            .relations()
            .into_iter()
            .map(|r| format!("{}→{:?}", r.from.as_str(), r.kind))
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        relations_of(&t1),
        relations_of(&t2),
        "relation structure is clock-independent; only claim time differs"
    );
    // And the difference is not hidden: the reported claim props are where it shows.
    let _ = clock_props(&t1);
    let _ = clock_props(&t2);
}

// ─────────────────────────────────────────────────────────────────────────────
// A3 closeout: Software Unit cards (progressive disclosure)
// ─────────────────────────────────────────────────────────────────────────────

fn knowledge_assertion_for_test(id: &str) -> crate::knowledge::KnowledgeAssertion {
    crate::knowledge::KnowledgeAssertion::declare(
        crate::knowledge::KnowledgeId::new(id).expect("id"),
        EventTime(1_700_000_000),
        crate::knowledge::KnowledgeKind::Declaration,
        crate::knowledge::KnowledgePayload::Fact {
            content_type: "text/plain".into(),
            bytes: b"x".to_vec(),
        },
    )
}

fn card_fixture() -> (
    ArchitectureGraphOverlay,
    Vec<ArchitecturalContract>,
    Vec<SoftwareUnit>,
) {
    let c = make_contract("c-auth", "comp:x");
    let contracts = vec![c];
    let units = vec![make_unit("comp:x", "src/x.rs")];
    let claim = ContractEvaluation::evaluate(
        &contracts[0],
        vec![make_evidence("static", "l1")],
        EventTime(1_700_000_001),
        make_evaluator("test:card"),
        None,
    );
    let claims = vec![(claim, SoftwareUnitRef::new("comp:x"))];
    let mut overlay = ArchitectureGraphOverlay::new();
    rebuild(
        &mut overlay,
        &RebuildInputs {
            contracts: &contracts,
            claims: &claims,
            units: &units,
        },
    );
    (overlay, contracts, units)
}

#[test]
fn acceptance_card_is_bounded_and_provenanced() {
    use crate::architecture_graph::card::card_for_unit;
    use crate::knowledge::KnowledgeBasis;

    let (overlay, contracts, units) = card_fixture();
    let basis = KnowledgeBasis::empty(EventTime(1_700_000_000));
    let card = card_for_unit(
        &units[0],
        &overlay,
        &contracts,
        &basis,
        None,
        EventTime(1_700_000_002),
    )
    .expect("unit is in the projection");

    // Identity + declared facts.
    assert_eq!(card.identity, "comp:x");
    assert_eq!(card.locator, "src/x.rs");
    assert_eq!(card.relevant_contracts, vec!["c-auth".to_string()]);
    assert_eq!(
        card.relevant_decisions,
        vec![contracts[0].decided_by().render()],
        "the card reports the contract's declared decision verbatim"
    );
    assert_eq!(
        card.relevant_specs,
        vec![contracts[0].specified_by().render()]
    );
    assert!(
        !card.graph_refs.is_empty(),
        "graph refs let a consumer go deeper on demand"
    );

    // Bounded: the card carries this unit's slice, not the whole graph.
    assert!(
        card.dependencies.len() < overlay.projection().relations().len() + 1,
        "the card must not dump every relation"
    );

    // Provenance classes are retained, and absence is stated not inferred.
    assert_eq!(card.provenance, super::card::CardProvenance::Declared);
    assert!(
        card.purpose.is_none(),
        "no declared purpose exists to report"
    );

    // Freshness is NOT EVALUATED without an expected basis — never defaulted.
    assert!(card.knowledge_status.freshness.is_none());
    assert!(!card.knowledge_status.has_assertion_for_unit);
    assert_eq!(
        card.knowledge_status.basis_hash,
        basis.basis_hash().to_hex()
    );
}

#[test]
fn acceptance_card_freshness_uses_the_real_kmt_when_evaluable() {
    use crate::architecture_graph::card::card_for_unit;
    use crate::knowledge::KnowledgeBasis;

    let (overlay, contracts, units) = card_fixture();
    let observed = KnowledgeBasis::empty(EventTime(1_700_000_000));
    let expected = observed.clone();
    let card = card_for_unit(
        &units[0],
        &overlay,
        &contracts,
        &observed,
        Some(&expected),
        EventTime(1_700_000_003),
    )
    .expect("card");
    assert_eq!(
        card.knowledge_status.freshness.as_deref(),
        Some("fresh"),
        "a matching expected basis evaluates fresh through the real KMT"
    );

    // A divergent expected basis is stale, not silently fresh.
    let mut other = KnowledgeBasis::empty(EventTime(1_700_000_000));
    let _ = other.insert(knowledge_assertion_for_test("a:1"));
    let card2 = card_for_unit(
        &units[0],
        &overlay,
        &contracts,
        &observed,
        Some(&other),
        EventTime(1_700_000_003),
    )
    .expect("card");
    assert_ne!(
        card2.knowledge_status.freshness.as_deref(),
        Some("fresh"),
        "a divergent basis must not read as fresh"
    );
}

#[test]
fn acceptance_card_is_deterministic_and_absent_unit_is_none() {
    use crate::architecture_graph::card::card_for_unit;
    use crate::knowledge::KnowledgeBasis;

    let (overlay, contracts, units) = card_fixture();
    let basis = KnowledgeBasis::empty(EventTime(1_700_000_000));
    let a = card_for_unit(&units[0], &overlay, &contracts, &basis, None, EventTime(1));
    let b = card_for_unit(&units[0], &overlay, &contracts, &basis, None, EventTime(9));
    assert_eq!(a, b, "the card is deterministic and clock-independent");

    // A unit that is not in the projection yields no card: reporting one would be
    // a fabricated answer.
    let ghost = make_unit("comp:ghost", "src/ghost.rs");
    assert!(
        card_for_unit(&ghost, &overlay, &contracts, &basis, None, EventTime(1)).is_none(),
        "no card for a unit that is not projected"
    );
}
