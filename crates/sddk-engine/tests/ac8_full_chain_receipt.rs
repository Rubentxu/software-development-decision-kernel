// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// ac8_full_chain_receipt.rs — A3-S9 crate-boundary integration test.
//
// AC-UAT-016: SDDK reproduces agreed historical conformance findings and emits
// the receipt. This test drives the whole native chain through the public API
// of five modules and composes the named receipt.

use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::architecture_conformance::{
    ConformanceInputs, ContractEvidence, compute_conformance_delta,
};
use sddk_engine::architecture_debverify::run_debverify_audit;
use sddk_engine::architecture_graph::{
    ArchitectureGraphOverlay, ArchitectureOverlayRelation, ArchitectureOverlayRelationKind,
    OverlayNodeRef, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use sddk_engine::architecture_mutation::{MutationSandbox, run_critical_mutations};
use sddk_engine::architecture_receipt::{ReceiptInputs, ReceiptVerdict, compose_receipt};
use sddk_engine::knowledge::EventTime;
use sddk_engine::paradigm_lens::{LensObservation, evaluate_lens};
use sddk_engine::paradigm_profile::{ParadigmAnchorRef, ParadigmLensKind, ProjectIntentRef};

const T0: i64 = 1_700_000_000;

fn cid(s: &str) -> ContractId {
    ContractId::new(s).expect("contract id")
}

fn single_authority(id: &str, comp: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        cid(id),
        ComponentRef::new(comp).expect("component"),
        DecisionRef::Decision(format!("decision:{id}")),
        SpecRef::Spec(format!("spec:{id}")),
        Revision::new(format!("rev:{id}")).expect("revision"),
        EventTime(T0),
    )
    .expect("declare_single_authority")
}

fn projection_only(id: &str, source_kind: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_projection_only(
        cid(id),
        source_kind.to_string(),
        DecisionRef::Decision(format!("decision:{id}")),
        SpecRef::Spec(format!("spec:{id}")),
        Revision::new(format!("rev:{id}")).expect("revision"),
        EventTime(T0),
    )
    .expect("declare_projection_only")
}

fn mutation_sandbox() -> MutationSandbox {
    MutationSandbox::from_sources([
        (
            "crates/sddk-engine/src/domain/order.rs",
            "// domain\npub struct Order { pub id: u64 }\n",
        ),
        (
            "crates/sddk-engine/src/alignment/lens.rs",
            "// alignment\npub fn evaluate() -> bool { true }\n",
        ),
        (
            "crates/sddk-engine/src/workbook/plan.rs",
            "// workbook\npub struct PlanView;\n",
        ),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ])
}

/// Build the representative revision that exhibits the historical classes.
fn representative_revision() -> (ArchitectureGraphOverlay, Vec<ArchitecturalContract>) {
    let contracts = vec![
        // duplicate/shadow authority
        single_authority("dup-a", "comp:auth"),
        single_authority("dup-b", "comp:auth"),
        // bounded compatibility (missing owner)
        single_authority("orphan", "comp:absent"),
        // projection/authority confusion
        single_authority("owner", "comp:ledger"),
        projection_only("proj", "comp:ledger"),
        // dependency boundary (a forbidden edge that exists in the graph)
        ArchitecturalContract::declare_forbidden_dependency(
            cid("no-edge"),
            ComponentRef::new("comp:domain").expect("c"),
            ComponentRef::new("comp:provider").expect("c"),
            "forbidden".into(),
            DecisionRef::Decision("decision:no-edge".into()),
            SpecRef::Spec("spec:no-edge".into()),
            Revision::new("rev:no-edge").expect("r"),
            EventTime(T0),
        )
        .expect("forbidden_dependency"),
    ];
    let mut overlay = ArchitectureGraphOverlay::new();
    for u in ["comp:auth", "comp:ledger", "comp:domain", "comp:provider"] {
        overlay.add_unit(&SoftwareUnit::new(
            SoftwareUnitRef::new(u),
            UnitKind::Module,
            u.to_string(),
        ));
    }
    // A live relation joining the forbidden pair (the bypass).
    overlay.add_relation(&ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:domain")),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:provider")),
        kind: ArchitectureOverlayRelationKind::DependsOn,
        evidence: vec![],
    });
    (overlay, contracts)
}

#[test]
fn ac8_full_chain_emits_receipt_and_reproduces_historical_classes() {
    let (overlay, contracts) = representative_revision();

    // AC2 graph digest is carried by AC5's audit.
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert!(
        !audit.is_clean(),
        "the revision is deliberately non-conformant"
    );

    // AC4 change-scoped delta.
    let evidence: ContractEvidence = std::collections::BTreeMap::new();
    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &evidence,
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 1),
        &[],
    )
    .expect("delta");

    // AC6 negative evidence (all four critical mutations detected).
    let mutations = run_critical_mutations(&mutation_sandbox()).expect("mutations");
    assert!(mutations.all_detected);

    // AC7 advisory lens result.
    let lenses = vec![evaluate_lens(
        ParadigmLensKind::DataOriented,
        ParadigmAnchorRef::ProjectIntent(ProjectIntentRef::new("p-63676b11dc0ef88f")),
        &[LensObservation::StringlyTypedStatus],
        T0,
    )];

    // AC8 composes the named receipt.
    let receipt = compose_receipt(
        ReceiptInputs {
            revision: "d1ed1d0".to_string(),
            knowledge_basis: "docs/architecture/specs/AC1..AC7".to_string(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &lenses,
            waivers: &[],
            provider_basis: &[],
            change_basis: None,
        },
        EventTime(T0 + 2),
    );

    // ── AC-UAT-016: all five historical classes reproduced ──────────────────
    assert!(
        receipt.all_classes_reproduced(),
        "unreproduced: {:?}",
        receipt.unreproduced_classes()
    );

    // ── Base-mode properties ────────────────────────────────────────────────
    assert!(
        receipt.provider_basis.is_empty(),
        "Base mode is provider-free"
    );
    assert_eq!(receipt.basis.revision, "d1ed1d0");
    assert_eq!(receipt.basis.contract_set_digest, delta.contract_set_digest);
    assert_eq!(receipt.basis.semantic_graph_digest, audit.graph_digest);
    assert_eq!(receipt.basis.verification_plan_digest, delta.plan_digest);

    // ── Verdict (AC-041-003): unwaived MUST findings block ──────────────────
    assert_eq!(receipt.verdict, ReceiptVerdict::Blocked);
    assert!(!receipt.mandatory_unresolved().is_empty());
    assert!(!receipt.uncovered_unresolved().is_empty());

    // ── Determinism: the same inputs yield the same named receipt ───────────
    let again = compose_receipt(
        ReceiptInputs {
            revision: "d1ed1d0".to_string(),
            knowledge_basis: "docs/architecture/specs/AC1..AC7".to_string(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &lenses,
            waivers: &[],
            provider_basis: &[],
            change_basis: None,
        },
        EventTime(T0 + 2),
    );
    assert_eq!(receipt, again);
    assert_eq!(receipt.id, again.id);
}

#[test]
fn ac8_waivers_turn_blocked_into_pass_with_waivers() {
    let (overlay, contracts) = representative_revision();
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &ContractEvidence::new(),
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 1),
        &[],
    )
    .expect("delta");
    let mutations = run_critical_mutations(&mutation_sandbox()).expect("mutations");

    // One waiver per mandatory finding kind present in this revision.
    let waivers: Vec<String> = audit
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                sddk_engine::architecture_debverify::FindingSeverity::Critical
                    | sddk_engine::architecture_debverify::FindingSeverity::High
            )
        })
        .map(|f| format!("waiver:{}", f.kind.canonical_tag()))
        .collect();
    assert!(!waivers.is_empty());

    let receipt = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &waivers,
            provider_basis: &[],
            change_basis: None,
        },
        EventTime(T0 + 2),
    );
    assert!(
        receipt.uncovered_unresolved().is_empty(),
        "waivers must cover every mandatory finding: {:?}",
        receipt.uncovered_unresolved()
    );
    assert_eq!(receipt.verdict, ReceiptVerdict::PassWithWaivers);
}

#[test]
fn ac8_projection_confusion_is_reproduced_via_an_overlay_relation() {
    // The dependency-boundary class needs a live relation; prove that wiring it
    // through the AC2 overlay changes the receipt's class coverage (i.e. the
    // receipt really does aggregate the graph, not just the contract set).
    use sddk_engine::architecture_receipt::HistoricalClass;

    let contracts = vec![
        ArchitecturalContract::declare_forbidden_dependency(
            cid("no-edge"),
            ComponentRef::new("comp:domain").expect("c"),
            ComponentRef::new("comp:provider").expect("c"),
            "forbidden".into(),
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").expect("r"),
            EventTime(T0),
        )
        .expect("forbidden"),
    ];
    let mut overlay = ArchitectureGraphOverlay::new();
    for u in ["comp:domain", "comp:provider"] {
        overlay.add_unit(&SoftwareUnit::new(
            SoftwareUnitRef::new(u),
            UnitKind::Module,
            u.to_string(),
        ));
    }
    let mutations = run_critical_mutations(&mutation_sandbox()).expect("mutations");

    // Without the edge: the class is NOT reproduced from the graph.
    let audit_no_edge = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("a");
    let cov_no_edge =
        sddk_engine::architecture_receipt::evaluate_class_coverage(&audit_no_edge, &mutations);
    let row = cov_no_edge
        .iter()
        .find(|c| c.class == HistoricalClass::DependencyBoundary)
        .expect("row");
    assert!(!row.reproduced);

    // With the live edge: the class IS reproduced.
    overlay.add_relation(&ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:domain")),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:provider")),
        kind: ArchitectureOverlayRelationKind::DependsOn,
        evidence: vec![],
    });
    let audit_edge = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("a");
    let cov_edge =
        sddk_engine::architecture_receipt::evaluate_class_coverage(&audit_edge, &mutations);
    let row = cov_edge
        .iter()
        .find(|c| c.class == HistoricalClass::DependencyBoundary)
        .expect("row");
    assert!(row.reproduced, "the live edge must reproduce the class");
    assert!(!row.evidence.is_empty());
}
