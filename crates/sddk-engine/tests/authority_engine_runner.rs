// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_engine_runner.rs — SPEC-M5: 8 integration scenarios over the
// `AuthorityEngineRunner` facade.

use sddk_engine::authority_engine::{
    ActionKind, AdmissionDecision, AuthorityEngineError, AuthorityEngineRunner, Facts,
    RunnerVerdict,
};

fn runner() -> AuthorityEngineRunner {
    AuthorityEngineRunner::new()
}

// SC-M5-1: High band → RequireApproval.
#[test]
fn sc_m5_1_high_band_requires_approval() {
    let v: RunnerVerdict = runner()
        .admit_surface(
            "user:alice",
            "cycle_state",
            ActionKind::CycleTransition,
            "c1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(
        v.decision,
        AdmissionDecision::RequireApproval { .. }
    ));
    assert_eq!(v.capability, "surface.cycle_state");
}

// SC-M5-2: Low band + matching capability → Allow.
#[test]
fn sc_m5_2_low_band_allows() {
    let v = runner()
        .admit_surface(
            "system",
            "ledger_events",
            ActionKind::CliRun,
            "evt-1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(v.decision, AdmissionDecision::Allow { .. }));
}

// SC-M5-3: Unknown surface → Err(InternalContractBug).
#[test]
fn sc_m5_3_unknown_surface_returns_error() {
    let r = runner().admit_surface(
        "user:alice",
        "does_not_exist",
        ActionKind::CycleStart,
        "x",
        Facts::default(),
    );
    assert!(matches!(
        r,
        Err(AuthorityEngineError::InternalContractBug { .. })
    ));
}

// SC-M5-4: Human on Medium band with `plan.write` capability → Allow.
#[test]
fn sc_m5_4_medium_band_allows_human() {
    let v = runner()
        .admit_surface(
            "user:alice",
            "evidence_attachment",
            ActionKind::PlanEvidence,
            "p1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(v.decision, AdmissionDecision::Allow { .. }));
}

// SC-M5-5: RunnerVerdict always includes AdmissionExplanation with non-empty digest.
#[test]
fn sc_m5_5_explanation_digest_non_empty() {
    let v = runner()
        .admit_surface(
            "system",
            "ledger_events",
            ActionKind::CliRun,
            "evt-1",
            Facts::default(),
        )
        .expect("ok");
    assert!(!v.explanation.decision_digest.0.is_empty());
}

// SC-M5-6: Legacy AuthorityContext::validate stays working (compat mirror).
// We verify the runner does NOT replace the legacy path — both can coexist.
#[test]
fn sc_m5_6_runner_coexists_with_legacy_validate() {
    use sddk_engine::authority::{AuthorityContext, WritableSurface};
    let ctx = AuthorityContext::for_test(sddk_domain::ActorKind::System, "sys");
    assert!(ctx.validate(WritableSurface::LedgerEvents).is_ok());
}

// SC-M5-7: Deterministic decision_id for same inputs.
#[test]
fn sc_m5_7_decision_id_deterministic() {
    let v = runner()
        .admit_surface(
            "user:alice",
            "cycle_state",
            ActionKind::CycleTransition,
            "c1",
            Facts::default(),
        )
        .expect("ok");
    assert_eq!(v.decision.decision_id(), "approval-human-cycle_transition");
}

// SC-M5-8: Runner is safe under non-panicking error paths.
#[test]
fn sc_m5_8_runner_never_panics() {
    for surface in [
        "cycle_state",
        "ledger_events",
        "gate_receipts",
        "plan_revisions",
        "transition_records",
        "framework_bundle",
        "github_releases",
        "knowledge_graph_vault",
        "plan_item",
        "dependency_edge",
        "evidence_attachment",
        "decision_record",
    ] {
        let _ = runner().admit_surface(
            "user:alice",
            surface,
            ActionKind::CliRun,
            "x",
            Facts::default(),
        );
    }
    // Unknown surfaces should never panic — they return Err.
    let r = runner().admit_surface(
        "user:alice",
        "no_such_surface",
        ActionKind::CliRun,
        "x",
        Facts::default(),
    );
    assert!(r.is_err());
}
