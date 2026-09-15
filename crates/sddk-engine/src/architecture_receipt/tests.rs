// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_receipt/tests.rs — A3-S9 / AC8 acceptance + anti-encroachment +
// boundary tests.
//
// REQ-AC8-001..024 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S9-ac8-self-audit.md`.

use super::*;
use crate::architectural_contract::ContractId;
use crate::architectural_contract::{
    ArchitecturalContract, ComponentRef, DecisionRef, EvidenceRef, Revision, SpecRef,
};
use crate::architecture_conformance::{
    ConformanceInputs, ContractEvidence, DeltaContractStatus, compute_conformance_delta,
};
use crate::architecture_debverify::{run_debverify_audit, run_debverify_audit as _audit};
use crate::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use crate::architecture_mutation::{MutationSandbox, run_critical_mutations};
use crate::knowledge::EventTime;
use crate::paradigm_lens::{LensObservation, evaluate_lens};
use crate::paradigm_profile::{LensStatus, ParadigmAnchorRef, ParadigmLensKind, ProjectIntentRef};

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

fn overlay_with(units: &[&str]) -> ArchitectureGraphOverlay {
    let mut g = ArchitectureGraphOverlay::new();
    for u in units {
        g.add_unit(&SoftwareUnit::new(
            SoftwareUnitRef::new(*u),
            UnitKind::Module,
            (*u).to_string(),
        ));
    }
    g
}

fn empty_evidence() -> ContractEvidence {
    ContractEvidence::new()
}

/// A representative "SDDK-like" triple: a duplicate authority (shadow), an
/// owner absent from the graph (bounded compatibility class), and a clean
/// contract. Plus a fully-detected mutation suite.
struct Fixture {
    overlay: ArchitectureGraphOverlay,
    contracts: Vec<ArchitecturalContract>,
    delta: crate::architecture_conformance::ArchitectureConformanceDelta,
    audit: crate::architecture_debverify::DebVerifyAudit,
    mutations: crate::architecture_mutation::MutationSuiteReceipt,
}

fn fixture() -> Fixture {
    // Two contracts claim `comp:auth` (shadow authority), one names an absent
    // owner (missing owner -> bounded-compatibility class).
    let contracts = vec![
        single_authority("shadow-a", "comp:auth"),
        single_authority("shadow-b", "comp:auth"),
        single_authority("orphan", "comp:absent"),
        single_authority("clean", "comp:auth-adjacent"),
    ];
    let overlay = overlay_with(&["comp:auth", "comp:auth-adjacent"]);

    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &empty_evidence(),
            contradiction_witnesses: &[],
            contract_filter: None,
        },
        EventTime(T0 + 1),
        &[],
    )
    .expect("delta");

    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");

    let sandbox = MutationSandbox::from_sources([
        (
            "crates/sddk-engine/src/domain/order.rs",
            "// domain model\npub struct Order { pub id: u64 }\n",
        ),
        (
            "crates/sddk-engine/src/alignment/lens.rs",
            "// alignment lens\npub fn evaluate() -> bool { true }\n",
        ),
        (
            "crates/sddk-engine/src/workbook/plan.rs",
            "// workbook view\npub struct PlanView;\n",
        ),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(evt: u64) -> Result<(), ()> { let _ = evt; Ok(()) }\n",
        ),
    ]);
    let mutations = run_critical_mutations(&sandbox).expect("mutations");

    Fixture {
        overlay,
        contracts,
        delta,
        audit,
        mutations,
    }
}

fn compose(
    f: &Fixture,
    waivers: &[String],
    providers: &[String],
    now: EventTime,
) -> ArchitectureConformanceReceipt {
    let lenses: Vec<crate::paradigm_lens::LensEvaluation> = vec![evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        ParadigmAnchorRef::ProjectIntent(ProjectIntentRef::new("p-63676b11dc0ef88f")),
        &[LensObservation::AnemicModelDetected],
        T0,
    )];
    compose_receipt(
        ReceiptInputs {
            revision: "d1ed1d0".to_string(),
            knowledge_basis: "specs:AC1..AC7".to_string(),
            delta: &f.delta,
            audit: &f.audit,
            mutations: &f.mutations,
            lenses: &lenses,
            waivers,
            provider_basis: providers,
            change_basis: None,
            contract_filter: None,
        },
        now,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — basis (REQ-AC8-001..003)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_basis_shape() {
    // REQ-AC8-001
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(r.basis.revision, "d1ed1d0");
    assert_eq!(r.basis.knowledge_basis, "specs:AC1..AC7");
    assert_ne!(r.basis.contract_set_digest, [0u8; 32]);
    assert!(!r.basis.semantic_graph_digest.is_empty());
    assert_ne!(r.basis.verification_plan_digest, [0u8; 32]);
}

#[test]
fn acceptance_basis_carries_ac4_and_ac2_digests() {
    // REQ-AC8-002
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(r.basis.contract_set_digest, f.delta.contract_set_digest);
    assert_eq!(r.basis.verification_plan_digest, f.delta.plan_digest);
    assert_eq!(r.basis.semantic_graph_digest, f.audit.graph_digest);
    assert_eq!(r.vector, f.delta.vector);
}

#[test]
fn acceptance_receipt_id_is_derived() {
    // REQ-AC8-003
    let f = fixture();
    let a = compose(&f, &[], &[], EventTime(T0 + 2));
    let b = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(a.id, b.id);
    let global = ReceiptScope {
        change_basis: None,
        contract_filter: None,
    };
    assert_eq!(a.id, ReceiptId::derive(&a.basis, a.verdict, &global));
    // A different revision changes the id.
    let mut inputs_rev = "other".to_string();
    inputs_rev.push_str("");
    assert_ne!(
        ReceiptId::derive(
            &ReceiptBasis {
                revision: inputs_rev,
                ..a.basis.clone()
            },
            a.verdict,
            &global
        ),
        a.id
    );
}

#[test]
fn acceptance_scope_is_part_of_the_receipt_id() {
    // REQ-A3S13-010. v1 hashed only the basis and the verdict, so a global run,
    // a `--changed` run and a `--contract` run over one declaration shared an
    // id while reporting different scopes. The id is the receipt's address.
    let f = fixture();
    let global = compose(&f, &[], &[], EventTime(T0 + 2));

    let basis = ChangeBasis {
        base: "origin/main".to_string(),
        changed_units: vec!["comp:auth".to_string()],
    };
    let lenses: Vec<crate::paradigm_lens::LensEvaluation> = vec![];
    let scoped = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &f.delta,
            audit: &f.audit,
            mutations: &f.mutations,
            lenses: &lenses,
            waivers: &[],
            provider_basis: &[],
            change_basis: Some(&basis),
            contract_filter: None,
        },
        EventTime(T0 + 2),
    );
    assert_ne!(scoped.id, global.id, "a change basis must change the id");

    let filtered = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &f.delta,
            audit: &f.audit,
            mutations: &f.mutations,
            lenses: &lenses,
            waivers: &[],
            provider_basis: &[],
            change_basis: Some(&basis),
            contract_filter: Some("c:auth".to_string()),
        },
        EventTime(T0 + 2),
    );
    assert_ne!(
        filtered.id, scoped.id,
        "a contract filter must change the id"
    );
    assert_eq!(filtered.contract_filter.as_deref(), Some("c:auth"));
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — verdict (REQ-AC8-004..010)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_verdict_closed_three() {
    // REQ-AC8-004
    assert_eq!(ReceiptVerdict::ALL.len(), 3);
    let mut tags: Vec<&str> = ReceiptVerdict::ALL
        .iter()
        .map(|v| v.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 3);
}

#[test]
fn acceptance_unresolved_derivation() {
    // REQ-AC8-005
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    assert!(!r.unresolved.is_empty());
    // The shadow authority is present as a mandatory finding.
    assert!(
        r.unresolved
            .iter()
            .any(|u| u.kind == "shadow_authority" && u.mandatory),
        "{:?}",
        r.unresolved
    );
    // The missing owner is present as a mandatory finding.
    assert!(
        r.unresolved
            .iter()
            .any(|u| u.kind == "missing_owner" && u.mandatory)
    );
    // No negative-evidence gap: all mutations were detected.
    assert!(
        !r.unresolved
            .iter()
            .any(|u| u.kind == "negative_evidence_gap")
    );
}

#[test]
fn acceptance_blocked_without_waiver() {
    // REQ-AC8-008
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(r.verdict, ReceiptVerdict::Blocked);
    assert!(!r.uncovered_unresolved().is_empty());
}

#[test]
fn acceptance_waiver_matching_is_explicit() {
    // REQ-AC8-009
    let f = fixture();
    let waivers = vec![
        "waiver:shadow_authority:gov-1".to_string(),
        "waiver:missing_owner:gov-2".to_string(),
    ];
    let r = compose(&f, &waivers, &[], EventTime(T0 + 2));
    // Every mandatory finding is covered explicitly.
    for fnd in r.mandatory_unresolved() {
        assert!(
            !fnd.waiver_refs.is_empty(),
            "mandatory finding {:?} unexpectedly uncovered",
            fnd.kind
        );
    }
    assert!(r.uncovered_unresolved().is_empty());
    assert_eq!(r.verdict, ReceiptVerdict::PassWithWaivers);
    // The composer records the waivers it was given (sorted), and invents none.
    let mut expected = waivers.clone();
    expected.sort();
    assert_eq!(r.waivers, expected);
}

#[test]
fn acceptance_pass_with_waivers() {
    // REQ-AC8-007
    let f = fixture();
    let r = compose(
        &f,
        &[
            "waiver:shadow_authority".into(),
            "waiver:missing_owner".into(),
        ],
        &[],
        EventTime(T0 + 2),
    );
    assert_eq!(r.verdict, ReceiptVerdict::PassWithWaivers);
}

#[test]
fn acceptance_pass_when_clean() {
    // REQ-AC8-006: a clean architecture with all mutations detected and no
    // audit findings yields Pass.
    use crate::architectural_contract::{ContractEvaluation, EvaluatorRef};
    let contracts = vec![single_authority("only", "comp:auth")];
    let unit_ref = SoftwareUnitRef::new("comp:auth");

    // Build ONE overlay: unit + verified claim + contract anchor.
    let mut overlay = overlay_with(&["comp:auth"]);
    {
        let claim = ContractEvaluation::evaluate(
            &contracts[0],
            vec![EvidenceRef::new("static", "x").unwrap()],
            EventTime(T0 + 1),
            EvaluatorRef::new("t").unwrap(),
            None,
        );
        let claim_id = overlay.add_claim(&claim);
        overlay.attach_claim_to_unit(&claim, &claim_id, &unit_ref);
        overlay.add_contract_metadata(
            &contracts[0],
            &DecisionRef::Decision("d".into()),
            &SpecRef::Spec("s".into()),
            &[],
        );
    }

    let evidence = {
        let mut m = ContractEvidence::new();
        m.insert(cid("only"), vec![EvidenceRef::new("static", "x").unwrap()]);
        m
    };
    let delta2 = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &evidence,
            contradiction_witnesses: &[],
            contract_filter: None,
        },
        EventTime(T0 + 1),
        std::slice::from_ref(&unit_ref),
    )
    .unwrap();
    assert!(
        delta2.affected.len() == 1 && delta2.claims.len() == 1,
        "fixture must produce one Verified contract: {:?}",
        delta2.statuses()
    );

    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    assert!(audit.is_clean(), "audit: {:?}", audit.findings);

    let sandbox = MutationSandbox::from_sources([
        ("crates/sddk-engine/src/domain/d.rs", "// clean\n"),
        ("crates/sddk-engine/src/alignment/a.rs", "// clean\n"),
        ("crates/sddk-engine/src/workbook/w.rs", "// clean\n"),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ]);
    let mutations = run_critical_mutations(&sandbox).unwrap();

    let r = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &delta2,
            audit: &audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &[],
            provider_basis: &[],
            change_basis: None,
            contract_filter: None,
        },
        EventTime(T0 + 2),
    );
    assert!(
        r.mandatory_unresolved().is_empty(),
        "must be clean: {:?}",
        r.mandatory_unresolved()
    );
    assert_eq!(r.verdict, ReceiptVerdict::Pass);
}

#[test]
fn acceptance_medium_is_advisory() {
    // REQ-AC8-010: an AC5 Medium finding is recorded but not MUST.
    let contracts = vec![
        // A stale compatibility window -> AC5 StaleCompatibility (Medium).
        ArchitecturalContract::declare_bounded_compatibility(
            cid("window"),
            EventTime(T0 + 1),
            None,
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").unwrap(),
            EventTime(T0),
        )
        .unwrap(),
    ];
    let overlay = overlay_with(&["comp:x"]);
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 100)).unwrap();
    let stale =
        audit.by_kind(crate::architecture_debverify::DebVerifyFindingKind::StaleCompatibility);
    assert_eq!(stale.len(), 1);

    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &empty_evidence(),
            contradiction_witnesses: &[],
            contract_filter: None,
        },
        EventTime(T0 + 100),
        &[],
    )
    .unwrap();
    let mutations = run_critical_mutations(&MutationSandbox::from_sources([
        ("crates/sddk-engine/src/domain/d.rs", "// c\n"),
        ("crates/sddk-engine/src/alignment/a.rs", "// c\n"),
        ("crates/sddk-engine/src/workbook/w.rs", "// c\n"),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ]))
    .unwrap();

    let r = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &[],
            provider_basis: &[],
            change_basis: None,
            contract_filter: None,
        },
        EventTime(T0 + 101),
    );
    let stale_row = r
        .unresolved
        .iter()
        .find(|u| u.kind == "stale_compatibility")
        .expect("stale row recorded");
    assert!(!stale_row.mandatory, "Medium must not be MUST");
    assert_eq!(r.verdict, ReceiptVerdict::Pass);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — historical classes (REQ-AC8-011..017, AC-UAT-016)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_class_closed_five() {
    // REQ-AC8-011
    assert_eq!(HistoricalClass::ALL.len(), 5);
    let mut tags: Vec<&str> = HistoricalClass::ALL
        .iter()
        .map(|c| c.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 5);
}

#[test]
fn acceptance_class_coverage_shape() {
    // REQ-AC8-017
    let f = fixture();
    let cov = evaluate_class_coverage(&f.audit, &f.mutations);
    assert_eq!(cov.len(), 5);
    for c in &cov {
        assert!(HistoricalClass::ALL.contains(&c.class));
        if c.reproduced {
            assert!(!c.evidence.is_empty(), "reproduced class needs evidence");
        }
    }
}

#[test]
fn acceptance_class_duplicate_authority() {
    // REQ-AC8-012
    let f = fixture();
    let cov = evaluate_class_coverage(&f.audit, &f.mutations);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::DuplicateAuthority)
        .unwrap();
    assert!(row.reproduced);
    assert!(row.evidence.iter().any(|e| e.contains("comp:auth")));
}

#[test]
fn acceptance_class_dependency_boundary() {
    // REQ-AC8-013
    use crate::architecture_graph::{
        ArchitectureOverlayRelation, ArchitectureOverlayRelationKind, OverlayNodeRef,
    };
    let contracts = vec![
        ArchitecturalContract::declare_forbidden_dependency(
            cid("no-edge"),
            ComponentRef::new("comp:domain").unwrap(),
            ComponentRef::new("comp:provider").unwrap(),
            "forbidden".into(),
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").unwrap(),
            EventTime(T0),
        )
        .unwrap(),
    ];
    let mut overlay = overlay_with(&["comp:domain", "comp:provider"]);
    overlay.add_relation(&ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:domain")),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:provider")),
        kind: ArchitectureOverlayRelationKind::DependsOn,
        evidence: vec![],
    });
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    let mutations = run_critical_mutations(&MutationSandbox::from_sources([
        ("crates/sddk-engine/src/domain/d.rs", "// c\n"),
        ("crates/sddk-engine/src/alignment/a.rs", "// c\n"),
        ("crates/sddk-engine/src/workbook/w.rs", "// c\n"),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ]))
    .unwrap();
    let cov = evaluate_class_coverage(&audit, &mutations);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::DependencyBoundary)
        .unwrap();
    assert!(row.reproduced, "bypass must reproduce the class: {cov:?}");
}

#[test]
fn acceptance_class_bounded_compatibility() {
    // REQ-AC8-014
    let f = fixture();
    let cov = evaluate_class_coverage(&f.audit, &f.mutations);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::BoundedCompatibility)
        .unwrap();
    assert!(row.reproduced, "missing owner must reproduce the class");
}

#[test]
fn acceptance_class_projection_only() {
    // REQ-AC8-015
    let contracts = vec![
        single_authority("owner", "comp:ledger"),
        ArchitecturalContract::declare_projection_only(
            cid("proj"),
            "comp:ledger".to_string(),
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").unwrap(),
            EventTime(T0),
        )
        .unwrap(),
    ];
    let overlay = overlay_with(&["comp:ledger"]);
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    let mutations = run_critical_mutations(&MutationSandbox::from_sources([
        ("crates/sddk-engine/src/domain/d.rs", "// c\n"),
        ("crates/sddk-engine/src/alignment/a.rs", "// c\n"),
        ("crates/sddk-engine/src/workbook/w.rs", "// c\n"),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ]))
    .unwrap();
    let cov = evaluate_class_coverage(&audit, &mutations);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::ProjectionOnly)
        .unwrap();
    assert!(row.reproduced);
}

#[test]
fn acceptance_class_missing_negative_evidence() {
    // REQ-AC8-016
    let f = fixture();
    let cov = evaluate_class_coverage(&f.audit, &f.mutations);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::MissingNegativeEvidence)
        .unwrap();
    assert!(row.reproduced);
    assert_eq!(row.evidence.len(), f.mutations.probes.len());
}

#[test]
fn bonus_empty_inputs_are_blocked_by_negative_evidence() {
    // REQ-AC8-016: an empty mutation suite is a negative-evidence gap only if
    // probes exist and were missed; an empty suite means the class is NOT
    // reproduced, so the receipt cannot claim full coverage.
    let f = fixture();
    let empty = crate::architecture_mutation::run_mutation_suite(&MutationSandbox::new(), &[], &[])
        .unwrap();
    let cov = evaluate_class_coverage(&f.audit, &empty);
    let row = cov
        .iter()
        .find(|c| c.class == HistoricalClass::MissingNegativeEvidence)
        .unwrap();
    assert!(!row.reproduced, "an empty suite proves nothing");
}

#[test]
fn acceptance_all_five_classes_reproduced() {
    // REQ-AC8-011..017 / AC-UAT-016: a representative fixture that exhibits all
    // five historical classes is fully reproduced by the native analysis.
    // Build a fixture with: duplicate authority, live forbidden edge,
    // missing owner, projection conflict, and a fully-detected mutation suite.
    use crate::architecture_graph::{
        ArchitectureOverlayRelation, ArchitectureOverlayRelationKind, OverlayNodeRef,
    };
    let contracts = vec![
        single_authority("dup-a", "comp:auth"),
        single_authority("dup-b", "comp:auth"),
        single_authority("orphan", "comp:absent"),
        single_authority("owner", "comp:ledger"),
        ArchitecturalContract::declare_projection_only(
            cid("proj"),
            "comp:ledger".to_string(),
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").unwrap(),
            EventTime(T0),
        )
        .unwrap(),
        ArchitecturalContract::declare_forbidden_dependency(
            cid("no-edge"),
            ComponentRef::new("comp:domain").unwrap(),
            ComponentRef::new("comp:provider").unwrap(),
            "forbidden".into(),
            DecisionRef::Decision("d".into()),
            SpecRef::Spec("s".into()),
            Revision::new("r").unwrap(),
            EventTime(T0),
        )
        .unwrap(),
    ];
    let mut overlay = overlay_with(&["comp:auth", "comp:ledger", "comp:domain", "comp:provider"]);
    overlay.add_relation(&ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:domain")),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:provider")),
        kind: ArchitectureOverlayRelationKind::DependsOn,
        evidence: vec![],
    });

    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    let mutations = run_critical_mutations(&MutationSandbox::from_sources([
        ("crates/sddk-engine/src/domain/d.rs", "// c\n"),
        ("crates/sddk-engine/src/alignment/a.rs", "// c\n"),
        ("crates/sddk-engine/src/workbook/w.rs", "// c\n"),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "pub fn append_canonical(e: u64) -> Result<(), ()> { let _ = e; Ok(()) }\n",
        ),
    ]))
    .unwrap();

    let cov = evaluate_class_coverage(&audit, &mutations);
    assert_eq!(cov.len(), 5);
    for c in &cov {
        assert!(
            c.reproduced,
            "class {:?} not reproduced; cov={cov:?}",
            c.class
        );
    }

    // And the composed receipt reports full coverage.
    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &empty_evidence(),
            contradiction_witnesses: &[],
            contract_filter: None,
        },
        EventTime(T0 + 1),
        &[],
    )
    .unwrap();
    let r = compose_receipt(
        ReceiptInputs {
            revision: "d1ed1d0".into(),
            knowledge_basis: "kb".into(),
            delta: &delta,
            audit: &audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &[],
            provider_basis: &[],
            change_basis: None,
            contract_filter: None,
        },
        EventTime(T0 + 2),
    );
    assert!(r.all_classes_reproduced(), "{:?}", r.unreproduced_classes());
    assert_eq!(
        r.verdict,
        ReceiptVerdict::Blocked,
        "unwaived MUST findings block"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — Base mode + providers (REQ-AC8-018..019)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_base_mode_is_provider_free() {
    // REQ-AC8-018
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    assert!(r.provider_basis.is_empty(), "Base mode has no providers");
}

#[test]
fn acceptance_providers_do_not_change_verdict() {
    // REQ-AC8-019: providers are additive evidence; the verdict rules are the
    // same with and without them.
    let f = fixture();
    let base = compose(&f, &[], &[], EventTime(T0 + 2));
    let with_providers = compose(
        &f,
        &[],
        &["cognicode:call-graph@v1".into(), "chronos:trace@v2".into()],
        EventTime(T0 + 2),
    );
    assert_eq!(base.verdict, with_providers.verdict);
    assert_eq!(with_providers.provider_basis.len(), 2);
    assert_eq!(base.verdict, ReceiptVerdict::Blocked);
    // Waiving the same findings yields the same verdict in both modes.
    let waivers = vec![
        "waiver:shadow_authority".to_string(),
        "waiver:missing_owner".to_string(),
        "waiver:contradiction".to_string(),
        "waiver:authority_bypass".to_string(),
    ];
    let base_w = compose(&f, &waivers, &[], EventTime(T0 + 2));
    let prov_w = compose(&f, &waivers, &["cognicode:x".into()], EventTime(T0 + 2));
    assert_eq!(base_w.verdict, prov_w.verdict);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — determinism + rebuild (REQ-AC8-021..022)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_receipt_is_deterministic() {
    // REQ-AC8-021
    let f = fixture();
    let a = compose(&f, &[], &[], EventTime(T0 + 2));
    let b = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(a, b);
    assert_eq!(a.id, b.id);
}

#[test]
fn acceptance_rebuild_preserves_findings() {
    // REQ-AC8-022 / AC-041-004: rebuilding the graph from the same units yields
    // an equivalent audit + receipt.
    let f = fixture();
    let rebuilt = overlay_with(&["comp:auth", "comp:auth-adjacent"]);
    let audit2 = _audit(&rebuilt, &f.contracts, EventTime(T0 + 1)).unwrap();
    assert_eq!(audit2.graph_digest, f.audit.graph_digest);
    assert_eq!(audit2.findings, f.audit.findings);

    let r1 = compose(&f, &[], &[], EventTime(T0 + 2));
    let f2 = Fixture {
        overlay: rebuilt,
        contracts: f.contracts.clone(),
        delta: f.delta.clone(),
        audit: audit2,
        mutations: f.mutations.clone(),
    };
    let r2 = compose(&f2, &[], &[], EventTime(T0 + 2));
    assert_eq!(r1.id, r2.id);
    assert_eq!(r1.class_coverage, r2.class_coverage);
    assert_eq!(r1.verdict, r2.verdict);
}

#[test]
fn acceptance_receipt_carries_change_basis() {
    // REQ-A3S12-005 (engine side): a supplied basis is copied through; absence
    // stays absence.
    let f = fixture();
    let global = compose(&f, &[], &[], EventTime(T0 + 2));
    assert!(global.change_basis.is_none());

    let basis = ChangeBasis {
        base: "origin/main".to_string(),
        changed_units: vec!["comp:auth".to_string()],
    };
    let lenses: Vec<crate::paradigm_lens::LensEvaluation> = vec![];
    let scoped = compose_receipt(
        ReceiptInputs {
            revision: "rev".into(),
            knowledge_basis: "kb".into(),
            delta: &f.delta,
            audit: &f.audit,
            mutations: &f.mutations,
            lenses: &lenses,
            waivers: &[],
            provider_basis: &[],
            change_basis: Some(&basis),
            contract_filter: None,
        },
        EventTime(T0 + 2),
    );
    let got = scoped.change_basis.expect("basis carried");
    assert_eq!(got.base, "origin/main");
    assert_eq!(got.changed_units, vec!["comp:auth".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment (REQ-AC8-020, 023, 024)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_provider_imports() {
    // REQ-AC8-020
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("compose.rs", include_str!("compose.rs")),
        ("self_audit.rs", include_str!("self_audit.rs")),
    ];
    let forbidden = [
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::agent_host",
        "use crate::cognicode",
        "use crate::chronos",
        "use crate::authority_engine",
        "use crate::capability",
        "use crate::effective_instructions",
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
        "architecture_receipt imports forbidden surfaces: {offenders:?}"
    );
}

#[test]
fn anti_encroachment_read_only_and_no_score() {
    // REQ-AC8-023
    let sources: &[(&str, &str)] = &[
        ("types.rs", include_str!("types.rs")),
        ("compose.rs", include_str!("compose.rs")),
        ("self_audit.rs", include_str!("self_audit.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "ArchitectureClaim {",
            "ArchitectureClaim::",
            "use std::fs",
            "use std::io::Write",
            "std::fs::",
            "File::create(",
            "pub score",
            "pub weight",
            "fn score(",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain `{forbidden}`"
            );
        }
    }
    // Behavioural: composing does not mutate the overlay it consulted.
    let f = fixture();
    let before = f.overlay.digest();
    let _ = compose(&f, &[], &[], EventTime(T0 + 2));
    assert_eq!(before, f.overlay.digest());
}

#[test]
fn anti_encroachment_aggregator_only() {
    // REQ-AC8-024: the module re-derives nothing — it contains no observation,
    // probe or detector logic.
    let sources: &[(&str, &str)] = &[
        ("compose.rs", include_str!("compose.rs")),
        ("self_audit.rs", include_str!("self_audit.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "LensObservation::",
            "MutationInjection::",
            "GuardCheck::",
            ".find_contracts_for_unit(",
            "probe_oo_observations",
            "probe_adt_observations",
            "shadow_authority(",
            "authority_bypass(",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain detector logic `{forbidden}`"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// boundary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_statuses_cover_the_delta() {
    let f = fixture();
    let r = compose(&f, &[], &[], EventTime(T0 + 2));
    for (cid_, status) in r
        .claim_results
        .iter()
        .map(|c| (c.contract.clone(), c.status))
        .collect::<Vec<_>>()
    {
        // Every claim row is one of the closed AC4 statuses.
        assert!(DeltaContractStatus::ALL.contains(&status));
        let _ = cid_;
    }
    // Lens rows use AC3's closed status vocabulary.
    for l in &r.lens_results {
        assert!(LensStatus::ALL.contains(&l.status));
    }
}
