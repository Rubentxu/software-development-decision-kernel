// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// ac5_not_verify_full.rs — A3-S7 exit-criterion integration test.
//
// AC5's exit criterion (AC-034-002) is that DebVerify is NOT `verify --full`.
// Operationally: DebVerify is global and finds incoherence even when AC4's
// change-scoped delta is empty.
//
// This test exercises both modules through their public APIs (no mocks) and
// asserts the contrast.

use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::architecture_conformance::{
    ConformanceInputs, ContractEvidence, compute_conformance_delta,
};
use sddk_engine::architecture_debverify::{
    DebVerifyFindingKind, FindingSeverity, run_debverify_audit,
};
use sddk_engine::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use sddk_engine::knowledge::EventTime;

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

#[test]
fn debverify_is_global_while_conformance_delta_is_change_scoped() {
    // Two contracts claim authority over the same component (the incoherence),
    // and a unit exists so AC4 *could* see them if a change were declared.
    let unit_ref = SoftwareUnitRef::new("unit:comp:auth");
    let mut overlay = ArchitectureGraphOverlay::new();
    overlay.add_unit(&SoftwareUnit::new(
        unit_ref.clone(),
        UnitKind::Module,
        "unit:comp:auth".to_string(),
    ));

    let shadow_a = single_authority("shadow-a", "comp:auth");
    let shadow_b = single_authority("shadow-b", "comp:auth");
    let contracts = vec![shadow_a.clone(), shadow_b.clone()];

    // ── AC4 (change-scoped) with an EMPTY change basis ──────────────────────
    let evidence: ContractEvidence = std::collections::BTreeMap::new();
    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &evidence,
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 1),
        &[], // no changed units
    )
    .expect("delta");
    assert!(
        delta.affected.is_empty(),
        "AC4 with an empty change basis affects nothing"
    );
    assert!(delta.claims.is_empty());
    assert!(delta.contradictions().is_empty());

    // ── AC5 (global) with no change basis at all ────────────────────────────
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert!(
        !audit.is_clean(),
        "AC-UAT-009: DebVerify must find the global duplicate authority"
    );

    let shadows = audit.by_kind(DebVerifyFindingKind::ShadowAuthority);
    assert_eq!(shadows.len(), 1);
    assert_eq!(shadows[0].severity, FindingSeverity::Critical);
    assert_eq!(shadows[0].subjects, vec!["comp:auth"]);
    assert_eq!(
        shadows[0].contract_ids,
        vec![cid("shadow-a"), cid("shadow-b")]
    );

    // ── The contrast is the exit criterion ──────────────────────────────────
    // Same overlay, same contracts, same `now`:
    //   AC4 sees nothing (no change); AC5 reports Critical incoherence.
    assert_eq!(delta.affected.len(), 0);
    assert_eq!(
        audit.counts().get(&DebVerifyFindingKind::ShadowAuthority),
        Some(&1)
    );
}

#[test]
fn debverify_audit_is_global_and_delta_independent() {
    // Reordering or partitioning the change basis cannot influence the audit:
    // the audit does not take one.
    let overlay = ArchitectureGraphOverlay::new();
    let contracts = vec![
        single_authority("a", "comp:x"),
        single_authority("b", "comp:x"),
    ];
    let first = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    let second = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert_eq!(first, second);
    assert_eq!(first.digest, second.digest);
    // And the audit is unaffected by a "change" that AC4 would consume.
    let _ = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: &contracts,
            evidence: &ContractEvidence::new(),
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 1),
        &[SoftwareUnitRef::new("unit:comp:x")],
    )
    .expect("delta");
    let third = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert_eq!(first, third, "AC4 activity must not change the AC5 audit");
}
