// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// ac6_ac4_witness_bridge.rs — A3-S6 cross-module integration test.
//
// Closes the loop AC4 left open: AC6 mutation probes produce contract
// witnesses, and AC4 consumes them as `contradiction_witnesses`.
//
// This exercises the real public API of both modules (no mocks): an
// ArchitectureGraphOverlay, an ArchitecturalContract, the mutation sandbox,
// and `compute_conformance_delta`.

use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractEvaluation, DecisionRef, EvaluatorRef,
    EvidenceRef, Revision, SpecRef,
};
use sddk_engine::architecture_conformance::{
    ConformanceInputs, DeltaContractStatus, compute_conformance_delta,
};
use sddk_engine::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use sddk_engine::architecture_mutation::{MutationKind, MutationSandbox, run_critical_mutations};
use sddk_engine::knowledge::EventTime;

const T0: i64 = 1_700_000_000;

/// A minimal representative sandbox (one clean file per guarded subtree).
fn seed_sandbox() -> MutationSandbox {
    MutationSandbox::from_sources([
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
            "// workbook view (projection only)\npub struct PlanView;\n",
        ),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "// canonical fact log\npub fn append_canonical(evt: u64) -> Result<(), ()> { let _ = evt; Ok(()) }\n",
        ),
    ])
}

#[test]
fn ac6_witnesses_drive_ac4_contradiction() {
    // 1. AC6: run the four critical mutations; every one is detected.
    let sandbox = seed_sandbox();
    let receipt = run_critical_mutations(&sandbox).expect("mutation suite");
    assert!(
        receipt.all_detected,
        "undetected: {:?}",
        receipt.undetected()
    );

    // The provider-boundary witness is the one AC-UAT-010 names.
    let provider_contract = receipt
        .probes
        .iter()
        .find(|p| p.kind == MutationKind::ProviderTypeLeak)
        .and_then(|p| p.expected_contract().cloned())
        .expect("provider-boundary witness");
    let witnesses = receipt.witnesses();
    assert!(
        witnesses.contains(&provider_contract),
        "witnesses must include the provider boundary contract"
    );

    // 2. AC4: build an overlay where that contract is affected by a changed unit.
    let contract_id = provider_contract.clone();
    let contract = ArchitecturalContract::declare_single_authority(
        contract_id.clone(),
        ComponentRef::new("comp:domain").expect("component"),
        DecisionRef::Decision("decision:ac6".into()),
        SpecRef::Spec("spec:ac6".into()),
        Revision::new("rev:ac6").expect("revision"),
        EventTime(T0),
    )
    .expect("declare contract");

    let unit_ref = SoftwareUnitRef::new("unit:domain-order");
    let mut overlay = ArchitectureGraphOverlay::new();
    // A Verified claim makes the contract discoverable from the changed unit.
    let claim = ContractEvaluation::evaluate(
        &contract,
        vec![EvidenceRef::new("static", "obs:1").expect("evidence")],
        EventTime(T0 + 1),
        EvaluatorRef::new("test:ac6").expect("evaluator"),
        None,
    );
    let unit = SoftwareUnit::new(
        unit_ref.clone(),
        UnitKind::Module,
        "unit:domain-order".to_string(),
    );
    overlay.add_unit(&unit);
    let claim_id = overlay.add_claim(&claim);
    overlay.attach_claim_to_unit(&claim, &claim_id, &unit_ref);
    overlay.add_contract_metadata(
        &contract,
        &DecisionRef::Decision("decision:ac6".into()),
        &SpecRef::Spec("spec:ac6".into()),
        &[],
    );

    // 3. Feed AC6's witnesses into AC4 as contradiction witnesses.
    let evidence: sddk_engine::architecture_conformance::ContractEvidence =
        std::collections::BTreeMap::new();
    let delta = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &evidence,
            contradiction_witnesses: &witnesses,
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&unit_ref),
    )
    .expect("compute delta");

    // 4. The loop is closed: the witnessed contract is Contradicted.
    assert_eq!(
        delta.status_of(&contract_id),
        Some(DeltaContractStatus::Contradicted),
        "an AC6 mutation witness must drive the AC4 contract to Contradicted"
    );
    assert_eq!(delta.contradictions(), std::slice::from_ref(&contract_id));
    // And it is not also evaluated into a claim.
    assert!(delta.claims.is_empty());

    // 5. Control: without the witnesses, the same contract is Verified
    //    (non-empty evidence), so the contradiction is solely AC6's doing.
    let delta_no_witness = compute_conformance_delta(
        &overlay,
        ConformanceInputs {
            contracts: std::slice::from_ref(&contract),
            evidence: &{
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    contract_id.clone(),
                    vec![EvidenceRef::new("static", "obs:1").expect("evidence")],
                );
                m
            },
            contradiction_witnesses: &[],
        },
        EventTime(T0 + 2),
        std::slice::from_ref(&unit_ref),
    )
    .expect("compute delta");
    assert_eq!(
        delta_no_witness.status_of(&contract_id),
        Some(DeltaContractStatus::Verified)
    );
}

#[test]
fn ac6_undetected_mutation_yields_no_witness() {
    // A mutation whose guard is scoped away is not detected and contributes no
    // witness, so AC4 would see no contradiction from it.
    let sandbox = seed_sandbox();
    let mut specs = sddk_engine::architecture_mutation::critical_mutations();
    for s in &mut specs {
        // Move every target outside the guards' scope.
        s.target_path = format!("outside/{}", s.target_path.rsplit('/').next().unwrap());
    }
    let receipt = sddk_engine::architecture_mutation::run_mutation_suite(
        &sandbox,
        &specs,
        &sddk_engine::architecture_mutation::critical_guards(),
    )
    .expect("suite");
    assert!(!receipt.all_detected);
    assert!(
        receipt.witnesses().is_empty(),
        "undetected probes must not produce witnesses"
    );
}
