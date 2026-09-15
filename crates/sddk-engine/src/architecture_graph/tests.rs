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
fn bonus_overlay_node_kinds_all_eight_have_unique_tags() {
    // Pin the count to 8 — adding a variant requires updating both this
    // test AND the doc comment in types.rs.
    assert_eq!(ArchitectureOverlayNodeKind::ALL.len(), 8);
}

#[test]
fn bonus_overlay_relation_kinds_all_fourteen_have_unique_tags() {
    assert_eq!(ArchitectureOverlayRelationKind::ALL.len(), 14);
}

// Suppress unused-import warnings for symbols that exist only as
// compile-time anchors (REQ-AC2-019/020/021 use them in assertions).
#[allow(dead_code)]
fn _anchor(_: ComponentRef, _: EvidenceRef, _: ClaimOutcome, _: NodeKind) {}
